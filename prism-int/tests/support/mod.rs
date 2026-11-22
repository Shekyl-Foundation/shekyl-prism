// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! What the oracle tests share: a fixed stream of operands, and the
//! comparison with `num-bigint`.

use num_bigint::{BigInt, Sign};
use prism_int::{Int, IntError};

/// A fixed stream of limbs (SplitMix64).
pub struct Stream(pub u64);

impl Stream {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    /// A limb that is often an edge: zero, one, all ones, the top bit.
    pub fn limb(&mut self) -> u64 {
        match self.below(8) {
            0 => 0,
            1 => u64::MAX,
            2 => 1,
            3 => 1 << 63,
            4 => u64::MAX - 1,
            _ => self.next(),
        }
    }

    /// A signed integer of up to `limbs` limbs, of a length drawn first.
    pub fn int<const N: usize>(&mut self, limbs: usize) -> Int<N> {
        let used = self.below(limbs as u64 + 1) as usize;
        let mut magnitude = vec![0u64; used];
        for limb in &mut magnitude {
            *limb = self.limb();
        }
        if used > 0 && self.below(2) == 0 {
            // A top limb of a random length, so that bit lengths vary.
            magnitude[used - 1] = self.next() >> self.below(64);
        }
        Int::from_le_limbs(&magnitude, self.below(2) == 0).unwrap()
    }
}

pub fn big<const N: usize>(value: &Int<N>) -> BigInt {
    let mut bytes = vec![0u8; 8 * N];
    value.to_le_bytes(&mut bytes).unwrap();
    let sign = if value.is_negative() {
        Sign::Minus
    } else {
        Sign::Plus
    };
    BigInt::from_bytes_le(sign, &bytes)
}

pub fn fits<const N: usize>(value: &BigInt) -> bool {
    value.bits() <= 64 * N as u64
}

/// The crate's result is the oracle's: the value when it fits, and
/// `Overflow` when it does not.
pub fn same<const N: usize>(got: Result<Int<N>, IntError>, want: &BigInt, what: &str) {
    match got {
        Ok(value) => assert_eq!(&big(&value), want, "{what} at {N} limbs"),
        Err(error) => {
            assert_eq!(error, IntError::Overflow, "{what} at {N} limbs");
            assert!(
                !fits::<N>(want),
                "{what} at {N} limbs: refused a result that fits"
            );
        }
    }
}
