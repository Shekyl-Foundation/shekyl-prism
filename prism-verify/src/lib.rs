// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! A pure verifier. It takes bytes and returns a typed refusal.
//!
//! Algorithm 3 of ePrint 2026/443 is specified in `docs/VERIFIER_SPEC.md`.
//! The public checks run first. A signature that passes them is run through
//! one chain with no splitting step. Both theta readings existing is
//! [`VerifyError::Unidentified`]: the transformation between the two
//! structures on the codomain is not fixed, so nothing here returns success.
//!
//! Each public check owns a module: [`identity`] decodes the key,
//! [`signature`] decodes the response and normalizes its sign, [`basis`]
//! re-derives the C reference's basis index, [`challenge`] derives the
//! challenge scalar, and [`prime`] decides its degree. [`checked`] runs them
//! in the order [`VerifyError`] declares.

#![forbid(unsafe_code)]

pub mod basis;
pub mod challenge;
pub mod identity;
pub mod level;
mod prime;
pub mod reference;
mod relation;
pub mod signature;

use prism_field::Level;

pub use basis::{basis_hint, search_bound, BasisHint, C_HINT_MAX_INDEX};
pub use challenge::{smallest_challenge_index, ChallengeScalar, IndexError, CHALLENGE_DOMAIN};
pub use identity::KeyIdentity;
pub use level::{verify_level, Nist, UnknownLevel};
pub use prime::{degree_is_prime, is_baillie_psw};
pub use prism_field::ChallengeIndex;
pub use reference::{prepared, reference_key, Prepared, PreparedError, ReferenceKey, VectorError};
pub use signature::{AffinePoint, Response};

/// Why a signature was refused.
///
/// Variants are declared in the order [`checked`] runs the checks, cheapest
/// first, and the derived order is that order. An input that fails more than
/// one check is refused with the least of them, so an attacker sending junk
/// pays for the cheapest refusal rather than the most expensive one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VerifyError {
    /// The byte string is not this level's length.
    Length,
    /// A field element is not the canonical representative in `0..p`.
    Canonical,
    /// `A = 0`, the starting curve. Its endomorphism ring is public.
    Origin,
    /// `A^2 = 4`. The Montgomery curve is singular.
    Singular,
    /// `y(P)` is zero or not strictly below `y(−P)` in `<_{F_{p^2}}`.
    Sign,
    /// No basis index within [`search_bound`] meets the C search.
    ///
    /// The search is the reference's, including the indices its `start = 128`
    /// fallback covers, so this refuses only keys whose smallest index
    /// exceeds `2λ` — about `2^-λ` of keys, the level's own security target.
    /// `docs/VERIFIER_SPEC.md` records the bound and what it costs.
    Basis,
    /// `2^{2e−1} − s_j^2` is not a Baillie–PSW prime.
    Composite,
    /// The `(2,2)` chain stopped before both readings, or a signature point
    /// is not on its curve.
    Kernel,
    /// Both theta readings exist, and the transformation between their
    /// structures is not fixed. This is not acceptance.
    Unidentified,
}

impl core::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reason = match self {
            Self::Length => "not this level's length",
            Self::Canonical => "a field element is not canonical",
            Self::Origin => "the starting curve",
            Self::Singular => "a singular Montgomery curve",
            Self::Sign => "y(P) is not the normalized representative",
            Self::Basis => "no basis index within the search bound",
            Self::Composite => "the challenge degree is composite",
            Self::Kernel => "the isogeny chain stopped before both readings",
            Self::Unidentified => "the two readings on the codomain are not identified",
        };
        f.write_str(reason)
    }
}

impl std::error::Error for VerifyError {}

/// Run the public checks, then Algorithm 3.
pub fn verify<L: Level>(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    let (identity, response, scalar) = checked::<L>(public_key, message, signature)?;
    relation::relate(&identity, &response, &scalar)
}

