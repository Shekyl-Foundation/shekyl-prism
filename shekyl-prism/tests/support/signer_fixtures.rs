// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Helpers shared by the example and tests on public signer fixtures.
use prism_int::{SecretBit, SecretError, SecretInt as Scalar};

pub(crate) fn check_bit(bit: SecretBit, expected: bool) {
    assert_eq!(bit.declassify(), Ok(expected));
}

pub(crate) fn check_invalid<const W: usize>(value: &Scalar<W>) {
    assert_eq!(value.declassify().unwrap_err(), SecretError::Overflowed);
}

pub(crate) fn magnitude(hex: &str) -> Vec<u8> {
    hex.trim_start_matches('-')
        .as_bytes()
        .rchunks(2)
        .map(|chunk| u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap())
        .collect()
}

pub(crate) fn scalar<const W: usize>(hex: &str) -> Scalar<W> {
    let x = Scalar::from_le_bytes(&magnitude(hex));
    if hex.starts_with('-') {
        x.neg()
    } else {
        x
    }
}
