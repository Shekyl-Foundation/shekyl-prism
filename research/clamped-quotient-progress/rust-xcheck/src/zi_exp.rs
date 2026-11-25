// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The exp64 quotient rule (research/clamped-quotient-progress/variants.py
//! `q_exp64`) on the rust-lagrange-ct prototype's arithmetic. Research
//! cross-check and cost measurement only; not the production reducer.
//!
//! Per step, always both paths, selected by a mask:
//!  * window (e = max bl(|h_j|) - bl(nu) <= 60): the prototype's 64-bit
//!    truncated quotient, then an exact +-1 fix from X = 2h + nu - 2 m nu, so the
//!    step is the exact round-half-up step;
//!  * big (e > 60): s = e - 60, qtop_j = floor((|h_j| >> (kA + s)) / ns),
//!    m_j = sign(h_j) (qtop_j - 1) if qtop_j >= 2 else 0, applied as
//!    m_j 2^s: the update vector is shifted left by the secret s.
//! No exact cleanup step.

use crate::ct::*;
use crate::zi::{nrd, Vec4};

pub const T0: i64 = 60;

/// a << k for a secret k in [0, 64L), two's complement. Every stage runs.
#[inline(always)]
pub fn shl<const L: usize>(a: &[u64; L], k: u64) -> [u64; L] {
    let ls = k >> 6;
    let scale = u128::from(bit_scale(k & 63));
    let mut x = *a;
    let mut j = 0u32;
    while (1usize << j) < L {
        let sh = 1usize << j;
        let mut t = [0u64; L];
        for i in 0..L {
            t[i] = if i >= sh { x[i - sh] } else { 0 }; // public index
        }
        x = select_arr(mask((ls >> j) & 1), &t, &x);
        j += 1;
    }
    let mut r = [0u64; L];
    let mut carry = 0;
    for i in 0..L {
        let wide = u128::from(x[i]) * scale;
        r[i] = wide as u64 | carry;
        carry = (wide >> 64) as u64;
    }
    r
}

#[inline(always)]
fn abs_of<const N: usize>(h: &[u64; N]) -> ([u64; N], u64) {
    let s = sign_mask(h);
    let mut a = *h;
    cond_neg(&mut a, s);
    (a, s)
}

/// Both paths use the same unsigned truncated quotient. In the window its
/// signed value is within one integer of the half-up answer. Outside the
/// window it is precisely the leading quotient used for the undershoot.
/// Returns (m0, m1, s, big).
#[inline(always)]
pub fn quotient_exp<const N: usize>(h0: &[u64; N], h1: &[u64; N], nu: &[u64; N]) -> (i64, i64, u64, u64) {
    let bla = bit_length(nu) as i64;
    let d = bla - 64;
    let ka = (d & !(d >> 63)) as u64;
    let ns = shr(nu, ka, false)[0] as u128;
    let (a0, s0) = abs_of(h0);
    let (a1, s1) = abs_of(h1);
    let b0 = bit_length(&a0) as i64;
    let b1 = bit_length(&a1) as i64;
    let bm = select(mask(((b0 - b1) >> 63) as u64 & 1), b1 as u64, b0 as u64) as i64; // max
    let e = bm - bla;
    let big = (((T0 - e) >> 63) as u64) & 1; // e > T0
    let s = select(mask(big), (e - T0) as u64, 0);
    let mut two_nu = *nu;
    add_assign(&mut two_nu, nu);
    let mut out = [0i64; 2];
    for (j, (h, a, sg)) in [(h0, &a0, s0), (h1, &a1, s1)].into_iter().enumerate() {
        let num = shr(a, ka.wrapping_add(s), false);
        let numl = num[0] as u128 | ((num[1] as u128) << 64);
        let qtop = div_u128_64q(numl, ns);
        // Apply the sign after truncation. Exact residual comparisons resolve ties.
        let mw = ((qtop ^ sg).wrapping_sub(sg)) as i64;
        let mn = mul_scalar(nu, mw);
        let mut x = *h;
        add_assign(&mut x, h);
        add_assign(&mut x, nu);
        sub_assign(&mut x, &mn);
        sub_assign(&mut x, &mn);
        let lo = sign_mask(&x) & 1;
        let mut t = x;
        sub_assign(&mut t, &two_nu);
        let hi = 1 ^ (sign_mask(&t) & 1);
        let mw = mw.wrapping_sub(lo as i64).wrapping_add(hi as i64);
        // big: undershoot by one unit of 2^s

        let ge2 = 1 ^ is_zero(qtop >> 1);
        let mag = select(mask(ge2), qtop.wrapping_sub(1), 0);
        let mb = ((mag ^ sg).wrapping_sub(sg)) as i64;
        out[j] = select(mask(big), mb as u64, mw as u64) as i64;
    }
    (out[0], out[1], s, big)
}

