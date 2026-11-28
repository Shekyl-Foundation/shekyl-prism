// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The layers under the interface: field arithmetic, isogenies, and the
//! verifier's steps, and the signer's integer and ideal layers.
//!
//! This is where the algorithm can be read, measured and extended. The
//! first three modules are the three layer crates of the verifier, whole:
//!
//! - [`field`] (`prism-field`): the base and quadratic fields of each
//!   parameter set, their encodings, and the [`ParameterSet`] trait;
//! - [`isogeny`] (`prism-isogeny`): Montgomery curves, torsion bases, the
//!   Weil pairing, and the theta-model isogeny chains;
//! - [`verify`] (`prism-verify`): decoding, the public checks, the
//!   challenge, and the relation, as separate steps, and the two entries
//!   that run them.
//!
//! The signer layers are here only when this crate is built with its `signer`
//! feature, which is off by default. A default build is the verifier and
//! holds no signer code and none of its dependencies. The signer is being
//! written from the bottom up and does not sign yet; its current layers are:
//!
//! - `int` (`prism-int`): the fixed-width signed integers of the
//!   signer's quaternion arithmetic, as a public type and a secret type.
//!   `ideal` calls it. It makes no constant-time claim; its own
//!   documentation says what the secret type does and does not promise.
//! - `ideal`: checked integral left-ideal components, the generic
//!   column-HNF of a norm and a root, and guarded challenge intersections.
//!
//! # What using this module means
//!
//! **These functions assume invariants that only the interface checks in
//! full.** [`crate::PublicKey`] and [`crate::Signature`] cannot hold a
//! value that failed a check. The functions here take field elements,
//! points and decoded responses as plain values, and many are correct
//! only for values that an earlier step has checked: a point that is on
//! the curve, a response whose sign is the normalized one, a key that is
//! not the starting curve. Each function's own documentation says what it
//! assumes, what it returns when the assumption fails, and which call
//! establishes it. In `prism-field` and `prism-isogeny` the call named is
//! the verifier's own check; in `prism-verify` it is the function of that
//! crate to call first. The interface's calls are those functions:
//! [`crate::PublicKey::from_bytes`] is
//! [`verify::prepared::PreparedKey::prepare`],
//! [`crate::Signature::from_bytes`] is
//! [`verify::prepared::checked_response`], and
//! [`crate::PublicKey::verify`] is [`verify::prepared::verify_prepared`].
//!
//! **Nothing here is promised stable.** These crates are changed as the
//! research needs. A name, a signature or a module can move between any
//! two versions, including while the interface above stays as it is.
//!
//! [`verify::verify`], the entry that takes three byte strings, checks
//! everything itself, and so does [`verify::set::verify_set`], which is
//! the same function for a set named at run time. Nothing else here
//! does. The typed interface accepts exactly what they accept; the crate
//! documentation of [`verify`] has the table of checks, and
//! `docs/VERIFIER_SPEC.md` in the repository states how the refusals of
//! the two correspond.
//!
//! [`ParameterSet`]: field::ParameterSet

pub use prism_field as field;
/// The signer's fixed-width integers, `prism-int`. Here only with the
/// `signer` feature.
#[cfg(feature = "signer")]
pub use prism_int as int;
pub use prism_isogeny as isogeny;
pub use prism_verify as verify;

/// The signer's ideal layer, `prism-ideal`: admission, the guarded
/// intersection and the reduction of a Gaussian pair, in O0 numerator
/// coordinates. Here only with the `signer` feature.
#[cfg(feature = "signer")]
pub use prism_ideal as ideal;
