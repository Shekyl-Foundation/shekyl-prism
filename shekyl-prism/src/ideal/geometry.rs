// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Widths and products shared by ideal admission and the normalized Gram.
//!
//! The order denominator is the source. The index and the numerator norm
//! square it. A shift that serves another job, such as the reducer's doubled
//! diagonal, is not one of these constants.
use super::Vector;
use prism_int::{Int, SecretBit, SecretInt};

/// Bits of the denominator of the maximal order `O0`.
///
/// Numerator coordinates are twice the quaternion coefficients. A norm-`N`
/// ideal's integer pivots are `2N`, and an element of `O0` has coordinates
/// congruent modulo two.
pub const ORDER_DENOMINATOR_BITS: u32 = 1;

/// Bits of the squared order denominator, `2 * ORDER_DENOMINATOR_BITS`.
///
/// A norm-`N` ideal has integer index `2^{DENOMINATOR_SQUARE_BITS} N²`,
/// and `N4(v) = 2^{DENOMINATOR_SQUARE_BITS} · nrd(v/2)`.
pub const DENOMINATOR_SQUARE_BITS: u32 = 2 * ORDER_DENOMINATOR_BITS;

/// Factor-256 guard on an admitted pre-reduced basis:
/// `0 < nrd(u) ≤ nrd(v) ≤ 2^GUARD_BITS · √p · N`.
pub const GUARD_BITS: u32 = 8;

/// Slack in the Q4 bound `B = ⌈L/2⌉ + Q4_SLACK_BITS`.
/// A secret norm is admitted only while it is strictly below `2^B`.
pub const Q4_SLACK_BITS: u32 = 32;

/// Coefficient of `e` in the verifier degree bound
/// `q < 2^(DEGREE_EXPONENT·e − DEGREE_SHIFT)`.
pub const DEGREE_EXPONENT: u32 = 2;

/// Subtrahend of that degree bound.
pub const DEGREE_SHIFT: u32 = 5;

/// `⌈L/2⌉ + Q4_SLACK_BITS`: the exclusive bit length of a Q4 secret norm.
pub fn secret_norm_bits(prime_bits: u32) -> u32 {
    prime_bits.div_ceil(2) + Q4_SLACK_BITS
}

/// Bit length of the verifier's strict degree bound at exponent `e`.
pub fn degree_bits(exponent: u32) -> u32 {
    DEGREE_EXPONENT * exponent - DEGREE_SHIFT
}

/// Left action of (i+j)/2 on raw O0 numerators at the widened constructor
/// width. The bit records integrality in the ordered basis (1, i, H, (1+k)/2).
pub(super) fn left_half_i_plus_j<const D: usize>(
    column: &Vector<D>,
    p_plus_one_over_two: &SecretInt<D>,
) -> (Vector<D>, SecretBit) {
    let [a, b, c, d] = column;
    let twice_real = a.sub(d);
    let twice_imag = b.sub(c);
    let integral = twice_real
        .low_bits_zero(ORDER_DENOMINATOR_BITS)
        .and(&twice_imag.low_bits_zero(ORDER_DENOMINATOR_BITS));
    let real = twice_real.shr(ORDER_DENOMINATOR_BITS);
    let imag = twice_imag.shr(ORDER_DENOMINATOR_BITS).neg();
    let image = [
        imag.sub(&p_plus_one_over_two.mul(c)),
        real.add(&p_plus_one_over_two.mul(d)),
        real,
        imag,
    ];
    (image, integral)
}

/// Real and imaginary parts of the numerator Hermitian product.
///
/// `left` and `right` are `O0` numerators `(1, i, j, k)`. `p` is the field
/// prime at the same width. The real part is the numerator norm `N4`.
pub fn hermitian<const D: usize>(
    left: &[SecretInt<D>; 4],
    right: &[SecretInt<D>; 4],
    p: &SecretInt<D>,
) -> (SecretInt<D>, SecretInt<D>) {
    let pair = |i: usize| {
        let a = &left[i];
        let b = &left[i + 1];
        let c = &right[i];
        let d = &right[i + 1];
        (a.mul(c).add(&b.mul(d)), b.mul(c).sub(&a.mul(d)))
    };
    let (r, i) = pair(0);
    let (s, j) = pair(2);
    (r.add(&s.mul(p)), i.add(&j.mul(p)))
}

/// Inverse of an odd integer modulo `2^(64W)`, by Newton iteration.
///
/// The low word starts correct modulo 2. An even `t` has no inverse;
/// the caller rejects that case with its own predicate. The iteration
/// count is the limb width, so it does not depend on the value.
pub fn inverse_odd<const W: usize>(t: &SecretInt<W>) -> SecretInt<W> {
    let max = SecretInt::from_public(&Int::<W>::from_le_limbs(&[u64::MAX; W], false).unwrap());
    let mut x = SecretInt::one();
    for _ in 0..(W * 64).next_power_of_two().trailing_zeros() {
        let diff = SecretInt::from_u64(2).sub(&t.mul_low(&x));
        let complement = max.sub(&diff.abs()).add(&SecretInt::one());
        x = x.mul_low(&SecretInt::select(&diff, &complement, &diff.is_negative()));
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_action_on_order_basis_and_fractional_inputs() {
        // p=11. H=(i+j)/2, K=(1+k)/2. The four products are H, -K,
        // -3, 3i. Their raw numerator coordinates fix signs and scale.
        let scalar = |x| SecretInt::<2>::from_public(&Int::from_i64(x));
        let weight = scalar(6);
        let cases = [
            ([2, 0, 0, 0], Some([0, 1, 1, 0])),
            ([0, 2, 0, 0], Some([-1, 0, 0, -1])),
            ([0, 1, 1, 0], Some([-6, 0, 0, 0])),
            ([1, 0, 0, 1], Some([0, 6, 0, 0])),
            ([1, 0, 0, 0], None),
            ([0, -1, 0, 0], None),
        ];
        for (column, expected) in cases {
            let (image, integral) = left_half_i_plus_j(&column.map(scalar), &weight);
            let verdict = match expected {
                Some(raw) => image
                    .iter()
                    .zip(raw)
                    .fold(integral, |ok, (x, want)| ok.and(&x.ct_eq(&scalar(want)))),
                None => integral.not(),
            };
            assert_eq!(verdict.declassify(), Ok(true));
        }
    }
}