#[inline(always)]
fn herm<const C: usize, const N: usize, const PL: usize>(
    v: &Vec4<C>, u: &Vec4<C>, p: &[u64; PL],
) -> ([u64; N], [u64; N]) {
    let mut h0: [u64; N] = mul_wide(&v[0], &u[0]);
    add_assign(&mut h0, &mul_wide(&v[1], &u[1]));
    let mut t0: [u64; N] = mul_wide(&v[2], &u[2]);
    add_assign(&mut t0, &mul_wide(&v[3], &u[3]));
    add_assign(&mut h0, &mul_trunc(&t0, p));
    let mut h1: [u64; N] = mul_wide(&v[1], &u[0]);
    sub_assign(&mut h1, &mul_wide(&v[0], &u[1]));
    let mut t1: [u64; N] = mul_wide(&v[3], &u[2]);
    sub_assign(&mut t1, &mul_wide(&v[2], &u[3]));
    add_assign(&mut h1, &mul_trunc(&t1, p));
    (h0, h1)
}

/// Update one projection h to r = h - m*A, with m = mantissa * 2^s.
/// Its contribution to the decrease in the second norm is m*(h+r).
#[inline(always)]
fn update_projection<const N: usize>(
    h: &mut [u64; N], a: &[u64; N], mantissa: i64, s: u64,
) -> [u64; N] {
    let mut sum = *h;
    sub_assign(h, &shl(&mul_scalar(a, mantissa), s));
    add_assign(&mut sum, h);
    shl(&mul_scalar(&sum, mantissa), s)
}

#[inline(always)]
fn step_exp<const C: usize, const N: usize>(
    u: &mut Vec4<C>, v: &mut Vec4<C>, nu: &mut [u64; N], nv: &mut [u64; N],
    h0: &mut [u64; N], h1: &mut [u64; N],
) -> u64 {
    let (m0, m1, s, big) = quotient_exp(h0, h1, nu);
    let mut a0 = mul_scalar(&u[0], m0);
    sub_assign(&mut a0, &mul_scalar(&u[1], m1));
    let mut a1 = mul_scalar(&u[1], m0);
    add_assign(&mut a1, &mul_scalar(&u[0], m1));
    let mut a2 = mul_scalar(&u[2], m0);
    sub_assign(&mut a2, &mul_scalar(&u[3], m1));
    let mut a3 = mul_scalar(&u[3], m0);
    add_assign(&mut a3, &mul_scalar(&u[2], m1));
    sub_assign(&mut v[0], &shl(&a0, s));
    sub_assign(&mut v[1], &shl(&a1, s));
    sub_assign(&mut v[2], &shl(&a2, s));
    sub_assign(&mut v[3], &shl(&a3, s));
    sub_assign(nv, &update_projection(h0, nu, m0, s));
    sub_assign(nv, &update_projection(h1, nu, m1, s));
    let m = mask(lt_unsigned(nv, nu));
    for r in 0..4 {
        cswap(m, &mut u[r], &mut v[r]);
    }
    cswap(m, nu, nv);
    cond_neg(h1, m); // Swapping the vectors conjugates the Hermitian product.
    big
}

/// Initial strict swap, then `steps` exp64 steps; no cleanup. Returns
/// (u, nrd(u), number of big sub-steps; the count is for the check only).
#[inline(never)]
pub fn reduce_exp<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], x: &[u64; C], y: &[u64; C], p: &[u64; PL], steps: usize,
) -> (Vec4<C>, [u64; N], u64) {
    let mut one = [0u64; C];
    one[0] = 1;
    let mut u: Vec4<C> = [*t, [0; C], [0; C], [0; C]];
    let mut v: Vec4<C> = [*x, *y, one, [0; C]];
    let mut nu: [u64; N] = nrd::<C, N, PL>(&u, p);
    let mut nv: [u64; N] = nrd::<C, N, PL>(&v, p);
    let m = mask(lt_unsigned(&nv, &nu));
    for r in 0..4 {
        cswap(m, &mut u[r], &mut v[r]);
    }
    cswap(m, &mut nu, &mut nv);
    let (mut h0, mut h1) = herm::<C, N, PL>(&v, &u, p);
    let mut nbig = 0u64;
    for _ in 0..steps {
        nbig += step_exp(&mut u, &mut v, &mut nu, &mut nv, &mut h0, &mut h1);
    }
    (u, nu, nbig)
}
