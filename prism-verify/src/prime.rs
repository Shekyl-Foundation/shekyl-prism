// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Baillie–PSW on a public little-endian integer.
//!
//! Products and exponentiations are variable-time Montgomery
//! multiplications on 64-bit limbs. The challenge degree is public, so
//! this code does not aim at a secret-independent running time.
//! shekyl-core pins `crypto-bigint` 0.5.5 for fixed-width constant-time
//! integers. A public modulus whose width changes with the parameter set
//! stays on these limbs instead.
//!
//! The width is a type, `W:` [`DegreeWidth`], and every integer and every
//! routine here is generic over it. A parameter set names its width
//! (`ParameterSet::Degree`), so the test for a set is compiled at that
//! width. The verdict on an integer does not depend on the width it is
//! held in, as long as it fits.
//!
//! The test is the one `docs/VERIFIER_SPEC.md` specifies: trial division by
//! the primes below 100, a perfect-square filter, one strong Miller–Rabin
//! test at base 2, then the strong Lucas test with Selfridge's parameters.
//! No random bases, so the verdict is a function of `n` alone.

use core::cmp::Ordering;

use prism_field::width::LIMB_BITS;
use prism_field::{DegreeWidth, Limbs};

/// A non-negative integer below `2^W::BITS`.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Nat<W: DegreeWidth> {
    /// Little-endian limbs: `limbs[0]` is the least significant.
    limbs: W::Limbs,
}

impl<W: DegreeWidth> core::fmt::Debug for Nat<W> {
    /// Big-endian hex, so a failing assertion names the integer.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let top = self
            .limbs
            .as_ref()
            .iter()
            .rposition(|limb| *limb != 0)
            .unwrap_or_default();
        write!(f, "0x{:x}", self.limbs[top])?;
        for limb in self.limbs.as_ref()[..top].iter().rev() {
            write!(f, "{limb:016x}")?;
        }
        Ok(())
    }
}

impl<W: DegreeWidth> Ord for Nat<W> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Most significant limb first. The limbs are little-endian, so the
        // order a derive would produce is not the integer order.
        for (left, right) in self
            .limbs
            .as_ref()
            .iter()
            .rev()
            .zip(other.limbs.as_ref().iter().rev())
        {
            match left.cmp(right) {
                Ordering::Equal => (),
                unequal => return unequal,
            }
        }
        Ordering::Equal
    }
}

impl<W: DegreeWidth> PartialOrd for Nat<W> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<W: DegreeWidth> Nat<W> {
    /// How many limbs this width has.
    const LIMBS: usize = <W::Limbs as Limbs>::COUNT;

    const ZERO: Self = Self {
        limbs: <W::Limbs as Limbs>::ZERO,
    };

    fn one() -> Self {
        Self::from_u64(1)
    }

    fn from_u64(value: u64) -> Self {
        let mut out = Self::ZERO;
        out.limbs[0] = value;
        out
    }

