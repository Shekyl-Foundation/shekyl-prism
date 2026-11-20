// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The mark a field implementation leaves at each counted operation.
//!
//! Private to the crate, and always compiled, so that the implementations
//! tally unconditionally and no call site carries a `cfg`. Without the
//! `op-count` feature [`tally`] is empty and a default build has no
//! counter. With it, the count goes to the table in `ops`, the public
//! module that exists only then. [`Kind`] and [`Op`] stay private to the
//! crate either way: a caller of `ops` reads counts by field and by
//! operation and names neither.

/// Which field an operation was in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `F_p`.
    Base,
    /// `F_{p²}`.
    Ext,
}

/// A counted operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `Field::mul`.
    Mul,
    /// `Field::square`.
    Square,
    /// `Field::inv`.
    Inv,
    /// `Field::sqrt`.
    Sqrt,
    /// `Field::is_square`.
    IsSquare,
}

/// Count one operation, if this build counts.
#[inline(always)]
pub(crate) fn tally(kind: Kind, op: Op) {
    #[cfg(feature = "op-count")]
    crate::ops::tally(kind, op);
    #[cfg(not(feature = "op-count"))]
    let _ = (kind, op);
}
