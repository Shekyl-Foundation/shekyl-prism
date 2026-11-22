// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Per-set widths, sized from public data only: bits(T) <= bits(q) + B,
//! bits(q) = 2e - 5, B = ceil(L/2) + 32, L = bits(p). C = ceil((bits(T)+3)/64)
//! coordinate limbs, N = 2C norm limbs, PL = limbs of p. Step counts are the
//! Python model's (REPORT.md section 4, secret product, sized from B).

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

pub struct SetInfo {
    pub name: &'static str,
    pub e: u32,
    pub f: u64,
    pub steps: usize,
    pub c: usize,
    pub n: usize,
    pub pl: usize,
}

pub const SETS: [SetInfo; 5] = [
    SetInfo { name: "p248", e: 248, f: 5, steps: 189, c: 11, n: 22, pl: 4 },
    SetInfo { name: "p324", e: 324, f: 3, steps: 239, c: 14, n: 28, pl: 6 },
    SetInfo { name: "p376", e: 376, f: 65, steps: 283, c: 16, n: 32, pl: 6 },
    SetInfo { name: "p500", e: 500, f: 27, steps: 359, c: 21, n: 42, pl: 8 },
    SetInfo { name: "p664", e: 664, f: 17, steps: 465, c: 27, n: 54, pl: 11 },
];

impl SetInfo {
    pub fn p(&self) -> BigInt {
        (BigInt::from(self.f) << self.e) - 1
    }
    pub fn t_bits_max(&self) -> u64 {
        let l = self.p().bits();
        (2 * self.e as u64 - 5) + (l + 1) / 2 + 32
    }
}

pub fn to_limbs<const L: usize>(a: &BigInt) -> [u64; L] {
    // two's complement, L limbs
    let m = BigInt::from(1u8) << (64 * L);
    let mut v = a % &m;
    if v.is_negative() {
        v += &m;
    }
    let mut r = [0u64; L];
    let mask = BigInt::from(u64::MAX);
    for i in 0..L {
        r[i] = ((&v >> (64 * i)) & &mask).to_u64().unwrap();
    }
    r
}

pub fn from_limbs_unsigned<const L: usize>(a: &[u64; L]) -> BigInt {
    let mut v = BigInt::zero();
    for i in (0..L).rev() {
        v = (v << 64) + BigInt::from(a[i]);
    }
    v
}

pub fn from_limbs_signed<const L: usize>(a: &[u64; L]) -> BigInt {
    let v = from_limbs_unsigned(a);
    if a[L - 1] >> 63 == 1 {
        v - (BigInt::from(1u8) << (64 * L))
    } else {
        v
    }
}

/// Dispatch a body generic over (C, N, PL) to the set's constants.
#[macro_export]
macro_rules! with_set {
    ($name:expr, $body:ident, $($arg:expr),*) => {
        match $name {
            "p248" => $body::<11, 22, 4>($($arg),*),
            "p324" => $body::<14, 28, 6>($($arg),*),
            "p376" => $body::<16, 32, 6>($($arg),*),
            "p500" => $body::<21, 42, 8>($($arg),*),
            "p664" => $body::<27, 54, 11>($($arg),*),
            _ => panic!("unknown set"),
        }
    };
}
