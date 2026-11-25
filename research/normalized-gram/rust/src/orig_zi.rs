//! Copy of zi.rs on the 1db834d primitives (orig_ct), for orig_exp only.
// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Fixed-schedule Z[i]-Lagrange reduction, constant time.
//!
//! Port of `zi_gauss_fixed` (research/fixed-schedule-reduction/fsr.py)
//! with the 64-bit bounded-precision quotient (`prec=64`). The lattice is
//! the left ideal with HNF basis {T, T i, alpha, i alpha}, alpha =
//! x + y i + j, reduced-norm form x0^2 + x1^2 + p (x2^2 + x3^2). As a
//! rank-2 Z[i]-lattice its basis is {T, alpha}.
//!
//! Widths: coordinates are C limbs and norms N = 2C limbs, sized per set
//! from the public bound bits(T) <= bits(q) + B, B = ceil(L/2) + 32.

use crate::orig_ct::*;

/// Quotient precision: the top PREC bits of nrd(u), as fsr.py `prec=64`.
pub const PREC: u64 = 64;

pub type Vec4<const C: usize> = [[u64; C]; 4];

/// Hermitian form: Z[i]-part (h0, h1) of v * conj(u).
#[inline(always)]
fn herm<const C: usize, const N: usize, const PL: usize>(
    v: &Vec4<C>,
    u: &Vec4<C>,
    p: &[u64; PL],
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

#[inline(always)]
pub fn nrd<const C: usize, const N: usize, const PL: usize>(v: &Vec4<C>, p: &[u64; PL]) -> [u64; N] {
    let mut n: [u64; N] = mul_wide(&v[0], &v[0]);
    add_assign(&mut n, &mul_wide(&v[1], &v[1]));
    let mut t: [u64; N] = mul_wide(&v[2], &v[2]);
    add_assign(&mut t, &mul_wide(&v[3], &v[3]));
    add_assign(&mut n, &mul_trunc(&t, p));
    n
}

/// Gaussian-rounding coefficient round(h / nu) from the top PREC bits of
/// nu, exactly fsr.py `_round_q(h, nu, 64)`:
///   k = max(0, bitlen(nu) - 64); hs = h >> k; ns = nu >> k;
///   m = floor((2 hs + ns) / (2 ns)).
/// hs is saturated at +-2^124 (|h / nu| > 2^60 never occurs in a reduced
/// step on these lattices; if it did the step would still be a valid
/// unimodular move, only a shorter one). Result |m| < 2^62.
#[inline(always)]
pub fn quotient<const N: usize>(h: &[u64; N], nu: &[u64; N]) -> i64 {
    let bl = bit_length(nu);
    let d = bl.wrapping_sub(PREC) as i64;
    let k = (d & !(d >> 63)) as u64;
    let hsf = shr(h, k, true);
    let nsf = shr(nu, k, false);
    let ns = nsf[0] as u128; // < 2^64 by construction

    let sg = sign_mask(&hsf);
    let mut ok = 1u64;
    for i in 2..N {
        ok &= is_zero(hsf[i] ^ sg);
    }
    ok &= is_zero((hsf[1] ^ sg) >> 60);
    let raw = (hsf[0] as u128 | ((hsf[1] as u128) << 64)) as i128;
    let s128 = (sg as i64) as i128; // 0 or -1
    let sat = ((1i128 << 124) ^ s128).wrapping_sub(s128);
    let mo = mask(ok);
    let mo128 = ((mo as u128) << 64 | mo as u128) as i128;
    let hs = (raw & mo128) | (sat & !mo128);

    let num = 2 * hs + ns as i128;
    let den = 2 * ns;
    let sn = num >> 127; // 0 or -1
    let mut a = ((num ^ sn).wrapping_sub(sn)) as u128; // |num|
    a += (den - 1) & (sn as u128); // floor for negative numerators
    // precondition of the 64-bit-quotient division; saturate otherwise
    let fits = (((a >> 64).wrapping_sub(den)) >> 127) as u64 & 1;
    let qa = div_u128_64q(a, den);
    let big = 1 ^ is_zero(qa >> 62);
    let qa = select(mask(fits & (1 ^ big)), qa, 1u64 << 62);
    let s = sn as i64 as u64;
    ((qa ^ s).wrapping_sub(s)) as i64
}

/// One fixed step. Always executed in full.
#[inline(always)]
fn step<const C: usize, const N: usize, const PL: usize>(
    u: &mut Vec4<C>,
    v: &mut Vec4<C>,
    nu: &mut [u64; N],
    nv: &mut [u64; N],
    p: &[u64; PL],
) {
    let (h0, h1) = herm::<C, N, PL>(v, u, p);
    let m0 = quotient(&h0, nu);
    let m1 = quotient(&h1, nu);
    // v <- v - m0 u - m1 (i u);  i (a + b i + c j + d k) = -b + a i - d j + c k
    let mut a0 = mul_scalar(&u[0], m0);
    sub_assign(&mut a0, &mul_scalar(&u[1], m1));
    let mut a1 = mul_scalar(&u[1], m0);
    add_assign(&mut a1, &mul_scalar(&u[0], m1));
    let mut a2 = mul_scalar(&u[2], m0);
    sub_assign(&mut a2, &mul_scalar(&u[3], m1));
    let mut a3 = mul_scalar(&u[3], m0);
    add_assign(&mut a3, &mul_scalar(&u[2], m1));
    sub_assign(&mut v[0], &a0);
    sub_assign(&mut v[1], &a1);
    sub_assign(&mut v[2], &a2);
    sub_assign(&mut v[3], &a3);
    *nv = nrd::<C, N, PL>(v, p);
    let m = mask(lt_unsigned(nv, nu));
    for r in 0..4 {
        cswap(m, &mut u[r], &mut v[r]);
    }
    cswap(m, nu, nv);
}

/// One exact cleanup step, always executed (fixed-iteration full-width
/// restoring divisions for both Gaussian coefficients). This is needed
/// for the finite-precision half-integer ping-pong found by the
/// lagrange-tour-bound study, not an input-dependent fallback.
#[inline(always)]
fn exact_cleanup<const C: usize, const N: usize, const PL: usize>(
    u: &mut Vec4<C>, v: &mut Vec4<C>, nu: &mut [u64; N], nv: &mut [u64; N], p: &[u64; PL],
) {
    let (h0,h1) = herm::<C,N,PL>(v,u,p);
    let m0 = exact_round(&h0, nu);
    let m1 = exact_round(&h1, nu);
    let mut a0 = mul_low(&u[0], &m0);
    sub_assign(&mut a0, &mul_low(&u[1], &m1));
    let mut a1 = mul_low(&u[1], &m0);
    add_assign(&mut a1, &mul_low(&u[0], &m1));
    let mut a2 = mul_low(&u[2], &m0);
    sub_assign(&mut a2, &mul_low(&u[3], &m1));
    let mut a3 = mul_low(&u[3], &m0);
    add_assign(&mut a3, &mul_low(&u[2], &m1));
    sub_assign(&mut v[0], &a0); sub_assign(&mut v[1], &a1);
    sub_assign(&mut v[2], &a2); sub_assign(&mut v[3], &a3);
    *nv = nrd::<C,N,PL>(v,p);
    let m = mask(lt_unsigned(nv,nu));
    for r in 0..4 { cswap(m,&mut u[r],&mut v[r]); }
    cswap(m,nu,nv);
}

/// The full fixed schedule: initial conditional swap, then `steps` steps
/// (a public count, from the parameter set). Returns (u, nrd(u)).
#[inline(never)]
pub fn reduce<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C],
    x: &[u64; C],
    y: &[u64; C],
    p: &[u64; PL],
    steps: usize,
) -> (Vec4<C>, [u64; N]) {
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
    for _ in 0..steps {
        step::<C, N, PL>(&mut u, &mut v, &mut nu, &mut nv, p);
    }
    if cfg!(not(feature = "empirical")) {
        exact_cleanup::<C,N,PL>(&mut u,&mut v,&mut nu,&mut nv,p);
    }
    (u, nu)
}

