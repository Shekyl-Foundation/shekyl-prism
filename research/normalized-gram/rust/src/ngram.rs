// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Normalized class Gram for the exp64 reducer (research/normalized-gram).
//!
//! For an O0 basis U, V of an integral left ideal of norm T, the numerator
//! entries A = N4(U), C = N4(V), h0 = D(V,U), h1 = D(V,JU) are multiples of
//! 2T (diagonal: 4T). Divided by 2T they form an integral Hermitian Gram of
//! determinant p. The reducer below is the a60fd06 combined candidate
//! (shared division, carried Gram, power-of-two shifts; `zi_exp::step_exp_t`)
//! with the Gram held in W limbs instead of N = 2C. W comes from the public
//! width bound of bounds_ng.py; it equals C at all five sets.
//!
//! Constructors:
//!  * `gram_hnf`: the generic HNF U = (2T,0,0,0), V = (x,y,1,0) gives
//!    (2T, (x^2+y^2+p)/(2T), x, y): two squarings and one exact division.
//!  * `gram_general`: any O0 basis (pre-reduced intersection, chained
//!    bases): the numerator Gram from coordinates, four exact divisions,
//!    then the determinant check A C - h0^2 - h1^2 = p.
//! Exact division is by Hensel lifting (T odd): Q = n * T^-1 mod 2^(64W),
//! then the full product Q T is compared with n. The comparison is the
//! integrality check: a remainder makes it fail. No secret-dependent branch,
//! address or loop count; the fixed Newton count for T^-1 is public. A
//! restoring division (`div_exact_restoring`) is kept for comparison.
//! Every check returns a secret all-ones/zero mask; the caller decides how
//! to declassify a failure (an invariant failure, SIGNER_SPEC 2.3).

use crate::ct::*;
use crate::zi::{nrd, Vec4};
use crate::zi_exp::{herm, step_exp_t};

/// (A, C, h0, h1) in `L` limbs, two's complement.
pub type Gram<const L: usize> = [[u64; L]; 4];

/// Low W limbs of a * b (both W limbs, any sign; modular).
#[inline(always)]
pub fn mul_lo<const W: usize>(a: &[u64; W], b: &[u64; W]) -> [u64; W] {
    let mut r = [0u64; W];
    for i in 0..W {
        let mut c: u128 = 0;
        let ai = a[i] as u128;
        for j in 0..W - i {
            let t = r[i + j] as u128 + ai * b[j] as u128 + c;
            r[i + j] = t as u64;
            c = t >> 64;
        }
    }
    r
}

/// T^-1 mod 2^(64W) for odd T. One-word start (5 correct bits from
/// 3T xor 2, then 4 Newton steps to 64 bits), then ceil(log2 W) full-width
/// Newton steps x <- x (2 - T x). All counts public.
#[inline(always)]
pub fn inv_pow2<const W: usize>(t: &[u64; W]) -> [u64; W] {
    let t0 = t[0];
    let mut x0 = t0.wrapping_mul(3) ^ 2;
    for _ in 0..4 {
        x0 = x0.wrapping_mul(2u64.wrapping_sub(t0.wrapping_mul(x0)));
    }
    let mut x = [0u64; W];
    x[0] = x0;
    let mut bits = 64usize;
    while bits < 64 * W {
        let tx = mul_lo(t, &x);
        let mut two_minus = [0u64; W];
        two_minus[0] = 2;
        sub_assign(&mut two_minus, &tx);
        x = mul_lo(&x, &two_minus);
        bits *= 2;
    }
    x
}

/// Zero- or sign-extend / truncate between limb counts (public).
#[inline(always)]
pub fn resize<const A: usize, const B: usize>(a: &[u64; A], signed: bool) -> [u64; B] {
    let fill = if signed { sign_mask(a) } else { 0 };
    let mut r = [fill; B];
    for i in 0..B.min(A) {
        r[i] = a[i];
    }
    r
}

/// a >> k for a public k < 64 (arithmetic if `signed`).
#[inline(always)]
fn shr_small<const L: usize>(a: &[u64; L], k: u32, signed: bool) -> [u64; L] {
    let fill = if signed { sign_mask(a) } else { 0 };
    let mut r = [0u64; L];
    for i in 0..L {
        let hi = if i + 1 < L { a[i + 1] } else { fill };
        r[i] = (a[i] >> k) | (hi << (64 - k));
    }
    r
}

/// All-ones if a == b.
#[inline(always)]
pub fn eq_mask<const L: usize>(a: &[u64; L], b: &[u64; L]) -> u64 {
    let mut d = 0u64;
    for i in 0..L {
        d |= a[i] ^ b[i];
    }
    mask(is_zero(d))
}

