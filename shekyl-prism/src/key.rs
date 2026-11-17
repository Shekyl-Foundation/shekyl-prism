// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

use core::fmt;
use core::hash::{Hash, Hasher};

use prism_field::ParameterSet;
use prism_verify::{verify_prepared, PreparedKey, VerifyError};

use crate::tagged::{bare_of, tagged, Container};
use crate::{SetId, Signature, TaggedError};

/// A public key of the parameter set `S`, checked.
///
/// A value of this type passed every check a key can pass alone: its
/// length, the canonical encoding of its field element, that it is
/// neither the starting curve nor a singular curve, and that its torsion
/// basis is found within the set's search bound. [`Self::from_bytes`] is
/// the only way to make one.
///
/// The key keeps what the basis search found, so [`Self::verify`] does
/// not search again.
///
/// # Equality
///
/// Two keys are equal exactly when their encodings are equal, and `Hash`
/// hashes the encoding. A key has one encoding, so this is equality of
/// keys.
#[derive(Clone)]
pub struct PublicKey<S: ParameterSet> {
    prepared: PreparedKey<S>,
}

impl<S: ParameterSet> PublicKey<S> {
    /// The length of a key of this set, in bytes.
    pub const LEN: usize = S::PUBLIC_KEY_LEN;

    /// Check `bytes` and make the key.
    ///
    /// # Errors
    ///
    /// In this order:
    ///
    /// - [`VerifyError::Length`]: not [`Self::LEN`] bytes;
    /// - [`VerifyError::Canonical`]: a field element is not below `p`;
    /// - [`VerifyError::Origin`]: the starting curve, for which anyone
    ///   can sign;
    /// - [`VerifyError::Singular`]: not an elliptic curve;
    /// - [`VerifyError::Basis`]: no basis index within the set's search
    ///   bound.
    ///
    /// # Cost
    ///
    /// The last check is a search whose length is chosen by whoever chose
    /// the bytes: up to `S::SEARCH_BOUND` candidates, each a test of
    /// whether a field element is a square. This function runs it before
    /// any signature has been looked at.
    ///
    /// That is the right order for a key that is validated once and then
    /// kept: a key read from stored state, used for many signatures. It
    /// is the wrong order for a caller that builds a key from every
    /// incoming message, where a sender can make each message cost the
    /// full search before its signature is even decoded. Such a caller
    /// should build the [`Signature`] first, which is cheap and refuses
    /// malformed input, or use the byte-level entry
    /// [`hazmat::verify::verify`](crate::hazmat::verify::verify), which
    /// orders its checks for that case.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VerifyError> {
        PreparedKey::prepare(bytes).map(|prepared| Self { prepared })
    }

    /// The key's encoding: the bytes it was made from.
    pub fn as_bytes(&self) -> &[u8] {
        self.prepared.as_bytes()
    }

    /// The key's encoding, owned. [`Self::from_bytes`] of it is this key.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    /// The length of a tagged key of this set, in bytes: a [`SetId`] and
    /// then the key.
    pub const TAGGED_LEN: usize = SetId::LEN + S::PUBLIC_KEY_LEN;

    /// The key with its set's identifier in front: [`SetId`] and then
    /// [`Self::as_bytes`]. This is the form to store or send where the
    /// reader does not already know the set.
    pub fn to_tagged(&self) -> Vec<u8> {
        tagged::<S>(self.as_bytes())
    }

    /// Read a tagged key of this set. [`Self::to_tagged`] of the result
    /// is `bytes`.
    ///
    /// # Errors
    ///
    /// The tag is judged first, and nothing after the first four bytes
    /// is looked at until it has passed:
    ///
    /// - [`TaggedError::TooShort`]: fewer than four bytes;
    /// - [`TaggedError::UnknownSet`]: the identifier names no built set;
    /// - [`TaggedError::WrongSet`]: it names a set other than `S`.
    ///
    /// Then [`TaggedError::Verify`] with the error of
    /// [`Self::from_bytes`] on what follows the identifier. The cost
    /// note of [`Self::from_bytes`] applies from there.
    pub fn from_tagged(bytes: &[u8]) -> Result<Self, TaggedError> {
        let bare = bare_of::<S>(Container::PublicKey, bytes)?;
        Ok(Self::from_bytes(bare)?)
    }

    /// Verify `signature` on `message` under this key.
    ///
    /// The checks left for this call are the ones that need all three
    /// inputs: the challenge is derived from the key, the message and the
    /// signature's salt; its degree must be prime; and the signature must
    /// be the isogeny of that degree from this key's curve.
    ///
    /// # Errors
    ///
    /// - [`VerifyError::Composite`]: the challenge degree is not prime.
    ///   A signature under another message or another key usually stops
    ///   here.
    /// - [`VerifyError::Kernel`], [`VerifyError::Pairing`],
    ///   [`VerifyError::Codomain`], [`VerifyError::Image`]: the relation
    ///   does not hold; which one says how far the check went.
    ///
    /// The errors of [`Self::from_bytes`] and [`Signature::from_bytes`]
    /// are not returned here: both values have already passed them.
    pub fn verify(&self, message: &[u8], signature: &Signature<S>) -> Result<(), VerifyError> {
        verify_prepared(&self.prepared, message, signature.response())
    }
}

impl<S: ParameterSet> PartialEq for PublicKey<S> {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<S: ParameterSet> Eq for PublicKey<S> {}

impl<S: ParameterSet> Hash for PublicKey<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

/// The set's name and the encoding in hex. A public key is public.
impl<S: ParameterSet> fmt::Debug for PublicKey<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey<{}>(", S::NAME)?;
        crate::hex(self.as_bytes(), f)?;
        f.write_str(")")
    }
}