/// Cost breakdown hooks (benchmark only): each part of one step, not inlined.
pub mod parts {
    use super::*;
    #[inline(never)]
    pub fn herm_part<const C: usize, const N: usize, const PL: usize>(
        v: &Vec4<C>, u: &Vec4<C>, p: &[u64; PL],
    ) -> ([u64; N], [u64; N]) {
        herm::<C, N, PL>(v, u, p)
    }
    #[inline(never)]
    pub fn quotient_part<const N: usize>(h: &[u64; N], nu: &[u64; N]) -> i64 {
        quotient(h, nu)
    }
    #[inline(never)]
    pub fn nrd_part<const C: usize, const N: usize, const PL: usize>(v: &Vec4<C>, p: &[u64; PL]) -> [u64; N] {
        nrd::<C, N, PL>(v, p)
    }
    #[inline(never)]
    pub fn cleanup_part<const C: usize, const N: usize, const PL: usize>(
        u: &mut Vec4<C>, v: &mut Vec4<C>, nu: &mut [u64; N], nv: &mut [u64; N], p: &[u64; PL],
    ) {
        exact_cleanup::<C, N, PL>(u, v, nu, nv, p)
    }
    #[inline(never)]
    pub fn update_part<const C: usize>(v: &mut Vec4<C>, u: &Vec4<C>, m0: i64, m1: i64) {
        let mut a0 = mul_scalar(&u[0], m0);
        sub_assign(&mut a0, &mul_scalar(&u[1], m1));
        let mut a1 = mul_scalar(&u[1], m0);
        add_assign(&mut a1, &mul_scalar(&u[0], m1));
        let mut a2 = mul_scalar(&u[2], m0);
        sub_assign(&mut a2, &mul_scalar(&u[3], m1));
        let mut a3 = mul_scalar(&u[3], m0);
        add_assign(&mut a3, &mul_scalar(&u[2], m1));
        sub_assign(&mut v[0], &a0);
        sub_assign(&mut v[1], &a1);
        sub_assign(&mut v[2], &a2);
        sub_assign(&mut v[3], &a3);
    }
}
