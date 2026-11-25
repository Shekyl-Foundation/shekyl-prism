// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Fixed-width two's-complement integers as `[u64; L]` (little-endian
//! limbs), with constant-time primitives.
//!
//! Rules kept throughout: every loop bound is a const generic or a public
//! parameter; no `if`/`match`/`?`/early return on a value derived from
//! secret data; selection by masks; comparisons from borrow chains;
//! divisions by a fixed-iteration restoring loop, never a `div`
//! instruction. Carries are `u128` sums (no `overflowing_add`, so the
//! 1.97.1 carry-folding miscompile, #163779 on the O32 watch list, has no
//! shape here) and there is no hand-written clamp.

/// A value barrier: an empty inline-asm block the optimiser cannot see
/// through, so it cannot learn that a mask is 0 or all-ones and turn a
/// mask select back into a branch. (A first version used
/// `core::hint::black_box` on the choice bit only; LLVM then knew the mask
/// was 0 or -1 and compiled the barrel shifter's array select into a
/// conditional jump, found by asm_scan.py.)
#[inline(always)]
pub fn barrier(mut x: u64) -> u64 {
    // SAFETY: an empty asm block that only names a register.
    unsafe {
        core::arch::asm!("/* {0} */", inout(reg) x, options(pure, nomem, nostack, preserves_flags));
    }
    x
}

/// `bit` in {0,1} -> 0 or all-ones, hidden behind the barrier.
#[cfg(not(feature = "weak-barrier"))]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    barrier(0u64.wrapping_sub(bit & 1))
}

/// Negative control only (feature `weak-barrier`): the first version,
/// `black_box` on the bit, which LLVM sees through. Not for use.
#[cfg(feature = "weak-barrier")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    0u64.wrapping_sub(core::hint::black_box(bit) & 1)
}

/// 1 if `x == 0`, else 0, without a comparison instruction.
#[inline(always)]
pub fn is_zero(x: u64) -> u64 {
    1 ^ ((x | x.wrapping_neg()) >> 63)
}

#[inline(always)]
pub fn select(m: u64, a: u64, b: u64) -> u64 {
    b ^ (m & (a ^ b))
}

#[inline(always)]
pub fn select_arr<const L: usize>(m: u64, a: &[u64; L], b: &[u64; L]) -> [u64; L] {
    let mut r = [0u64; L];
    for i in 0..L {
        r[i] = select(m, a[i], b[i]);
    }
    r
}

#[inline(always)]
pub fn cswap<const L: usize>(m: u64, a: &mut [u64; L], b: &mut [u64; L]) {
    for i in 0..L {
        let t = m & (a[i] ^ b[i]);
        a[i] ^= t;
        b[i] ^= t;
    }
}

/// Sign mask of a two's-complement value: all-ones if negative.
#[inline(always)]
pub fn sign_mask<const L: usize>(a: &[u64; L]) -> u64 {
    ((a[L - 1] as i64) >> 63) as u64
}

#[inline(always)]
pub fn add_assign<const L: usize>(a: &mut [u64; L], b: &[u64; L]) {
    let mut c: u128 = 0;
    for i in 0..L {
        let t = a[i] as u128 + b[i] as u128 + c;
        a[i] = t as u64;
        c = t >> 64;
    }
}

/// a -= b, returns the final borrow (1 if a < b as unsigned).
#[inline(always)]
pub fn sub_assign<const L: usize>(a: &mut [u64; L], b: &[u64; L]) -> u64 {
    let mut br: u64 = 0;
    for i in 0..L {
        let t = (a[i] as u128).wrapping_sub(b[i] as u128).wrapping_sub(br as u128);
        a[i] = t as u64;
        br = ((t >> 64) as u64) & 1;
    }
    br
}

/// 1 if a < b as unsigned (borrow of a - b), else 0.
#[inline(always)]
pub fn lt_unsigned<const L: usize>(a: &[u64; L], b: &[u64; L]) -> u64 {
    let mut t = *a;
    sub_assign(&mut t, b)
}

/// Conditional negation: m = 0 keeps a, m = all-ones gives -a.
#[inline(always)]
pub fn cond_neg<const L: usize>(a: &mut [u64; L], m: u64) {
    let mut c: u128 = (m & 1) as u128;
    for i in 0..L {
        let t = (a[i] ^ m) as u128 + c;
        a[i] = t as u64;
        c = t >> 64;
    }
}

