// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Division of the public integer, in the five roundings the signer uses.
//!
//! Variable time, all of it: the quotient digits are estimated and
//! corrected by loops whose length depends on the operands.

use core::cmp::Ordering;

use zeroize::Zeroizing;

use crate::error::IntError;
use crate::limbs::{self, LIMB_BITS};
use crate::public::{cmp_magnitude, significant, Int};

/// The dividend while it is being divided: `N` limbs and one more above
/// them, which the normalizing shift can fill. Both are wiped on drop.
struct Dividend<const N: usize> {
    low: Zeroizing<[u64; N]>,
    top: Zeroizing<u64>,
}

impl<const N: usize> Dividend<N> {
    fn get(&self, index: usize) -> u64 {
        if index < N {
            self.low[index]
        } else {
            *self.top
        }
    }

    fn set(&mut self, index: usize, limb: u64) {
        if index < N {
            self.low[index] = limb;
        } else {
            *self.top = limb;
        }
    }
}

/// The quotient and the remainder of two magnitudes, `b` not zero.
///
/// Schoolbook division by limbs (Knuth, volume 2, 4.3.1, algorithm D).
/// The divisor is shifted until its top limb has its top bit set; each
/// quotient limb is estimated from the top two limbs of what is left of
/// the dividend and the top limb of the divisor, lowered at most twice
/// while the next limb shows it too large, and lowered once more if the
/// subtraction still goes below zero.
pub(crate) fn div_rem_magnitude<const N: usize>(
    a: &[u64; N],
    b: &[u64; N],
) -> ([u64; N], [u64; N]) {
    let divisor_limbs = significant(b);
    let dividend_limbs = significant(a);
    debug_assert!(divisor_limbs > 0, "the caller refuses a zero divisor");
    let mut quotient = [0u64; N];
    if dividend_limbs < divisor_limbs {
        return (quotient, *a);
    }
    if divisor_limbs == 1 {
        let divisor = u128::from(b[0]);
        let mut rest = 0u128;
        for i in (0..dividend_limbs).rev() {
            let current = (rest << LIMB_BITS) + u128::from(a[i]);
            quotient[i] = (current / divisor) as u64;
            rest = current % divisor;
        }
        let mut remainder = [0u64; N];
        remainder[0] = rest as u64;
        return (quotient, remainder);
    }

    let shift = b[divisor_limbs - 1].leading_zeros();
    let divisor = Zeroizing::new(limbs::shl(b, shift).0);
    let mut dividend = Dividend {
        low: Zeroizing::new(limbs::shl(a, shift).0),
        // What the shift pushed out of the top limb.
        top: Zeroizing::new(((u128::from(a[N - 1]) << shift) >> LIMB_BITS) as u64),
    };
    let top = u128::from(divisor[divisor_limbs - 1]);
    let next = u128::from(divisor[divisor_limbs - 2]);

    for j in (0..=dividend_limbs - divisor_limbs).rev() {
        let high = (u128::from(dividend.get(j + divisor_limbs)) << LIMB_BITS)
            + u128::from(dividend.get(j + divisor_limbs - 1));
        let third = u128::from(dividend.get(j + divisor_limbs - 2));
        let mut estimate = high / top;
        let mut rest = high % top;
        // The estimate is never too small and at most two too large. It is
        // lowered while it has 65 bits or the divisor's next limb shows it
        // too large, and each lowering adds the divisor's top limb, at
        // least 2^63, to `rest`: after two, `rest` has left its limb and
        // the test has nothing more to say. So the loop is counted, and a
        // third round cannot happen. The product is formed only when the
        // estimate is below 2^64, so nothing here wraps.
        for _ in 0..2 {
            if estimate >> LIMB_BITS == 0 && estimate * next <= (rest << LIMB_BITS) + third {
                break;
            }
            estimate -= 1;
            rest += top;
            if rest >> LIMB_BITS != 0 {
                break;
            }
        }

        // dividend[j ..= j + divisor_limbs] −= estimate · divisor.
        let mut carry = 0u128;
        let mut borrow = 0u64;
        for i in 0..divisor_limbs {
            let product = estimate * u128::from(divisor[i]) + carry;
            carry = product >> LIMB_BITS;
            let difference = u128::from(dividend.get(j + i))
                .wrapping_sub(u128::from(product as u64))
                .wrapping_sub(u128::from(borrow));
            dividend.set(j + i, difference as u64);
            borrow = ((difference >> LIMB_BITS) as u64) & 1;
        }
        let difference = u128::from(dividend.get(j + divisor_limbs))
            .wrapping_sub(carry)
            .wrapping_sub(u128::from(borrow));
        dividend.set(j + divisor_limbs, difference as u64);
        let below_zero = ((difference >> LIMB_BITS) as u64) & 1;

        if below_zero == 1 {
            // One too many: add the divisor back.
            estimate -= 1;
            let mut carry = 0u64;
            for i in 0..divisor_limbs {
                let sum =
                    u128::from(dividend.get(j + i)) + u128::from(divisor[i]) + u128::from(carry);
                dividend.set(j + i, sum as u64);
                carry = (sum >> LIMB_BITS) as u64;
            }
            let restored = dividend.get(j + divisor_limbs).wrapping_add(carry);
            dividend.set(j + divisor_limbs, restored);
        }
        quotient[j] = estimate as u64;
    }
    // What is left is below the divisor, so it has no limb at `N`.
    (quotient, limbs::shr(&dividend.low, shift))
}