    /// The integer `bytes` spells, or `None` when it is wider than
    /// the width.
    fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > Self::LIMBS * 8 {
            return None;
        }
        let mut out = Self::ZERO;
        for (index, byte) in bytes.iter().enumerate() {
            out.limbs[index / 8] |= u64::from(*byte) << ((index % 8) * 8);
        }
        Some(out)
    }

    /// `2^bit`, or `None` when the bit lies outside this module's width.
    fn power_of_two(bit: usize) -> Option<Self> {
        if bit >= W::BITS {
            return None;
        }
        let mut out = Self::ZERO;
        out.limbs[bit / LIMB_BITS] = 1u64 << (bit % LIMB_BITS);
        Some(out)
    }

    fn is_zero(self) -> bool {
        self == Self::ZERO
    }

    fn is_even(self) -> bool {
        self.limbs[0] & 1 == 0
    }

    fn bit(self, index: usize) -> bool {
        let limb = index / LIMB_BITS;
        if limb >= Self::LIMBS {
            return false;
        }
        (self.limbs[limb] >> (index % LIMB_BITS)) & 1 == 1
    }

    /// Little-endian bytes, without high zeros. Zero is a single zero byte.
    fn to_le_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::LIMBS * 8);
        for limb in self.limbs.as_ref() {
            out.extend_from_slice(&limb.to_le_bytes());
        }
        while out.len() > 1 && out.last() == Some(&0) {
            out.pop();
        }
        out
    }

    /// Position of the highest set bit, counting from one. Zero has none.
    fn bit_len(self) -> usize {
        for (index, limb) in self.limbs.as_ref().iter().enumerate().rev() {
            if *limb != 0 {
                return index * LIMB_BITS + (LIMB_BITS - limb.leading_zeros() as usize);
            }
        }
        0
    }

    /// `self + rhs mod 2^W::BITS`, with the carry out of the top limb.
    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        let mut out = Self::ZERO;
        let mut carry = 0u128;
        for (index, slot) in out.limbs.as_mut().iter_mut().enumerate() {
            carry += u128::from(self.limbs[index]) + u128::from(rhs.limbs[index]);
            *slot = carry as u64;
            carry >>= LIMB_BITS;
        }
        (out, carry != 0)
    }

    /// `self − rhs mod 2^W::BITS`.
    fn wrapping_sub(self, rhs: Self) -> Self {
        let mut out = Self::ZERO;
        let mut borrow = 0i128;
        for (index, slot) in out.limbs.as_mut().iter_mut().enumerate() {
            borrow += i128::from(self.limbs[index]) - i128::from(rhs.limbs[index]);
            *slot = borrow as u64;
            borrow >>= LIMB_BITS;
        }
        out
    }

    /// `2^W::BITS − self`: adding this is subtracting `self` with the
    /// borrow out of the top limb discarded.
    fn twos_complement(self) -> Self {
        let mut out = Self::ZERO;
        let mut carry = 1u64;
        for (index, slot) in out.limbs.as_mut().iter_mut().enumerate() {
            let (sum, overflow) = (!self.limbs[index]).overflowing_add(carry);
            *slot = sum;
            carry = u64::from(overflow);
        }
        out
    }

    fn shr1(self) -> Self {
        let mut out = Self::ZERO;
        for index in 0..Self::LIMBS {
            out.limbs[index] = self.limbs[index] >> 1;
            if let Some(above) = self.limbs.as_ref().get(index + 1) {
                out.limbs[index] |= above << (LIMB_BITS - 1);
            }
        }
        out
    }

    /// `self + rhs mod modulus`, for operands already below `modulus`.
    fn add_mod(self, rhs: Self, modulus: Self) -> Self {
        let (sum, carry) = self.overflowing_add(rhs);
        if carry || sum >= modulus {
            // Both operands are reduced, so the true sum is below `2·modulus`
            // and one subtraction brings it back. A carry means the sum wrapped
            // through `2^W::BITS`; adding the two's complement subtracts
            // `modulus` from the wrapped residue, which is the same residue.
            sum.overflowing_add(modulus.twos_complement()).0
        } else {
            sum
        }
    }

    /// `self − rhs mod modulus`, for operands already below `modulus`.
    fn sub_mod(self, rhs: Self, modulus: Self) -> Self {
        if self < rhs {
            self.add_mod(modulus.wrapping_sub(rhs), modulus)
        } else {
            self.wrapping_sub(rhs)
        }
    }

    /// `self / 2 mod modulus`, for an odd `modulus`.
    fn half_mod(self, modulus: Self) -> Self {
        if self.is_even() {
            return self.shr1();
        }
        // `self + modulus` is even and may need the bit the shift drops.
        let (sum, carry) = self.overflowing_add(modulus);
        let mut out = sum.shr1();
        if carry {
            out.limbs[Self::LIMBS - 1] |= 1u64 << (LIMB_BITS - 1);
        }
        out
    }

    /// `self mod modulus`.
    fn rem(self, modulus: Self) -> Self {
        if self < modulus {
            return self;
        }
        let mut rest = Self::ZERO;
        for index in (0..self.bit_len()).rev() {
            rest = rest.add_mod(rest, modulus);
            if self.bit(index) {
                rest = rest.add_mod(Self::one(), modulus);
            }
        }
        rest
    }

    /// `self mod modulus` for a modulus that fits in a `u32`.
    fn rem_u32(self, modulus: u32) -> u32 {
        let modulus = u128::from(modulus);
        let mut rest = 0u128;
        for limb in self.limbs.as_ref().iter().rev() {
            rest = ((rest << LIMB_BITS) | u128::from(*limb)) % modulus;
        }
        rest as u32
    }

    /// `self^2`, or `None` when the product is wider than the width.
    fn checked_square(self) -> Option<Self> {
        // The scratch array is the length this routine indexes.
        let () = W::CONSISTENT;
        let mut wide = <W::Product as Limbs>::ZERO;
        for index in 0..Self::LIMBS {
            let mut carry = 0u128;
            for (offset, limb) in self.limbs.as_ref().iter().enumerate() {
                carry += u128::from(wide[index + offset])
                    + u128::from(self.limbs[index]) * u128::from(*limb);
                wide[index + offset] = carry as u64;
                carry >>= LIMB_BITS;
            }
            wide[index + Self::LIMBS] = carry as u64;
        }
        if wide.as_ref()[Self::LIMBS..].iter().any(|limb| *limb != 0) {
            return None;
        }
        let mut out = Self::ZERO;
        out.limbs
            .as_mut()
            .copy_from_slice(&wide.as_ref()[..Self::LIMBS]);
        Some(out)
    }

    /// `true` when `self` is the square of an integer.
    ///
    /// Digit-by-digit extraction in base 4: the loop walks the odd powers of
    /// two downward and keeps the remainder, which is zero exactly for a
    /// perfect square.
    fn is_perfect_square(self) -> bool {
        if self.is_zero() {
            return true;
        }
        let mut rest = self;
        let mut root = Self::ZERO;
        // The highest even exponent at or below the top set bit.
        let top = (self.bit_len() - 1) & !1;
        let Some(mut bit) = Self::power_of_two(top) else {
            return false;
        };
        while !bit.is_zero() {
            let (trial, carry) = root.overflowing_add(bit);
            if !carry && rest >= trial {
                rest = rest.wrapping_sub(trial);
                root = root.shr1().overflowing_add(bit).0;
            } else {
                root = root.shr1();
            }
            bit = bit.shr1().shr1();
        }
        rest.is_zero()
    }
}

/// Montgomery arithmetic modulo an odd `modulus`, with `R = 2^W::BITS`.
struct Mod<W: DegreeWidth> {
    modulus: Nat<W>,
    /// `−modulus^{-1} mod 2^64`.
    modulus_prime: u64,
    /// `R^2 mod modulus`, the factor that lifts a value into Montgomery form.
    r_squared: Nat<W>,
}

