// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The tag layer: an identifier in front of a bare key or signature.
//!
//! Everything here reads at most the first four bytes of an input and
//! compares small integers. No field element is decoded and no field
//! operation runs until the tag layer has let an input through.

use core::fmt;

use prism_field::ParameterSet;
use prism_verify::{set::verify_set, VerifyError};

use crate::{AcceptList, SetId, UnknownSetId};

/// Which of the two tagged containers an error is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Container {
    /// A tagged public key: a [`SetId`] and then the bare key.
    PublicKey,
    /// A tagged signature: a [`SetId`] and then the bare signature.
    Signature,
}

impl fmt::Display for Container {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PublicKey => "public key",
            Self::Signature => "signature",
        })
    }
}

/// Why tagged bytes were refused.
///
/// Every variant but [`Self::Verify`] is a refusal of the tag layer: it
/// was decided from the identifiers and the accept list alone, before
/// any check of the bare bytes, and it cost no field arithmetic.
///
/// # Nothing here is a secret
///
/// The variants distinguish an identifier that names no set from a set
/// the list does not hold, and say which sets were named. Those are
/// facts about public bytes and about a policy that is itself public, so
/// returning them to whoever sent the bytes discloses nothing. The
/// distinction does not need hiding, and hiding it would cost the
/// operator the reason a peer was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaggedError {
    /// The input is shorter than an identifier.
    TooShort {
        /// Which input.
        container: Container,
        /// Its length, from 0 to 3.
        len: usize,
    },
    /// The identifier names no set this library is built with.
    UnknownSet {
        /// Which input carried it.
        container: Container,
        /// The `e` and `f` read.
        id: UnknownSetId,
    },
    /// The key and the signature are tagged with different sets.
    /// Returned by [`verify_tagged`] alone.
    Mismatch {
        /// The key's identifier.
        public_key: SetId,
        /// The signature's identifier.
        signature: SetId,
    },
    /// The identifier is not that of the type being read. Returned by
    /// [`PublicKey::from_tagged`](crate::PublicKey::from_tagged) and
    /// [`Signature::from_tagged`](crate::Signature::from_tagged) alone.
    WrongSet {
        /// The set of the type.
        expected: SetId,
        /// The set the bytes are tagged with.
        found: SetId,
    },
    /// The set is not in the accept list. Returned by [`verify_tagged`]
    /// alone.
    NotAccepted(SetId),
    /// The tag layer let the input through and the bare bytes were
    /// refused: the verifier's error, unaltered.
    Verify(VerifyError),
}

impl fmt::Display for TaggedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { container, len } => write!(
                f,
                "the tagged {container} is {len} bytes, shorter than a set identifier"
            ),
            Self::UnknownSet { container, id } => write!(f, "the tagged {container}: {id}"),
            Self::Mismatch {
                public_key,
                signature,
            } => write!(
                f,
                "the public key is tagged {public_key} and the signature {signature}"
            ),
            Self::WrongSet { expected, found } => {
                write!(f, "tagged {found}, and {expected} was expected")
            }
            Self::NotAccepted(set) => write!(f, "{set} is not in the accept list"),
            Self::Verify(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for TaggedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnknownSet { id, .. } => Some(id),
            Self::Verify(error) => Some(error),
            _ => None,
        }
    }
}

impl From<VerifyError> for TaggedError {
    fn from(error: VerifyError) -> Self {
        Self::Verify(error)
    }
}

/// The four identifier bytes and what follows them.
fn split(container: Container, bytes: &[u8]) -> Result<([u8; SetId::LEN], &[u8]), TaggedError> {
    match bytes.split_first_chunk::<{ SetId::LEN }>() {
        Some((tag, bare)) => Ok((*tag, bare)),
        None => Err(TaggedError::TooShort {
            container,
            len: bytes.len(),
        }),
    }
}

/// The set a tag names.
fn named(container: Container, tag: [u8; SetId::LEN]) -> Result<SetId, TaggedError> {
    SetId::from_bytes(tag).map_err(|id| TaggedError::UnknownSet { container, id })
}

