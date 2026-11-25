// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Fixed-width signed integers for the PRISM signer.
//!
//! The signer's quaternion arithmetic runs on integers far wider than a
//! field element: the coordinates of quaternions, the entries of lattice
//! bases, norms. The C reference holds them in integers that grow to fit.
//! These do not: an integer has `N` limbs of 64 bits, `N` is chosen once
//! for a parameter set, and a result that does not fit is an error the
//! caller sees. `docs/SIGNER_SPEC.md`, section 2.1, derives the widths
//! and states every convention used here.
//!
//! # Two types
//!
//! - [`Int`] is a **public** integer. It has division, greatest common
//!   divisors, square roots and the rest, and the time they take depends
//!   on the values.
//! - [`SecretInt`] is a **secret** integer. It has only the operations
//!   whose work depends on `N` alone: addition, subtraction, negation,
//!   multiplication, shifts by a public count, selection, and comparisons
//!   that return a [`SecretBit`] and not a `bool`.
//!
//! A secret leaves by one of three functions and no other way:
//! [`SecretInt::declassify`], [`SecretBit::declassify`] and
//! [`SecretInt::export_le_bytes`]. Each call site of one of them outside
//! this crate has a row in the secret-treatment table of the
//! specification, and CI compares the two counts.
//!
//! # Overflow
//!
//! A public operation that overflows returns [`IntError::Overflow`]. A
//! secret operation cannot return an error without showing that it
//! happened, so it sets a mark that every later operation passes on, to
//! secret bits as well, and the three exits refuse a marked value with
//! [`SecretError::Overflowed`].
//!
//! # What is not claimed
//!
//! **This crate does not claim constant time.** The secret type's
//! operations are written without a branch or an index that depends on a
//! secret limb. Whether the compiled code keeps that shape has not been
//! measured. Research code, unreviewed, and not for protecting anything of
//! value.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod division;
mod error;
mod limbs;
mod number;
mod public;
mod secret;

pub use error::{IntError, SecretError};
pub use public::Int;
pub use secret::{SecretBit, SecretInt};

/// Counts the wipes of the current thread, for the tests that show a drop
/// wipes. In no library build.
#[cfg(test)]
pub(crate) mod wipes {
    use std::cell::Cell;

    thread_local! {
        static WIPES: Cell<usize> = const { Cell::new(0) };
    }

    /// One more wipe.
    pub(crate) fn count() {
        WIPES.with(|wipes| wipes.set(wipes.get() + 1));
    }

    /// The wipes so far.
    pub(crate) fn seen() -> usize {
        WIPES.with(Cell::get)
    }
}
