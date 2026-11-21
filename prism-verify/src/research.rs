// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Not PRISM: the reduced-width experiment, absent from default builds.
//!
//! [`verify_scalar`] takes a scalar the caller supplies and does not
//! hash, so that scalar is the scheme's challenge only when the caller
//! has already derived it. [`verify_width`] derives the challenge as
//! [`crate::verify`] does and keeps its low `bits`. When `bits` is the
//! set's [`prism_field::ParameterSet::CHALLENGE_BITS`], the derivation
//! and the verdict are [`crate::verify`]'s. A shorter width is a
//! different challenge.
//!
//! The two functions are deliberately insecure. They exist for the
//! experiment that replays and forges at a challenge of 16 to 32 bits,
//! against the real signer and this verifier's real relation, to measure
//! how the number of queries grows with the width
//! (`docs/SECURITY_ARGUMENT.md`, "Ran, at reduced width only"). No
//! full-width forgery was attempted there. They are not entries of the
//! scheme.
//!
//! What they share with the verifier is everything but the challenge:
//! length, decoding, sign, model, the basis search, primality and the
//! relation are the same functions [`crate::verify`] runs, in the same
//! order.
//!
//! The module exists only with the feature `research-reduced-width`,
//! which `scripts/ci/check_research_boundary.py` holds out of the front
//! crate's build.

use crate::{accept, challenge::ChallengeScalar, open, VerifyError};
use prism_field::ParameterSet;

/// Not PRISM. Verify `signature` against a scalar the caller supplies.
///
/// `scalar` is the little-endian challenge at `bits`: odd, exactly
/// `bits` wide in whole bytes, and no wider than the set's challenge.
/// Anything else is [`VerifyError::Length`], after the key's and the
/// signature's own checks. Nothing is hashed: the message is not an
/// argument, and the signature's salt is read and not used.
pub fn verify_scalar<L: ParameterSet>(
    key: &[u8],
    signature: &[u8],
    scalar: &[u8],
    bits: u32,
) -> Result<(), VerifyError> {
    let opened = open::<L>(key, signature)?;
    let scalar = ChallengeScalar::research_bytes::<L>(scalar, bits).ok_or(VerifyError::Length)?;
    accept(&opened, &scalar)
}

/// Not PRISM. Derive the challenge as [`crate::verify`] does, cut it to
/// `bits` bits, and verify against that.
///
/// At `bits` equal to the set's challenge width, the challenge and the
/// verdict are [`crate::verify`]'s. A shorter width is the reduced-width
/// experiment.
///
/// `bits == 0` or `bits` above the set's width is
/// [`VerifyError::Length`], after the key's and the signature's own
/// checks. A short signature is `Length` even when the key is the
/// starting curve.
pub fn verify_width<L: ParameterSet>(
    key: &[u8],
    message: &[u8],
    signature: &[u8],
    bits: u32,
) -> Result<(), VerifyError> {
    let opened = open::<L>(key, signature)?;
    let scalar = ChallengeScalar::derive_width(
        &opened.identity,
        message,
        opened.checked.response().salt,
        bits,
    )
    .ok_or(VerifyError::Length)?;
    accept(&opened, &scalar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{ParameterSet, P248};

    #[test]
    fn short_signature_is_length_before_origin() {
        let key = vec![0u8; P248::PUBLIC_KEY_LEN];
        let err = verify_width::<P248>(&key, b"m", &[], P248::CHALLENGE_BITS);
        assert_eq!(err, Err(VerifyError::Length));
        let err = verify_scalar::<P248>(&key, &[], &[1], 1);
        assert_eq!(err, Err(VerifyError::Length));
    }

    /// The full challenge width is the scheme's challenge: the same
    /// verdict as [`crate::verify`] on a committed honest signature.
    #[test]
    fn the_full_width_is_verify() {
        let vector = prism_vectors::corpus::Primary::Lvl1.vector();
        let key = vector.public_key().expect("the vector's public key");
        let message = vector.message().expect("the vector's message");
        let signature = vector.signature().expect("the vector's signature");
        let verdict = crate::verify::<P248>(&key, message, &signature);
        assert_eq!(verdict, Ok(()));
        assert_eq!(
            verify_width::<P248>(&key, message, &signature, P248::CHALLENGE_BITS),
            verdict
        );
    }
}
