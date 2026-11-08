// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! A pure verifier. It takes bytes and returns a typed refusal.
//!
//! Success is not implemented. Algorithm 3 steps 3–8 of ePrint 2026/443
//! are specified in `docs/VERIFIER_SPEC.md` and are not this crate's accept
//! condition. `Relation` is what a signature gets once the public checks
//! have passed, so a missing isogeny equality cannot be mistaken for a
//! valid authorization. The one-dimensional walk is not one of those checks.

#![forbid(unsafe_code)]

mod prime;

use core::cmp::Ordering;
use prism_field::{Field, Level, Level1, Level3, Level5};
use sha3::digest::core_api::CoreWrapper;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::{CShake256, CShake256Core};

pub use prime::{degree_is_prime, is_baillie_psw};

/// Customization string for the challenge hash. One context, versioned.
/// Nothing has consumed `-v1`.
pub const CHALLENGE_DOMAIN: &[u8] = b"shekyl-prism-challenge-v1";

/// Why a signature was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyError {
    /// The byte string is not this level's length.
    Length,
    /// A field element is not the canonical representative in `0..p`.
    Canonical,
    /// `A = 0`, the starting curve. Its endomorphism ring is public.
    Origin,
    /// `A^2 = 4`. The Montgomery curve is singular.
    Singular,
    /// No basis index in `1..=127` meets the C search. Key generation
    /// regenerates such a key.
    Basis,
    /// `y(P)` is zero or not strictly below `y(−P)` in `<_{F_{p^2}}`.
    Sign,
    /// `2^{2e−1} − s_j^2` is not a Baillie–PSW prime.
    Composite,
    /// Algorithm 3 steps 3–8 are not yet an accept condition.
    Relation,
}

/// Run the public checks, then refuse with [`VerifyError::Relation`].
///
/// Order: length, canonical decoding, the starting-curve refusal, the
/// singular test, the deterministic basis index, the sign test, then one
/// Baillie–PSW test of `q_j`. The isogeny chain is not run.
pub fn verify<L: Level>(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    checked::<L>(public_key, message, signature)?;
    Err(VerifyError::Relation)
}

fn checked<L: Level>(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    if public_key.len() != L::PUBLIC_KEY_LEN || signature.len() != L::SIGNATURE_LEN {
        return Err(VerifyError::Length);
    }
    let identity = public_key;
    let coefficient = L::ext_decode(identity).ok_or(VerifyError::Canonical)?;
    if coefficient.is_zero() {
        return Err(VerifyError::Origin);
    }
    if singular::<L>(coefficient) {
        return Err(VerifyError::Singular);
    }
    if c_basis_hint::<L>(coefficient).is_none() {
        return Err(VerifyError::Basis);
    }
    let y = decode_response_y::<L>(signature)?;
    if !canonical_sign::<L>(y) {
        return Err(VerifyError::Sign);
    }
    let j = u16::from_le_bytes([
        signature[signature.len() - 2],
        signature[signature.len() - 1],
    ]);
    let s = challenge_s::<L>(identity, message, j);
    if !degree_is_prime(L::E, &s) {
        return Err(VerifyError::Composite);
    }
    Ok(())
}

/// The smallest `j` in `0..=65535` whose challenge degree is prime.
///
/// This is the signer's search. The verifier does not call it, and it does
/// not check that a carried `j` is the smallest. Exhausting the range is a
/// signing error: there is no fallback degree.
pub fn smallest_challenge_index<L: Level>(identity: &[u8], message: &[u8]) -> Option<u16> {
    if identity.len() != L::FP2_LEN {
        return None;
    }
    for j in 0..=u16::MAX {
        let s = challenge_s::<L>(identity, message, j);
        if degree_is_prime(L::E, &s) {
            return Some(j);
        }
    }
    None
}

fn decode_response_y<L: Level>(signature: &[u8]) -> Result<L::Ext, VerifyError> {
    let n = L::FP2_LEN;
    // A_sig, P.x, P.y, Q.x, Q.y, then j.
    if signature.len() != 5 * n + L::INDEX_LEN {
        return Err(VerifyError::Length);
    }
    for i in 0..5 {
        if L::ext_decode(&signature[i * n..(i + 1) * n]).is_none() {
            return Err(VerifyError::Canonical);
        }
    }
    L::ext_decode(&signature[2 * n..3 * n]).ok_or(VerifyError::Canonical)
}

fn canonical_sign<L: Level>(y: L::Ext) -> bool {
    if y.is_zero() {
        return false;
    }
    ext_less::<L>(y, y.negate())
}

