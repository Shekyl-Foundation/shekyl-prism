// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The ideal layer of the PRISM signer: stages 2b to 2d.
//!
//! **A prototype.** `docs/SIGNER_SPEC.md`, sections 2.2 to 2.4, specifies
//! this layer, and the table at the head of its section 2 gives the state
//! of each part. Nothing here claims constant time.
//!
//! - [`GenericHnf`] is the one column normal form of a norm and a root.
//! - [`GuardedIdeal::admit`] is admission: generation offers the reduced
//!   generators of that form, import offers its decoded columns, and both
//!   get the same verdict.
//! - [`GuardedIdeal::intersection`] is the guarded intersection with a
//!   challenge ideal.
//! - [`reduce`] is the reduction of a Gaussian pair.
//!
//! The crate depends on the whole verifier, `prism-verify`, and that is
//! intended. The challenge degree and its primality predicate are the
//! verifier's own functions, so the signer and the verifier cannot
//! disagree about which degrees are accepted. They are not copied here.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod admission;
pub mod geometry;
mod intersection;
mod modulus;
pub mod reduce;

pub use admission::{GenericHnf, GuardedIdeal};
pub use intersection::Intersection;

use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
use prism_int::{Int, SecretInt as Scalar};

/// Four O0 numerator coordinates, in order `(1, i, j, k)`.
pub type Vector<const W: usize> = [Scalar<W>; 4];

/// A Gaussian integer at the same limb width, real part then imaginary.
pub type Gaussian<const W: usize> = [Scalar<W>; 2];

/// Coordinate and constructor widths for ideal arithmetic at this public set.
pub trait IdealSet<const W: usize, const D: usize>: ParameterSet {}
impl IdealSet<11, 22> for P248 {}
impl IdealSet<14, 28> for P324 {}
impl IdealSet<16, 32> for P376 {}
impl IdealSet<21, 42> for P500 {}
impl IdealSet<27, 54> for P664 {}

/// The field prime `p = f·2^e − 1` at this set's coordinate width.
///
/// `W` and `D` are the set's [`IdealSet`] widths, so a caller cannot build
/// the prime in a width the set did not bind.
pub fn field_prime<S: IdealSet<W, D>, const W: usize, const D: usize>() -> Option<Int<W>> {
    Int::from_u64(S::COFACTOR.value())
        .shl(S::E)
        .ok()?
        .sub(&Int::one())
        .ok()
}
