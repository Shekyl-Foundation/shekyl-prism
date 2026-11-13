// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! `square` against `mul` by itself, in both fields at every level.
//!
//! The input is cut into field encodings. One that is not canonical has
//! the top byte of each base-field half cleared, which is below `p` at
//! every level, so every input reaches the comparison.

#![no_main]

use libfuzzer_sys::fuzz_target;
use prism_field::{Field, Level, Level1, Level3, Level5};

fn level<L: Level>(data: &[u8]) {
    for chunk in data.chunks_exact(L::FP2_LEN) {
        let mut bytes = chunk.to_vec();
        let x = L::ext_decode(&bytes).unwrap_or_else(|| {
            bytes[L::FP_LEN - 1] = 0;
            bytes[L::FP2_LEN - 1] = 0;
            L::ext_decode(&bytes).expect("below 2^(8(len-1)) in both halves")
        });
        assert!(x.square().same(x.mul(x)));
        let (re, im) = L::ext_parts(x);
        assert!(re.square().same(re.mul(re)));
        assert!(im.square().same(im.mul(im)));
    }
}

fuzz_target!(|data: &[u8]| {
    level::<Level1>(data);
    level::<Level3>(data);
    level::<Level5>(data);
});