/// The bare bytes of a container tagged with the set `S`.
///
/// The tag layer of the typed readers: too short, then an identifier
/// that names no set, then one that names a set other than `S`.
pub(crate) fn bare_of<S: ParameterSet>(
    container: Container,
    bytes: &[u8],
) -> Result<&[u8], TaggedError> {
    let (tag, bare) = split(container, bytes)?;
    let found = named(container, tag)?;
    let expected = SetId::of::<S>();
    if found != expected {
        return Err(TaggedError::WrongSet { expected, found });
    }
    Ok(bare)
}

/// `SetId ‖ bare`, for the set `S`.
pub(crate) fn tagged<S: ParameterSet>(bare: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(SetId::LEN + bare.len());
    bytes.extend_from_slice(&SetId::of::<S>().to_bytes());
    bytes.extend_from_slice(bare);
    bytes
}

/// Verify a tagged signature on `message` under a tagged public key,
/// accepting only the sets in `accept`.
///
/// `public_key` is a [`SetId`] and then a bare key; `signature` is a
/// [`SetId`] and then a bare signature. Each is its exact length:
/// nothing may follow the bare part. `Ok` carries the set that was
/// accepted, so the caller can bind it to what it stores.
///
/// This is the entry for a caller that reads a key and a signature
/// together from one message and learns the set from the bytes. A caller
/// that names the set in its source, or keeps a key and checks many
/// signatures under it, uses [`PublicKey`](crate::PublicKey) and
/// [`Signature`](crate::Signature).
///
/// ```
/// use prism_vectors::corpus::Primary;
/// use shekyl_prism::{verify_tagged, AcceptList, PublicKey, SetId, Signature, P248};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let vector = Primary::Lvl1.vector();
/// let key = PublicKey::<P248>::from_bytes(&vector.public_key()?)?.to_tagged();
/// let signature = Signature::<P248>::from_bytes(&vector.signature()?)?.to_tagged();
///
/// let accept = AcceptList::new(&[SetId::of::<P248>()])?;
/// let set = verify_tagged(&accept, &key, vector.message()?, &signature)?;
/// assert_eq!(set, SetId::of::<P248>());
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// The tag layer refuses first, in this order, each before the next:
///
/// 1. [`TaggedError::TooShort`]: an input is shorter than an identifier.
///    The key is looked at before the signature;
/// 2. [`TaggedError::UnknownSet`] for the key: its identifier names no
///    built set;
/// 3. [`TaggedError::UnknownSet`] for the signature;
/// 4. [`TaggedError::Mismatch`]: the two identifiers differ;
/// 5. [`TaggedError::NotAccepted`]: the set is not in `accept`.
///
/// Then the bare parts go to the byte-level entry
/// [`hazmat::verify::verify`](crate::hazmat::verify::verify) of that
/// set, and its refusal comes back as [`TaggedError::Verify`], unaltered.
/// A bare part of the wrong length for its identifier is refused there,
/// as [`VerifyError::Length`].
///
/// # Cost
///
/// The five refusals above cost no field operation, whatever the bytes
/// are: a sender cannot make this function work before the list has been
/// consulted. After them the cost is the byte-level entry's, which
/// checks the signature's encoding before it searches for the key's
/// basis.
///
/// # What `Ok` means
///
/// That the relation holds for these bytes under the set returned, and
/// that the set is in `accept`. It does not mean the key's owner chose
/// that set: see [`AcceptList`].
pub fn verify_tagged(
    accept: &AcceptList,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<SetId, TaggedError> {
    let (key_tag, bare_key) = split(Container::PublicKey, public_key)?;
    let (signature_tag, bare_signature) = split(Container::Signature, signature)?;
    let key_set = named(Container::PublicKey, key_tag)?;
    let signature_set = named(Container::Signature, signature_tag)?;
    if key_set != signature_set {
        return Err(TaggedError::Mismatch {
            public_key: key_set,
            signature: signature_set,
        });
    }
    if !accept.contains(key_set) {
        return Err(TaggedError::NotAccepted(key_set));
    }
    verify_set(key_set.set(), bare_key, message, bare_signature)?;
    Ok(key_set)
}
