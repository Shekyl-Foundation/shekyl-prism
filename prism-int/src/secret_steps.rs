// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Secret arithmetic for normalized Gaussian reduction.
use super::{is_zero, limbs, Scratch, SecretBit, SecretInt};
use prism_ct_barrier::mask_from_bit;

fn blend(current: u64, other: u64, mask: u64) -> u64 {
    current ^ ((current ^ other) & mask)
}

impl SecretBit {
    /// Conjunction. Either input's mark invalidates the result.
    pub fn and(&self, rhs: &Self) -> Self {
        Self {
            bit: self.bit & rhs.bit,
            overflowed: self.overflowed | rhs.overflowed,
        }
    }

    /// Disjunction. Either input's mark invalidates the result.
    pub fn or(&self, rhs: &Self) -> Self {
        Self {
            bit: self.bit | rhs.bit,
            overflowed: self.overflowed | rhs.overflowed,
        }
    }

    /// Complement, preserving the mark.
    pub fn not(&self) -> Self {
        Self {
            bit: self.bit ^ 1,
            overflowed: self.overflowed,
        }
    }
}

impl<const N: usize> SecretInt<N> {
    /// Permanently invalidate this value if the condition is set or itself invalid.
    pub fn mark_if(&mut self, condition: &SecretBit) {
        self.overflowed |= condition.bit | condition.overflowed;
    }

    /// Number of bits in the magnitude, zero for zero.
    pub fn bit_length(&self) -> SecretInt<1> {
        let mut length = Scratch::new(0_u64);
        for (i, limb) in self.limbs.iter().enumerate() {
            // Give bit-scan a nonzero operand even on targets without LZCNT.
            let bits = Scratch::new(64 - (limb | 1).leading_zeros());
            *length = blend(
                *length,
                64 * i as u64 + u64::from(*bits),
                mask_from_bit(u64::from(1 ^ is_zero(*limb))),
            );
        }
        SecretInt::from_parts(&[*length], 0, self.overflowed)
    }

    /// Absolute value.
    pub fn abs(&self) -> Self {
        Self::from_parts(&self.limbs, 0, self.overflowed)
    }

    /// Whether the integer is negative.
    pub fn is_negative(&self) -> SecretBit {
        SecretBit {
            bit: self.negative,
            overflowed: self.overflowed,
        }
    }

    /// Whether the integer is odd.
    pub fn is_odd(&self) -> SecretBit {
        SecretBit {
            bit: (self.limbs[0] & 1) as u8,
            overflowed: self.overflowed,
        }
    }

    /// Whether the low public number of bits (0 through 64) are all zero.
    pub fn low_bits_zero(&self, bits: u32) -> SecretBit {
        assert!(bits <= 64);
        let mask = ((1_u128 << bits) - 1) as u64;
        SecretBit {
            bit: is_zero(self.limbs[0] & mask),
            overflowed: self.overflowed,
        }
    }

    /// Low M limbs of the magnitude. Preserves an existing mark.
    pub fn truncate<const M: usize>(&self) -> SecretInt<M> {
        let mut value = SecretInt::zero();
        let common = M.min(N);
        value.limbs[..common].copy_from_slice(&self.limbs[..common]);
        value.overflowed = self.overflowed;
        value
    }

    /// Signed, exact width conversion. A discarded nonzero limb sets the mark.
    pub fn resize<const M: usize>(&self) -> SecretInt<M> {
        let mut value = self.truncate::<M>();
        let high = Scratch::new(self.limbs[M.min(N)..].iter().fold(0, |a, b| a | b));
        value.negative = self.negative & (1 ^ is_zero(limbs::fold(&value.limbs)));
        value.overflowed |= 1 ^ is_zero(*high);
        value
    }

    /// Low N limbs of the product of the magnitudes, preserving input marks.
    pub fn mul_low(&self, rhs: &Self) -> Self {
        let mut out = Self::zero();
        for (i, a) in self.limbs.iter().enumerate() {
            let mut carry = Scratch::new(0_u128);
            for (slot, b) in out.limbs[i..].iter_mut().zip(&rhs.limbs) {
                *carry += u128::from(*a) * u128::from(*b) + u128::from(*slot);
                *slot = *carry as u64;
                *carry >>= 64;
            }
        }
        out.overflowed = self.overflowed | rhs.overflowed;
        out
    }

