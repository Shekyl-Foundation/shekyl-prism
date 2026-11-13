// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Deliberately insecure research surface, absent from default builds.
//! The caller owns archived challenge hashing, or asks for a narrower
//! projection of the production transcript. Length, sign, model, basis,
//! primality and the algebraic relation are the production checks.

use crate::{accept, open, ChallengeScalar, VerifyError};
use prism_field::Level;

/// Verify `signature` against a scalar the caller already derived.
///
/// `scalar` is the little-endian challenge at `bits`, odd, and no wider
/// than the row. The signature's salt is not hashed again.
pub fn verify_scalar<L: Level>(
    key: &[u8],
    signature: &[u8],
    scalar: &[u8],
    bits: u32,
) -> Result<(), VerifyError> {
    let opened = open::<L>(key, signature)?;
    let scalar = ChallengeScalar::research_bytes::<L>(scalar, bits).ok_or(VerifyError::Length)?;
    accept(&opened, &scalar)
}

/// Derive the production transcript at `bits` and verify it.
///
/// `bits == 0` or `bits` above the row is [`VerifyError::Length`]. A short
/// signature is `Length` even when the key is the starting curve.
pub fn verify_width<L: Level>(
    key: &[u8],
    message: &[u8],
    signature: &[u8],
    bits: u32,
) -> Result<(), VerifyError> {
    let opened = open::<L>(key, signature)?;
    let scalar =
        ChallengeScalar::derive_width(&opened.identity, message, opened.response.salt, bits)
            .ok_or(VerifyError::Length)?;
    accept(&opened, &scalar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{Level, Level1};

    #[test]
    fn short_signature_is_length_before_origin() {
        let key = vec![0u8; Level1::PUBLIC_KEY_LEN];
        let err = verify_width::<Level1>(&key, b"m", &[], Level1::CHALLENGE_BITS);
        assert_eq!(err, Err(VerifyError::Length));
        let err = verify_scalar::<Level1>(&key, &[], &[1], 1);
        assert_eq!(err, Err(VerifyError::Length));
    }
}