impl<W: DegreeWidth> Mod<W> {
    /// `modulus` must be odd and above one.
    fn new(modulus: Nat<W>) -> Self {
        let mut r_squared = Nat::one();
        // `R^2 = 2^{2·W::BITS}`, reached by doubling one that many times.
        for _ in 0..(2 * W::BITS) {
            r_squared = r_squared.add_mod(r_squared, modulus);
        }
        Self {
            modulus,
            modulus_prime: negated_inverse_limb(modulus.limbs[0]),
            r_squared,
        }
    }

    /// `left · right · R^{-1} mod modulus`, by coarsely integrated operand
    /// scanning: one limb product row, then one reduction row.
    fn mont_mul(&self, left: Nat<W>, right: Nat<W>) -> Nat<W> {
        // The scratch array is the length this routine indexes.
        let () = W::CONSISTENT;
        let mut t = <W::Reduction as Limbs>::ZERO;
        for index in 0..Nat::<W>::LIMBS {
            let mut carry = 0u128;
            for (offset, slot) in t.as_mut().iter_mut().enumerate().take(Nat::<W>::LIMBS) {
                carry += u128::from(*slot)
                    + u128::from(left.limbs[offset]) * u128::from(right.limbs[index]);
                *slot = carry as u64;
                carry >>= LIMB_BITS;
            }
            carry += u128::from(t[Nat::<W>::LIMBS]);
            t[Nat::<W>::LIMBS] = carry as u64;
            t[Nat::<W>::LIMBS + 1] = (carry >> LIMB_BITS) as u64;

            let factor = t[0].wrapping_mul(self.modulus_prime);
            carry = 0;
            for (offset, slot) in t.as_mut().iter_mut().enumerate().take(Nat::<W>::LIMBS) {
                carry +=
                    u128::from(*slot) + u128::from(factor) * u128::from(self.modulus.limbs[offset]);
                *slot = carry as u64;
                carry >>= LIMB_BITS;
            }
            carry += u128::from(t[Nat::<W>::LIMBS]);
            t[Nat::<W>::LIMBS] = carry as u64;
            t[Nat::<W>::LIMBS + 1] += (carry >> LIMB_BITS) as u64;

            t.as_mut().copy_within(1.., 0);
            t[Nat::<W>::LIMBS + 1] = 0;
        }
        let mut out = Nat::<W>::ZERO;
        out.limbs
            .as_mut()
            .copy_from_slice(&t.as_ref()[..Nat::<W>::LIMBS]);
        if t[Nat::<W>::LIMBS] != 0 {
            // The residue overflowed the width; the borrow out of the top limb
            // cancels the overflow bit.
            out.overflowing_add(self.modulus.twos_complement()).0
        } else if out >= self.modulus {
            out.wrapping_sub(self.modulus)
        } else {
            out
        }
    }

    /// Into Montgomery form: `value · R mod modulus`.
    fn lift(&self, value: Nat<W>) -> Nat<W> {
        self.mont_mul(value, self.r_squared)
    }

    /// Out of Montgomery form: `value · R^{-1} mod modulus`.
    fn lower(&self, value: Nat<W>) -> Nat<W> {
        self.mont_mul(value, Nat::one())
    }

    /// `left · right mod modulus`, both arguments and the result in normal
    /// form. Lifting one operand is enough: `mont_mul` strips the `R` again,
    /// so this is two Montgomery multiplications rather than four.
    fn mul(&self, left: Nat<W>, right: Nat<W>) -> Nat<W> {
        self.mont_mul(self.lift(left), right)
    }

    /// `value · factor mod modulus` for a small signed factor.
    fn mul_small(&self, value: Nat<W>, factor: i32) -> Nat<W> {
        self.mul(value, reduce_i32(factor, self.modulus))
    }

    /// `base^exponent mod modulus`, square and multiply from the top bit.
    fn pow(&self, base: Nat<W>, exponent: Nat<W>) -> Nat<W> {
        let base = self.lift(base);
        let mut result = self.lift(Nat::one());
        for index in (0..exponent.bit_len()).rev() {
            result = self.mont_mul(result, result);
            if exponent.bit(index) {
                result = self.mont_mul(result, base);
            }
        }
        self.lower(result)
    }
}

/// `−value^{-1} mod 2^64` for an odd `value`.
fn negated_inverse_limb(value: u64) -> u64 {
    // Newton's iteration doubles the number of correct bits each step, so six
    // steps carry the seed's one bit to all 64.
    const NEWTON_STEPS: u32 = u64::BITS.ilog2();
    let mut inverse = 1u64;
    for _ in 0..NEWTON_STEPS {
        inverse = inverse.wrapping_mul(2u64.wrapping_sub(value.wrapping_mul(inverse)));
    }
    inverse.wrapping_neg()
}

/// `value mod modulus` for a small signed `value`.
fn reduce_i32<W: DegreeWidth>(value: i32, modulus: Nat<W>) -> Nat<W> {
    let magnitude = Nat::from_u64(u64::from(value.unsigned_abs())).rem(modulus);
    if value >= 0 || magnitude.is_zero() {
        magnitude
    } else {
        modulus.wrapping_sub(magnitude)
    }
}

/// The Jacobi symbol `(a/n)` for an odd `n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Jacobi {
    /// `a` and `n` share a factor.
    Zero,
    /// `+1`.
    Residue,
    /// `−1`.
    NonResidue,
}

impl Jacobi {
    fn negate(self) -> Self {
        match self {
            Self::Zero => Self::Zero,
            Self::Residue => Self::NonResidue,
            Self::NonResidue => Self::Residue,
        }
    }