/// Exact quotient n / T for n of N = 2C limbs (two's complement), T odd and
/// positive of C limbs, quotient of W = C limbs: Hensel, then the check
/// Q T == n over all N limbs. Returns (Q, ok mask).
#[inline(always)]
pub fn div_exact_hensel<const C: usize, const N: usize>(
    n: &[u64; N], t: &[u64; C], tinv: &[u64; C],
) -> ([u64; C], u64) {
    let lo: [u64; C] = resize(n, false);
    let q = mul_lo(&lo, tinv);
    let back: [u64; N] = mul_wide(&q, t);
    (q, eq_mask(&back, n))
}

/// Same contract by a restoring division with a fixed 64C quotient bits.
/// Requires n >= 0 (the diagonal entries). Returns (Q, ok mask).
#[inline(always)]
pub fn div_exact_restoring<const C: usize, const N: usize>(n: &[u64; N], t: &[u64; C]) -> ([u64; C], u64) {
    // remainder in C + 1 limbs (held in N limbs for simplicity of types)
    let mut r = [0u64; N];
    // high part: n >> 64C, must be < T (true when the quotient fits C limbs)
    for i in 0..C {
        r[i] = n[C + i];
    }
    let tt: [u64; N] = resize(t, false);
    let mut q = [0u64; C];
    let mut pos = 64 * C;
    while pos > 0 {
        pos -= 1;
        let bit = (n[pos / 64] >> (pos % 64)) & 1; // public index
        let mut c = bit;
        for i in 0..C + 1 {
            let nc = r[i] >> 63;
            r[i] = (r[i] << 1) | c;
            c = nc;
        }
        let mut sub = r;
        let borrow = sub_assign(&mut sub, &tt);
        let ge = 1 ^ borrow;
        r = select_arr(mask(ge), &sub, &r);
        q[pos / 64] |= ge << (pos % 64);
    }
    let zero = [0u64; N];
    // the initial high part must also have been below T
    let hi_ok = lt_unsigned(&resize::<C, N>(&{
        let mut h = [0u64; C];
        h.copy_from_slice(&n[C..2 * C]);
        h
    }, false), &tt);
    (q, eq_mask(&r, &zero) & mask(hi_ok))
}

/// Generic HNF constructor: U = (2T,0,0,0), V = (x,y,1,0), T odd, x even,
/// y odd in [0, 2T). Returns (U, V, normalized Gram, ok mask).
/// `tinv` = T^-1 mod 2^(64C) (computed by the caller: cold or cached).
#[inline(always)]
pub fn gram_hnf<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], x: &[u64; C], y: &[u64; C], p: &[u64; PL], tinv: &[u64; C],
) -> (Vec4<C>, Vec4<C>, Gram<C>, u64) {
    let mut n: [u64; N] = mul_wide(x, x);
    add_assign(&mut n, &mul_wide(y, y));
    add_assign(&mut n, &resize::<PL, N>(p, false));
    let ok_low = mask(is_zero(n[0] & 3)); // 4T | n needs 4 | n
    let ok_odd = mask(t[0] & 1);
    let n4 = shr_small(&n, 2, false);
    let (q, ok_div) = div_exact_hensel::<C, N>(&n4, t, tinv);
    let mut two_t = *t;
    add_assign(&mut two_t, t);
    let mut cc = q;
    add_assign(&mut cc, &q); // C = 2 Q
    let mut one = [0u64; C];
    one[0] = 1;
    let u: Vec4<C> = [two_t, [0; C], [0; C], [0; C]];
    let v: Vec4<C> = [*x, *y, one, [0; C]];
    (u, v, [two_t, cc, *x, *y], ok_low & ok_odd & ok_div)
}

/// The same with the restoring division (comparison only).
#[inline(always)]
pub fn gram_hnf_restoring<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], x: &[u64; C], y: &[u64; C], p: &[u64; PL],
) -> (Vec4<C>, Vec4<C>, Gram<C>, u64) {
    let mut n: [u64; N] = mul_wide(x, x);
    add_assign(&mut n, &mul_wide(y, y));
    add_assign(&mut n, &resize::<PL, N>(p, false));
    let ok_low = mask(is_zero(n[0] & 3));
    let ok_odd = mask(t[0] & 1);
    let n4 = shr_small(&n, 2, false);
    let (q, ok_div) = div_exact_restoring::<C, N>(&n4, t);
    let mut two_t = *t;
    add_assign(&mut two_t, t);
    let mut cc = q;
    add_assign(&mut cc, &q);
    let mut one = [0u64; C];
    one[0] = 1;
    let u: Vec4<C> = [two_t, [0; C], [0; C], [0; C]];
    let v: Vec4<C> = [*x, *y, one, [0; C]];
    (u, v, [two_t, cc, *x, *y], ok_low & ok_odd & ok_div)
}

