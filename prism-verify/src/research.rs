// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Deliberately insecure research surface, absent from default builds.
//! The caller owns archived challenge hashing. The real algebraic relation,
//! normalization, length, field, primality and torsion checks still execute.
use crate::{ChallengeScalar, KeyIdentity, Response, VerifyError};
use prism_field::Level;

pub fn verify_scalar<L: Level>(
    key: &[u8],
    signature: &[u8],
    scalar: &[u8],
    bits: u32,
) -> Result<(), VerifyError> {
    if key.len() != L::PUBLIC_KEY_LEN || signature.len() != L::SIGNATURE_LEN {
        return Err(VerifyError::Length);
    }
    let id = KeyIdentity::<L>::decode(key)?;
    let response = Response::<L>::decode(signature)?;
    if !crate::signature::canonical_sign::<L>(response.p.y) {
        return Err(VerifyError::Sign);
    }
    crate::model::check::<L>(response.curve)?;
    crate::basis_hint::<L>(id.coefficient())?;
    let scalar = ChallengeScalar::research_bytes::<L>(scalar, bits).ok_or(VerifyError::Length)?;
    if !scalar.degree_is_prime::<L>() {
        return Err(VerifyError::Composite);
    }
    crate::relation::relate(&id, &response, &scalar)
}
