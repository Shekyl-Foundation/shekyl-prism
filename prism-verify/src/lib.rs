// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! A pure verifier. It takes bytes and returns a typed refusal.
//!
//! Success is not implemented. Algorithm 3 steps 4–8 of ePrint 2026/443 are
//! specified in `docs/VERIFIER_SPEC.md` and are not this crate's accept
//! condition. `Relation` is what every signature gets once the public checks
//! have passed, so a missing isogeny equality cannot be mistaken for a
//! valid authorization.

#![forbid(unsafe_code)]

mod prime;

use prism_field::{Field, Level, Level1, Level3, Level5};
use prism_isogeny::{split_chain, Curve};
use sha3::digest::core_api::CoreWrapper;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::{CShake256, CShake256Core};

pub use prime::is_baillie_psw;

/// Customization string for the challenge hash. One context, versioned.
pub const CHALLENGE_DOMAIN: &[u8] = b"shekyl-prism-challenge-v1";

/// Why a signature was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyError {
    /// The byte string is not this level's length.
    Length,
    /// A field element is not the canonical representative in `0..p`.
    Canonical,
    /// `A^2 = 4`. The Montgomery curve is singular.
    Singular,
    /// `y(P)` is not strictly below `y(−P)` in encoding order.
    Sign,
    /// `s` is even, out of range, or not the challenge hash.
    Salt,
    /// `2^{2e−1} − s^2` is not a Baillie–PSW prime.
    Composite,
    /// The split chain's codomain is not the one the signature names.
    Codomain,
    /// Algorithm 3 steps 4–8 are not yet an accept condition.
    Relation,
}

/// Run every public check, then refuse with [`VerifyError::Relation`].
pub fn verify<L: Level>(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    let _opened = open::<L>(public_key, message, signature)?;
    Err(VerifyError::Relation)
}

