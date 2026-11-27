// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The bit comparison shared by the stage 2c example and `signer_layers`.
use prism_int::SecretBit;

pub(crate) fn check_bit(bit: SecretBit, expected: bool) {
    assert_eq!(bit.declassify(), Ok(expected));
}
