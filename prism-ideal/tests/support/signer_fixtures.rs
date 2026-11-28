// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Fixture exits shared by the stage 2c example, its replay, and `signer_layers`.
use prism_int::{SecretBit, SecretError, SecretInt};

pub(crate) fn check_bit(bit: SecretBit, expected: bool) {
    assert_eq!(bit.declassify(), Ok(expected));
}

/// A marked coordinate is the refusal the fixture asked for.
// CLIPPY: dead_code. signer_layers includes this file for check_bit and
// does not call this function. The ideal crate and the stage 2c example do.
#[allow(dead_code)]
pub(crate) fn check_invalid<const W: usize>(value: &SecretInt<W>) {
    assert_eq!(value.declassify().unwrap_err(), SecretError::Overflowed);
}