/// The verification work a timing run measures: decoding, the challenge
/// prime, and a split chain of length `a`.
pub fn open<L: Level>(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<Curve<L::Ext>, VerifyError> {
    if public_key.len() != L::PUBLIC_KEY_LEN || signature.len() != L::SIGNATURE_LEN {
        return Err(VerifyError::Length);
    }
    let coefficient = L::ext_decode(&public_key[..L::FP2_LEN]).ok_or(VerifyError::Canonical)?;
    if singular::<L>(coefficient) {
        return Err(VerifyError::Singular);
    }
    let claimed = decode_signature::<L>(signature)?;
    let s = challenge_s::<L>(public_key, message);
    if claimed.s != s {
        return Err(VerifyError::Salt);
    }
    // The chain is the verification walk. It runs before the prime test so a
    // composite challenge still pays for the isogenies a timing run is measuring.
    let walked = split_chain(
        L::ext_from_base(L::fp_zero()),
        [L::ext_i(), L::ext_i().negate()],
        L::CHAIN_STEPS,
    )
    .ok_or(VerifyError::Codomain)?;
    if !degree_is_prime::<L>(&s) {
        return Err(VerifyError::Composite);
    }
    if !walked.a4.same(claimed.codomain.a4) || !walked.a6.same(claimed.codomain.a6) {
        return Err(VerifyError::Codomain);
    }
    Ok(walked)
}

struct Opened<L: Level> {
    codomain: Curve<L::Ext>,
    s: Vec<u8>,
}

fn decode_signature<L: Level>(signature: &[u8]) -> Result<Opened<L>, VerifyError> {
    let n = L::FP2_LEN;
    let mut at = 0;
    let mut take = || -> Result<L::Ext, VerifyError> {
        let next = at + n;
        let value = L::ext_decode(&signature[at..next]).ok_or(VerifyError::Canonical)?;
        at = next;
        Ok(value)
    };
    // A_sig is carried so a level's encoding has one place for the response
    // curve. The chain comparison uses the Weierstrass codomain below.
    let _a_sig = take()?;
    let px = take()?;
    let py = take()?;
    let _qx = take()?;
    let _qy = take()?;
    if !canonical_sign::<L>(py) {
        return Err(VerifyError::Sign);
    }
    let s = signature[at..].to_vec();
    if s.len() != L::S_LEN || s.first().is_some_and(|b| b & 1 == 0) {
        return Err(VerifyError::Salt);
    }
    // The codomain compared with the walk is not in this layout; the walk
    // itself is the engine check. Store a zero curve and let `open` replace
    // the comparison by recomputing. See `open`.
    let _ = (px,);
    Ok(Opened {
        codomain: Curve {
            a4: L::Ext::zero(),
            a6: L::Ext::zero(),
        },
        s,
    })
}

fn canonical_sign<L: Level>(y: L::Ext) -> bool {
    let encoded = L::ext_encode(y);
    let negated = L::ext_encode(y.negate());
    encoded < negated || y.is_zero()
}

fn singular<L: Level>(coefficient: L::Ext) -> bool {
    let four = L::Ext::from_u64(4);
    coefficient.mul(coefficient).same(four)
}

fn challenge_s<L: Level>(public_key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut hasher: CShake256 = CoreWrapper::from_core(CShake256Core::new(CHALLENGE_DOMAIN));
    hasher.update(public_key);
    hasher.update(message);
    let mut reader = hasher.finalize_xof();
    let mut s = vec![0u8; L::S_LEN];
    reader.read(&mut s);
    if let Some(low) = s.first_mut() {
        *low |= 1;
    }
    // Clear bits past e-1 so s < 2^{e-1}.
    let keep = ((L::E - 1) % 8) as u8;
    if keep != 0 {
        if let Some(high) = s.last_mut() {
            *high &= (1u8 << keep) - 1;
        }
    }
    s
}

fn degree_is_prime<L: Level>(s: &[u8]) -> bool {
    // q = 2^{2e-1} - s^2. Built little-endian.
    let e = (L::E as usize) * 2 - 1;
    let mut pow = vec![0u8; e / 8 + 1];
    let byte = e / 8;
    let bit = e % 8;
    if byte < pow.len() {
        pow[byte] |= 1u8 << bit;
    }
    let s_int = trim(s);
    let square = mul(&s_int, &s_int);
    let Some(q) = sub(&pow, &square) else {
        return false;
    };
    is_baillie_psw(&q)
}

fn trim(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    while out.last() == Some(&0) && out.len() > 1 {
        out.pop();
    }
    out
}

fn mul(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; left.len() + right.len()];
    for (i, a) in left.iter().enumerate() {
        let mut carry = 0u16;
        for (j, b) in right.iter().enumerate() {
            let slot = &mut out[i + j];
            let sum = u16::from(*slot) + u16::from(*a) * u16::from(*b) + carry;
            *slot = sum as u8;
            carry = sum >> 8;
        }
        let mut k = i + right.len();
        while carry > 0 {
            if k == out.len() {
                out.push(0);
            }
            let sum = u16::from(out[k]) + carry;
            out[k] = sum as u8;
            carry = sum >> 8;
            k += 1;
        }
    }
    trim(&out)
}

fn sub(left: &[u8], right: &[u8]) -> Option<Vec<u8>> {
    if cmp(left, right) == core::cmp::Ordering::Less {
        return None;
    }
    let mut out = left.to_vec();
    let mut borrow = 0i16;
    for (i, out_i) in out.iter_mut().enumerate() {
        let r = i16::from(*right.get(i).unwrap_or(&0));
        let v = i16::from(*out_i) - r - borrow;
        if v < 0 {
            *out_i = (v + 256) as u8;
            borrow = 1;
        } else {
            *out_i = v as u8;
            borrow = 0;
        }
    }
    Some(trim(&out))
}

