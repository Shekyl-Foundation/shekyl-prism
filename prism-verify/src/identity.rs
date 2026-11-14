// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The public key's identity, and every refusal that is a property of the
//! key alone.

use prism_field::{Field, Level};

use crate::VerifyError;

/// `A^2` on the one Montgomery curve that is singular.
const SINGULAR_SQUARE: u64 = 4;

/// A decoded public key: the Montgomery coefficient `A` of
/// `E_vk : y^2 = x^3 + A x^2 + x`, together with the bytes that name it.
///
/// This constructor checks the encoding and excluded coefficients. It is
/// the challenge identity, not proof of a full rational torsion basis:
/// the verifier checks that basis's order and span before pairing/chains.
/// Both verification and index search hash these exact canonical bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyIdentity<'key, L: Level> {
    bytes: &'key [u8],
    coefficient: L::Ext,
}

impl<'key, L: Level> KeyIdentity<'key, L> {
    /// Decode `public_key`, refusing the lengths and curves no key may have.
    ///
    /// `Length`, then `Canonical`, then `Origin`, then `Singular`.
    pub fn decode(public_key: &'key [u8]) -> Result<Self, VerifyError> {
        if public_key.len() != L::PUBLIC_KEY_LEN {
            return Err(VerifyError::Length);
        }
        let coefficient = L::ext_decode(public_key).ok_or(VerifyError::Canonical)?;
        if coefficient.is_zero() {
            // `A = 0` is the starting curve `E0`. Its endomorphism ring is
            // public, so anyone could sign for it.
            return Err(VerifyError::Origin);
        }
        if coefficient.square().same(L::Ext::from_u64(SINGULAR_SQUARE)) {
            return Err(VerifyError::Singular);
        }
        Ok(Self {
            bytes: public_key,
            coefficient,
        })
    }

    /// The bytes the challenge hash absorbs. This *is* the key's identity,
    /// so nothing else may stand in for it.
    pub fn as_bytes(&self) -> &'key [u8] {
        self.bytes
    }

    /// `A`.
    pub fn coefficient(&self) -> L::Ext {
        self.coefficient
    }
}
