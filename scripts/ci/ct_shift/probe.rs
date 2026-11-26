// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Release measurement harness. All inputs cross the C ABI by value.
#[allow(dead_code)] // The fixture-only half-tie entry is exercised by the native example.
#[path = "../../../shekyl-prism/examples/stage2c/reducer.rs"]
mod reducer;

#[cfg(stage2c_set = "p248")]
use prism_field::P248 as Selected;
#[cfg(stage2c_set = "p324")]
use prism_field::P324 as Selected;
#[cfg(stage2c_set = "p376")]
use prism_field::P376 as Selected;
#[cfg(stage2c_set = "p500")]
use prism_field::P500 as Selected;
#[cfg(stage2c_set = "p664")]
use prism_field::P664 as Selected;
use prism_int::SecretInt;
use reducer::ReductionSet;
use std::hint::black_box;

const LIMBS: usize = Selected::LIMBS;
type Scalar = SecretInt<LIMBS>;

#[no_mangle]
pub extern "C" fn prism_scalar_bytes() -> usize {
    LIMBS * 8
}

#[no_mangle]
pub extern "C" fn prism_shift_probe(input: u64, count: u64) {
    let x = Scalar::from_u64(input);
    let wide = black_box(x.shl(64 * (LIMBS as u32 - 2)).add(&x.shl(63)).add(&x));
    let k = SecretInt::from_u64(count);
    let left = wide.shl_secret(&k);
    let right = wide.shr_secret(&k);
    black_box((left, right, wide.bit_length()));
    let divisor = SecretInt::from_u64(input | 1);
    black_box(wide.truncate::<2>().div_word(&divisor));
}

/// Test-only C ABI, little-endian sign and magnitude.
#[repr(C)]
pub struct Encoded {
    magnitude: [u8; LIMBS * 8],
    negative: u8,
}

impl Encoded {
    fn read(&self) -> Scalar {
        let n = Scalar::from_le_bytes(&self.magnitude);
        let sign = SecretInt::<1>::from_u64(u64::from(self.negative)).is_odd();
        Scalar::select(&n, &n.neg(), &sign)
    }
}

#[repr(C)]
pub struct Input {
    norm: Encoded,
    u: [Encoded; 4],
    v: [Encoded; 4],
}

#[no_mangle]
pub extern "C" fn prism_reducer_probe(input: Input, guarded: u32) {
    let class = if guarded == 1 {
        reducer::Class::Guarded
    } else {
        reducer::Class::Generic
    };
    let basis = reducer::Basis::from_intersection(
        &input.norm.read(),
        input.u.each_ref().map(Encoded::read),
        input.v.each_ref().map(Encoded::read),
        class,
        &reducer::Bounds::new::<Selected>(),
    );
    let output = basis.reduce(reducer::Schedule::Reviewed);
    black_box((output.u, output.v, output.form));
}
