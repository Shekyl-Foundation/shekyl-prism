// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Fixed-count divstep modular inverse (research prototype).
//!
//! The algorithm is Bernstein–Yang's divstep with the starting point
//! δ₀ = 1/2 of Bernstein, Chen, Harrison, Huang, Maxwell, Wang, Wuille and
//! Yang, *Accelerating and verifying constant-time modular inversion*
//! (EUROCRYPT 2026), Theorem 1: for odd `f` and `0 ≤ g ≤ f ≤ 2^b`,
//! `g_n = 0` once `n ≥ ⌈(9437 b + 1)/4096⌉`. The layout (signed 62-bit
//! limbs, 62-divstep jumps on the low words, the `(−2m, m)` coefficient
//! range and the `m⁻¹ mod 2^62` correction) follows libsecp256k1's
//! `modinv64` and its `doc/safegcd_implementation.md`; it is reimplemented
//! here for any limb count `K`.
//!
//! δ is held doubled, as the odd integer `2δ`.
//!
//! Every loop is counted by `K` or by the public jump count. No branch and
//! no index depends on the modulus or the operand: choices are masks. The
//! modulus may be secret; only its bit bound `b` (hence `K` and the jump
//! count) is public. **No constant-time claim**: see the report.

#![forbid(unsafe_code)]

pub mod rng;

/// Bits per limb of the internal signed representation.
pub const LIMB: u32 = 62;
const M62: u64 = u64::MAX >> 2;

/// Divsteps per jump.
pub const STEPS_PER_JUMP: u32 = 62;

/// EUROCRYPT 2026 Theorem 1 bound: divsteps for `0 ≤ g ≤ f ≤ 2^b`, δ₀ = 1/2.
pub const fn divsteps_2026(b: u32) -> u32 {
    let num = 9437u64 * b as u64 + 1;
    num.div_ceil(4096) as u32
}

/// Bernstein–Yang 2019 Theorem 11.2 bound (δ₀ = 1, `|f|,|g| ≤ 2^b`, `b ≥ 46`).
pub const fn divsteps_2019(b: u32) -> u32 {
    (49 * b + 80) / 17
}

/// Jumps of [`STEPS_PER_JUMP`] divsteps covering `divsteps`.
pub const fn jumps(divsteps: u32) -> u32 {
    divsteps.div_ceil(STEPS_PER_JUMP)
}

/// Internal limbs for a modulus below `2^b`: room for the sign and for the
/// `(−2m, m)` range of the coefficients, with the top-limb products kept
/// inside `i128` (checked by the debug-build tests, which trap on overflow).
pub const fn limbs_for_bits(b: u32) -> usize {
    ((b + 3) as usize).div_ceil(LIMB as usize)
}

/// A signed integer `Σ v[i]·2^(62 i)`: limbs `0..K−1` in `[0, 2^62)` when
/// normalized, the top limb signed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct S62<const K: usize>(pub [i64; K]);

impl<const K: usize> S62<K> {
    /// From little-endian 64-bit limbs (a magnitude, as `prism-int` holds
    /// one). Bits at or above `62·K − 1` must be zero (the caller's width).
    pub fn from_u64_limbs(src: &[u64]) -> Self {
        let mut out = [0i64; K];
        for (i, limb) in out.iter_mut().enumerate() {
            let bit = 62 * i;
            let (w, s) = (bit / 64, (bit % 64) as u32);
            let lo = if w < src.len() { src[w] >> s } else { 0 };
            // s ≤ 62, so 64 − s ≥ 2 and the shift is in range when s > 0.
            let hi = if s > 0 && w + 1 < src.len() { src[w + 1] << (64 - s) } else { 0 };
            *limb = ((lo | hi) & M62) as i64;
        }
        Self(out)
    }

    /// To little-endian 64-bit limbs. `self` must be normalized and non-negative.
    pub fn to_u64_limbs(&self, dst: &mut [u64]) {
        dst.iter_mut().for_each(|d| *d = 0);
        for i in 0..K {
            let bit = 62 * i;
            let (w, s) = (bit / 64, (bit % 64) as u32);
            let v = self.0[i] as u64;
            if w < dst.len() {
                dst[w] |= v << s;
            }
            if s > 2 && w + 1 < dst.len() {
                dst[w + 1] |= v >> (64 - s);
            }
        }
    }
}

/// All ones when `x < 0`, else zero.
#[inline(always)]
fn neg_mask(x: i64) -> i64 {
    barrier(x >> 63)
}

/// The signer's audited register identity (`prism-ct-barrier`): LLVM cannot
/// see that a mask is 0 or −1 after it, so it cannot turn a masked select
/// back into a branch. Without it, release builds branched on `ok` (audit,
/// REPORT.md §5).
#[inline(always)]
fn barrier(x: i64) -> i64 {
    prism_ct_barrier::value_barrier_u64(x as u64) as i64
}