impl<const N: usize> Int<N> {
    /// The quotient rounded toward zero and the remainder, which has the
    /// sign of `self`: `self = q·divisor + r` with `|r| < |divisor|`.
    ///
    /// This is the reference's `ibz_div` (GMP's `tdiv`).
    ///
    /// # Errors
    ///
    /// [`IntError::DivisionByZero`] if `divisor` is zero.
    pub fn div_rem_trunc(&self, divisor: &Self) -> Result<(Self, Self), IntError> {
        if divisor.is_zero() {
            return Err(IntError::DivisionByZero);
        }
        let (quotient, remainder) = div_rem_magnitude(self.magnitude(), divisor.magnitude());
        Ok((
            Self::from_parts(quotient, self.is_negative() != divisor.is_negative()),
            Self::from_parts(remainder, self.is_negative()),
        ))
    }

    /// The quotient rounded down and the remainder, which has the sign of
    /// `divisor`: `self = q·divisor + r` with `|r| < |divisor|`.
    ///
    /// This is the reference's `ibz_div_floor`.
    ///
    /// # Errors
    ///
    /// [`IntError::DivisionByZero`] if `divisor` is zero.
    pub fn div_rem_floor(&self, divisor: &Self) -> Result<(Self, Self), IntError> {
        let (quotient, remainder) = self.div_rem_trunc(divisor)?;
        if remainder.is_zero() || remainder.is_negative() == divisor.is_negative() {
            return Ok((quotient, remainder));
        }
        Ok((quotient.sub(&Self::one())?, remainder.add(divisor)?))
    }

    /// The remainder in `[0, |modulus|)`, whatever the signs.
    ///
    /// This is the reference's `ibz_mod` (GMP's `mpz_mod`).
    ///
    /// # Errors
    ///
    /// [`IntError::DivisionByZero`] if `modulus` is zero.
    pub fn rem_euclid(&self, modulus: &Self) -> Result<Self, IntError> {
        let (_, remainder) = self.div_rem_trunc(modulus)?;
        if remainder.is_negative() {
            return remainder.add(&modulus.abs());
        }
        Ok(remainder)
    }

    /// The remainder in `(−modulus/2, modulus/2]`, for a positive modulus:
    /// of the two residues at the same distance from zero, the positive
    /// one.
    ///
    /// This is the reference's `ibz_centered_mod`.
    ///
    /// # Errors
    ///
    /// [`IntError::Modulus`] if `modulus` is not positive.
    pub fn rem_centred(&self, modulus: &Self) -> Result<Self, IntError> {
        if modulus.is_zero() || modulus.is_negative() {
            return Err(IntError::Modulus);
        }
        let remainder = self.rem_euclid(modulus)?;
        // remainder > modulus − remainder, which is 2·remainder > modulus
        // without the doubling.
        let rest = modulus.sub(&remainder)?;
        if remainder > rest {
            return Ok(rest.neg());
        }
        Ok(remainder)
    }

    /// The integer nearest `self / divisor`. When two are equally near,
    /// the one nearer zero.
    ///
    /// This is the reference's `ibz_rounded_div`: the quotient rounded
    /// toward zero, moved one away from zero only when twice the
    /// remainder is strictly more than the divisor.
    ///
    /// # Errors
    ///
    /// [`IntError::DivisionByZero`] if `divisor` is zero.
    pub fn div_round(&self, divisor: &Self) -> Result<Self, IntError> {
        let (quotient, remainder) = self.div_rem_trunc(divisor)?;
        // |r| > |divisor| − |r|, which is 2·|r| > |divisor|.
        let rest = limbs::sub(divisor.magnitude(), remainder.magnitude()).0;
        if cmp_magnitude(remainder.magnitude(), &rest) != Ordering::Greater {
            return Ok(quotient);
        }
        let away = Self::from_parts(
            *Self::one().magnitude(),
            self.is_negative() != divisor.is_negative(),
        );
        quotient.add(&away)
    }

    /// `self / divisor`, when that is an integer.
    ///
    /// # Errors
    ///
    /// [`IntError::DivisionByZero`] if `divisor` is zero, and
    /// [`IntError::Inexact`] if the division has a remainder.
    pub fn div_exact(&self, divisor: &Self) -> Result<Self, IntError> {
        let (quotient, remainder) = self.div_rem_trunc(divisor)?;
        if !remainder.is_zero() {
            return Err(IntError::Inexact);
        }
        Ok(quotient)
    }
}
