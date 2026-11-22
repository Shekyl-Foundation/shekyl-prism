// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! What the public integer builds on division: greatest common divisors,
//! inverses, square roots, the Jacobi symbol, and arithmetic modulo an odd
//! number. Variable time, all of it.

use crate::error::IntError;
use crate::public::Int;

impl<const N: usize> Int<N> {
    /// The greatest common divisor, which is not negative. `gcd(0, 0)` is
    /// zero.
    pub fn gcd(&self, other: &Self) -> Self {
        let mut a = self.abs();
        let mut b = other.abs();
        while !b.is_zero() {
            let (_, remainder) = crate::division::div_rem_magnitude(a.magnitude(), b.magnitude());
            a = b;
            b = Self::from_parts(remainder, false);
        }
        a
    }

    /// `(g, u, v)` with `g = gcd(self, other)` and `g = u·self + v·other`.
    ///
    /// The cofactors are those of the Euclidean algorithm run on the two
    /// magnitudes in the order given, each then given the sign of its
    /// operand. So when neither operand is zero, `|u| ≤ |other|/g` and
    /// `|v| ≤ |self|/g`. When `other` is zero they are `(±1, 0)`, when
    /// `self` is zero `(0, ±1)`, and when both are, `(0, 0)`.
    ///
    /// The reference normalizes its cofactors in another way
    /// (`ibz_xgcd_with_u_not_0`). Nothing compared with it depends on
    /// them: `docs/SIGNER_SPEC.md`, 2.1.4.
    ///
    /// # Errors
    ///
    /// None is expected: every intermediate is bounded by the operands.
    /// [`IntError::Overflow`] is passed on if one did not fit.
    pub fn xgcd(&self, other: &Self) -> Result<(Self, Self, Self), IntError> {
        let mut r0 = self.abs();
        let mut r1 = other.abs();
        let mut s0 = if r0.is_zero() && r1.is_zero() {
            Self::zero()
        } else {
            Self::one()
        };
        let mut s1 = Self::zero();
        let mut t0 = Self::zero();
        let mut t1 = Self::one();
        while !r1.is_zero() {
            let (quotient, remainder) = r0.div_rem_trunc(&r1)?;
            let s2 = s0.sub(&quotient.mul(&s1)?)?;
            let t2 = t0.sub(&quotient.mul(&t1)?)?;
            r0 = r1;
            r1 = remainder;
            s0 = s1;
            s1 = s2;
            t0 = t1;
            t1 = t2;
        }
        let u = if self.is_negative() { s0.neg() } else { s0 };
        let v = if other.is_negative() { t0.neg() } else { t0 };
        Ok((r0, u, v))
    }

    /// The inverse of `self` modulo `modulus`, in `[1, modulus)`, or `None`
    /// when `self` and `modulus` have a common factor.
    ///
    /// The reference's `ibz_invmod` returns 0 where this returns `None`.
    ///
    /// # Errors
    ///
    /// [`IntError::Modulus`] if `modulus` is not above one.
    pub fn inv_mod(&self, modulus: &Self) -> Result<Option<Self>, IntError> {
        if modulus.is_negative() || *modulus <= Self::one() {
            return Err(IntError::Modulus);
        }
        let residue = self.rem_euclid(modulus)?;
        let (g, u, _) = residue.xgcd(modulus)?;
        if g != Self::one() {
            return Ok(None);
        }
        Ok(Some(u.rem_euclid(modulus)?))
    }

    /// `⌊√self⌋`.
    ///
    /// # Errors
    ///
    /// [`IntError::Negative`] if `self` is negative.
    pub fn sqrt_floor(&self) -> Result<Self, IntError> {
        if self.is_negative() {
            return Err(IntError::Negative);
        }
        if self.is_zero() {
            return Ok(Self::zero());
        }
        // Newton's iteration from a power of two that is not below the
        // root: it falls until it reaches the floor, and then does not.
        let mut root = Self::one().shl(self.bit_length().div_ceil(2))?;
        loop {
            let (quotient, _) = self.div_rem_trunc(&root)?;
            let next = root.add(&quotient)?.shr(1);
            if next >= root {
                return Ok(root);
            }
            root = next;
        }
    }