    fn from_negated(negated: bool) -> Self {
        if negated {
            Self::NonResidue
        } else {
            Self::Residue
        }
    }
}

fn jacobi<W: DegreeWidth>(a: i32, n: Nat<W>) -> Jacobi {
    // `(a/n) = (−1/n)·(|a|/n)`, and `(−1/n)` is `−1` exactly when `n ≡ 3 (mod 4)`.
    let symbol = jacobi_nonneg(Nat::from_u64(u64::from(a.unsigned_abs())), n);
    if a < 0 && n.rem_u32(4) == 3 {
        symbol.negate()
    } else {
        symbol
    }
}

fn jacobi_nonneg<W: DegreeWidth>(a: Nat<W>, n: Nat<W>) -> Jacobi {
    let mut numerator = a;
    let mut modulus = n;
    let mut negated = false;
    loop {
        numerator = numerator.rem(modulus);
        if numerator.is_zero() {
            return Jacobi::Zero;
        }
        while numerator.is_even() {
            numerator = numerator.shr1();
            // `(2/m)` is `−1` for `m ≡ 3, 5 (mod 8)`.
            if matches!(modulus.rem_u32(8), 3 | 5) {
                negated = !negated;
            }
        }
        if numerator == Nat::one() {
            return Jacobi::from_negated(negated);
        }
        // Quadratic reciprocity flips the sign when both are `3 (mod 4)`.
        if numerator.rem_u32(4) == 3 && modulus.rem_u32(4) == 3 {
            negated = !negated;
        }
        core::mem::swap(&mut numerator, &mut modulus);
    }
}

/// Selfridge's discriminant: `D` from `5, −7, 9, −11, …` until the Jacobi
/// symbol is `−1`. `None` when a shared factor turns up, which proves `n`
/// composite.
///
/// The sequence terminates for every `n` that is not a perfect square, and
/// [`is_prime`] filters those out first. The bound keeps a direct call to
/// [`strong_lucas_selfridge`] from looping on one.
fn selfridge_discriminant<W: DegreeWidth>(n: Nat<W>) -> Option<i32> {
    const CANDIDATE_LIMIT: usize = 256;
    let mut d = 5i32;
    for _ in 0..CANDIDATE_LIMIT {
        match jacobi(d, n) {
            Jacobi::NonResidue => return Some(d),
            Jacobi::Zero => return None,
            Jacobi::Residue => d = if d > 0 { -(d + 2) } else { -d + 2 },
        }
    }
    None
}

/// `U_k`, `V_k` and `Q^k` of the Lucas sequence with `P = 1`.
fn lucas_chain<W: DegreeWidth>(
    ctx: &Mod<W>,
    k: Nat<W>,
    discriminant: i32,
    q: i32,
) -> (Nat<W>, Nat<W>, Nat<W>) {
    let mut u = Nat::one();
    let mut v = Nat::one();
    let mut q_power = reduce_i32(q, ctx.modulus);
    let bits = k.bit_len();
    if bits <= 1 {
        return (u, v, q_power);
    }
    for index in (0..bits - 1).rev() {
        // `k → 2k`: `U_{2k} = U_k V_k`, `V_{2k} = V_k^2 − 2 Q^k`.
        let doubled_u = ctx.mul(u, v);
        let doubled_v = ctx
            .mul(v, v)
            .sub_mod(q_power.add_mod(q_power, ctx.modulus), ctx.modulus);
        q_power = ctx.mul(q_power, q_power);
        u = doubled_u;
        v = doubled_v;
        if k.bit(index) {
            // `2k → 2k+1`, with `P = 1`: halving the sums gives the step.
            let stepped_u = u.add_mod(v, ctx.modulus).half_mod(ctx.modulus);
            let stepped_v = ctx
                .mul_small(u, discriminant)
                .add_mod(v, ctx.modulus)
                .half_mod(ctx.modulus);
            u = stepped_u;
            v = stepped_v;
            q_power = ctx.mul_small(q_power, q);
        }
    }
    (u, v, q_power)
}

/// One strong Miller–Rabin test at base 2, for an odd `n` above three.
fn miller_rabin_base2<W: DegreeWidth>(n: Nat<W>) -> bool {
    let below = n.wrapping_sub(Nat::one());
    let mut odd_part = below;
    let mut twos = 0u32;
    while odd_part.is_even() {
        odd_part = odd_part.shr1();
        twos += 1;
    }
    let ctx = Mod::new(n);
    let mut witness = ctx.pow(Nat::from_u64(2), odd_part);
    if witness == Nat::one() || witness == below {
        return true;
    }
    for _ in 1..twos {
        witness = ctx.mul(witness, witness);
        if witness == below {
            return true;
        }
    }
    false
}

/// The strong Lucas test with Selfridge's parameters, for an odd `n` above
/// three that is not a perfect square.
fn strong_lucas_selfridge<W: DegreeWidth>(n: Nat<W>) -> bool {
    let Some(discriminant) = selfridge_discriminant(n) else {
        return false;
    };
    let q = (1 - discriminant) / 4;
    let above = n.overflowing_add(Nat::one()).0;
    let mut odd_part = above;
    let mut twos = 0u32;
    while odd_part.is_even() {
        odd_part = odd_part.shr1();
        twos += 1;
    }
    let ctx = Mod::new(n);
    // Accept when `U_d ≡ 0` or `V_{d·2^t} ≡ 0` for some `0 ≤ t < twos`.
    let (u, mut v, mut q_power) = lucas_chain(&ctx, odd_part, discriminant, q);
    if u.is_zero() || v.is_zero() {
        return true;
    }
    for _ in 1..twos {
        v = ctx.mul(v, v).sub_mod(q_power.add_mod(q_power, n), n);
        q_power = ctx.mul(q_power, q_power);
        if v.is_zero() {
            return true;
        }
    }
    false
}

