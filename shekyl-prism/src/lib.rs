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
//! policy and is not encoded in this crate. [`ParameterSet`] is the trait
//! they share; a user of this interface needs it to name a set in a
//! generic function and to read a set's sizes, and
//! [`PublicKey::LEN`] and [`Signature::LEN`] give the two sizes most
//! callers want without it.
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
//! # The layers underneath
//!
//! [`hazmat`] re-exports the three crates this interface is built on:
//! field arithmetic, isogenies, and the verifier's steps. That is where
//! the algorithm can be read, measured and extended. Its functions assume
//! invariants that only this interface checks in full, and nothing there
//! is promised stable.
//!
//! # What is missing
//!
//! - **Signing and key generation.** They will appear here as a
//!   `SecretKey<S>` beside [`PublicKey`], with `sign` and key generation
//!   on it. The repository's present signer, the `prism-signer` crate, is
//!   a research wrapper around the C reference implementation: it exists
//!   to produce test vectors and is not part of this interface.
//! - **Choosing a set at run time.** A set identifier, containers that
//!   carry it, and a list of accepted sets are the next part of this
//!   interface.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod hazmat;
mod key;
mod signature;

pub use key::PublicKey;
pub use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
pub use prism_verify::VerifyError;
pub use signature::Signature;

/// Bytes as lower-case hex, for the `Debug` of the two types.
fn hex(bytes: &[u8], f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}
