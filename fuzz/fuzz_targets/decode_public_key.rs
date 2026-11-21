// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Public-key decode at every set. A bad encoding is a refusal, never a panic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use prism_field::{P248, P324, P376, P500, P664};
use prism_verify::identity::KeyIdentity;

fuzz_target!(|data: &[u8]| {
    let _ = KeyIdentity::<P248>::decode(data);
    let _ = KeyIdentity::<P324>::decode(data);
    let _ = KeyIdentity::<P376>::decode(data);
    let _ = KeyIdentity::<P500>::decode(data);
    let _ = KeyIdentity::<P664>::decode(data);
});