/// The primes the sieve divides by: every prime below 100.
///
/// Two is in the table, so the parity test is the table's own first step
/// rather than a special case beside it.
const TRIAL_PRIMES: &[u32] = &[
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
];

/// The smallest prime. Zero and one are below it and are not prime.
const SMALLEST_PRIME: u64 = 2;

/// `true` when the little-endian integer `bytes` passes Baillie–PSW.
///
/// An integer wider than the width is refused rather than truncated.
pub fn is_baillie_psw<W: DegreeWidth>(bytes: &[u8]) -> bool {
    Nat::<W>::from_le_bytes(bytes).is_some_and(is_prime)
}

fn is_prime<W: DegreeWidth>(n: Nat<W>) -> bool {
    if n < Nat::from_u64(SMALLEST_PRIME) {
        return false;
    }
    for &prime in TRIAL_PRIMES {
        // The comparison is against the whole integer, not its low limb: a
        // wide `n` whose low limb happens to spell a small prime is not that
        // prime, and the division below is what refuses it.
        if n == Nat::from_u64(u64::from(prime)) {
            return true;
        }
        if n.rem_u32(prime) == 0 {
            return false;
        }
    }
    // `n` is now coprime to every prime below 100 and larger than all of
    // them, which is what both halves below assume.
    if n.is_perfect_square() {
        return false;
    }
    miller_rabin_base2(n) && strong_lucas_selfridge(n)
}

/// The challenge degree `q = 2^{2·exponent−1} − s^2`, little-endian.
///
/// `None` when `s` is not a positive odd integer strictly below
/// `2^{exponent−1}`. Zero would make `q` a power of two, the prime 2
/// exactly at exponent 1, and that integer is not a challenge degree.
pub fn challenge_degree<W: DegreeWidth>(exponent: u32, scalar: &[u8]) -> Option<Vec<u8>> {
    let scalar = Nat::<W>::from_le_bytes(scalar)?;
    if scalar.is_zero() || scalar.is_even() {
        return None;
    }
    let power = degree_exponent(exponent).and_then(Nat::power_of_two)?;
    let square = scalar.checked_square()?;
    if square >= power {
        return None;
    }
    Some(power.wrapping_sub(square).to_le_bytes())
}

/// `true` when the challenge degree `q = 2^{2e−1} − s^2` is a Baillie–PSW
/// prime.
///
/// `exponent` is the chain exponent (`E − 2` for the current verifier),
/// and `scalar` is `s` little-endian. The helper requires a positive odd
/// scalar with `s² < 2^{2·exponent−1}`; it does not enforce the narrower
/// challenge domain `s < 2^{exponent−1}`. The verifier gets that domain
/// from `ChallengeScalar::derive`, rather than reducing supplied bytes.
pub fn degree_is_prime<W: DegreeWidth>(exponent: u32, scalar: &[u8]) -> bool {
    challenge_degree::<W>(exponent, scalar).is_some_and(|degree| is_baillie_psw::<W>(&degree))
}

/// `2e − 1`, the exponent in `q = 2^{2e−1} − s^2`.
fn degree_exponent(e: u32) -> Option<usize> {
    let doubled = e.checked_mul(2)?.checked_sub(1)?;
    usize::try_from(doubled).ok()
}

#[cfg(test)]
mod probe {
    //! Byte entry points for the two halves, so a test can show that a
    //! pseudoprime for one half is refused by the other.

    use super::Nat;
    use prism_field::DegreeWidth;

    pub(crate) fn miller_rabin_base2<W: DegreeWidth>(bytes: &[u8]) -> bool {
        Nat::<W>::from_le_bytes(bytes).is_some_and(super::miller_rabin_base2)
    }

    pub(crate) fn strong_lucas_selfridge<W: DegreeWidth>(bytes: &[u8]) -> bool {
        Nat::<W>::from_le_bytes(bytes).is_some_and(super::strong_lucas_selfridge)
    }
}

