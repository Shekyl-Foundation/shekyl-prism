// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! What an operation refuses. No error carries a value.

use core::fmt;

/// Why an operation of the public integer [`Int`](crate::Int) returned no
/// value.
///
/// An error names what went wrong and holds nothing else: no operand and
/// no partial result. A public integer may be a declassified secret, and
/// an error is printed by whoever unwraps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntError {
    /// The value does not fit where it was to go: the `N` limbs of the
    /// result, or the buffer of bytes it was to be written to.
    Overflow,
    /// The divisor, or the modulus of a remainder, is zero.
    DivisionByZero,
    /// An exact division has a remainder.
    Inexact,
    /// The operand is negative where only a non-negative one has a
    /// meaning: a square root, an exponent.
    Negative,
    /// The modulus is not one the operation takes: not positive, not above
    /// one, even, or wider than half of the limbs. The operation's
    /// documentation says which of these it asks for.
    Modulus,
    /// A count of bits is above what the operation takes.
    Count,
    /// A loop reached the number of steps its width allows and had not
    /// ended. No operand does this: the bound is derived from the width,
    /// in the documentation of the function that returns it. It is
    /// returned so that no loop of this crate runs on the word of its own
    /// arithmetic, and seeing it means that arithmetic is wrong.
    StepBound,
}

impl fmt::Display for IntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Overflow => "the value does not fit",
            Self::DivisionByZero => "division by zero",
            Self::Inexact => "the division is not exact",
            Self::Negative => "the operand is negative",
            Self::Modulus => "the modulus is not one this operation takes",
            Self::Count => "the count of bits is too large",
            Self::StepBound => "a loop reached the bound of its steps",
        })
    }
}

impl std::error::Error for IntError {}

/// Why an exit of the secret types returned no value.
///
/// To be told either of these is to learn something about a secret. Each
/// call site of an exit has a row in the secret-treatment table of
/// `docs/SIGNER_SPEC.md` (2.1.3) that says why the error cannot happen
/// there, or that this is not yet shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretError {
    /// A previous operation exceeded its range or a secret admission predicate marked the result.
    Overflowed,
    /// [`SecretInt::export_le_bytes`](crate::SecretInt::export_le_bytes)
    /// was given a buffer shorter than the magnitude.
    DoesNotFit,
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Overflowed => "a secret computation overflowed",
            Self::DoesNotFit => "the secret does not fit the buffer",
        })
    }
}

impl std::error::Error for SecretError {}
