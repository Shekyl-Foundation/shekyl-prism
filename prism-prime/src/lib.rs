// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Baillie–PSW on a public little-endian integer: the test of the
//! challenge degree.
//!
//! [`degree_is_prime`] and [`challenge_degree`] are the two public
//! functions. Both take the width as a type and the exponent as a
//! number. For a parameter set, the verifier's
//! `ChallengeScalar::degree_is_prime` passes the set's own width and
//! chain exponent, and is the call to prefer.
//!
//! The verifier re-exports this crate as `prism_verify::prime`. The
//! signer's ideal layer calls it here. Neither copies the test.
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
//! the primes below 100, one strong Miller–Rabin test at base 2, then the
//! strong Lucas test with Selfridge's parameters. No random bases, so the
//! verdict is a function of `n` alone.
//!
//! A perfect square is refused by the Lucas half, with no filter of its
//! own: the search for Selfridge's discriminant ends empty on a square
//! (`selfridge_discriminant`, in this file).
//!
//! # Every loop is counted
//!
//! No loop here ends on a value the arithmetic computes. Each runs a
//! number of times fixed by the width, by a bit length read once before
//! it starts, or by a constant, so a wrong operator gives a wrong verdict
//! and never a verifier that does not return. The two that are searches
//! by nature, the strip of the factors of two (`Nat::split_twos`) and
//! Euclid's algorithm in `jacobi_nonneg`, have a count they provably do
//! not need in full and stop early inside it.

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

    /// `(odd, twos)` with `self = odd · 2^twos` and `odd` odd.
    ///
    /// Zero has no odd part. It comes back as zero with `twos` the whole
    /// width, after as many steps as the width has bits and no more.
    fn split_twos(self) -> (Self, u32) {
        let mut odd = self;
        let mut twos = 0u32;
        for _ in 0..W::BITS {
            if !odd.is_even() {
                break;
            }
            odd = odd.shr1();
            twos += 1;
        }
        (odd, twos)
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

    /// `self mod modulus`, for a `modulus` above zero.
    ///
    /// One doubling and one addition for each bit of `self`, whatever the
    /// two values are: a `self` already below `modulus` takes the same
    /// path and comes back unchanged.
    fn rem(self, modulus: Self) -> Self {
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
    let symbol = jacobi_nonneg(a.unsigned_abs(), n);
    if a < 0 && n.rem_u32(4) == 3 {
        symbol.negate()
    } else {
        symbol
    }
}

/// The divisions Euclid's algorithm is given for a numerator below `2^32`.
///
/// After the first division both values are below the numerator, and
/// Euclid's algorithm on integers below `2^32` divides fewer than 48
/// times (Lamé: at most `log_φ(√5 · 2^32)`). Removing the factors of two
/// between divisions only shortens it.
const JACOBI_DIVISIONS: usize = 64;

/// `(a/n)` for an odd `n` and a numerator that fits 32 bits.
fn jacobi_nonneg<W: DegreeWidth>(a: u32, n: Nat<W>) -> Jacobi {
    let mut numerator = Nat::from_u64(u64::from(a));
    let mut modulus = n;
    let mut negated = false;
    for _ in 0..JACOBI_DIVISIONS {
        numerator = numerator.rem(modulus);
        if numerator.is_zero() {
            return Jacobi::Zero;
        }
        let (odd, twos) = numerator.split_twos();
        numerator = odd;
        // `(2/m)` is `−1` for `m ≡ 3, 5 (mod 8)`, once for each factor of two.
        if twos % 2 == 1 && matches!(modulus.rem_u32(8), 3 | 5) {
            negated = !negated;
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
    // Not reached: the divisions above are more than the algorithm takes.
    // No symbol was found, and `Zero` is the answer that refuses.
    Jacobi::Zero
}

/// How many discriminants [`selfridge_discriminant`] tries.
const DISCRIMINANT_CANDIDATES: i32 = 256;

/// Selfridge's discriminant: the first `D` of `5, −7, 9, −11, …` whose
/// Jacobi symbol is `−1`. `None` when a shared factor turns up, which
/// proves `n` composite, and `None` when no candidate has that symbol.
///
/// # This is what refuses a perfect square
///
/// The Jacobi symbol modulo `m^2` is the square of the symbol modulo
/// `m`, so it is `0` or `+1` and never `−1`: for a perfect square the
/// search ends at a shared factor or runs out of candidates, and either
/// way the Lucas half refuses. Baillie–PSW as usually written tests for a
/// square first, because an unbounded search does not end on one. This
/// search is bounded, so the square needs no test of its own. A challenge
/// degree `2^(2e−1) − s^2` with `s` odd is `7 (mod 8)` and is never a
/// square, so on the verifier's path the case does not arise at all.
///
/// A non-square `n` gives `−1` for about half of the candidates. One that
/// gives none in 256 is refused, and that is a refusal of a possible
/// prime: none is known, and `docs/VERIFIER_SPEC.md` records the bound.
fn selfridge_discriminant<W: DegreeWidth>(n: Nat<W>) -> Option<i32> {
    for index in 0..DISCRIMINANT_CANDIDATES {
        // `5, −7, 9, −11, …`: the odd numbers from five, alternating in sign.
        let magnitude = 5 + 2 * index;
        let d = if index % 2 == 0 {
            magnitude
        } else {
            -magnitude
        };
        match jacobi(d, n) {
            Jacobi::NonResidue => return Some(d),
            Jacobi::Zero => return None,
            Jacobi::Residue => (),
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
    // The two small constants of the sequence, reduced once.
    let discriminant = reduce_i32(discriminant, ctx.modulus);
    let q = reduce_i32(q, ctx.modulus);
    let mut q_power = q;
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
                .mul(u, discriminant)
                .add_mod(v, ctx.modulus)
                .half_mod(ctx.modulus);
            u = stepped_u;
            v = stepped_v;
            q_power = ctx.mul(q_power, q);
        }
    }
    (u, v, q_power)
}

/// One strong Miller–Rabin test at base 2, for an odd `n` above three.
fn miller_rabin_base2<W: DegreeWidth>(n: Nat<W>) -> bool {
    let below = n.wrapping_sub(Nat::one());
    let (odd_part, twos) = below.split_twos();
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
/// three. A perfect square is refused: [`selfridge_discriminant`].
fn strong_lucas_selfridge<W: DegreeWidth>(n: Nat<W>) -> bool {
    let Some(discriminant) = selfridge_discriminant(n) else {
        return false;
    };
    let q = (1 - discriminant) / 4;
    let (above, carry) = n.overflowing_add(Nat::one());
    if carry {
        // `n` is every bit of the width set: `2^W::BITS − 1`. The width is
        // a multiple of 64, so that is a multiple of three, and `n + 1`
        // does not fit the width to be tested.
        return false;
    }
    let (odd_part, twos) = above.split_twos();
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
pub(crate) fn is_baillie_psw<W: DegreeWidth>(bytes: &[u8]) -> bool {
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
    miller_rabin_base2(n) && strong_lucas_selfridge(n)
}

/// The challenge degree `q = 2^{2·exponent−1} − s^2`, little-endian,
/// without high zero bytes.
///
/// It assumes nothing of its arguments and tests what it needs. `None`
/// when `s` is zero or even, when `s`, its square or `2^{2·exponent−1}`
/// does not fit the width `W`, or when `s² ≥ 2^{2·exponent−1}`. Zero
/// would make `q` a power of two, the prime 2 exactly at exponent 1, and
/// that integer is not a challenge degree.
///
/// The bound it enforces is `s² < 2^{2·exponent−1}`. A challenge is
/// narrower: `s < 2^{exponent−1}`. A scalar between the two gives a
/// degree here and is no challenge. The verifier's
/// `ChallengeScalar::derive` is what keeps a scalar in the challenge's
/// range, and the verifier passes nothing else.
///
/// `W` must be wide enough for the set: for a parameter set `L`, pass
/// `L::Degree` and `L::CHAIN_STEPS`, as the verifier's
/// `ChallengeScalar::degree_is_prime` does.
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
/// `exponent` is the chain exponent (`E − 2` for the current verifier,
/// [`prism_field::ParameterSet::CHAIN_STEPS`]), and `scalar` is `s`
/// little-endian.
///
/// It assumes nothing: `false` for every scalar [`challenge_degree`]
/// gives no degree for, as well as for a composite degree, so a caller
/// cannot tell a malformed scalar from a composite one. Like that
/// function it requires a positive odd scalar with
/// `s² < 2^{2·exponent−1}` and does not enforce the narrower challenge
/// domain `s < 2^{exponent−1}`. The verifier gets that domain from
/// `ChallengeScalar::derive`, and does not reduce
/// supplied bytes.
///
/// "Prime" is Baillie–PSW's verdict: no composite that passes is known,
/// and none is proved not to exist. `docs/VERIFIER_SPEC.md` states the
/// test.
pub fn degree_is_prime<W: DegreeWidth>(exponent: u32, scalar: &[u8]) -> bool {
    challenge_degree::<W>(exponent, scalar).is_some_and(|degree| is_baillie_psw::<W>(&degree))
}

/// `2e − 1`, the exponent in `q = 2^{2e−1} − s^2`.
fn degree_exponent(e: u32) -> Option<usize> {
    let doubled = e.checked_mul(2)?.checked_sub(1)?;
    usize::try_from(doubled).ok()
}

#[cfg(test)]
mod tests;