/// The tests of this module at one width. Every width a parameter set
/// names runs all of them: the verdict on an integer must not depend on
/// the width it is held in.
#[cfg(test)]
macro_rules! self_test_at {
    ($name:ident, $width:ty) => {
        mod $name {
            use super::probe;
            use super::{Mod, Nat, TRIAL_PRIMES};
            use prism_field::DegreeWidth;

            type Width = $width;
            const WIDTH_BITS: usize = <Width as DegreeWidth>::BITS;

            fn is_baillie_psw(bytes: &[u8]) -> bool {
                super::is_baillie_psw::<Width>(bytes)
            }
            fn challenge_degree(exponent: u32, scalar: &[u8]) -> Option<Vec<u8>> {
                super::challenge_degree::<Width>(exponent, scalar)
            }
            fn degree_is_prime(exponent: u32, scalar: &[u8]) -> bool {
                super::degree_is_prime::<Width>(exponent, scalar)
            }

            /// The primes below 300, for a dense sweep of the small cases.
            const PRIMES_BELOW_300: &[u32] = &[
                2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79,
                83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167,
                173, 179, 181, 191, 193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257,
                263, 269, 271, 277, 281, 283, 293,
            ];

            fn le(value: u128) -> Vec<u8> {
                let bytes = value.to_le_bytes();
                let len = bytes.iter().rposition(|b| *b != 0).map_or(1, |at| at + 1);
                bytes[..len].to_vec()
            }

            #[test]
            fn every_integer_below_three_hundred_gets_the_right_verdict() {
                for value in 0u32..300 {
                    let expected = PRIMES_BELOW_300.contains(&value);
                    assert_eq!(is_baillie_psw(&le(u128::from(value))), expected, "{value}");
                }
            }

            #[test]
            fn a_wide_integer_is_not_its_low_limb() {
                // The low limb spells a trial prime; the integer does not. Each of
                // these is even or a multiple of three, so the sieve refuses it.
                for prime in [2u128, 3, 5, 97] {
                    for multiple in 1u128..=5 {
                        let wide = multiple * (1u128 << 64) + prime;
                        assert!(!is_baillie_psw(&le(wide)), "2^64·{multiple} + {prime}");
                    }
                }
                // 2^64 + 13 is prime and 2^64 + 15 is not, so the sieve is not simply
                // refusing everything above one limb.
                assert!(is_baillie_psw(&le((1u128 << 64) + 13)));
                assert!(!is_baillie_psw(&le((1u128 << 64) + 15)));
            }

            #[test]
            fn the_degree_is_the_power_of_two_minus_the_square() {
                // `s = 1` at exponent 4: `q = 2^7 − 1 = 127`.
                assert_eq!(challenge_degree(4, &[1]).as_deref(), Some(&[127][..]));
                assert!(degree_is_prime(4, &[1]));
                // An even scalar names no degree. `12^2` is already past `2^7`.
                assert!(challenge_degree(4, &[2]).is_none());
                assert!(challenge_degree(4, &[12]).is_none());
            }

            #[test]
            fn an_integer_wider_than_the_module_is_refused() {
                assert!(!is_baillie_psw(&[0xff; WIDTH_BITS / 8 + 1]));
                // All ones at the full width fits, and it is composite: the width
                // is a multiple of 64, and `2^n − 1` is composite for composite `n`.
                assert!(!is_baillie_psw(&[0xff; WIDTH_BITS / 8]));
            }

            #[test]
            fn the_sieve_table_is_the_primes_below_one_hundred() {
                for &prime in TRIAL_PRIMES {
                    assert!(PRIMES_BELOW_300.contains(&prime), "{prime}");
                    assert!(prime < 100, "{prime}");
                }
                let expected = PRIMES_BELOW_300.iter().filter(|p| **p < 100).count();
                assert_eq!(TRIAL_PRIMES.len(), expected);
            }

            #[test]
            fn base2_pseudoprimes_fail_the_lucas_half() {
                for n in [2047u128, 3277, 4033, 4681, 8321] {
                    assert!(probe::miller_rabin_base2::<Width>(&le(n)), "{n}");
                    assert!(!is_baillie_psw(&le(n)), "{n}");
                }
            }

            #[test]
            fn strong_lucas_pseudoprimes_fail_the_miller_half() {
                for n in [5459u128, 5777, 10877, 16109, 18971] {
                    assert!(probe::strong_lucas_selfridge::<Width>(&le(n)), "{n}");
                    assert!(!is_baillie_psw(&le(n)), "{n}");
                }
                // The standard Lucas test accepts 323. The strong test does not.
                assert!(!probe::strong_lucas_selfridge::<Width>(&le(323)));
            }

            #[test]
            fn carmichael_numbers_are_refused() {
                for n in [561u128, 1105, 1729, 2465, 2821, 6601, 8911] {
                    assert!(!is_baillie_psw(&le(n)), "{n}");
                }
            }

            #[test]
            fn mersenne_primes_pass_at_every_width() {
                for exponent in [61u32, 89, 107, 127, 521] {
                    let mut bytes = vec![0xffu8; (exponent as usize).div_ceil(8)];
                    if let Some(top) = bytes.last_mut() {
                        *top = u8::MAX >> (7 - (exponent - 1) % 8);
                    }
                    assert!(is_baillie_psw(&bytes), "2^{exponent} − 1");
                }
            }

            #[test]
            fn montgomery_multiplication_agrees_with_small_products() {
                let modulus = Nat::<Width>::from_u64(1_000_003);
                let ctx = Mod::new(modulus);
                for left in [0u64, 1, 2, 999, 1_000_002] {
                    for right in [0u64, 1, 7, 65_537, 1_000_002] {
                        let expected = Nat::<Width>::from_u64((left * right) % 1_000_003);
                        assert_eq!(
                            ctx.mul(Nat::<Width>::from_u64(left), Nat::<Width>::from_u64(right)),
                            expected,
                            "{left}·{right}"
                        );
                    }
                }
                // `2^(p−1) = 1 (mod p)` for the prime above.
                assert_eq!(
                    ctx.pow(Nat::<Width>::from_u64(2), Nat::<Width>::from_u64(1_000_002)),
                    Nat::<Width>::one()
                );
            }

            #[test]
            fn a_perfect_square_is_refused_before_either_half() {
                for root in [3u128, 5, 101, 1_000_003, 4_294_967_291] {
                    assert!(!is_baillie_psw(&le(root * root)), "{root}^2");
                }
            }

            #[test]
            fn a_known_prime_degree_passes() {
                // q = 2^{17} − 3^2 = 131063, which is prime. e = 9 gives the exponent
                // 2e−1 = 17, and s = 3 is the odd integer below 2^{8}.
                assert!(degree_is_prime(9, &[3]));
                // 2^{17} − 2^2 is even.
                assert!(!degree_is_prime(9, &[2]));
            }

            #[test]
            fn a_scalar_at_or_above_the_power_names_no_degree() {
                // s^2 = 2^{2e−1} leaves q = 0, and anything larger is negative.
                assert!(!degree_is_prime(1, &[1]));
                assert!(!degree_is_prime(9, &le(1 << 9)));
                // An exponent whose power exceeds the module's width is refused.
                assert!(!degree_is_prime(u32::MAX, &[3]));
                assert!(!degree_is_prime(0, &[3]));
                assert!(!degree_is_prime((WIDTH_BITS as u32) / 2 + 1, &le(3)));
            }
        }
    };
}

