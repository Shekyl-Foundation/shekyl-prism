// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! PRISM signatures in pure Rust.
//!
//! PRISM is an isogeny-based signature scheme: a signature is an isogeny
//! of prime degree from the signer's public curve, and the degree is
//! bound to the message by a hash. This crate is the interface to the
//! implementation in this repository. Today it verifies. Signing and key
//! generation are not here yet; see [What is missing](#what-is-missing).
//!
//! # Status
//!
//! Research code, unreviewed, and not for protecting anything of value.
//! `Ok(())` from [`PublicKey::verify`] means the relation this repository
//! specifies holds; it is not a claim of a reviewed security proof. The
//! repository's `README.md` and `docs/SECURITY_REVIEW.md` say what has
//! and has not been examined.
//!
//! The repository is in alpha on its `dev` branch until it has a solid
//! verifier and a signer in Rust; a beta then goes to `main`. Until then
//! this interface can change from one change to the next.
//!
//! # Verifying
//!
//! A [`PublicKey`] and a [`Signature`] are made from bytes, and each is
//! valid by construction: every check that depends on the key alone runs
//! in [`PublicKey::from_bytes`], every check that depends on the
//! signature alone runs in [`Signature::from_bytes`], and
//! [`PublicKey::verify`] runs the rest. Both types carry their parameter
//! set, so a key of one set is not given a signature of another.
//!
//! This example reads one of the committed test vectors and verifies it.
//! The vectors are in the `prism-vectors` crate of this repository.
//!
//! ```
//! use prism_vectors::corpus::Primary;
//! use shekyl_prism::{PublicKey, Signature, P248};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let vector = Primary::Lvl1.vector();
//!
//! let key = PublicKey::<P248>::from_bytes(&vector.public_key()?)?;
//! let signature = Signature::<P248>::from_bytes(&vector.signature()?)?;
//! key.verify(vector.message()?, &signature)?;
//! # Ok(())
//! # }
//! ```
//!
//! # What is refused, and where
//!
//! Every refusal is a [`VerifyError`], and which call returns it says
//! which input is at fault. A signature whose bytes are not a canonical
//! encoding never becomes a [`Signature`]; a well-formed signature that
//! does not belong to the message is refused by [`PublicKey::verify`].
//!
//! ```
//! use prism_vectors::corpus::Primary;
//! use shekyl_prism::{ParameterSet, PublicKey, Signature, VerifyError, P248};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let vector = Primary::Lvl1.vector();
//! let key = PublicKey::<P248>::from_bytes(&vector.public_key()?)?;
//! let bytes = vector.signature()?;
//!
//! // One byte changed so that a field element is no longer below `p`:
//! // the bytes are not a signature, and construction says so.
//! let mut malformed = bytes.clone();
//! malformed[P248::FP_LEN - 1] = 0xff;
//! assert_eq!(
//!     Signature::<P248>::from_bytes(&malformed).unwrap_err(),
//!     VerifyError::Canonical
//! );
//!
//! // The signature itself, under another message: it is a signature,
//! // and verification refuses it.
//! let signature = Signature::<P248>::from_bytes(&bytes)?;
//! assert_eq!(
//!     key.verify(b"another message", &signature),
//!     Err(VerifyError::Composite)
//! );
//! # Ok(())
//! # }
//! ```
//!
//! # Parameter sets
//!
//! [`P248`], [`P324`], [`P376`], [`P500`] and [`P664`] are the five
//! sets, each named for the bit length of its prime. All five are
//! first-class here. Which of them a system accepts is that system's
//! policy: an [`AcceptList`], which this crate gives no default for.
//! [`ParameterSet`] is the trait they share; a user of this interface
//! needs it to name a set in a generic function and to read a set's
//! sizes, and [`PublicKey::LEN`] and [`Signature::LEN`] give the two
//! sizes most callers want without it.
//!
//! The set is part of the type, so a key and a signature of different
//! sets do not meet. This compiles:
//!
//! ```
//! use shekyl_prism::{PublicKey, Signature, VerifyError, P248};
//!
//! fn check(key: &PublicKey<P248>, signature: &Signature<P248>) -> Result<(), VerifyError> {
//!     key.verify(b"message", signature)
//! }
//! ```
//!
//! and the same function with the signature at another set does not:
//!
//! ```compile_fail
//! use shekyl_prism::{PublicKey, Signature, VerifyError, P248, P500};
//!
//! fn check(key: &PublicKey<P248>, signature: &Signature<P500>) -> Result<(), VerifyError> {
//!     key.verify(b"message", signature)
//! }
//! ```
//!
//! # Choosing a set at run time
//!
//! The types above serve a caller that names the prime in its source. A
//! node reads keys and signatures from the wire or from stored state,
//! where the bytes have to say which prime they are under and the node
//! has to say which primes it accepts. Three things do that:
//!
//! - [`SetId`] is the identifier: the pair `(e, f)` of the prime
//!   `p = f·2^e − 1`, in four bytes;
//! - a tagged key or signature is a [`SetId`] and then the bare
//!   encoding. [`PublicKey::to_tagged`] and [`Signature::to_tagged`]
//!   write it, and [`PublicKey::from_tagged`] and
//!   [`Signature::from_tagged`] read it at a set named in the source;
//! - [`AcceptList`] is the caller's policy, and [`verify_tagged`] takes
//!   one, reads the set from the bytes, and returns the set it accepted.
//!
//! ```
//! use prism_vectors::corpus::Primary;
//! use shekyl_prism::{
//!     verify_tagged, AcceptList, PublicKey, SetId, Signature, TaggedError, P248, P500,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // What a signer and a key's owner would publish: tagged bytes.
//! let vector = Primary::Lvl1.vector();
//! let key = PublicKey::<P248>::from_bytes(&vector.public_key()?)?.to_tagged();
//! let signature = Signature::<P248>::from_bytes(&vector.signature()?)?.to_tagged();
//! assert_eq!(key[..SetId::LEN], SetId::of::<P248>().to_bytes());
//!
//! // A verifier that accepts `p248` learns the set from the bytes.
//! let accept = AcceptList::new(&[SetId::of::<P248>()])?;
//! let set = verify_tagged(&accept, &key, vector.message()?, &signature)?;
//! assert_eq!(set, SetId::of::<P248>());
//!
//! // One that accepts `p500` alone refuses the same bytes, and does no
//! // arithmetic to decide it.
//! let accept = AcceptList::new(&[SetId::of::<P500>()])?;
//! assert_eq!(
//!     verify_tagged(&accept, &key, vector.message()?, &signature),
//!     Err(TaggedError::NotAccepted(SetId::of::<P248>()))
//! );
//! # Ok(())
//! # }
//! ```
//!
//! There is no default list and no list of every set: a system that
//! accepts more than one prime writes them out, and is then as strong as
//! the weakest it wrote. The identifier says which prime bytes are
//! under; it does not say the key's owner chose that prime, and it is
//! not part of what a signature covers. [`AcceptList`] states what
//! follows for a system that accepts more than one set.
//!
//! # The layers underneath
//!
//! [`hazmat`] re-exports the three crates this interface is built on:
//! field arithmetic, isogenies, and the verifier's steps. That is where
//! the algorithm can be read, measured and extended. Its functions assume
//! invariants that only this interface checks in full, and nothing there
//! is promised stable.
//!
//! # The `signer` feature
//!
//! Off by default, and a default build is the verifier alone: it compiles
//! no signer code and none of the signer's dependencies. With it,
//! [`hazmat`] also exposes the layers of the signer that exist. Today
//! those are the fixed-width integers, `hazmat::int`, and the ideal
//! layer, `hazmat::ideal`. The typed interface does not call them
//! yet. Signing and key generation will be behind this feature when they
//! are written.
//!
//! # What is missing
//!
//! - **Signing and key generation.** They will appear here as a
//!   `SecretKey<S>` beside [`PublicKey`], with `sign` and key generation
//!   on it. The repository's present signer, the `prism-signer` crate, is
//!   a research wrapper around the C reference implementation: it exists
//!   to produce test vectors and is not part of this interface.
//! - **A key or signature whose set is chosen at run time, as a value.**
//!   [`verify_tagged`] takes bytes and returns a verdict; a caller that
//!   keeps a tagged key and checks many signatures under it reads the
//!   identifier and uses [`PublicKey`] at that set.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod accept;
pub mod hazmat;
mod key;
mod set_id;
mod signature;
mod tagged;

pub use accept::{AcceptList, EmptyAcceptList};
pub use key::PublicKey;
pub use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
pub use prism_verify::VerifyError;
pub use set_id::{SetId, UnknownSetId};
pub use signature::Signature;
pub use tagged::{verify_tagged, Container, TaggedError};

/// Bytes as lower-case hex, for the `Debug` of the two types.
fn hex(bytes: &[u8], f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}
