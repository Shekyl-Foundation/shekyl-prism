// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Generic O0 signing HNF on admitted inputs: coordinates and Gram entries have
//! separate public widths. All entries and coordinates use two's complement.

use crate::ct::{add_assign, cond_neg, cswap, lt_unsigned, mask, mul_wide};
use crate::zi::Vec4;
use crate::zi_exp::step_exp;

pub type Gram<const L: usize> = [[u64; L]; 4];

fn norm_v<const C: usize, const W: usize, const P: usize>(
    x: &[u64; C],
    y: &[u64; C],
    p: &[u64; P],
) -> [u64; W] {
    let mut out = mul_wide::<C, W>(x, x);
    add_assign(&mut out, &mul_wide(y, y));
    let mut weight = [0; W];
    weight[..P].copy_from_slice(p);
    add_assign(&mut out, &weight);
    out
}

/// Raw numerator Gram of (2T, 0, 0, 0) and (x, y, 1, 0).
#[inline(never)]
pub fn raw<const C: usize, const W: usize, const P: usize>(
    t: &[u64; C],
    x: &[u64; C],
    y: &[u64; C],
    p: &[u64; P],
) -> Gram<W> {
    let mut d = *t;
    add_assign(&mut d, t);
    [
        mul_wide(&d, &d),
        norm_v(x, y, p),
        mul_wide(&d, x),
        mul_wide(&d, y),
    ]
}

/// Recover the exact quotient modulo 2^(64*C), one low limb at a time.
/// Requires an odd divisor and an exact quotient in the admitted width.
fn divide_exact_odd<const C: usize>(mut residual: [u64; C], t: &[u64; C]) -> [u64; C] {
    let mut inv = 1u64;
    for _ in 0..6 {
        inv = inv.wrapping_mul(2u64.wrapping_sub(inv.wrapping_mul(t[0])));
    }
    let mut quotient = [0; C];
    for i in 0..C {
        let digit = residual[i].wrapping_mul(inv);
        quotient[i] = digit;
        let mut borrow = 0u128;
        for (limb, divisor) in residual[i..].iter_mut().zip(t) {
            let product = u128::from(digit) * u128::from(*divisor) + borrow;
            let (value, underflow) = limb.overflowing_sub(product as u64);
            *limb = value;
            borrow = (product >> 64) + u128::from(underflow);
        }
    }
    quotient
}

/// Integral class Gram. The constructor supplies an odd T, even x, odd y,
/// 0 <= x,y < 2T, and 4T dividing x²+y²+p.
#[inline(never)]
pub fn normalized<const C: usize, const W: usize, const P: usize>(
    t: &[u64; C],
    x: &[u64; C],
    y: &[u64; C],
    p: &[u64; P],
) -> Gram<C> {
    let c = norm_v::<C, W, P>(x, y, p);
    let half = std::array::from_fn(|i| (c[i] >> 1) | (c[i + 1] << 63));
    let mut a = *t;
    add_assign(&mut a, t);
    [a, divide_exact_odd(half, t), *x, *y]
}

/// Run both physical variants with identical coordinate arithmetic and swaps.
#[inline(never)]
pub fn reduce<const C: usize, const G: usize>(
    t: &[u64; C],
    x: &[u64; C],
    y: &[u64; C],
    gram: Gram<G>,
    steps: usize,
) -> (Vec4<C>, Vec4<C>, Gram<G>) {
    let mut u = [[0; C]; 4];
    u[0] = *t;
    add_assign(&mut u[0], t);
    let mut v = [*x, *y, [0; C], [0; C]];
    v[2][0] = 1;
    let [mut a, mut c, mut h0, mut h1] = gram;
    let swap = mask(lt_unsigned(&c, &a));
    for (x, y) in u.iter_mut().zip(&mut v) {
        cswap(swap, x, y);
    }
    cswap(swap, &mut a, &mut c);
    cond_neg(&mut h1, swap);
    for _ in 0..steps {
        step_exp(&mut u, &mut v, &mut a, &mut c, &mut h0, &mut h1);
    }
    (u, v, [a, c, h0, h1])
}

#[cfg(test)]
mod tests {
    use super::divide_exact_odd;
    use crate::sets::{from_limbs_unsigned, to_limbs};

    #[test]
    fn odd_division_recovers_every_quotient_limb() {
        let max = u64::MAX;
        for t in [[1, 0, 0], [3, max, 0], [max, 0, max], [max; 3]] {
            for q in [[0; 3], [1, 0, 0], [max, 0, max], [0, max, max], [max; 3]] {
                let product = from_limbs_unsigned(&q) * from_limbs_unsigned(&t);
                assert_eq!(divide_exact_odd(to_limbs(&product), &t), q);
            }
        }
    }
}