/// `<_{F_{p^2}}`: the real part as an integer in `[0, p)`, then the imaginary part.
fn ext_less<L: Level>(left: L::Ext, right: L::Ext) -> bool {
    let (left_re, left_im) = L::ext_parts(left);
    let (right_re, right_im) = L::ext_parts(right);
    match int_cmp(&L::base_encode(left_re), &L::base_encode(right_re)) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => {
            int_cmp(&L::base_encode(left_im), &L::base_encode(right_im)) == Ordering::Less
        }
    }
}

fn int_cmp(left: &[u8], right: &[u8]) -> Ordering {
    for (l, r) in left.iter().rev().zip(right.iter().rev()) {
        if l != r {
            return l.cmp(r);
        }
    }
    Ordering::Equal
}

fn singular<L: Level>(coefficient: L::Ext) -> bool {
    coefficient.mul(coefficient).same(L::Ext::from_u64(4))
}

/// The byte `ec_curve_to_basis_2f_to_hint` returns: `(index << 1) | hint_A`.
///
/// Square `A`: the first `b` in `1..=127` for which `1 + b^2` is a non-square
/// in `F_p` and `x(P) = −A/(1 + i·b)` is a non-square on the curve.
/// Non-square `A`: the first `n` in `1..=127` for which `x(P) = n·A` is on
/// the curve. That is the C search order. No index in the range is a
/// fallback; absence is [`VerifyError::Basis`], and key generation
/// regenerates the key. `A = 0` is not handled here.
pub fn c_basis_hint<L: Level>(coefficient: L::Ext) -> Option<u8> {
    if coefficient.is_square() {
        let mut n = 1u32;
        while n <= 127 {
            let b = n;
            let one_plus = L::Base::from_u64(u64::from(b) * u64::from(b) + 1);
            n += 1;
            if one_plus.is_square() {
                continue;
            }
            let z = L::Ext::one().add(L::ext_i().mul(L::Ext::from_u64(u64::from(b))));
            let Some(inv) = z.inv() else {
                continue;
            };
            let x = coefficient.mul(inv).negate();
            if !x.is_square() && montgomery_rhs::<L>(coefficient, x).is_square() {
                return Some((b as u8) << 1 | 1);
            }
        }
        None
    } else {
        let mut n = 1u32;
        let mut x = coefficient;
        while n < 128 {
            if montgomery_rhs::<L>(coefficient, x).is_square() {
                return Some((n as u8) << 1);
            }
            x = x.add(coefficient);
            n += 1;
        }
        None
    }
}

fn montgomery_rhs<L: Level>(coefficient: L::Ext, x: L::Ext) -> L::Ext {
    // x^3 + A x^2 + x.
    let x2 = x.mul(x);
    x2.mul(x).add(coefficient.mul(x2)).add(x)
}

fn challenge_s<L: Level>(identity: &[u8], message: &[u8], j: u16) -> Vec<u8> {
    let mut hasher: CShake256 = CoreWrapper::from_core(CShake256Core::new(CHALLENGE_DOMAIN));
    hasher.update(identity);
    hasher.update(message);
    hasher.update(&j.to_le_bytes());
    let mut reader = hasher.finalize_xof();
    let mut s = vec![0u8; L::S_LEN];
    reader.read(&mut s);
    if let Some(low) = s.first_mut() {
        *low |= 1;
    }
    let keep = ((L::E - 1) % 8) as u8;
    if keep != 0 {
        if let Some(high) = s.last_mut() {
            *high &= (1u8 << keep) - 1;
        }
    }
    s
}

/// NIST levels the bench and the tests can name.
pub fn verify_level(
    level: &str,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    match level {
        "1" | "I" => verify::<Level1>(public_key, message, signature),
        "3" | "III" => verify::<Level3>(public_key, message, signature),
        "5" | "V" => verify::<Level5>(public_key, message, signature),
        _ => Err(VerifyError::Length),
    }
}

/// A public key, message, and signature whose challenge degree is prime.
///
/// The index search is the signer's. A timing run builds this before the
/// clock starts, then times [`verify_level`].
pub struct Prepared {
    pub name: &'static str,
    pub public_key: Vec<u8>,
    pub message: &'static [u8],
    pub signature: Vec<u8>,
}

