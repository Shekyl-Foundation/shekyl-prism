// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! What the public integer builds on division: greatest common divisors,
//! inverses, square roots, the Jacobi symbol, and arithmetic modulo an odd
//! number. Variable time, all of it.
//!
//! Four of these are loops that end when the arithmetic says so: Euclid's
//! algorithm twice, Newton's iteration, and the reduction of the Jacobi
//! symbol. None is left to run on that alone. Each is counted to a bound
//! that follows from the width, and returns [`IntError::StepBound`] if the
//! bound is reached first, which no operand can bring about. The loop
//! itself is in a private function that takes the bound, so that the tests
//! at the foot of this file can give it one too small and see the refusal.

use crate::error::IntError;
use crate::public::Int;

impl<const N: usize> Int<N> {
    /// The divisions Euclid's algorithm is allowed: two for each bit of
    /// the width.
    ///
    /// Write `r₀, r₁` for the two magnitudes and `rₖ₊₁ = rₖ₋₁ mod rₖ`.
    /// From `k = 1` on each remainder is below its divisor, so
    /// `rₖ₊₂ = rₖ mod rₖ₊₁ < rₖ / 2`. With `r₁ < 2^BITS` that makes
    /// `r₁₊₂ⱼ < 2^(BITS − j)`, which is zero at `j = BITS`: no divisor
    /// after the `2·BITS`-th is other than zero. (Two neighbours of the
    /// Fibonacci sequence, the slowest operands, take about `1.44·BITS`.)
    ///
    /// The Jacobi symbol's loop has the same bound. There the divisor is
    /// the odd part of the last remainder, which is no larger than it, so
    /// the remainders fall at least as fast.
    const EUCLID_STEPS: u32 = 2 * Self::BITS;

    /// The steps Newton's iteration for the square root is allowed: one
    /// for each bit of the width.
    ///
    /// The start `x₀` is a power of two with `√n ≤ x₀ ≤ 2·√n`. While `x`
    /// is above `√n`, a step takes it to at most
    /// `(x + n/x) / 2 = √n + (x − √n)² / 2x`, which is no further from
    /// `√n` than half of what `x` was, and never below `⌊√n⌋`. The first
    /// distance is below `2^(BITS/2)`, so after `BITS/2` steps it is below
    /// one, one more step reaches the floor, and one more sees that the
    /// iteration has stopped falling: `BITS/2 + 2` in all, which is at
    /// most `BITS` for any `N`. (The steps taken are about `log₂ BITS`:
    /// the distance is squared, not halved, once it is small.)
    const SQRT_STEPS: u32 = Self::BITS;

    /// The greatest common divisor, which is not negative. `gcd(0, 0)` is
    /// zero.
    ///
    /// # Errors
    ///
    /// None is expected. [`IntError::StepBound`] if Euclid's algorithm had
    /// not ended after two divisions for each bit of the width, which no
    /// operands do.
    pub fn gcd(&self, other: &Self) -> Result<Self, IntError> {
        self.gcd_within(other, Self::EUCLID_STEPS)
    }