#[cfg(test)]
self_test_at!(at_1024_bits, prism_field::DegreeLimbs<16>);
#[cfg(test)]
self_test_at!(at_1344_bits, prism_field::DegreeLimbs<21>);

/// The arithmetic at one and two limbs, against `u128`.
///
/// A challenge degree leaves the top limb of its width far from full, so
/// at the widths the sets name no test reaches the branches that handle a
/// carry out of the top limb: the wrapped sum in `add_mod`, the dropped
/// bit in `half_mod`, the overflow limb in `mont_mul`. A modulus with its
/// top bit set reaches all three. Two limbs is the widest a `u128` can
/// check exactly, and one limb puts every carry on the only limb there is.
#[cfg(test)]
mod narrow {
    use super::{is_baillie_psw, Mod, Nat};
    use prism_field::DegreeWidth;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct OneLimb;
    impl DegreeWidth for OneLimb {
        type Limbs = [u64; 1];
        type Product = [u64; 2];
        type Reduction = [u64; 3];
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TwoLimbs;
    impl DegreeWidth for TwoLimbs {
        type Limbs = [u64; 2];
        type Product = [u64; 4];
        type Reduction = [u64; 4];
    }

    fn nat<W: DegreeWidth>(value: u128) -> Nat<W> {
        Nat::from_le_bytes(&value.to_le_bytes()[..W::BITS / 8]).expect("the value fits the width")
    }

    fn value<W: DegreeWidth>(nat: Nat<W>) -> u128 {
        let mut bytes = [0u8; 16];
        let le = nat.to_le_bytes();
        bytes[..le.len()].copy_from_slice(&le);
        u128::from_le_bytes(bytes)
    }

    /// SplitMix64, twice for a `u128`.
    struct Stream(u64);

    impl Stream {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        fn wide(&mut self) -> u128 {
            u128::from(self.next()) << 64 | u128::from(self.next())
        }
    }

    /// `left · right mod modulus` by doubling, with every sum taken in a
    /// way that cannot overflow.
    fn mul_mod(left: u128, right: u128, modulus: u128) -> u128 {
        let add = |a: u128, b: u128| {
            let gap = modulus - b;
            if a >= gap {
                a - gap
            } else {
                a + b
            }
        };
        let (mut sum, mut term) = (0, left % modulus);
        for bit in 0..u128::BITS {
            if (right >> bit) & 1 == 1 {
                sum = add(sum, term);
            }
            term = add(term, term);
        }
        sum
    }

    /// Every modular routine against the reference, for moduli of the full
    /// width and operands spread over the residues. Returns how many of
    /// the sums wrapped past the width, so the caller can require that the
    /// carry branches ran.
    fn arithmetic_agrees<W: DegreeWidth>(seed: u64) -> u32 {
        let mask = if W::BITS == u128::BITS as usize {
            u128::MAX
        } else {
            (1u128 << W::BITS) - 1
        };
        let top = 1u128 << (W::BITS - 1);
        let mut stream = Stream(seed);
        let mut wrapped = 0;
        for round in 0..2_000 {
            // Odd, with the top bit of the width set. Every fourth one is
            // within a few units of the top of the width.
            let modulus = if round % 4 == 0 {
                mask - 2 * u128::from(stream.next() % 64)
            } else {
                (stream.wide() & mask) | top | 1
            };
            let ctx = Mod::<W>::new(nat(modulus));
            let mut residue = || match stream.next() % 4 {
                0 => modulus - 1 - u128::from(stream.next() % 4),
                1 => u128::from(stream.next() % 4),
                _ => (stream.wide() & mask) % modulus,
            };
            let (a, b) = (residue(), residue());
            let (na, nb, n) = (nat::<W>(a), nat::<W>(b), nat::<W>(modulus));
            let sum = mul_mod(1, a, modulus);
            assert_eq!(sum, a);
            let expected_add = {
                let gap = modulus - b;
                if a >= gap {
                    a - gap
                } else {
                    a + b
                }
            };
            wrapped += u32::from((a & mask).checked_add(b).is_none_or(|sum| sum > mask));
            assert_eq!(
                value(na.add_mod(nb, n)),
                expected_add,
                "{a} + {b} mod {modulus}"
            );
            let expected_sub = if a >= b { a - b } else { a + (modulus - b) };
            assert_eq!(
                value(na.sub_mod(nb, n)),
                expected_sub,
                "{a} − {b} mod {modulus}"
            );
            let half = na.half_mod(n);
            assert_eq!(
                value(half.add_mod(half, n)),
                a,
                "twice half of {a} mod {modulus}"
            );
            assert_eq!(value(na.rem(n)), a);
            assert_eq!(
                value(ctx.mul(na, nb)),
                mul_mod(a, b, modulus),
                "{a} · {b} mod {modulus}"
            );
            assert_eq!(value(ctx.lower(ctx.lift(na))), a, "lift then lower");
            // A short power, by repeated reference multiplication.
            let exponent = stream.next() % 50;
            let mut expected = 1 % modulus;
            for _ in 0..exponent {
                expected = mul_mod(expected, a, modulus);
            }
            assert_eq!(
                value(ctx.pow(na, Nat::from_u64(exponent))),
                expected,
                "{a}^{exponent} mod {modulus}"
            );
        }
        wrapped
    }

