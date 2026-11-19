// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Test inputs for a PRISM verifier or signer.
//!
//! Someone who changes the verifier, or writes a signer, needs inputs that
//! must be refused as much as inputs that must be accepted. This crate
//! builds them, and one dependency on it gives all of them:
//!
//! - [`negatives`]: every input the records derive from an honest
//!   committed vector that a verifier must refuse. There are 2,742 over
//!   the six primary vectors, each with a name and the refusals it may
//!   meet;
//! - [`identity_response`]: a response built from a public key alone that
//!   passes the pairing check and is refused after it, at any of the five
//!   parameter sets, with no secret and no committed signature;
//! - [`refusal_vectors`]: the rows of the committed refusal file of
//!   `prism-vectors`, built here, so the file is never edited by hand;
//! - [`vector`], [`set`], [`files`] and [`primary`]: the committed honest
//!   vectors as the verifier's types;
//! - [`error_of`]: the verifier's error for a verdict of the committed
//!   refusal file.
//!
//! # What this crate is not
//!
//! It is not a verifier. It builds inputs with curve arithmetic and the
//! challenge hash, and it carries the expected refusal of each input as
//! data ([`Case::allowed`], [`Form::allowed`]). It never asks a verifier
//! what the verdict is. The verifier is what a test runs on these inputs.
//!
//! It is not published, and it is not part of any library build: it
//! depends on `prism-vectors`, which holds test data. Take it as a
//! dev-dependency.
//!
//! # Where it sits
//!
//! It depends on `prism-field`, `prism-isogeny`, `prism-verify` and
//! `prism-vectors`, and on nothing above them. The front crate
//! `shekyl-prism` takes it as a dev-dependency; the tests that run both
//! of the verifier's entries on these inputs live there. `prism-verify`
//! cannot take it, because the builders use the verifier's own types.
//!
//! # Example
//!
//! The negatives of one committed vector. Each is a public key, a message
//! and a signature, with the refusals a verifier may return for them.
//!
//! ```
//! use prism_field::P248;
//! use prism_testkit::negatives;
//! use prism_vectors::corpus::Primary;
//!
//! let cases = negatives::<P248>(Primary::Lvl1.text());
//! assert_eq!(cases.len(), 329);
//!
//! let first = &cases[0];
//! assert_eq!(first.name, "both-y-negated");
//! assert_eq!(
//!     prism_verify::verify::<P248>(&first.key, &first.message, &first.signature),
//!     Err(first.allowed[0])
//! );
//! ```
//!
//! # Tools
//!
//! Three examples of this crate use the builders outside a test:
//!
//! - `op_counts` prints the operation-count records
//!   (`scripts/op_counts.sh` runs it);
//! - `verify_once` runs one verification for a memory profiler
//!   (`scripts/peak_heap.sh` runs it);
//! - `identity_refusals` prints the committed refusal file.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod identity;
mod negatives;
pub mod refusal_vectors;
mod vectors;
mod verdict;

pub use identity::{first_prime_salt_counter, identity_response, salt_of, Form, IdentityResponse};
pub use negatives::{negatives, Case};
pub use vectors::{files, primary, set, vector};
pub use verdict::error_of;