/// Build [`Prepared`] for a NIST level name.
pub fn prepared(level: &str) -> Result<Prepared, VerifyError> {
    let (name, sample) = match level {
        "1" | "I" => (Level1::NAME, sample::<Level1>()),
        "3" | "III" => (Level3::NAME, sample::<Level3>()),
        "5" | "V" => (Level5::NAME, sample::<Level5>()),
        _ => return Err(VerifyError::Length),
    };
    let (public_key, message, signature) = sample.ok_or(VerifyError::Composite)?;
    Ok(Prepared {
        name,
        public_key,
        message,
        signature,
    })
}

fn sample<L: Level>() -> Option<(Vec<u8>, &'static [u8], Vec<u8>)> {
    let public_key = reference_public_key(L::NAME)?.to_vec();
    let message = b"shekyl-prism";
    let j = smallest_challenge_index::<L>(&public_key, message)?;
    let mut signature = vec![0u8; L::SIGNATURE_LEN];
    let one = L::ext_encode(L::Ext::one());
    let at = 2 * L::FP2_LEN;
    signature[at..at + one.len()].copy_from_slice(&one);
    let end = signature.len();
    signature[end - 2..].copy_from_slice(&j.to_le_bytes());
    Some((public_key, message, signature))
}

/// `A` from the C tree's key generation, one row per NIST level.
///
/// Lines are `<level-name> <hex of A> <hint byte>`. The hint is what
/// `ec_curve_to_basis_2f_to_hint` returned. It is not part of the public key.
const C_KEYS: &str = include_str!("../testdata/c_basis_hints.txt");

struct CKey {
    name: &'static str,
    key: Vec<u8>,
    #[cfg_attr(not(test), allow(dead_code))]
    hint: u8,
}

fn c_keys() -> &'static [CKey] {
    use std::sync::OnceLock;
    static KEYS: OnceLock<Vec<CKey>> = OnceLock::new();
    KEYS.get_or_init(|| {
        C_KEYS
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut parts = line.split_whitespace();
                let name = match parts.next().expect("level") {
                    "NIST-I" => Level1::NAME,
                    "NIST-III" => Level3::NAME,
                    "NIST-V" => Level5::NAME,
                    other => panic!("unknown level {other}"),
                };
                let key = decode_hex(parts.next().expect("key"));
                let hint: u8 = parts.next().expect("hint").parse().expect("hint byte");
                CKey { name, key, hint }
            })
            .collect()
    })
}

fn reference_public_key(name: &str) -> Option<&'static [u8]> {
    c_keys()
        .iter()
        .find(|row| row.name == name)
        .map(|row| row.key.as_slice())
}

#[cfg(test)]
fn reference_hint(name: &str) -> Option<u8> {
    c_keys()
        .iter()
        .find(|row| row.name == name)
        .map(|row| row.hint)
}