/// One jump: 62 divsteps on the low words of `f` and `g`, returning the
/// new doubled δ and the transition matrix `(u, v, q, r)` with
/// `2^62·[f'; g'] = [[u, v], [q, r]]·[f; g]`. Entries satisfy
/// `|u| + |v| ≤ 2^62` and `|q| + |r| ≤ 2^62`.
#[inline(always)]
fn divsteps_62(mut d2: i64, f0: u64, g0: u64) -> (i64, [i64; 4]) {
    let (mut f, mut g) = (f0 as i64, g0 as i64);
    let (mut u, mut v, mut q, mut r) = (1i64, 0i64, 0i64, 1i64);
    for _ in 0..STEPS_PER_JUMP {
        // c1: δ > 0 (2δ is odd, never zero). c2: g odd.
        let c1 = neg_mask(d2.wrapping_neg());
        let c2 = barrier(-(g & 1));
        let c = c1 & c2;
        // Swap case: (1 − δ, g, (g − f)/2); else (1 + δ, f, (g + (g mod 2) f)/2).
        let nf = (g & c) | (f & !c);
        let ng = (g.wrapping_sub(f) & c) | (g.wrapping_add(f & c2) & !c);
        let (nu, nv) = ((q & c) | (u & !c), (r & c) | (v & !c));
        let nq = (q.wrapping_sub(u) & c) | (q.wrapping_add(u & c2) & !c);
        let nr = (r.wrapping_sub(v) & c) | (r.wrapping_add(v & c2) & !c);
        d2 = ((2i64.wrapping_sub(d2)) & c) | ((2i64.wrapping_add(d2)) & !c);
        f = nf;
        g = ng >> 1; // exact: ng is even
        u = nu.wrapping_shl(1);
        v = nv.wrapping_shl(1);
        q = nq;
        r = nr;
    }
    // Invariant per step k: 2^k·f_k = u·f₀ + v·g₀ and 2^k·g_k = q·f₀ + r·g₀.
    // The f-row is doubled because f is not halved; the g-row is not doubled
    // because g is. After 62 steps both rows carry the factor 2^62.
    (d2, [u, v, q, r])
}

/// `(u·a + v·b) / 2^62` limb-wise for normalized `a`, `b` (exact division
/// guaranteed by the caller), written normalized.
#[inline(always)]
fn update_fg<const K: usize>(f: &mut S62<K>, g: &mut S62<K>, t: [i64; 4]) {
    let [u, v, q, r] = t.map(i128::from);
    let mut cf = u * f.0[0] as i128 + v * g.0[0] as i128;
    let mut cg = q * f.0[0] as i128 + r * g.0[0] as i128;
    debug_assert_eq!(cf as u64 & M62, 0);
    debug_assert_eq!(cg as u64 & M62, 0);
    cf >>= 62;
    cg >>= 62;
    for i in 1..K {
        cf += u * f.0[i] as i128 + v * g.0[i] as i128;
        cg += q * f.0[i] as i128 + r * g.0[i] as i128;
        f.0[i - 1] = (cf as u64 & M62) as i64;
        g.0[i - 1] = (cg as u64 & M62) as i64;
        cf >>= 62;
        cg >>= 62;
    }
    f.0[K - 1] = cf as i64;
    g.0[K - 1] = cg as i64;
}

/// `(u·d + v·e) / 2^62 mod m` and `(q·d + r·e) / 2^62 mod m`, keeping both
/// in `(−2m, m)` (libsecp256k1 `update_de_62`, any `K`).
#[inline(always)]
fn update_de<const K: usize>(d: &mut S62<K>, e: &mut S62<K>, t: [i64; 4], m: &S62<K>, m_inv62: u64) {
    let [u, v, q, r] = t;
    let sd = neg_mask(d.0[K - 1]);
    let se = neg_mask(e.0[K - 1]);
    let mut md = (u & sd).wrapping_add(v & se);
    let mut me = (q & sd).wrapping_add(r & se);
    let (u, v, q, r) = (u as i128, v as i128, q as i128, r as i128);
    let mut cd = u * d.0[0] as i128 + v * e.0[0] as i128;
    let mut ce = q * d.0[0] as i128 + r * e.0[0] as i128;
    md = md.wrapping_sub(((m_inv62.wrapping_mul(cd as u64)).wrapping_add(md as u64) & M62) as i64);
    me = me.wrapping_sub(((m_inv62.wrapping_mul(ce as u64)).wrapping_add(me as u64) & M62) as i64);
    let (mdw, mew) = (md as i128, me as i128);
    cd += m.0[0] as i128 * mdw;
    ce += m.0[0] as i128 * mew;
    debug_assert_eq!(cd as u64 & M62, 0);
    debug_assert_eq!(ce as u64 & M62, 0);
    cd >>= 62;
    ce >>= 62;
    for i in 1..K {
        cd += u * d.0[i] as i128 + v * e.0[i] as i128 + m.0[i] as i128 * mdw;
        ce += q * d.0[i] as i128 + r * e.0[i] as i128 + m.0[i] as i128 * mew;
        d.0[i - 1] = (cd as u64 & M62) as i64;
        e.0[i - 1] = (ce as u64 & M62) as i64;
        cd >>= 62;
        ce >>= 62;
    }
    d.0[K - 1] = cd as i64;
    e.0[K - 1] = ce as i64;
}