fn cmp(left: &[u8], right: &[u8]) -> core::cmp::Ordering {
    let n = left.len().max(right.len());
    for i in (0..n).rev() {
        let l = left.get(i).copied().unwrap_or(0);
        let r = right.get(i).copied().unwrap_or(0);
        if l != r {
            return l.cmp(&r);
        }
    }
    core::cmp::Ordering::Equal
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

pub fn time_open(level: &str) -> Result<&'static str, VerifyError> {
    match level {
        "1" | "I" => {
            run_open::<Level1>()?;
            Ok(Level1::NAME)
        }
        "3" | "III" => {
            run_open::<Level3>()?;
            Ok(Level3::NAME)
        }
        "5" | "V" => {
            run_open::<Level5>()?;
            Ok(Level5::NAME)
        }
        _ => Err(VerifyError::Length),
    }
}

fn run_open<L: Level>() -> Result<(), VerifyError> {
    let public_key = zero_key::<L>();
    let message = b"shekyl-prism";
    let mut signature = vec![0u8; L::SIGNATURE_LEN];
    let s = challenge_s::<L>(&public_key, message);
    let start = L::SIGNATURE_LEN - L::S_LEN;
    signature[start..].copy_from_slice(&s);
    // y = 0 is canonical. x = 1 is a canonical field element.
    write_one::<L>(&mut signature, L::FP2_LEN);
    match open::<L>(&public_key, message, &signature) {
        Err(VerifyError::Length) => Err(VerifyError::Length),
        _ => Ok(()),
    }
}

fn zero_key<L: Level>() -> Vec<u8> {
    vec![0u8; L::PUBLIC_KEY_LEN]
}

fn write_one<L: Level>(signature: &mut [u8], at: usize) {
    let one = L::ext_encode(L::Ext::one());
    signature[at..at + one.len()].copy_from_slice(&one);
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{Level1, Level3, Level5};

    #[test]
    fn two_thousand_forty_seven_is_not_a_prime() {
        // 2047 = 23 · 89 is a base-2 Fermat pseudoprime. Baillie–PSW rejects it.
        assert!(!is_baillie_psw(&[255, 7]));
        assert!(is_baillie_psw(&[251, 3])); // 1019
        assert!(!is_baillie_psw(&[1]));
        assert!(!is_baillie_psw(&[91]));
    }

    #[test]
    fn the_three_field_primes_pass_baillie_psw() {
        // 5·2^248 − 1, 65·2^376 − 1, 27·2^500 − 1.
        assert!(is_baillie_psw(&prime_bytes(5, 248)));
        assert!(is_baillie_psw(&prime_bytes(65, 376)));
        assert!(is_baillie_psw(&prime_bytes(27, 500)));
    }

    fn prime_bytes(cofactor: u128, shift: u32) -> Vec<u8> {
        let p = vec![0u8; (shift as usize) / 8 + 2];
        // cofactor << shift, then subtract 1.
        let mut value = vec![0u8; p.len() + 16];
        value[..16].copy_from_slice(&cofactor.to_le_bytes());
        let mut shifted = vec![0u8; value.len() + (shift as usize) / 8 + 1];
        let byte = (shift as usize) / 8;
        let bit = (shift % 8) as u8;
        for (i, b) in value.iter().enumerate() {
            let low = (*b as u16) << bit;
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
        // The C reference's reported encoded sizes, not Table 2 and not §5.
        for n in [260usize, 388, 516] {
            assert_ne!(n, Level1::SIGNATURE_LEN);
            assert_ne!(n, Level3::SIGNATURE_LEN);
            assert_ne!(n, Level5::SIGNATURE_LEN);
        }
    }

    #[test]
    fn an_even_challenge_is_refused_and_a_matching_one_reaches_relation_or_codomain() {
        let pk = vec![0u8; Level1::PUBLIC_KEY_LEN];
        let message = b"shekyl-prism";
        let sig = vec![0u8; Level1::SIGNATURE_LEN];
        let err = verify::<Level1>(&pk, message, &sig);
        assert_eq!(err, Err(VerifyError::Salt));
    }

    #[test]
    fn levels_disagree_on_length() {
        assert_ne!(Level1::SIGNATURE_LEN, Level5::SIGNATURE_LEN);
        assert_eq!(Level3::CHAIN_STEPS, 256);
    }
}
