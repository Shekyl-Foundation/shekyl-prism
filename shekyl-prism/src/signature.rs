// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

use core::fmt;
use core::hash::{Hash, Hasher};

use prism_field::ParameterSet;
use prism_verify::{checked_response, CheckedResponse, VerifyError};

/// A signature of the parameter set `S`, decoded and checked.
///
/// A value of this type passed every check a signature can pass alone:
/// its length, the canonical encoding of its field elements, the sign
/// rule on its first point, and that its curve is the one permitted model
/// of its isomorphism class. [`Self::from_bytes`] is the only way to make
/// one.
///
/// Those checks say the bytes are the one encoding of a well-formed
/// signature. They say nothing about any key or message; that is
/// [`PublicKey::verify`](crate::PublicKey::verify).
///
/// # Equality
///
/// Two signatures are equal exactly when their encodings are equal, and
/// `Hash` hashes the encoding. A signature has one encoding, so this is
/// equality of signatures.
#[derive(Clone)]
pub struct Signature<S: ParameterSet> {
    bytes: Vec<u8>,
    response: CheckedResponse<S>,
}

impl<S: ParameterSet> Signature<S> {
    /// The length of a signature of this set, in bytes.
    pub const LEN: usize = S::SIGNATURE_LEN;

    /// Check `bytes` and make the signature.
    ///
    /// None of these checks is dear: there is no search, so whoever chose
    /// the bytes cannot make them cost more than a signature that passes
    /// them all, and a refusal returns earlier.
    ///
    /// # Errors
    ///
    /// In this order:
    ///
    /// - [`VerifyError::Length`]: not [`Self::LEN`] bytes;
    /// - [`VerifyError::Canonical`]: a field element is not below `p`;
    /// - [`VerifyError::Sign`]: the first point's `y` is zero or is not
    ///   the smaller of `y` and `−y`;
    /// - [`VerifyError::Singular`] or [`VerifyError::Model`]: the
    ///   signature's curve is singular, or is not the permitted model.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VerifyError> {
        checked_response::<S>(bytes).map(|response| Self {
            bytes: bytes.to_vec(),
            response,
        })
    }

    /// The signature's encoding: the bytes it was made from.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The signature's encoding, owned. [`Self::from_bytes`] of it is
    /// this signature.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// The decoded form the verifier's steps take.
    pub(crate) fn response(&self) -> &CheckedResponse<S> {
        &self.response
    }
}

impl<S: ParameterSet> PartialEq for Signature<S> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<S: ParameterSet> Eq for Signature<S> {}

impl<S: ParameterSet> Hash for Signature<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

/// The set's name and the encoding in hex. A signature is public.
impl<S: ParameterSet> fmt::Debug for Signature<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature<{}>(", S::NAME)?;
        crate::hex(&self.bytes, f)?;
        f.write_str(")")
    }
}