    #[test]
    fn two_limb_arithmetic_is_u128_arithmetic_with_the_top_bit_set() {
        let wrapped = arithmetic_agrees::<TwoLimbs>(0x5052_4953_4D32);
        assert!(wrapped > 500, "{wrapped} sums wrapped past the width");
    }

    #[test]
    fn one_limb_arithmetic_is_u128_arithmetic_with_the_top_bit_set() {
        let wrapped = arithmetic_agrees::<OneLimb>(0x5052_4953_4D31);
        assert!(wrapped > 500, "{wrapped} sums wrapped past the width");
    }

    /// Trial division, for an integer of at most 64 bits.
    fn is_prime_u64(n: u64) -> bool {
        n >= 2 && (2..=n.isqrt()).all(|d| !n.is_multiple_of(d))
    }

    /// The verdict at one limb, where the modulus can fill the width.
    #[test]
    fn one_limb_verdicts_are_trial_division() {
        for n in 0u64..5_000 {
            assert_eq!(
                is_baillie_psw::<OneLimb>(&n.to_le_bytes()),
                is_prime_u64(n),
                "{n}"
            );
        }
        // The top of a 32-bit range, where trial division is still quick,
        // and the largest primes below `2^64`, which fill the limb.
        for n in (u64::from(u32::MAX) - 2_000..=u64::from(u32::MAX)).step_by(1) {
            assert_eq!(
                is_baillie_psw::<OneLimb>(&n.to_le_bytes()),
                is_prime_u64(n),
                "{n}"
            );
        }
        for (offset, prime) in [
            (59u64, true),
            (83, true),
            (95, true),
            (57, false),
            (1, false),
        ] {
            let n = u64::MAX - offset + 1;
            assert_eq!(
                is_baillie_psw::<OneLimb>(&n.to_le_bytes()),
                prime,
                "2^64 − {offset}"
            );
        }
        // One limb holds nothing wider.
        assert!(!is_baillie_psw::<OneLimb>(&[0xff; 9]));
    }

    /// A prime that fits the 21-limb width and not the 16-limb one:
    /// `2^1279 − 1`, a Mersenne prime. The challenge degree of `p664` has
    /// 1,323 bits, so this is the size the wide width exists for, and the
    /// one place in this crate where a "prime" verdict above 1,024 bits is
    /// held against a number known to be prime from outside it. The
    /// 16-limb width refuses the integer as too wide; it does not judge
    /// its low limbs.
    #[test]
    fn a_mersenne_prime_above_the_narrower_width_passes_at_the_wider() {
        use prism_field::DegreeLimbs;
        const EXPONENT: usize = 1279;
        let mut mersenne = vec![0xffu8; EXPONENT.div_ceil(8)];
        *mersenne.last_mut().expect("nonempty") = u8::MAX >> (8 - EXPONENT % 8);
        const {
            assert!(EXPONENT > <DegreeLimbs<16> as DegreeWidth>::BITS);
            assert!(EXPONENT < <DegreeLimbs<21> as DegreeWidth>::BITS);
        }
        assert!(is_baillie_psw::<DegreeLimbs<21>>(&mersenne));
        assert!(!is_baillie_psw::<DegreeLimbs<16>>(&mersenne));
        // Its odd neighbours below are composite: 2^1279 − 3 and −5.
        for below in [2u8, 4] {
            let mut neighbour = mersenne.clone();
            neighbour[0] -= below;
            assert!(
                !is_baillie_psw::<DegreeLimbs<21>>(&neighbour),
                "2^1279 − {}",
                below + 1
            );
        }
        // 2^1277 − 1 has a prime exponent and is composite.
        let mut composite = vec![0xffu8; 1277usize.div_ceil(8)];
        *composite.last_mut().expect("nonempty") = u8::MAX >> (8 - 1277 % 8);
        assert!(!is_baillie_psw::<DegreeLimbs<21>>(&composite));
    }

    /// The same integer has one verdict at every width that holds it.
    #[test]
    fn a_verdict_does_not_depend_on_the_width() {
        use prism_field::DegreeLimbs;
        let mut stream = Stream(0x5052_4953_4D57);
        let mut primes = 0;
        for _ in 0..3_000 {
            let n = stream.wide() | 1;
            let bytes = n.to_le_bytes();
            let verdict = is_baillie_psw::<TwoLimbs>(&bytes);
            assert_eq!(is_baillie_psw::<DegreeLimbs<16>>(&bytes), verdict, "{n}");
            assert_eq!(is_baillie_psw::<DegreeLimbs<21>>(&bytes), verdict, "{n}");
            primes += u32::from(verdict);
        }
        // About one odd 128-bit integer in 44 is prime.
        assert!((30..150).contains(&primes), "{primes} primes of 3,000");
    }
}