/// A C - h0^2 - h1^2 == p, all mask arithmetic, at 2C limbs.
#[inline(always)]
pub fn det_is_p<const C: usize, const N: usize, const PL: usize>(g: &Gram<C>, p: &[u64; PL]) -> u64 {
    let mut d: [u64; N] = mul_wide(&g[0], &g[1]);
    sub_assign(&mut d, &mul_wide(&g[2], &g[2]));
    sub_assign(&mut d, &mul_wide(&g[3], &g[3]));
    eq_mask(&d, &resize::<PL, N>(p, false))
}

/// General constructor for any O0 pair (U, V) of a norm-T ideal: numerator
/// Gram from coordinates, exact division of the diagonal by 4T and of the
/// projections by 2T, then the determinant check. Returns (Gram, ok mask).
#[inline(always)]
pub fn gram_general<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], u: &Vec4<C>, v: &Vec4<C>, p: &[u64; PL], tinv: &[u64; C],
) -> (Gram<C>, u64) {
    let a: [u64; N] = nrd::<C, N, PL>(u, p);
    let c: [u64; N] = nrd::<C, N, PL>(v, p);
    let (h0, h1): ([u64; N], [u64; N]) = herm::<C, N, PL>(v, u, p);
    let mut ok = mask(t[0] & 1);
    let mut g: Gram<C> = [[0; C]; 4];
    for (k, (val, diag)) in [(&a, true), (&c, true), (&h0, false), (&h1, false)].into_iter().enumerate() {
        let sh = if diag { 2 } else { 1 }; // public
        ok &= mask(is_zero(val[0] & ((1 << sh) - 1)));
        let n = shr_small(val, sh, !diag);
        let (q, okd) = div_exact_hensel::<C, N>(&n, t, tinv);
        ok &= okd;
        g[k] = q;
        if diag {
            let qq = q;
            add_assign(&mut g[k], &qq); // diagonal: 2 (N4 / 4T)
        }
    }
    ok &= det_is_p::<C, N, PL>(&g, p);
    (g, ok)
}

/// Strict initial ordering on the normalized Gram.
#[inline(always)]
pub fn order<const C: usize, const W: usize>(u: &mut Vec4<C>, v: &mut Vec4<C>, g: &mut Gram<W>) {
    let m = mask(lt_unsigned(&g[1], &g[0]));
    for r in 0..4 {
        cswap(m, &mut u[r], &mut v[r]);
    }
    let (a, rest) = g.split_at_mut(1);
    cswap(m, &mut a[0], &mut rest[0]);
    cond_neg(&mut g[3], m);
}

/// `steps` exp64 steps on any Gram scaling; `sink` sees every step's
/// decisions (a no-op closure in the benchmarks).
#[inline(always)]
pub fn reduce_loop<const C: usize, const W: usize, F: FnMut(i64, i64, u64, u64, u64)>(
    u: &mut Vec4<C>, v: &mut Vec4<C>, g: &mut Gram<W>, steps: usize, mut sink: F,
) {
    let [ref mut a, ref mut c, ref mut h0, ref mut h1] = *g;
    for _ in 0..steps {
        let (m0, m1, s, big, sw) = step_exp_t::<C, W>(u, v, a, c, h0, h1);
        sink(m0, m1, s, big, sw);
    }
}

/// Rick's combined candidate setup (a60fd06 reduce_exp order) on any pair:
/// both numerator norms, strict swap, then the Hermitian products.
#[inline(always)]
pub fn setup_numerator<const C: usize, const N: usize, const PL: usize>(
    u: &mut Vec4<C>, v: &mut Vec4<C>, p: &[u64; PL],
) -> Gram<N> {
    let mut nu: [u64; N] = nrd::<C, N, PL>(u, p);
    let mut nv: [u64; N] = nrd::<C, N, PL>(v, p);
    let m = mask(lt_unsigned(&nv, &nu));
    for r in 0..4 {
        cswap(m, &mut u[r], &mut v[r]);
    }
    cswap(m, &mut nu, &mut nv);
    let (h0, h1) = herm::<C, N, PL>(v, u, p);
    [nu, nv, h0, h1]
}

// ---- non-inlined entry points for timing and Memcheck ----

