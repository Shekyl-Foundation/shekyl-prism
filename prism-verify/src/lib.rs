// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Experimental prime-degree PRISM verifier at `e′ = e − 2`.
//!
//! Canonical decoding and public checks precede two adapted theta chains,
//! dual-codomain identification and both components of the image check.
//! `Ok(())` means this research relation holds; it is not a claim of a
//! reviewed SUF-CMA security proof. See `docs/SECURITY_REVIEW.md`.
//!
//! Each public check owns a module: [`identity`] decodes the key,
//! [`signature`] decodes the response and normalizes its sign, [`basis`]
//! re-derives the C reference's basis index, [`challenge`] derives the
//! challenge scalar, and [`prime`] decides its degree. [`checked`] runs them
//! before the relation checks.

#![forbid(unsafe_code)]

mod accept;
pub mod basis;
pub mod challenge;
pub mod identity;
pub mod level;
mod model;
mod prime;
pub mod reference;
mod relation;
#[cfg(feature = "research-reduced-width")]
pub mod research;
pub mod signature;
pub mod stage;

use prism_field::Level;

pub use basis::{basis_hint, search_bound, BasisHint, C_HINT_MAX_INDEX};
pub use challenge::{ChallengeScalar, CHALLENGE_DOMAIN};
pub use identity::KeyIdentity;
pub use level::{verify_level, Nist, UnknownLevel};
pub use prime::{challenge_degree, degree_is_prime, is_baillie_psw};
pub use prism_field::Salt;
pub use reference::{prepared, reference_key, Prepared, PreparedError, ReferenceKey, VectorError};
pub use signature::{AffinePoint, Response};
pub use stage::Stage;

/// Why a signature was refused.
///
/// Public decoding checks precede curve/basis checks, pairing, chains,
/// codomain identification and images. `Singular` can arise on either
/// curve and `Kernel` can arise before or after pairing, so the enum's
/// derived order is not a global precedence rule for malformed inputs.
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
    /// A response curve is not its least model, or has extra automorphisms.
    Model,
    /// No basis index within [`search_bound`] meets the C search.
    ///
    /// The search is the reference's, including the indices its `start = 128`
    /// fallback covers, so this refuses only keys whose smallest index
    /// exceeds `2λ` — about `2^-λ` of keys, the level's own security target.
    /// `docs/VERIFIER_SPEC.md` records the bound and what it costs.
    Basis,
    /// `2^{2e′−1} − s_j^2` is not a Baillie–PSW prime, `e′ = e − 2`.
    Composite,
    /// Curve, torsion, or adapted-chain construction failed.
    Kernel,
    /// `e(R₁, R₂) ≠ e(P₁, P₂)^q`, with `R_i = [s]` of the signature points.
    /// This pins the determinant of the basis change. It runs after the
    /// order and span checks and before either chain.
    Pairing,
    /// The adapted chains do not meet in dual theta structures.
    Codomain,
    /// The composed chain does not have the challenge-bound images.
    Image,
}

impl core::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reason = match self {
            Self::Length => "not this level's length",
            Self::Canonical => "a field element is not canonical",
            Self::Origin => "the starting curve",
            Self::Singular => "a singular Montgomery curve",
            Self::Model => "the response curve is not the unique permitted model",
            Self::Sign => "y(P) is not the normalized representative",
            Self::Basis => "no basis index within the search bound",
            Self::Composite => "the challenge degree is composite",
            Self::Kernel => "the curve, torsion, or adapted chain is invalid",
            Self::Pairing => "the Weil pairing does not match the challenge degree",
            Self::Codomain => "the adapted codomains do not match",
            Self::Image => "the composed images do not match the challenge",
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
    let opened = open::<L>(public_key, signature)?;
    Stage::Challenge.enter();
    let scalar = ChallengeScalar::derive(&opened.identity, message, opened.response.salt);
    accept(&opened, &scalar)
}

/// A key and response that passed the checks shared by every verifier entry.
pub(crate) struct Opened<'a, L: Level> {
    pub identity: KeyIdentity<'a, L>,
    pub response: Response<L>,
}

