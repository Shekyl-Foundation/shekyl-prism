// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The layers under the interface: field arithmetic, isogenies, and the
//! verifier's steps.
//!
//! This is where the algorithm can be read, measured and extended. The
//! three modules are the three layer crates of the repository, whole:
//!
//! - [`field`] (`prism-field`): the base and quadratic fields of each
//!   parameter set, their encodings, and the [`ParameterSet`] trait;
//! - [`isogeny`] (`prism-isogeny`): Montgomery curves, torsion bases, the
//!   Weil pairing, and the theta-model isogeny chains;
//! - [`verify`] (`prism-verify`): decoding, the public checks, the
//!   challenge, and the relation, as separate steps, and the two entries
//!   that run them.
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
//! assumes; where it names a check, the interface call that establishes
//! it is named with it. That documentation is not yet complete in the
//! three crates: until it is, read a function's callers before relying
//! on it with a value the interface did not make.
//!
//! **Nothing here is promised stable.** These crates are changed as the
//! research needs. A name, a signature or a module can move between any
//! two versions, including while the interface above stays as it is.
//!
//! [`verify::verify`], the entry that takes three byte strings, is the
//! one function here that checks everything itself. The typed interface
//! accepts exactly what it accepts; `docs/VERIFIER_SPEC.md` in the
//! repository states how their refusals correspond.
//!
//! [`ParameterSet`]: field::ParameterSet

pub use prism_field as field;
pub use prism_isogeny as isogeny;
pub use prism_verify as verify;