/// Normalized setup, generic HNF, cold (Newton inverse of T included).
#[inline(never)]
pub fn setup_norm_cold<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], x: &[u64; C], y: &[u64; C], p: &[u64; PL],
) -> (Vec4<C>, Vec4<C>, Gram<C>, u64) {
    let tinv = inv_pow2(t);
    let (mut u, mut v, mut g, ok) = gram_hnf::<C, N, PL>(t, x, y, p, &tinv);
    order(&mut u, &mut v, &mut g);
    (u, v, g, ok)
}

/// Normalized setup, generic HNF, warm: T^-1 = N^-1 q^-1 from a cached
/// per-key N^-1 and the public challenge's q^-1 (one truncated product).
#[inline(never)]
pub fn setup_norm_warm<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], x: &[u64; C], y: &[u64; C], p: &[u64; PL], ninv: &[u64; C], qinv: &[u64; C],
) -> (Vec4<C>, Vec4<C>, Gram<C>, u64) {
    let tinv = mul_lo(ninv, qinv);
    let (mut u, mut v, mut g, ok) = gram_hnf::<C, N, PL>(t, x, y, p, &tinv);
    order(&mut u, &mut v, &mut g);
    (u, v, g, ok)
}

/// Normalized setup, generic HNF, restoring division.
#[inline(never)]
pub fn setup_norm_restoring<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], x: &[u64; C], y: &[u64; C], p: &[u64; PL],
) -> (Vec4<C>, Vec4<C>, Gram<C>, u64) {
    let (mut u, mut v, mut g, ok) = gram_hnf_restoring::<C, N, PL>(t, x, y, p);
    order(&mut u, &mut v, &mut g);
    (u, v, g, ok)
}

/// Normalized setup for any O0 pair (general constructor, cold).
#[inline(never)]
pub fn setup_norm_general<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], u0: &Vec4<C>, v0: &Vec4<C>, p: &[u64; PL],
) -> (Vec4<C>, Vec4<C>, Gram<C>, u64) {
    let tinv = inv_pow2(t);
    let (mut u, mut v) = (*u0, *v0);
    let (mut g, ok) = gram_general::<C, N, PL>(t, &u, &v, p, &tinv);
    order(&mut u, &mut v, &mut g);
    (u, v, g, ok)
}

/// Normalized reduction only.
#[inline(never)]
pub fn reduce_norm<const C: usize>(u: &mut Vec4<C>, v: &mut Vec4<C>, g: &mut Gram<C>, steps: usize) {
    reduce_loop::<C, C, _>(u, v, g, steps, |_, _, _, _, _| {});
}

/// Combined-candidate setup (numerator Gram).
#[inline(never)]
pub fn setup_comb<const C: usize, const N: usize, const PL: usize>(
    u: &mut Vec4<C>, v: &mut Vec4<C>, p: &[u64; PL],
) -> Gram<N> {
    setup_numerator::<C, N, PL>(u, v, p)
}

/// Combined-candidate reduction only.
#[inline(never)]
pub fn reduce_comb<const C: usize, const N: usize>(u: &mut Vec4<C>, v: &mut Vec4<C>, g: &mut Gram<N>, steps: usize) {
    reduce_loop::<C, N, _>(u, v, g, steps, |_, _, _, _, _| {});
}

/// Normalized, setup plus reduction in one call (single signature, cold).
#[inline(never)]
pub fn full_norm<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], x: &[u64; C], y: &[u64; C], p: &[u64; PL], steps: usize,
) -> (Vec4<C>, Vec4<C>, u64) {
    let (mut u, mut v, mut g, ok) = setup_norm_cold::<C, N, PL>(t, x, y, p);
    reduce_loop::<C, C, _>(&mut u, &mut v, &mut g, steps, |_, _, _, _, _| {});
    (u, v, ok)
}

/// Combined candidate, setup plus reduction in one call.
#[inline(never)]
pub fn full_comb<const C: usize, const N: usize, const PL: usize>(
    t: &[u64; C], x: &[u64; C], y: &[u64; C], p: &[u64; PL], steps: usize,
) -> (Vec4<C>, Vec4<C>) {
    let mut one = [0u64; C];
    one[0] = 1;
    let mut u: Vec4<C> = [*t, [0; C], [0; C], [0; C]];
    let mut v: Vec4<C> = [*x, *y, one, [0; C]];
    let mut g = setup_numerator::<C, N, PL>(&mut u, &mut v, p);
    reduce_loop::<C, N, _>(&mut u, &mut v, &mut g, steps, |_, _, _, _, _| {});
    (u, v)
}
