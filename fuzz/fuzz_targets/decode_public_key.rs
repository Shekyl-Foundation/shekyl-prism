// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Public-key decode at every level. A bad encoding is a refusal, never a panic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use prism_field::{Level1, Level3, Level5};
use prism_verify::KeyIdentity;

fuzz_target!(|data: &[u8]| {
    let _ = KeyIdentity::<Level1>::decode(data);
    let _ = KeyIdentity::<Level3>::decode(data);
    let _ = KeyIdentity::<Level5>::decode(data);
});