/// Length, decode, sign, model, and basis. Both lengths come before either
/// decode, so `Length` does not depend on which field is wrong and a short
/// signature cannot surface as `Origin`. The sign test is a byte comparison
/// on an element already decoded, so it comes before the basis search, whose
/// length an attacker chooses with `A` — up to [`search_bound`] Legendre
/// symbols.
pub(crate) fn open<'a, L: Level>(
    public_key: &'a [u8],
    signature: &[u8],
) -> Result<Opened<'a, L>, VerifyError> {
    Stage::Public.enter();
    if public_key.len() != L::PUBLIC_KEY_LEN || signature.len() != L::SIGNATURE_LEN {
        return Err(VerifyError::Length);
    }
    let identity = KeyIdentity::<L>::decode(public_key)?;
    let response = Response::<L>::decode(signature)?;
    if !signature::canonical_sign::<L>(response.p.y) {
        return Err(VerifyError::Sign);
    }
    model::check::<L>(response.curve)?;
    basis_hint::<L>(identity.coefficient())?;
    Ok(Opened { identity, response })
}

/// Primality, then the relation. The challenge hash has already run.
pub(crate) fn accept<L: Level>(
    opened: &Opened<'_, L>,
    scalar: &ChallengeScalar,
) -> Result<(), VerifyError> {
    Stage::Challenge.enter();
    if !scalar.degree_is_prime::<L>() {
        return Err(VerifyError::Composite);
    }
    relation::relate(&opened.identity, &opened.response, scalar)
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
    fn public_refusals_precede_the_expensive_checks() {
        // The declared order is the documented one, and `Sign` precedes the
        // basis search an attacker can lengthen.
        let order = [
            VerifyError::Length,
            VerifyError::Canonical,
            VerifyError::Origin,
            VerifyError::Singular,
            VerifyError::Sign,
            VerifyError::Model,
            VerifyError::Basis,
            VerifyError::Composite,
            VerifyError::Kernel,
            VerifyError::Pairing,
            VerifyError::Codomain,
            VerifyError::Image,
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
        assert_eq!(Level1::SIGNATURE_LEN, 352);
        assert_eq!(Level3::SIGNATURE_LEN, 528);
        assert_eq!(Level5::SIGNATURE_LEN, 704);
        assert_eq!(Level3::CHAIN_STEPS, Level3::E - 2);
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
        // The key's basis exists and the salt names a prime degree,
        // so the only failing check below `Composite` is the sign test.
        let sample = prepared(Nist::I).expect("level I sample");
        let mut response =
            Response::<Level1>::decode(&sample.signature).expect("the sample decodes");
        response.p.y = response.p.y.negate();
        assert_eq!(
            verify::<Level1>(&sample.public_key, sample.message, &response.encode()),
            Err(VerifyError::Sign)
        );
        // And with the sign restored, another salt whose degree is composite
        // is what the same signature is refused for.
        response.p.y = response.p.y.negate();
        let composite = (0u64..65_536)
            .find_map(|n| {
                let mut probe = response;
                probe.salt = <Level1 as Level>::Salt::from_counter(n).unwrap();
                (verify::<Level1>(&sample.public_key, sample.message, &probe.encode())
                    == Err(VerifyError::Composite))
                .then_some(probe)
            })
            .expect("a composite salt");
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
        // The hint bytes `scripts/export_c_basis_hints.py` wrote from its
        // frozen seed. A regeneration that changes a byte updates this table.
        for (level, byte) in [(Nist::I, 11u8), (Nist::III, 13), (Nist::V, 21)] {
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
    fn a_carried_salt_reaches_the_real_relation_without_a_search() {
        let sample = prepared(Nist::I).expect("sample");
        assert_eq!(
            verify_level(
                Nist::I,
                &sample.public_key,
                sample.message,
                &sample.signature
            ),
            Err(VerifyError::Kernel)
        );
        let mut response = Response::<Level1>::decode(&sample.signature).unwrap();
        let mut salt = response.salt.as_bytes().to_vec();
        salt[0] ^= 1;
        response.salt = <Level1 as Level>::Salt::from_bytes(&salt).unwrap();
        let id = KeyIdentity::<Level1>::decode(&sample.public_key).unwrap();
        let prime =
            ChallengeScalar::derive(&id, sample.message, response.salt).degree_is_prime::<Level1>();
        assert_eq!(
            verify::<Level1>(&sample.public_key, sample.message, &response.encode()),
            Err(if prime {
                VerifyError::Kernel
            } else {
                VerifyError::Composite
            })
        );
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
