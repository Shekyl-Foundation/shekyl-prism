// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Magnitudes of `N` limbs, least significant first.
//!
//! Every function here does the same work whatever its operands hold: the
//! loops are counted by `N` or by a public count, and no limb decides a
//! branch or an index. Both integer types are built on them, and the
//! secret type is built on nothing else.
//!
//! Carries are sums widened to `u128`. No `overflowing_add` follows a `+`
//! of the same operands, and nothing is clamped by hand: the two shapes of
//! ledger O32 this file could have had.

/// The bits of one limb.
pub(crate) const LIMB_BITS: u32 = 64;

/// `a + b` modulo `2^(64N)`, and the carry out, 0 or 1.
pub(crate) fn add<const N: usize>(a: &[u64; N], b: &[u64; N]) -> ([u64; N], u64) {
    let mut out = [0u64; N];
    let mut carry = 0u64;
    for i in 0..N {
        let sum = u128::from(a[i]) + u128::from(b[i]) + u128::from(carry);
        out[i] = sum as u64;
        carry = (sum >> LIMB_BITS) as u64;
    }
    (out, carry)
}

/// `a − b` modulo `2^(64N)`, and the borrow out: 1 when `a < b`, else 0.
pub(crate) fn sub<const N: usize>(a: &[u64; N], b: &[u64; N]) -> ([u64; N], u64) {
    let mut out = [0u64; N];
    let mut borrow = 0u64;
    for i in 0..N {
        let difference = u128::from(a[i])
            .wrapping_sub(u128::from(b[i]))
            .wrapping_sub(u128::from(borrow));
        out[i] = difference as u64;
        borrow = ((difference >> LIMB_BITS) as u64) & 1;
    }
    (out, borrow)
}

/// The low `N` limbs of `a · b`, and a limb that is non-zero exactly when
/// the product has a set bit above them.
///
/// All `N²` limb products are formed. One that lands at or above limb `N`
/// is not stored: its bits are gathered into the second result.
pub(crate) fn mul<const N: usize>(a: &[u64; N], b: &[u64; N]) -> ([u64; N], u64) {
    let mut out = [0u64; N];
    let mut lost = 0u64;
    for (i, &a_limb) in a.iter().enumerate() {
        let mut carry = 0u64;
        for (j, &b_limb) in b.iter().enumerate() {
            let product = u128::from(a_limb) * u128::from(b_limb);
            let place = i + j;
            if place < N {
                // At most (2^64 − 1)² + 2·(2^64 − 1) = 2^128 − 1.
                let sum = u128::from(out[place]) + product + u128::from(carry);
                out[place] = sum as u64;
                carry = (sum >> LIMB_BITS) as u64;
            } else {
                lost |= (product as u64) | ((product >> LIMB_BITS) as u64);
            }
        }
        // The carry out of limb N − 1, which is where row i's stored part ends.
        lost |= carry;
    }
    (out, lost)
}

/// `a · 2^count` modulo `2^(64N)`, and a limb that is non-zero exactly
/// when a set bit left at the top. `count` is public.
pub(crate) fn shl<const N: usize>(a: &[u64; N], count: u32) -> ([u64; N], u64) {
    let whole = (count / LIMB_BITS) as usize;
    let part = count % LIMB_BITS;
    let mut out = [0u64; N];
    let mut lost = 0u64;
    for (i, &limb) in a.iter().enumerate() {
        let wide = u128::from(limb) << part;
        // `whole` is at most 2^26 and `i` is below `N`: the places fit.
        let place = i + whole;
        for (place, piece) in [
            (place, wide as u64),
            (place + 1, (wide >> LIMB_BITS) as u64),
        ] {
            if place < N {
                out[place] |= piece;
            } else {
                lost |= piece;
            }
        }
    }
    (out, lost)
}

/// `⌊a / 2^count⌋`. `count` is public.
pub(crate) fn shr<const N: usize>(a: &[u64; N], count: u32) -> [u64; N] {
    let whole = (count / LIMB_BITS) as usize;
    let part = count % LIMB_BITS;
    let mut out = [0u64; N];
    for (i, &limb) in a.iter().enumerate() {
        // The limb with 64 zero bits under it, moved down by `part`: the
        // high half stays in the limb's own place, the low half is what
        // crossed into the place below.
        let wide = (u128::from(limb) << LIMB_BITS) >> part;
        if i >= whole {
            out[i - whole] |= (wide >> LIMB_BITS) as u64;
        }
        if i > whole {
            out[i - whole - 1] |= wide as u64;
        }
    }
    out
}

/// The limbs OR-ed together: zero exactly when the magnitude is zero.
pub(crate) fn fold<const N: usize>(a: &[u64; N]) -> u64 {
    let mut all = 0u64;
    for &limb in a {
        all |= limb;
    }
    all
}

/// The limbs of `a XOR b` OR-ed together: zero exactly when they are equal.
pub(crate) fn difference<const N: usize>(a: &[u64; N], b: &[u64; N]) -> u64 {
    let mut all = 0u64;
    for i in 0..N {
        all |= a[i] ^ b[i];
    }
    all
}