    /// Signed multiplication by a one-limb integer with overflow detection.
    pub fn mul_word(&self, rhs: &SecretInt<1>) -> Self {
        let mut out = Scratch::new([0; N]);
        let mut carry = Scratch::new(0_u128);
        for (slot, a) in out.iter_mut().zip(self.limbs.iter()) {
            *carry += u128::from(*a) * u128::from(rhs.limbs[0]);
            *slot = *carry as u64;
            *carry >>= 64;
        }
        Self::from_parts(
            &out,
            self.negative ^ rhs.negative,
            self.overflowed | rhs.overflowed | (1 ^ is_zero(*carry as u64)),
        )
    }

    /// Multiply by a secret power of two. A negative count or a lost bit marks the result.
    pub fn shl_secret(&self, count: &SecretInt<1>) -> Self {
        self.shift::<true>(count)
    }

    /// Divide the magnitude by a secret power of two, rounding toward zero.
    /// Negative counts mark the result. Counts at least the width produce zero.
    pub fn shr_secret(&self, count: &SecretInt<1>) -> Self {
        self.shift::<false>(count)
    }

    fn shift<const LEFT: bool>(&self, count: &SecretInt<1>) -> Self {
        let mut words = Scratch::new(self.limbs);
        let amount = &count.limbs[0];
        let mut lost = Scratch::new(0_u64);
        for bit in 0..N.next_power_of_two().trailing_zeros() {
            let distance = 1_usize << bit;
            let mask = Scratch::new(mask_from_bit(amount >> (6 + bit)));
            if LEFT {
                for x in &words[N - distance..] {
                    *lost |= x & *mask;
                }
                for i in (0..N).rev() {
                    let src = Scratch::new(i.checked_sub(distance).map_or(0, |j| words[j]));
                    words[i] = blend(words[i], *src, *mask);
                }
            } else {
                for i in 0..N {
                    let src = Scratch::new(words.get(i + distance).copied().unwrap_or(0));
                    words[i] = blend(words[i], *src, *mask);
                }
            }
        }
        // Build 2^b with public shifts, then shift limbs by multiplying.
        let bits = Scratch::new(if LEFT {
            amount & 63
        } else {
            63 - (amount & 63)
        });
        let mut scale = Scratch::new(1_u64);
        for bit in 0..6 {
            *scale = blend(*scale, *scale << (1 << bit), mask_from_bit(*bits >> bit));
        }
        let mut carry = Scratch::new(0_u64);
        let too_far = Scratch::new(mask_from_bit(u64::from(*amount >= N as u64 * 64)));
        for j in 0..N {
            let i = if LEFT { j } else { N - 1 - j };
            let wide = Scratch::new(u128::from(words[i]) * u128::from(*scale));
            let part = Scratch::new(if LEFT {
                *wide as u64
            } else {
                (*wide >> 63) as u64
            });
            words[i] = (*part | *carry) & !*too_far;
            *carry = if LEFT {
                (*wide >> 64) as u64
            } else {
                (*wide as u64) << 1
            };
        }
        if LEFT {
            *lost |= *carry | (limbs::fold(&self.limbs) & *too_far);
        }
        Self::from_parts(
            &words,
            self.negative,
            self.overflowed | count.overflowed | count.negative | (1 ^ is_zero(*lost)),
        )
    }
}

impl SecretInt<2> {
    /// Unsigned 128-by-64 division in 64 rounds. Negative inputs, zero divisor,
    /// and a quotient wider than one limb mark the result.
    pub fn div_word(&self, divisor: &SecretInt<1>) -> SecretInt<1> {
        let d = &divisor.limbs[0];
        let mut rem = Scratch::new(self.limbs[1]);
        let mut q = Scratch::new(self.limbs[0]);
        for _ in 0..64 {
            let high = Scratch::new(*rem >> 63);
            *rem = (*rem << 1) | (*q >> 63);
            let take = Scratch::new(*high | u64::from(*rem >= *d));
            *rem = rem.wrapping_sub(d & mask_from_bit(*take));
            *q = (*q << 1) | *take;
        }
        SecretInt::from_parts(
            &[*q],
            0,
            self.overflowed
                | divisor.overflowed
                | self.negative
                | divisor.negative
                | u8::from(self.limbs[1] >= *d),
        )
    }
}
