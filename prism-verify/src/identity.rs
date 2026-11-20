// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The public key's identity, and every refusal that is a property of the
//! key alone.

use prism_field::{Field, ParameterSet};

use crate::VerifyError;

/// `A^2` on the one Montgomery curve that is singular.
const SINGULAR_SQUARE: u64 = 4;

/// A decoded public key: the Montgomery coefficient `A` of
/// `E_vk : y^2 = x^3 + A x^2 + x`, together with the bytes that name it.
///
/// The fields are private and [`Self::decode`] is the only public
/// constructor, so a value is a key of the set's length, canonically
/// encoded, on neither excluded curve, and its coefficient is the decode
/// of its bytes. (Inside the crate a prepared key reads its stored bytes
/// again without repeating the checks; those bytes passed `decode` when
/// the key was prepared.)
///
/// This is the challenge identity, not proof of a full rational torsion
/// basis: a value of this type may be a curve whose `2^e`-torsion is not
/// rational, which is no key. [`crate::prepared::PreparedKey`] is the
/// type that holds that test. The challenge hashes these exact bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyIdentity<'key, L: ParameterSet> {
    bytes: &'key [u8],
    coefficient: L::Ext,
}

impl<'key, L: ParameterSet> KeyIdentity<'key, L> {
    /// Decode `public_key`, refusing the lengths and curves no key may have.
    ///
    /// `Length`, then `Canonical`, then `Origin`, then `Singular`. It
    /// assumes nothing of the bytes.
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

    /// The identity of bytes [`Self::decode`] has already accepted.
    ///
    /// The coefficient is [`ParameterSet::ext_decode`] of `bytes`. This
    /// does not repeat [`Self::decode`]'s checks: the singular test squares,
    /// and a second squaring would be a field operation the typed entry
    /// does not pay. The only caller has run [`Self::decode`] on these bytes.
    pub(crate) fn from_accepted_bytes(bytes: &'key [u8]) -> Self {
        let coefficient = L::ext_decode(bytes).expect("accepted key bytes decode");
        Self { bytes, coefficient }
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