fn decode_hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{Level, Level1, Level3, Level5};

    type Ext1 = <Level1 as Level>::Ext;

    fn le(n: u32) -> Vec<u8> {
        let bytes = n.to_le_bytes();
        let len = bytes.iter().rposition(|b| *b != 0).unwrap() + 1;
        bytes[..len].to_vec()
    }

    #[test]
    fn two_thousand_forty_seven_is_not_a_prime() {
        assert!(!is_baillie_psw(&le(2047)));
        assert!(is_baillie_psw(&le(1019)));
        assert!(!is_baillie_psw(&[1]));
        assert!(!is_baillie_psw(&le(91)));
        assert!(!is_baillie_psw(&le(561)));
    }

    #[test]
    fn the_three_field_primes_pass_baillie_psw() {
        assert!(is_baillie_psw(&prime_bytes(5, 248)));
        assert!(is_baillie_psw(&prime_bytes(65, 376)));
        assert!(is_baillie_psw(&prime_bytes(27, 500)));
    }

    fn prime_bytes(cofactor: u128, shift: u32) -> Vec<u8> {
        let mut value = [0u8; 16];
        value[..16].copy_from_slice(&cofactor.to_le_bytes());
        let byte = (shift as usize) / 8;
        let bit = (shift % 8) as u8;
        let mut shifted = vec![0u8; value.len() + byte + 1];
        for (i, b) in value.iter().enumerate() {
            let low = u16::from(*b) << bit;
            shifted[i + byte] |= low as u8;
            if i + byte + 1 < shifted.len() {
                shifted[i + byte + 1] |= (low >> 8) as u8;
            }
        }
        let mut borrow = 1u8;
        for b in &mut shifted {
            let v = i16::from(*b) - i16::from(borrow);
            if v < 0 {
                *b = (v + 256) as u8;
                borrow = 1;
            } else {
                *b = v as u8;
                borrow = 0;
            }
        }
        while shifted.last() == Some(&0) {
            shifted.pop();
        }
        shifted
    }

    #[test]
    fn a_short_signature_is_a_length_refusal() {
        let err = verify::<Level1>(&[0u8; Level1::PUBLIC_KEY_LEN], b"m", &[0u8; 4]);
        assert_eq!(err, Err(VerifyError::Length));
    }

    #[test]
    fn salt_prism_lengths_are_not_this_encoding() {
        for n in [260usize, 388, 516] {
            assert_ne!(n, Level1::SIGNATURE_LEN);
            assert_ne!(n, Level3::SIGNATURE_LEN);
            assert_ne!(n, Level5::SIGNATURE_LEN);
        }
        assert_eq!(Level1::SIGNATURE_LEN, 322);
        assert_eq!(Level3::SIGNATURE_LEN, 482);
        assert_eq!(Level5::SIGNATURE_LEN, 642);
        assert_eq!(Level3::CHAIN_STEPS, Level3::E);
    }

    #[test]
    fn the_starting_curve_is_refused() {
        let pk = vec![0u8; Level1::PUBLIC_KEY_LEN];
        let sig = vec![0u8; Level1::SIGNATURE_LEN];
        assert_eq!(
            verify::<Level1>(&pk, b"shekyl-prism", &sig),
            Err(VerifyError::Origin)
        );
    }

    #[test]
    fn zero_y_is_refused() {
        let pk = Level1::ext_encode(Ext1::from_u64(3));
        assert_eq!(pk.len(), Level1::PUBLIC_KEY_LEN);
        let sig = vec![0u8; Level1::SIGNATURE_LEN];
        assert_eq!(
            verify::<Level1>(&pk, b"shekyl-prism", &sig),
            Err(VerifyError::Sign)
        );
    }

    #[test]
    fn the_sign_order_is_the_integer_order() {
        let one = Ext1::one();
        assert!(ext_less::<Level1>(one, one.negate()));
        assert!(!ext_less::<Level1>(one.negate(), one));
        let i = Level1::ext_i();
        assert!(ext_less::<Level1>(i, i.negate()));
        assert!(!canonical_sign::<Level1>(Ext1::zero()));
        assert!(canonical_sign::<Level1>(one));
        assert!(!canonical_sign::<Level1>(one.negate()));
    }

    #[test]
    fn the_derived_index_equals_the_c_hint() {
        for name in [Level1::NAME, Level3::NAME, Level5::NAME] {
            let key = reference_public_key(name).expect(name);
            let hint = reference_hint(name).expect(name);
            let coefficient = match name {
                "NIST-I" => Level1::ext_decode(key).map(c_basis_hint::<Level1>),
                "NIST-III" => Level3::ext_decode(key).map(c_basis_hint::<Level3>),
                "NIST-V" => Level5::ext_decode(key).map(c_basis_hint::<Level5>),
                _ => None,
            };
            assert_eq!(coefficient, Some(Some(hint)), "{name}");
        }
    }

    #[test]
    fn a_prime_index_reaches_relation_without_being_checked_for_smallest() {
        let message = b"shekyl-prism";
        let pk = reference_public_key(Level1::NAME)
            .expect("level I key")
            .to_vec();
        let j = smallest_challenge_index::<Level1>(&pk, message).expect("a prime degree");
        let mut sig = vec![0u8; Level1::SIGNATURE_LEN];
        let one = Level1::ext_encode(Ext1::one());
        let at = 2 * Level1::FP2_LEN;
        sig[at..at + one.len()].copy_from_slice(&one);
        let end = sig.len();
        sig[end - 2..].copy_from_slice(&j.to_le_bytes());
        assert_eq!(
            verify::<Level1>(&pk, message, &sig),
            Err(VerifyError::Relation)
        );
        if j != u16::MAX {
            let mut later = sig.clone();
            let end = later.len();
            later[end - 2..].copy_from_slice(&(j.wrapping_add(1)).to_le_bytes());
            let err = verify::<Level1>(&pk, message, &later);
            // A different index is a different degree. It is refused when
            // that degree is composite, and it is not refused for being larger.
            if degree_is_prime(Level1::E, &challenge_s::<Level1>(&pk, message, j + 1)) {
                assert_eq!(err, Err(VerifyError::Relation));
            } else {
                assert_eq!(err, Err(VerifyError::Composite));
            }
        }
    }
}