/// The public checks, cheapest refusal first.
///
/// Both lengths come before either decode, because a wrong length is the
/// cheapest thing to refuse and `Length` must not depend on which field is
/// wrong. The sign test is a byte comparison on an element already decoded,
/// so it comes before the basis search, whose length an attacker chooses
/// with `A` — up to [`search_bound`] Legendre symbols. The challenge hash
/// and the one Baillie–PSW test are last: they cost more than everything
/// above them together.
fn checked<'a, L: Level>(
    public_key: &'a [u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(KeyIdentity<'a, L>, Response<L>, ChallengeScalar), VerifyError> {
    if public_key.len() != L::PUBLIC_KEY_LEN || signature.len() != L::SIGNATURE_LEN {
        return Err(VerifyError::Length);
    }
    let identity = KeyIdentity::<L>::decode(public_key)?;
    let response = Response::<L>::decode(signature)?;
    if !signature::canonical_sign::<L>(response.p.y) {
        return Err(VerifyError::Sign);
    }
    basis_hint::<L>(identity.coefficient())?;
    let scalar = ChallengeScalar::derive(&identity, message, response.index);
    if !scalar.degree_is_prime::<L>() {
        return Err(VerifyError::Composite);
    }
    Ok((identity, response, scalar))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{Field, Level, Level1, Level3, Level5};
    use signature::{canonical_sign, ext_less};

    type Ext1 = <Level1 as Level>::Ext;

    fn key<L: Level>(coefficient: u64) -> Vec<u8> {
        L::ext_encode(L::Ext::from_u64(coefficient))
    }

    #[test]
    fn the_refusal_order_is_cheapest_first() {
        // The declared order is the documented one, and `Sign` precedes the
        // basis search an attacker can lengthen.
        let order = [
            VerifyError::Length,
            VerifyError::Canonical,
            VerifyError::Origin,
            VerifyError::Singular,
            VerifyError::Sign,
            VerifyError::Basis,
            VerifyError::Composite,
            VerifyError::Kernel,
            VerifyError::Unidentified,
        ];
        let mut sorted = order;
        sorted.sort_unstable();
        assert_eq!(order, sorted);
        assert!(VerifyError::Sign < VerifyError::Basis);
        assert!(VerifyError::Basis < VerifyError::Composite);
    }

    #[test]
    fn a_short_signature_is_a_length_refusal() {
        let err = verify::<Level1>(&[0u8; Level1::PUBLIC_KEY_LEN], b"m", &[0u8; 4]);
        assert_eq!(err, Err(VerifyError::Length));
        // A wrong length is refused before the key is looked at, so the
        // starting curve does not shadow it.
        let err = verify::<Level1>(&[0u8; Level1::PUBLIC_KEY_LEN], b"m", &[]);
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
    fn a_singular_curve_is_refused() {
        // `A = 2` is a square root of four, so the curve is singular.
        let sig = vec![0u8; Level1::SIGNATURE_LEN];
        assert_eq!(
            verify::<Level1>(&key::<Level1>(2), b"shekyl-prism", &sig),
            Err(VerifyError::Singular)
        );
    }

    #[test]
    fn a_noncanonical_key_is_refused() {
        let mut pk = vec![0xffu8; Level1::PUBLIC_KEY_LEN];
        pk[Level1::FP_LEN - 1] = 0xff;
        let sig = vec![0u8; Level1::SIGNATURE_LEN];
        assert_eq!(
            verify::<Level1>(&pk, b"shekyl-prism", &sig),
            Err(VerifyError::Canonical)
        );
    }

    #[test]
    fn a_noncanonical_signature_is_refused_before_the_basis_search() {
        // The key is canonical, non-zero and nonsingular, so nothing above
        // `Canonical` can shadow the refusal; every field element of the
        // signature is out of range. The basis search is the next check
        // after the decode, so a `Canonical` answer here is what keeps a
        // junk signature from paying for up to `2λ` Legendre symbols.
        let pk = key::<Level1>(3);
        let sig = vec![0xffu8; Level1::SIGNATURE_LEN];
        assert_eq!(
            verify::<Level1>(&pk, b"shekyl-prism", &sig),
            Err(VerifyError::Canonical)
        );
    }

    #[test]
    fn zero_y_is_refused() {
        let pk = key::<Level1>(3);
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
    fn a_bad_sign_is_refused_before_the_challenge_is_hashed() {
        // The key's basis index exists and the index names a prime degree,
        // so the only failing check below `Composite` is the sign test.
        let sample = prepared(Nist::I).expect("level I sample");
        let mut response =
            Response::<Level1>::decode(&sample.signature).expect("the sample decodes");
        response.p.y = response.p.y.negate();
        assert_eq!(
            verify::<Level1>(&sample.public_key, sample.message, &response.encode()),
            Err(VerifyError::Sign)
        );
        // And with the sign restored, a later index whose degree is composite
        // is what the same signature is refused for.
        response.p.y = response.p.y.negate();
        let mut composite = None;
        let mut index = response.index;
        while let Some(next) = index.next() {
            index = next;
            let mut probe = response;
            probe.index = next;
            if verify::<Level1>(&sample.public_key, sample.message, &probe.encode())
                == Err(VerifyError::Composite)
            {
                composite = Some(probe);
                break;
            }
        }
        let composite = composite.expect("a composite degree above the smallest index");
        let mut both = composite;
        both.p.y = both.p.y.negate();
        assert_eq!(
            verify::<Level1>(&sample.public_key, sample.message, &both.encode()),
            Err(VerifyError::Sign),
            "a signature failing Sign and Composite is refused with Sign"
        );
    }

    #[test]
    fn the_derived_index_equals_the_c_hint() {
        for level in Nist::ALL {
            let row = reference_key(level).unwrap_or_else(|err| panic!("{level}: {err}"));
            let derived = match level {
                Nist::I => derive_hint::<Level1>(&row.public_key),
                Nist::III => derive_hint::<Level3>(&row.public_key),
                Nist::V => derive_hint::<Level5>(&row.public_key),
            };
            assert_eq!(derived, Ok(row.hint), "{level}");
            // The byte the C returned round-trips through the typed hint.
            let byte = row.hint.to_c_byte().expect("the C named this index");
            assert_eq!(BasisHint::from_c_byte(byte), Some(row.hint), "{level}");
        }
    }

    fn derive_hint<L: Level>(public_key: &[u8]) -> Result<BasisHint, VerifyError> {
        let identity = KeyIdentity::<L>::decode(public_key)?;
        basis_hint::<L>(identity.coefficient())
    }

    #[test]
    fn the_pinned_hints_are_the_committed_bytes() {
        // The vectors `scripts/export_c_basis_hints.py` wrote, unchanged.
        for (level, byte) in [(Nist::I, 11u8), (Nist::III, 21), (Nist::V, 2)] {
            let row = reference_key(level).unwrap_or_else(|err| panic!("{level}: {err}"));
            assert_eq!(row.hint.to_c_byte(), Some(byte), "{level}");
            assert!(row.hint.index() >= 1 && row.hint.index() <= C_HINT_MAX_INDEX);
        }
    }

    #[test]
    fn a_signature_round_trips_through_the_layout() {
        for level in Nist::ALL {
            let sample = prepared(level).unwrap_or_else(|err| panic!("{level}: {err}"));
            let decoded = match level {
                Nist::I => Response::<Level1>::decode(&sample.signature).map(|r| r.encode()),
                Nist::III => Response::<Level3>::decode(&sample.signature).map(|r| r.encode()),
                Nist::V => Response::<Level5>::decode(&sample.signature).map(|r| r.encode()),
            };
            assert_eq!(decoded, Ok(sample.signature.clone()), "{level}");
        }
    }

    #[test]
    fn a_prime_index_reaches_relation_without_being_checked_for_smallest() {
        let sample = prepared(Nist::I).expect("level I sample");
        assert_eq!(
            verify_level(
                Nist::I,
                &sample.public_key,
                sample.message,
                &sample.signature
            ),
            Err(VerifyError::Kernel)
        );
        let response = Response::<Level1>::decode(&sample.signature).expect("the sample decodes");
        if let Some(later) = response.index.next() {
            let mut probe = response;
            probe.index = later;
            let err = verify::<Level1>(&sample.public_key, sample.message, &probe.encode());
            // A different index is a different degree. It is refused when
            // that degree is composite, and never for being larger.
            let identity = KeyIdentity::<Level1>::decode(&sample.public_key).expect("the key");
            let scalar = ChallengeScalar::derive(&identity, sample.message, later);
            if scalar.degree_is_prime::<Level1>() {
                assert_eq!(err, Err(VerifyError::Kernel));
            } else {
                assert_eq!(err, Err(VerifyError::Composite));
            }
        }
    }

    #[test]
    fn a_level_name_parses_or_is_its_own_error() {
        for (text, level) in [
            ("1", Nist::I),
            ("I", Nist::I),
            ("NIST-I", Nist::I),
            ("3", Nist::III),
            ("III", Nist::III),
            ("5", Nist::V),
            ("V", Nist::V),
        ] {
            assert_eq!(text.parse::<Nist>(), Ok(level), "{text}");
        }
        for text in ["", "2", "iv", "NIST-II"] {
            assert_eq!(text.parse::<Nist>(), Err(UnknownLevel), "{text}");
        }
        // An unknown level is not a length refusal.
        assert_ne!(UnknownLevel.to_string(), VerifyError::Length.to_string());
    }

    #[test]
    fn two_thousand_forty_seven_is_not_a_prime() {
        assert!(!is_baillie_psw(&2047u32.to_le_bytes()[..2]));
        assert!(is_baillie_psw(&1019u32.to_le_bytes()[..2]));
        assert!(!is_baillie_psw(&[1]));
        assert!(!is_baillie_psw(&[91]));
        assert!(!is_baillie_psw(&[49, 2]));
    }

    #[test]
    fn the_three_field_primes_pass_baillie_psw() {
        assert!(is_baillie_psw(&prime_bytes(5, Level1::E)));
        assert!(is_baillie_psw(&prime_bytes(65, Level3::E)));
        assert!(is_baillie_psw(&prime_bytes(27, Level5::E)));
    }

    /// `cofactor·2^shift − 1`, little-endian.
    fn prime_bytes(cofactor: u128, shift: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; (shift as usize) / 8 + 17];
        let byte = (shift as usize) / 8;
        let bit = shift % 8;
        for (index, value) in cofactor.to_le_bytes().iter().enumerate() {
            let shifted = u16::from(*value) << bit;
            bytes[index + byte] |= shifted as u8;
            if let Some(slot) = bytes.get_mut(index + byte + 1) {
                *slot |= (shifted >> 8) as u8;
            }
        }
        let mut borrow = true;
        for slot in &mut bytes {
            if !borrow {
                break;
            }
            let (value, wrapped) = slot.overflowing_sub(1);
            *slot = value;
            borrow = wrapped;
        }
        while bytes.last() == Some(&0) {
            bytes.pop();
        }
        bytes
    }
}