    /// [`Int::gcd`], allowed `steps` divisions.
    fn gcd_within(&self, other: &Self, steps: u32) -> Result<Self, IntError> {
        let mut a = self.abs();
        let mut b = other.abs();
        for _ in 0..steps {
            if b.is_zero() {
                return Ok(a);
            }
            let (_, remainder) = crate::division::div_rem_magnitude(a.magnitude(), b.magnitude());
            a = b;
            b = Self::from_parts(remainder, false);
        }
        if b.is_zero() {
            Ok(a)
        } else {
            Err(IntError::StepBound)
        }
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
    /// None is expected: every intermediate is bounded by the operands,
    /// and the algorithm ends within two divisions for each bit of the
    /// width. [`IntError::Overflow`] is passed on if an intermediate did
    /// not fit, and [`IntError::StepBound`] returned if the divisions ran
    /// out.
    pub fn xgcd(&self, other: &Self) -> Result<(Self, Self, Self), IntError> {
        self.xgcd_within(other, Self::EUCLID_STEPS)
    }

    /// [`Int::xgcd`], allowed `steps` divisions.
    fn xgcd_within(&self, other: &Self, steps: u32) -> Result<(Self, Self, Self), IntError> {
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
        for _ in 0..steps {
            if r1.is_zero() {
                break;
            }
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
        if !r1.is_zero() {
            return Err(IntError::StepBound);
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
    /// [`IntError::Modulus`] if `modulus` is not above one. What
    /// [`Int::xgcd`] refuses is passed on.
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
    /// [`IntError::Negative`] if `self` is negative. None other is
    /// expected: [`IntError::StepBound`] if Newton's iteration had not
    /// ended after a step for each bit of the width, which it does for
    /// every operand.
    pub fn sqrt_floor(&self) -> Result<Self, IntError> {
        self.sqrt_floor_within(Self::SQRT_STEPS)
    }

    /// [`Int::sqrt_floor`], allowed `steps` steps of the iteration.
    fn sqrt_floor_within(&self, steps: u32) -> Result<Self, IntError> {
        if self.is_negative() {
            return Err(IntError::Negative);
        }
        if self.is_zero() {
            return Ok(Self::zero());
        }
        // Newton's iteration from a power of two that is not below the
        // root: it falls until it reaches the floor, and then does not.
        let mut root = Self::one().shl(self.bit_length().div_ceil(2))?;
        for _ in 0..steps {
            let (quotient, _) = self.div_rem_trunc(&root)?;
            let next = root.add(&quotient)?.shr(1);
            if next >= root {
                return Ok(root);
            }
            root = next;
        }
        Err(IntError::StepBound)
    }

    /// The square root, when `self` is a perfect square, and `None` when
    /// it is not. A negative number is not one.
    ///
    /// The reference's `ibz_sqrt` returns 0 and writes nothing where this
    /// returns `None`.
    ///
    /// # Errors
    ///
    /// None is expected. What [`Int::sqrt_floor`] refuses for an operand
    /// that is not negative is passed on, and is not turned into `None`:
    /// a root that was not found is not a number shown to have none.
    pub fn sqrt_exact(&self) -> Result<Option<Self>, IntError> {
        if self.is_negative() {
            return Ok(None);
        }
        let root = self.sqrt_floor()?;
        let square = root.mul(&root)?;
        Ok((square == *self).then_some(root))
    }

    /// The Jacobi symbol `(self / n)` for `n` odd and positive: −1, 0 or
    /// 1, and 0 exactly when `self` and `n` have a common factor.
    ///
    /// For `n` prime it is the Legendre symbol, the reference's
    /// `ibz_legendre`.
    ///
    /// # Errors
    ///
    /// [`IntError::Modulus`] if `n` is even or not positive. None other
    /// is expected: [`IntError::StepBound`] if the reduction had not ended
    /// after two steps for each bit of the width, which it does for every
    /// operand.
    pub fn jacobi(&self, n: &Self) -> Result<i8, IntError> {
        self.jacobi_within(n, Self::EUCLID_STEPS)
    }

    /// [`Int::jacobi`], allowed `steps` reductions.
    fn jacobi_within(&self, n: &Self, steps: u32) -> Result<i8, IntError> {
        if n.is_negative() || !n.is_odd() {
            return Err(IntError::Modulus);
        }
        let mut a = self.rem_euclid(n)?;
        let mut n = n.clone();
        let mut symbol = 1i8;
        for _ in 0..steps {
            let Some(twos) = a.trailing_zeros() else {
                break;
            };
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
        if !a.is_zero() {
            return Err(IntError::StepBound);
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

#[cfg(test)]
mod tests {
    //! The four counted loops: that each refuses when its count runs out,
    //! and that the count the public function gives is enough for the
    //! slowest operands the width holds.
    //!
    //! No operand reaches the refusal through the public functions, which
    //! is the point of the bound. So these tests call the private function
    //! that takes the count, find the least count that returns an answer,
    //! and see [`IntError::StepBound`] at every count below it.

    use crate::error::IntError;
    use crate::public::Int;

    /// The least count at which `run` returns its answer, with the answer.
    ///
    /// A count that is enough stays enough, so the least is found by
    /// halving the interval between a count that is refused and one that
    /// answers. Every refusal on the way must be the one for a count run
    /// out, and so must the refusal at one below the least. (Trying every
    /// count from zero costs the square of the bound in divisions, which
    /// is half an hour at 65 limbs.)
    fn least<T: PartialEq + core::fmt::Debug>(
        run: impl Fn(u32) -> Result<T, IntError>,
        bound: u32,
    ) -> (u32, T) {
        let refused = |steps: u32| match run(steps) {
            Ok(_) => false,
            Err(error) => {
                assert_eq!(error, IntError::StepBound, "at {steps} steps");
                true
            }
        };
        assert!(
            !refused(bound),
            "no answer within the bound of {bound} steps"
        );
        if !refused(0) {
            return (0, run(0).unwrap());
        }
        // `low` is refused and `high` answers.
        let (mut low, mut high) = (0, bound);
        while high - low > 1 {
            let middle = low + (high - low) / 2;
            if refused(middle) {
                low = middle;
            } else {
                high = middle;
            }
        }
        assert!(refused(high - 1));
        (high, run(high).unwrap())
    }

    /// The last two Fibonacci numbers that fit `N` limbs, the larger
    /// first, and the index of the larger (`F₁ = F₂ = 1`). They are the
    /// operands Euclid's algorithm is slowest on: every quotient but the
    /// last is one.
    fn fibonacci<const N: usize>() -> (Int<N>, Int<N>, u32) {
        let (mut before, mut last, mut index) = (Int::<N>::one(), Int::<N>::one(), 2u32);
        while let Ok(next) = last.add(&before) {
            before = last;
            last = next;
            index += 1;
        }
        (last, before, index)
    }

    fn euclid<const N: usize>() {
        let bound = Int::<N>::EUCLID_STEPS;
        assert_eq!(bound, 128 * N as u32);
        let (larger, smaller, index) = fibonacci::<N>();
        // Above the count a bound of `BITS + 2` would allow, at every
        // width: the bound is two for each bit and not less.
        assert!(index - 2 > Int::<N>::BITS + 2);

        // F_k and F_(k−1) take k − 2 divisions, and one more when the
        // smaller comes first.
        let (steps, g) = least(|steps| larger.gcd_within(&smaller, steps), bound);
        assert_eq!((steps, &g), (index - 2, &Int::one()));
        let (steps, g) = least(|steps| smaller.gcd_within(&larger, steps), bound);
        assert_eq!((steps, &g), (index - 1, &Int::one()));
        assert_eq!(larger.gcd(&smaller), Ok(Int::one()));
        assert_eq!(smaller.gcd(&larger), Ok(Int::one()));

        let (steps, found) = least(|steps| larger.xgcd_within(&smaller, steps), bound);
        assert_eq!(steps, index - 2);
        assert_eq!(larger.xgcd(&smaller).as_ref(), Ok(&found));
        // The cofactors of two neighbours are the two before them, with
        // opposite signs: too wide to multiply back at this width, which
        // `tests/oracle_number.rs` does for operands of every size.
        let (g, u, v) = found;
        assert_eq!(g, Int::one());
        assert!(u.is_negative() != v.is_negative());
        assert_eq!(u.gcd(&v), Ok(Int::one()));
        let (steps, _) = least(|steps| smaller.xgcd_within(&larger, steps), bound);
        assert_eq!(steps, index - 1);

        // A zero divisor needs no division, and is not refused at none.
        assert_eq!(larger.gcd_within(&Int::zero(), 0).as_ref(), Ok(&larger));
        assert_eq!(
            larger.xgcd_within(&Int::zero(), 0),
            Ok((larger.clone(), Int::one(), Int::zero()))
        );

        // The Jacobi symbol of the same pair, with the odd one of the two
        // below. Its count is not the Fibonacci index, since the powers
        // of two come out on the way; it is found, and is within the bound.
        let (a, n) = if larger.is_odd() {
            (&smaller, &larger)
        } else {
            (&larger, &smaller)
        };
        let (steps, symbol) = least(|steps| a.jacobi_within(n, steps), bound);
        assert!(steps > 1 && symbol != 0);
        assert_eq!(a.jacobi(n), Ok(symbol));
        assert_eq!(a.jacobi_within(n, steps - 1), Err(IntError::StepBound));
        // Zero over an odd number needs no reduction.
        assert_eq!(Int::<N>::zero().jacobi_within(n, 0), Ok(0));
        assert_eq!(Int::<N>::zero().jacobi_within(&Int::one(), 0), Ok(1));
    }

    fn newton<const N: usize>() {
        let bound = Int::<N>::SQRT_STEPS;
        assert_eq!(bound, 64 * N as u32);
        // The largest value of the width, the smallest above a power of
        // four, and a square.
        let largest = Int::<N>::one()
            .shl(Int::<N>::BITS - 1)
            .unwrap()
            .sub(&Int::one())
            .unwrap();
        let largest = largest.add(&largest).unwrap().add(&Int::one()).unwrap();
        let above = Int::<N>::one()
            .shl(Int::<N>::BITS - 2)
            .unwrap()
            .add(&Int::one())
            .unwrap();
        let square =
            Int::<N>::from_u64(u64::from(u32::MAX)).mul(&Int::from_u64(u64::from(u32::MAX)));
        for value in [largest, above, square.unwrap(), Int::one()] {
            let (steps, root) = least(|steps| value.sqrt_floor_within(steps), bound);
            assert!(steps >= 1);
            assert_eq!(value.sqrt_floor().as_ref(), Ok(&root));
            assert_eq!(value.sqrt_floor_within(steps - 1), Err(IntError::StepBound));
            let below = root.mul(&root).unwrap();
            assert!(below <= value);
        }
        // Zero is answered before the iteration, and a negative number is
        // refused as negative whatever the count.
        assert_eq!(Int::<N>::zero().sqrt_floor_within(0), Ok(Int::zero()));
        assert_eq!(
            Int::<N>::from_i64(-4).sqrt_floor_within(0),
            Err(IntError::Negative)
        );
        assert_eq!(Int::<N>::from_i64(-4).sqrt_exact(), Ok(None));
    }

    #[test]
    fn euclid_and_jacobi_refuse_when_the_count_runs_out() {
        euclid::<1>();
        euclid::<2>();
        euclid::<3>();
        euclid::<26>();
        euclid::<33>();
        euclid::<38>();
        euclid::<49>();
        euclid::<65>();
    }

    #[test]
    fn newton_refuses_when_the_count_runs_out() {
        newton::<1>();
        newton::<2>();
        newton::<3>();
        newton::<26>();
        newton::<33>();
        newton::<38>();
        newton::<49>();
        newton::<65>();
    }
}