/// Signed C x C -> N (= 2C) limb product.
#[inline(always)]
pub fn mul_wide<const C: usize, const N: usize>(a: &[u64; C], b: &[u64; C]) -> [u64; N] {
    debug_assert!(N == 2 * C);
    let mut r = [0u64; N];
    for i in 0..C {
        let mut c: u128 = 0;
        let ai = a[i] as u128;
        for j in 0..C {
            let t = r[i + j] as u128 + ai * b[j] as u128 + c;
            r[i + j] = t as u64;
            c = t >> 64;
        }
        r[i + C] = c as u64;
    }
    // two's-complement correction: subtract 2^(64C) * ([a<0] b + [b<0] a)
    let ma = sign_mask(a);
    let mb = sign_mask(b);
    let mut br: u64 = 0;
    for j in 0..C {
        let t = (r[C + j] as u128)
            .wrapping_sub((b[j] & ma) as u128)
            .wrapping_sub(br as u128);
        r[C + j] = t as u64;
        br = ((t >> 64) as u64) & 1;
    }
    br = 0;
    for j in 0..C {
        let t = (r[C + j] as u128)
            .wrapping_sub((a[j] & mb) as u128)
            .wrapping_sub(br as u128);
        r[C + j] = t as u64;
        br = ((t >> 64) as u64) & 1;
    }
    r
}

/// a * p mod 2^(64N), p a public positive constant of PL limbs.
#[inline(always)]
pub fn mul_trunc<const N: usize, const PL: usize>(a: &[u64; N], p: &[u64; PL]) -> [u64; N] {
    let mut r = [0u64; N];
    for i in 0..N {
        let mut c: u128 = 0;
        let ai = a[i] as u128;
        let top = if N - i < PL { N - i } else { PL }; // public bound
        for j in 0..top {
            let t = r[i + j] as u128 + ai * p[j] as u128 + c;
            r[i + j] = t as u64;
            c = t >> 64;
        }
        if i + PL < N {
            r[i + PL] = c as u64; // public index
        }
    }
    r
}

/// a * m mod 2^(64C) for a signed scalar m (|m| < 2^63).
#[inline(always)]
pub fn mul_scalar<const C: usize>(a: &[u64; C], m: i64) -> [u64; C] {
    let s = ((m >> 63) as u64) & 0xffff_ffff_ffff_ffff;
    let am = ((m as u64) ^ s).wrapping_sub(s) as u128;
    let mut r = [0u64; C];
    let mut c: u128 = 0;
    for i in 0..C {
        let t = a[i] as u128 * am + c;
        r[i] = t as u64;
        c = t >> 64;
    }
    cond_neg(&mut r, s);
    r
}

/// Branch-free count of leading zeros of a 64-bit word (64 for 0).
#[inline(always)]
pub fn lzcnt(mut w: u64) -> u64 {
    let zero = is_zero(w);
    let mut n = 0u64;
    for s in [32u32, 16, 8, 4, 2, 1] {
        let z = is_zero(w >> (64 - s));
        n += z * s as u64;
        w = select(mask(z), w << s, w);
    }
    // w == 0 gives n = 63 here; fix to 64
    n + zero
}

/// Bit length of an unsigned N-limb value, constant time.
#[inline(always)]
pub fn bit_length<const N: usize>(a: &[u64; N]) -> u64 {
    let mut bl = 0u64;
    for i in 0..N {
        let nz = 1 ^ is_zero(a[i]);
        let cand = (i as u64) * 64 + 64 - lzcnt(a[i]);
        bl = select(mask(nz), cand, bl);
    }
    bl
}

/// 2^bits for a secret bits in 0..64. Shift counts are public at all six stages.
#[inline(always)]
pub fn bit_scale(bits: u64) -> u64 {
    let mut factor = 1u64;
    for j in 0..6 {
        factor = select(mask((bits >> j) & 1), factor << (1 << j), factor);
    }
    factor
}

