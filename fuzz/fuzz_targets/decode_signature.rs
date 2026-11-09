// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Signature decode at every level. A bad encoding is a refusal, never a panic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use prism_field::{Level1, Level3, Level5};
use prism_verify::Response;

fuzz_target!(|data: &[u8]| {
    let _ = Response::<Level1>::decode(data);
    let _ = Response::<Level3>::decode(data);
    let _ = Response::<Level5>::decode(data);
});
