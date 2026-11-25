// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Release measurement harness. All inputs cross the C ABI by value.
use prism_int::SecretInt;
use std::hint::black_box;

#[no_mangle]
pub extern "C" fn prism_shift_probe(input: u64, count: u64) {
    type Scalar = SecretInt<21>;
    let x = Scalar::from_u64(input);
    let wide = x.shl(1216).add(&x.shl(63)).add(&x);
    let k = SecretInt::from_u64(count);
    let left = wide.shl_secret(&k);
    let right = wide.shr_secret(&k);
    black_box((left, right, wide.bit_length()));
    let divisor = SecretInt::from_u64(input | 1);
    black_box(wide.truncate::<2>().div_word(&divisor));
}