/// `a + (b & mask)` with carries, normalized; fixed shape.
#[inline(always)]
fn add_masked<const K: usize>(a: &mut S62<K>, b: &S62<K>, mask: i64) {
    let mut c: i64 = 0;
    for i in 0..K - 1 {
        c += a.0[i] + (b.0[i] & mask);
        a.0[i] = c & M62 as i64;
        c >>= 62;
    }
    a.0[K - 1] = a.0[K - 1] + (b.0[K - 1] & mask) + c;
}

/// `−a` when `mask` is all ones, else `a`; normalized; fixed shape.
#[inline(always)]
fn negate_masked<const K: usize>(a: &mut S62<K>, mask: i64) {
    // (a XOR mask) − mask, limb-wise with borrow propagation, = a or −a.
    let mut c: i64 = 0;
    for i in 0..K - 1 {
        c += (a.0[i] ^ mask) - mask;
        a.0[i] = c & M62 as i64;
        c >>= 62;
    }
    a.0[K - 1] = (a.0[K - 1] ^ mask) - mask + c;
    // (a ^ mask) − mask on each limb separately is −limb when mask = −1; the
    // carries then re-normalize. For mask = 0 it is the identity.
}

/// Propagate carries so limbs `0..K−1` lie in `[0, 2^62)`.
#[inline(always)]
fn normalize_carries<const K: usize>(a: &mut S62<K>) {
    let mut c: i64 = 0;
    for i in 0..K - 1 {
        c += a.0[i];
        a.0[i] = c & M62 as i64;
        c >>= 62;
    }
    a.0[K - 1] += c;
}

/// `m⁻¹ mod 2^64` for odd `m0`, Newton with a fixed count (correct to 3
/// bits at the start, doubling each step: 3·2^5 = 96 ≥ 64).
#[inline(always)]
pub fn inv_mod_2_64(m0: u64) -> u64 {
    let mut x = m0; // m·m ≡ 1 mod 8 for odd m
    for _ in 0..5 {
        x = x.wrapping_mul(2u64.wrapping_sub(m0.wrapping_mul(x)));
    }
    x
}

/// Result of [`inv_mod_ct`].
pub struct Inverse<const K: usize> {
    /// `x⁻¹ mod m` in `[0, m)` when `ok` is all ones; zero otherwise.
    pub value: S62<K>,
    /// All ones (as `u64`) when `gcd(x, m) = 1`, zero otherwise.
    pub ok: u64,
}

/// `x⁻¹ mod m` for odd `m > 1` and `0 ≤ x < m`, with `m < 2^b` and
/// `jumps ≥ ⌈⌈(9437 b + 1)/4096⌉ / 62⌉`. Fixed shape in `K` and `jumps`.
///
/// Non-invertible `x` (including `x = 0`) gives `ok = 0` and value zero.
/// Preconditions on `m` (odd, `> 1`) and the range of `x` are the caller's:
/// they are not checked here, because checking would need a branch.
pub fn inv_mod_ct<const K: usize>(x: &S62<K>, m: &S62<K>, jumps: u32) -> Inverse<K> {
    let m_inv62 = inv_mod_2_64(m.0[0] as u64) & M62;
    let mut d2: i64 = 1; // 2δ, δ₀ = 1/2
    let mut f = *m;
    let mut g = *x;
    let mut d = S62([0i64; K]);
    let mut e = S62([0i64; K]);
    e.0[0] = 1;
    for _ in 0..jumps {
        let (nd2, t) = divsteps_62(d2, f.0[0] as u64 | ((f.0[1] as u64) << 62), g.0[0] as u64 | ((g.0[1] as u64) << 62));
        d2 = nd2;
        update_fg(&mut f, &mut g, t);
        update_de(&mut d, &mut e, t, m, m_inv62);
    }
    // gcd = |f|. ok ⇔ g = 0 and f = ±1.
    let sf = neg_mask(f.0[K - 1]);
    let mut absf = f;
    negate_masked(&mut absf, sf);
    let mut acc: u64 = (absf.0[0] ^ 1) as u64;
    for i in 1..K {
        acc |= absf.0[i] as u64;
    }
    for i in 0..K {
        acc |= g.0[i] as u64;
    }
    // ok = all ones iff acc == 0.
    let ok = prism_ct_barrier::value_barrier_u64(((acc | acc.wrapping_neg()) >> 63).wrapping_sub(1));
    // d ∈ (−2m, m): add m if negative, negate if f < 0, add m if negative.
    normalize_carries(&mut d);
    let s = neg_mask(d.0[K - 1]);
    add_masked(&mut d, m, s);
    negate_masked(&mut d, sf);
    let s = neg_mask(d.0[K - 1]);
    add_masked(&mut d, m, s);
    for limb in d.0.iter_mut() {
        *limb &= ok as i64;
    }
    Inverse { value: d, ok }
}

