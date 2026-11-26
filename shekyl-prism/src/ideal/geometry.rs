// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Arithmetic shared by ideal admission and the normalized Gram constructor.
use prism_int::{Int, SecretInt};

pub(crate) const GUARD_BITS: u32 = 8;

pub(crate) fn secret_norm_bits(prime_bits: u32) -> u32 {
    prime_bits.div_ceil(2) + 32
}

pub(crate) fn degree_bits(exponent: u32) -> u32 {
    2 * exponent - 5
}

pub(crate) fn hermitian<const D: usize>(
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

pub(crate) fn inverse_odd<const W: usize>(t: &SecretInt<W>) -> SecretInt<W> {
    // Newton in Z/(2^(64W)). The low word starts correct modulo 2.
    let max = SecretInt::from_public(&Int::<W>::from_le_limbs(&[u64::MAX; W], false).unwrap());
    let mut x = SecretInt::one();
    for _ in 0..(W * 64).next_power_of_two().trailing_zeros() {
        let diff = SecretInt::from_u64(2).sub(&t.mul_low(&x));
        let complement = max.sub(&diff.abs()).add(&SecretInt::one());
        x = x.mul_low(&SecretInt::select(&diff, &complement, &diff.is_negative()));
    }
    x
}
