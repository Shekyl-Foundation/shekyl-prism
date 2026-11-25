// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Litmus kernels: known compiler hazards for mask-select code, each
//! `#[no_mangle] #[inline(never)]`, fixed sizes so loops unroll. A
//! constant-time kernel should contain no conditional branch at all.
//! The mask implementations are copied from ../harness/src/ct.rs.

#![no_std]

/// Name of the compiled-in barrier candidate.
pub const BARRIER: &str = env!("CTB_BARRIER");

/// The in-crate inline-asm value barrier (candidate `asm`).
#[cfg(barrier = "asm")]
#[inline(always)]
pub fn barrier(mut x: u64) -> u64 {
    // SAFETY: an empty asm block that only names a register.
    unsafe {
        core::arch::asm!("/* {0} */", inout(reg) x, options(pure, nomem, nostack, preserves_flags));
    }
    x
}

#[cfg(barrier = "asm")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    barrier(0u64.wrapping_sub(bit & 1))
}

/// Negative control: no barrier.
#[cfg(barrier = "none")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    0u64.wrapping_sub(bit & 1)
}

/// Negative control: the first version, black_box on the bit.
#[cfg(barrier = "weak")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    0u64.wrapping_sub(core::hint::black_box(bit) & 1)
}

/// Safe: black_box on the finished mask.
#[cfg(barrier = "bbmask")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    core::hint::black_box(0u64.wrapping_sub(bit & 1))
}

/// Safe: plain mask here; black_box at each use (see `use_m`).
#[cfg(barrier = "bbuse")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    0u64.wrapping_sub(bit & 1)
}

/// Safe: the mask comes out of a function that is never inlined.
#[cfg(barrier = "noinline")]
#[inline(never)]
pub fn mask(bit: u64) -> u64 {
    0u64.wrapping_sub(bit & 1)
}

/// Unsafe in this crate: a volatile read of the mask.
#[cfg(barrier = "volatile")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    let m = 0u64.wrapping_sub(bit & 1);
    // SAFETY: reads a live, aligned local.
    unsafe { core::ptr::read_volatile(&m) }
}

/// subtle 2.6.1: Choice::from (its barrier) + ConditionallySelectable.
#[cfg(any(barrier = "subtle-vol", barrier = "subtle-bb"))]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    use subtle::{Choice, ConditionallySelectable};
    u64::conditional_select(&0, &u64::MAX, Choice::from((bit & 1) as u8))
}

/// RustCrypto cmov 0.5.4: an inline-asm cmovnz / csel inside the crate.
#[cfg(barrier = "cmov")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    use cmov::Cmov;
    let mut m = 0u64;
    m.cmovnz(&u64::MAX, (bit & 1) as u8);
    m
}

/// RustCrypto ctutils 0.4.8: Choice + CtSelect (on cmov).
#[cfg(barrier = "ctutils")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    use ctutils::{Choice, CtSelect};
    0u64.ct_select(&u64::MAX, Choice::from_u64_lsb(bit))
}

/// Safe, Rust >= 1.88: core::hint::select_unpredictable (a cmov/csel
/// hint, documented as not a constant-time guarantee).
#[cfg(barrier = "selunpred")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    core::hint::select_unpredictable(bit & 1 == 1, u64::MAX, 0)
}

/// The dedicated internal crate: one audited asm block per arch.
#[cfg(barrier = "crate")]
#[inline(always)]
pub fn mask(bit: u64) -> u64 {
    prism_ct_barrier::mask_from_bit(bit)
}

/// Identity, except black_box under the `bbuse` candidate.
#[inline(always)]
pub fn use_m(m: u64) -> u64 {
    #[cfg(barrier = "bbuse")]
    {
        core::hint::black_box(m)
    }
    #[cfg(not(barrier = "bbuse"))]
    {
        m
    }
}


#[inline(always)]
fn is_zero(x: u64) -> u64 {
    1 ^ ((x | x.wrapping_neg()) >> 63)
}

#[inline(always)]
fn select(m: u64, a: u64, b: u64) -> u64 {
    b ^ (use_m(m) & (a ^ b))
}

/// RUSTSEC-2024-0344 shape (curve25519-dalek Scalar52::sub): subtract,
/// then add the modulus back under the borrow mask.
#[no_mangle]
#[inline(never)]
pub fn k_dalek_sub(a: &[u64; 5], b: &[u64; 5], l: &[u64; 5]) -> [u64; 5] {
    let mask52 = (1u64 << 52) - 1;
    let mut d = [0u64; 5];
    let mut borrow = 0u64;
    for i in 0..5 {
        borrow = a[i].wrapping_sub(b[i].wrapping_add(borrow >> 63));
        d[i] = borrow & mask52;
    }
    let m = mask(borrow >> 63);
    let mut carry = 0u64;
    for i in 0..5 {
        carry = (carry >> 52) + d[i] + (l[i] & use_m(m));
        d[i] = carry & mask52;
    }
    d
}

/// The prototype's first-barrier failure: barrel shift of an array by a
/// secret limb count, every stage a mask select of two arrays.
#[no_mangle]
#[inline(never)]
pub fn k_barrel(a: &[u64; 16], k: u64) -> [u64; 16] {
    let mut x = *a;
    for j in 0..4 {
        let sh = 1usize << j;
        let mut t = [0u64; 16];
        for i in 0..16 {
            t[i] = if i + sh < 16 { x[i + sh] } else { 0 };
        }
        let m = mask((k >> j) & 1);
        for i in 0..16 {
            x[i] = select(m, t[i], x[i]);
        }
    }
    x
}

/// Conditional swap of two arrays.
#[no_mangle]
#[inline(never)]
pub fn k_cswap(bit: u64, a: &mut [u64; 16], b: &mut [u64; 16]) {
    let m = use_m(mask(bit));
    for i in 0..16 {
        let t = m & (a[i] ^ b[i]);
        a[i] ^= t;
        b[i] ^= t;
    }
}

/// RUSTSEC-2026-0003 shape (cmov on ARM32): byte move if c != 0.
#[no_mangle]
#[inline(never)]
pub fn k_cmov_u8(a: &mut u8, b: u8, c: u8) {
    let nz = 1 ^ is_zero(c as u64);
    let m = use_m(mask(nz)) as u8;
    *a ^= m & (*a ^ b);
}

/// Full-scan table lookup by a secret index.
#[no_mangle]
#[inline(never)]
pub fn k_table(t: &[[u64; 4]; 16], idx: u64) -> [u64; 4] {
    let mut r = [0u64; 4];
    for i in 0..16 {
        let m = use_m(mask(is_zero(i as u64 ^ idx)));
        for w in 0..4 {
            r[w] |= t[i][w] & m;
        }
    }
    r
}

/// Accumulate-if (borrow-chain compare then masked add), 8 limbs.
#[no_mangle]
#[inline(never)]
pub fn k_cond_sub(a: &mut [u64; 8], p: &[u64; 8]) {
    // a -= p if a >= p
    let mut t = [0u64; 8];
    let mut br = 0u64;
    for i in 0..8 {
        let d = (a[i] as u128).wrapping_sub(p[i] as u128).wrapping_sub(br as u128);
        t[i] = d as u64;
        br = ((d >> 64) as u64) & 1;
    }
    let m = mask(1 ^ br);
    for i in 0..8 {
        a[i] = select(m, t[i], a[i]);
    }
}