/// Divsteps (δ₀ = 1/2) until `g = 0`, one at a time, **variable time**:
/// the measuring instrument for the bound tests, never for secrets.
pub fn divsteps_to_zero_vartime(f: &num_like::Big, g: &num_like::Big, cap: u32) -> Option<u32> {
    num_like::count(f, g, cap)
}

/// A tiny arbitrary-precision helper for the step counter (no dependency).
pub mod num_like {
    /// Little-endian u64 magnitude with a sign.
    #[derive(Clone, Debug)]
    pub struct Big {
        /// Magnitude limbs.
        pub mag: Vec<u64>,
        /// Negative.
        pub neg: bool,
    }
    fn is_zero(a: &Big) -> bool {
        a.mag.iter().all(|&x| x == 0)
    }
    fn cmp_mag(a: &[u64], b: &[u64]) -> core::cmp::Ordering {
        let n = a.len().max(b.len());
        for i in (0..n).rev() {
            let (x, y) = (*a.get(i).unwrap_or(&0), *b.get(i).unwrap_or(&0));
            if x != y {
                return x.cmp(&y);
            }
        }
        core::cmp::Ordering::Equal
    }
    fn add_mag(a: &[u64], b: &[u64]) -> Vec<u64> {
        let n = a.len().max(b.len()) + 1;
        let mut out = vec![0; n];
        let mut c = 0u128;
        for i in 0..n {
            c += *a.get(i).unwrap_or(&0) as u128 + *b.get(i).unwrap_or(&0) as u128;
            out[i] = c as u64;
            c >>= 64;
        }
        out
    }
    fn sub_mag(a: &[u64], b: &[u64]) -> Vec<u64> {
        // a ≥ b
        let mut out = vec![0; a.len()];
        let mut br = 0i128;
        for i in 0..a.len() {
            let v = a[i] as i128 - *b.get(i).unwrap_or(&0) as i128 - br;
            out[i] = v as u64;
            br = if v < 0 { 1 } else { 0 };
        }
        out
    }
    fn add(a: &Big, b: &Big) -> Big {
        if a.neg == b.neg {
            Big { mag: add_mag(&a.mag, &b.mag), neg: a.neg }
        } else if cmp_mag(&a.mag, &b.mag) != core::cmp::Ordering::Less {
            Big { mag: sub_mag(&a.mag, &b.mag), neg: a.neg }
        } else {
            Big { mag: sub_mag(&b.mag, &a.mag), neg: b.neg }
        }
    }
    fn negate(a: &Big) -> Big {
        Big { mag: a.mag.clone(), neg: !a.neg }
    }
    fn half(a: &Big) -> Big {
        // exact halving of an even value
        let n = a.mag.len();
        let mut out = vec![0; n];
        for i in 0..n {
            out[i] = (a.mag[i] >> 1) | (a.mag.get(i + 1).map_or(0, |x| x << 63));
        }
        Big { mag: out, neg: a.neg }
    }
    fn odd(a: &Big) -> bool {
        a.mag.first().map_or(false, |x| x & 1 == 1)
    }
    /// Divsteps from (1/2, f, g) until g = 0, or `None` past `cap`.
    pub fn count(f0: &Big, g0: &Big, cap: u32) -> Option<u32> {
        let (mut f, mut g) = (f0.clone(), g0.clone());
        let mut d2: i64 = 1;
        let mut n = 0;
        while !is_zero(&g) {
            if n >= cap {
                return None;
            }
            if d2 > 0 && odd(&g) {
                let ng = half(&add(&g, &negate(&f)));
                f = g;
                g = ng;
                d2 = 2 - d2;
            } else {
                let ng = if odd(&g) { half(&add(&g, &f)) } else { half(&g) };
                g = ng;
                d2 += 2;
            }
            n += 1;
        }
        Some(n)
    }
}