/// a >> k for a secret k in [0, 64N): arithmetic (sign fill) if `arith`
/// (public), else logical. Barrel shifter: every stage always executes.
#[inline(always)]
pub fn shr<const N: usize>(a: &[u64; N], k: u64, arith: bool) -> [u64; N] {
    let fill = if arith { sign_mask(a) } else { 0 };
    let ls = k >> 6;
    let scale = u128::from(bit_scale(63 - (k & 63)));
    let mut x = *a;
    let mut j = 0u32;
    while (1usize << j) < N {
        let sh = 1usize << j;
        let mut t = [0u64; N];
        for i in 0..N {
            t[i] = if i + sh < N { x[i + sh] } else { fill }; // public index
        }
        x = select_arr(mask((ls >> j) & 1), &t, &x);
        j += 1;
    }
    let mut r = [0u64; N];
    let mut carry = ((u128::from(fill) * scale) << 1) as u64;
    for i in (0..N).rev() {
        // 2*scale = 2^(64-bs), including 2^64 when bs = 0.
        let wide = (u128::from(x[i]) * scale) << 1;
        r[i] = (wide >> 64) as u64 | carry;
        carry = wide as u64;
    }
    r
}

/// floor(a / d) for 64 quotient bits: requires (a >> 64) < d, else the
/// caller saturates. Fixed 64 iterations, no `div`.
#[inline(always)]
pub fn div_u128_64q(a: u128, d: u128) -> u64 {
    let mut r: u128 = a >> 64;
    let mut q: u64 = 0;
    let mut i = 64;
    while i > 0 {
        i -= 1;
        r = (r << 1) | ((a >> i) & 1);
        let t = r.wrapping_sub(d);
        let ge = 1 ^ (((t >> 127) as u64) & 1);
        let m = mask(ge) as u128;
        let m = m | (m << 64);
        r = (t & m) | (r & !m);
        q |= ge << i;
    }
    q
}

/// Exact floor((2 h + nu) / (2 nu)), returning a full-width two's-
/// complement quotient. Fixed N*64-bit restoring division. Used only for
/// the one final cleanup step, unconditionally, never an early exit.
#[inline(never)]
pub fn exact_round<const N: usize>(h: &[u64; N], nu: &[u64; N]) -> [u64; N] {
    // (2h + nu)/(2nu) = floor((h + floor(nu/2)) / nu), except that nu odd
    // rounds ties the same way too: floor(h/nu + 1/2), integer h.
    let mut a = *h;
    let mut half = [0u64; N];
    for i in 0..N {
        let hi = if i + 1 < N { nu[i + 1] } else { 0 };
        half[i] = (nu[i] >> 1) | (hi << 63);
    }
    add_assign(&mut a, &half);
    let sg = sign_mask(&a);
    cond_neg(&mut a, sg);
    // floor negative division: |a| += nu - 1 then negate the quotient
    let mut add = *nu;
    let mut one = [0u64; N]; one[0] = 1;
    sub_assign(&mut add, &one);
    for i in 0..N { add[i] &= sg; }
    add_assign(&mut a, &add);
    let mut r = [0u64; N];
    let mut q = [0u64; N];
    for pos in (0..N*64).rev() {
        let bit = (a[pos / 64] >> (pos % 64)) & 1;
        let mut c = bit;
        for i in 0..N {
            let nc = r[i] >> 63;
            r[i] = (r[i] << 1) | c;
            c = nc;
        }
        let mut sub = r;
        let borrow = sub_assign(&mut sub, nu);
        let ge = 1 ^ borrow;
        r = select_arr(mask(ge), &sub, &r);
        q[pos / 64] |= ge << (pos % 64);
    }
    cond_neg(&mut q, sg);
    q
}

/// Low C limbs of the signed product of a C-limb coordinate by an N-limb
/// quotient, modulo 2^(64C). The final cleanup's full-width quotient is
/// used, so even a very large coefficient is not silently truncated.
#[inline(always)]
pub fn mul_low<const C: usize, const N: usize>(a: &[u64; C], b: &[u64; N]) -> [u64; C] {
    let mut r = [0u64; C];
    for i in 0..C {
        let mut c = 0u128;
        for j in 0..C-i {
            let t = r[i+j] as u128 + a[i] as u128 * b[j] as u128 + c;
            r[i+j] = t as u64;
            c = t >> 64;
        }
    }
    r
}