    /// The square root, when `self` is a perfect square, and `None` when
    /// it is not. A negative number is not one.
    ///
    /// The reference's `ibz_sqrt` returns 0 and writes nothing where this
    /// returns `None`.
    pub fn sqrt_exact(&self) -> Option<Self> {
        let root = self.sqrt_floor().ok()?;
        let square = root.mul(&root).ok()?;
        (square == *self).then_some(root)
    }

    /// The Jacobi symbol `(self / n)` for `n` odd and positive: −1, 0 or
    /// 1, and 0 exactly when `self` and `n` have a common factor.
    ///
    /// For `n` prime it is the Legendre symbol, the reference's
    /// `ibz_legendre`.
    ///
    /// # Errors
    ///
    /// [`IntError::Modulus`] if `n` is even or not positive.
    pub fn jacobi(&self, n: &Self) -> Result<i8, IntError> {
        if n.is_negative() || !n.is_odd() {
            return Err(IntError::Modulus);
        }
        let mut a = self.rem_euclid(n)?;
        let mut n = n.clone();
        let mut symbol = 1i8;
        while let Some(twos) = a.trailing_zeros() {
            a = a.shr(twos);
            // (2 / n) is −1 exactly when n is 3 or 5 modulo 8.
            let n_mod_8 = n.low_bits(3)?;
            if twos % 2 == 1 && (n_mod_8 == 3 || n_mod_8 == 5) {
                symbol = -symbol;
            }
            // Both odd now: reciprocity changes the sign exactly when
            // both are 3 modulo 4.
            if a.low_bits(2)? == 3 && n_mod_8 % 4 == 3 {
                symbol = -symbol;
            }
            let remainder = n.rem_euclid(&a)?;
            n = a;
            a = remainder;
        }
        Ok(if n == Self::one() { symbol } else { 0 })
    }

    /// Whether `self` is a modulus [`Int::mul_mod`] and [`Int::pow_mod`]
    /// take: odd, positive, and of at most `32·N` bits, so that a product
    /// of two residues fits `N` limbs.
    fn check_odd_modulus(&self) -> Result<(), IntError> {
        if self.is_negative() || !self.is_odd() || self.bit_length() > Self::BITS / 2 {
            return Err(IntError::Modulus);
        }
        Ok(())
    }

    /// `self · rhs` modulo `modulus`, in `[0, modulus)`.
    ///
    /// # Errors
    ///
    /// [`IntError::Modulus`] if `modulus` is even, not positive, or longer
    /// than half of the limbs' bits (`32·N`).
    pub fn mul_mod(&self, rhs: &Self, modulus: &Self) -> Result<Self, IntError> {
        modulus.check_odd_modulus()?;
        let a = self.rem_euclid(modulus)?;
        let b = rhs.rem_euclid(modulus)?;
        a.mul(&b)?.rem_euclid(modulus)
    }

    /// `self^exponent` modulo `modulus`, in `[0, modulus)`. A zeroth power
    /// is 1 modulo `modulus`.
    ///
    /// # Errors
    ///
    /// [`IntError::Modulus`] as for [`Int::mul_mod`], and
    /// [`IntError::Negative`] if `exponent` is negative.
    pub fn pow_mod(&self, exponent: &Self, modulus: &Self) -> Result<Self, IntError> {
        modulus.check_odd_modulus()?;
        if exponent.is_negative() {
            return Err(IntError::Negative);
        }
        let base = self.rem_euclid(modulus)?;
        let mut power = Self::one().rem_euclid(modulus)?;
        for index in (0..exponent.bit_length()).rev() {
            power = power.mul(&power)?.rem_euclid(modulus)?;
            if exponent.bit(index) {
                power = power.mul(&base)?.rem_euclid(modulus)?;
            }
        }
        Ok(power)
    }
}
