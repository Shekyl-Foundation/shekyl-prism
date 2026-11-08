// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Baillie–PSW on a public little-endian integer.
//!
//! Products and exponentiations are variable-time Montgomery
//! multiplications on 64-bit limbs. The challenge degree is public, so
//! this code does not aim at a secret-independent running time.
//! shekyl-core pins `crypto-bigint` 0.5.5 for fixed-width constant-time
//! integers. A public modulus whose width changes with the NIST level
//! stays on these limbs instead.

/// Sixteen little-endian limbs. Every integer this module builds fits:
/// a challenge degree is below `2^{999}`.
const LIMBS: usize = 16;

#[derive(Clone, Copy)]
struct Nat {
    l: [u64; LIMBS],
}

impl Nat {
    fn zero() -> Self {
        Self { l: [0; LIMBS] }
    }

    fn one() -> Self {
        let mut out = Self::zero();
        out.l[0] = 1;
        out
    }

    fn from_u64(v: u64) -> Self {
        let mut out = Self::zero();
        out.l[0] = v;
        out
    }

    fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > LIMBS * 8 {
            return None;
        }
        let mut out = Self::zero();
        for (i, byte) in bytes.iter().enumerate() {
            out.l[i / 8] |= u64::from(*byte) << ((i % 8) * 8);
        }
        Some(out)
    }

    fn is_zero(self) -> bool {
        self.l.iter().all(|limb| *limb == 0)
    }

    fn is_one(self) -> bool {
        self.l[0] == 1 && self.l[1..].iter().all(|limb| *limb == 0)
    }

    fn is_even(self) -> bool {
        self.l[0] & 1 == 0
    }

    fn bit(self, index: usize) -> bool {
        let limb = index / 64;
        if limb >= LIMBS {
            return false;
        }
        (self.l[limb] >> (index % 64)) & 1 == 1
    }

    fn bit_len(self) -> usize {
        for i in (0..LIMBS).rev() {
            if self.l[i] != 0 {
                return i * 64 + (64 - self.l[i].leading_zeros() as usize);
            }
        }
        0
    }
}

fn cmp(left: &Nat, right: &Nat) -> core::cmp::Ordering {
    for i in (0..LIMBS).rev() {
        if left.l[i] != right.l[i] {
            return left.l[i].cmp(&right.l[i]);
        }
    }
    core::cmp::Ordering::Equal
}

fn add_raw(left: &Nat, right: &Nat) -> (Nat, u64) {
    let mut out = Nat::zero();
    let mut carry = 0u128;
    for i in 0..LIMBS {
        carry += u128::from(left.l[i]) + u128::from(right.l[i]);
        out.l[i] = carry as u64;
        carry >>= 64;
    }
    (out, carry as u64)
}

fn sub_raw(left: &Nat, right: &Nat) -> Nat {
    let mut out = Nat::zero();
    let mut borrow = 0i128;
    for i in 0..LIMBS {
        borrow += i128::from(left.l[i]) - i128::from(right.l[i]);
        out.l[i] = borrow as u64;
        borrow >>= 64;
    }
    out
}

fn add_mod(left: &Nat, right: &Nat, n: &Nat) -> Nat {
    let (sum, carry) = add_raw(left, right);
    if carry == 1 || cmp(&sum, n) != core::cmp::Ordering::Less {
        // `left` and `right` are both reduced, so the sum is below `2n`
        // and one subtraction brings it back. A carry means the sum wrapped
        // through `2^{1024}`; `n` then still exceeds the wrapped residue.
        let (wrapped, _) = add_raw(&sum, &twos_complement(n));
        wrapped
    } else {
        sum
    }
}

fn twos_complement(n: &Nat) -> Nat {
    let mut out = Nat::zero();
    let mut carry = 1u64;
    for i in 0..LIMBS {
        let (sum, c) = (!n.l[i]).overflowing_add(carry);
        out.l[i] = sum;
        carry = u64::from(c);
    }
    out
}

fn sub_mod(left: &Nat, right: &Nat, n: &Nat) -> Nat {
    if cmp(left, right) == core::cmp::Ordering::Less {
        add_mod(left, &sub_raw(n, right), n)
    } else {
        sub_raw(left, right)
    }
}

fn shr1(value: &Nat) -> Nat {
    let mut out = Nat::zero();
    for i in 0..LIMBS {
        out.l[i] = value.l[i] >> 1;
        if i + 1 < LIMBS {
            out.l[i] |= value.l[i + 1] << 63;
        }
    }
    out
}

fn half_mod(value: &Nat, n: &Nat) -> Nat {
    if value.is_even() {
        shr1(value)
    } else {
        let (sum, carry) = add_raw(value, n);
        let mut out = shr1(&sum);
        if carry == 1 {
            out.l[LIMBS - 1] |= 1u64 << 63;
        }
        out
    }
}

/// `-n[0]^{-1} (mod 2^{64})` for an odd modulus.
fn neg_inv_limb(n0: u64) -> u64 {
    let mut inv = 1u64;
    for _ in 0..6 {
        inv = inv.wrapping_mul(2u64.wrapping_sub(n0.wrapping_mul(inv)));
    }
    inv.wrapping_neg()
}

struct Mod {
    n: Nat,
    ninv: u64,
    r2: Nat,
}

impl Mod {
    fn new(n: Nat) -> Self {
        let ninv = neg_inv_limb(n.l[0]);
        let mut r2 = Nat::one();
        for _ in 0..(2 * 64 * LIMBS) {
            r2 = add_mod(&r2, &r2, &n);
        }
        Self { n, ninv, r2 }
    }

    fn mont_mul(&self, left: &Nat, right: &Nat) -> Nat {
        let mut t = [0u64; LIMBS + 2];
        for i in 0..LIMBS {
            let mut carry = 0u128;
            for (j, slot) in t.iter_mut().enumerate().take(LIMBS) {
                carry += u128::from(*slot) + u128::from(left.l[j]) * u128::from(right.l[i]);
                *slot = carry as u64;
                carry >>= 64;
            }
            carry += u128::from(t[LIMBS]);
            t[LIMBS] = carry as u64;
            t[LIMBS + 1] = (carry >> 64) as u64;

            let m = t[0].wrapping_mul(self.ninv);
            carry = 0;
            for (j, slot) in t.iter_mut().enumerate().take(LIMBS) {
                carry += u128::from(*slot) + u128::from(m) * u128::from(self.n.l[j]);
                *slot = carry as u64;
                carry >>= 64;
            }
            carry += u128::from(t[LIMBS]);
            t[LIMBS] = carry as u64;
            t[LIMBS + 1] += (carry >> 64) as u64;

            for j in 0..LIMBS + 1 {
                t[j] = t[j + 1];
            }
            t[LIMBS + 1] = 0;
        }
        let mut out = Nat::zero();
        out.l.copy_from_slice(&t[..LIMBS]);
        let extra = t[LIMBS];
        if extra != 0 || cmp(&out, &self.n) != core::cmp::Ordering::Less {
            if extra != 0 {
                let (wrapped, _) = add_raw(&out, &twos_complement(&self.n));
                wrapped
            } else {
                sub_raw(&out, &self.n)
            }
        } else {
            out
        }
    }

    fn to_mont(&self, value: &Nat) -> Nat {
        self.mont_mul(value, &self.r2)
    }

    fn reduce(&self, value: &Nat) -> Nat {
        self.mont_mul(value, &Nat::one())
    }

    fn mul(&self, left: &Nat, right: &Nat) -> Nat {
        self.reduce(&self.mont_mul(&self.to_mont(left), &self.to_mont(right)))
    }

    fn mul_i32(&self, value: &Nat, factor: i32) -> Nat {
        self.mul(value, &mod_i32(factor, &self.n))
    }

    fn pow(&self, base: &Nat, exp: &Nat) -> Nat {
        let mut result = self.to_mont(&Nat::one());
        let base = self.to_mont(base);
        for i in (0..exp.bit_len()).rev() {
            result = self.mont_mul(&result, &result);
            if exp.bit(i) {
                result = self.mont_mul(&result, &base);
            }
        }
        self.reduce(&result)
    }
}

fn mod_i32(value: i32, n: &Nat) -> Nat {
    let magnitude = Nat::from_u64(value.unsigned_abs().into());
    let reduced = rem(&magnitude, n);
    if value >= 0 || reduced.is_zero() {
        reduced
    } else {
        sub_raw(n, &reduced)
    }
}

fn rem(value: &Nat, n: &Nat) -> Nat {
    if cmp(value, n) == core::cmp::Ordering::Less {
        return *value;
    }
    let mut rest = Nat::zero();
    for i in (0..value.bit_len()).rev() {
        rest = add_mod(&rest, &rest, n);
        if value.bit(i) {
            rest = add_mod(&rest, &Nat::one(), n);
        }
    }
    rest
}

fn is_square(n: &Nat) -> bool {
    if n.is_zero() {
        return true;
    }
    let mut rest = *n;
    let mut root = Nat::zero();
    let mut bit = Nat::zero();
    let len = n.bit_len();
    let power = len - 1 - (len - 1) % 2;
    bit.l[power / 64] = 1u64 << (power % 64);
    while !bit.is_zero() {
        let (trial, carry) = add_raw(&root, &bit);
        if carry == 0 && cmp(&rest, &trial) != core::cmp::Ordering::Less {
            rest = sub_raw(&rest, &trial);
            root = add_raw(&shr1(&root), &bit).0;
        } else {
            root = shr1(&root);
        }
        bit = shr1(&shr1(&bit));
    }
    rest.is_zero()
}

fn rem_u32(n: &Nat, modulus: u32) -> u32 {
    let m = u128::from(modulus);
    let mut acc = 0u128;
    for limb in n.l.iter().rev() {
        acc = ((acc << 64) | u128::from(*limb)) % m;
    }
    acc as u32
}

fn jacobi(a: i32, n: &Nat) -> i32 {
    if a == 0 {
        return 0;
    }
    if a < 0 {
        let sign = if rem_u32(n, 4) == 3 { -1 } else { 1 };
        return sign * jacobi(-a, n);
    }
    let mut acc = Nat::from_u64(a as u64);
    let mut modulus = *n;
    let mut sign = 1i32;
    loop {
        acc = rem(&acc, &modulus);
        if acc.is_zero() {
            return 0;
        }
        while acc.is_even() {
            acc = shr1(&acc);
            let mod8 = rem_u32(&modulus, 8);
            if mod8 == 3 || mod8 == 5 {
                sign = -sign;
            }
        }
        if acc.is_one() {
            return sign;
        }
        if rem_u32(&acc, 4) == 3 && rem_u32(&modulus, 4) == 3 {
            sign = -sign;
        }
        core::mem::swap(&mut acc, &mut modulus);
    }
}

const SMALL: &[u32] = &[
    3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
];

/// `true` when `n` passes Baillie–PSW.
pub fn is_baillie_psw(bytes: &[u8]) -> bool {
    let Some(n) = Nat::from_le_bytes(bytes) else {
        return false;
    };
    if cmp(&n, &Nat::from_u64(2)) == core::cmp::Ordering::Less {
        return false;
    }
    if n.is_one() {
        return false;
    }
    if n.l[0] == 2 || n.l[0] == 3 && n.bit_len() <= 2 {
        return true;
    }
    if n.is_even() {
        return false;
    }
    for &prime in SMALL {
        if n.bit_len() <= 8 && n.l[0] == u64::from(prime) {
            return true;
        }
        if rem_u32(&n, prime) == 0 {
            return false;
        }
    }
    if is_square(&n) {
        return false;
    }
    miller_rabin(&n) && strong_lucas(&n)
}

#[cfg(test)]
pub(crate) fn miller_rabin_base2(bytes: &[u8]) -> bool {
    Nat::from_le_bytes(bytes).is_some_and(|n| miller_rabin(&n))
}

fn miller_rabin(n: &Nat) -> bool {
    let n1 = sub_raw(n, &Nat::one());
    let mut d = n1;
    let mut s = 0u32;
    while d.is_even() {
        d = shr1(&d);
        s += 1;
    }
    let ctx = Mod::new(*n);
    let mut x = ctx.pow(&Nat::from_u64(2), &d);
    if x.is_one() || cmp(&x, &n1) == core::cmp::Ordering::Equal {
        return true;
    }
    for _ in 1..s {
        x = ctx.mul(&x, &x);
        if cmp(&x, &n1) == core::cmp::Ordering::Equal {
            return true;
        }
    }
    false
}

#[cfg(test)]
pub(crate) fn strong_lucas_selfridge(bytes: &[u8]) -> bool {
    Nat::from_le_bytes(bytes).is_some_and(|n| strong_lucas(&n))
}

fn strong_lucas(n: &Nat) -> bool {
    let Some(discriminant) = selfridge(n) else {
        return false;
    };
    let q = (1 - discriminant) / 4;
    let n1 = add_raw(n, &Nat::one()).0;
    let mut d = n1;
    let mut s = 0u32;
    while d.is_even() {
        d = shr1(&d);
        s += 1;
    }
    let ctx = Mod::new(*n);
    let (u, mut v, mut qk) = lucas_chain(&ctx, &d, discriminant, q);
    if u.is_zero() || v.is_zero() {
        return true;
    }
    for _ in 1..s {
        let v2 = ctx.mul(&v, &v);
        let two_q = add_mod(&qk, &qk, n);
        v = sub_mod(&v2, &two_q, n);
        qk = ctx.mul(&qk, &qk);
        if v.is_zero() {
            return true;
        }
    }
    false
}

fn selfridge(n: &Nat) -> Option<i32> {
    let mut d = 5i32;
    for _ in 0..256 {
        match jacobi(d, n) {
            -1 => return Some(d),
            0 => return None,
            _ => d = if d > 0 { -(d + 2) } else { -d + 2 },
        }
    }
    None
}

/// `U_k`, `V_k` and `Q^k` for `P = 1`.
fn lucas_chain(ctx: &Mod, k: &Nat, discriminant: i32, q: i32) -> (Nat, Nat, Nat) {
    let mut u = Nat::one();
    let mut v = Nat::one();
    let mut qk = mod_i32(q, &ctx.n);
    let bits = k.bit_len();
    if bits <= 1 {
        return (u, v, qk);
    }
    for i in (0..bits - 1).rev() {
        let u2 = ctx.mul(&u, &v);
        let v2 = sub_mod(&ctx.mul(&v, &v), &add_mod(&qk, &qk, &ctx.n), &ctx.n);
        let q2 = ctx.mul(&qk, &qk);
        u = u2;
        v = v2;
        qk = q2;
        if k.bit(i) {
            let u_next = half_mod(&add_mod(&u, &v, &ctx.n), &ctx.n);
            let d_u = ctx.mul_i32(&u, discriminant);
            let v_next = half_mod(&add_mod(&d_u, &v, &ctx.n), &ctx.n);
            u = u_next;
            v = v_next;
            qk = ctx.mul_i32(&qk, q);
        }
    }
    (u, v, qk)
}

/// `q = 2^{2e−1} − s^2` passes Baillie–PSW.
pub fn degree_is_prime(e: u32, s: &[u8]) -> bool {
    let Some(s) = Nat::from_le_bytes(s) else {
        return false;
    };
    let bit = (u64::from(e) * 2).saturating_sub(1) as usize;
    if bit >= LIMBS * 64 {
        return false;
    }
    let mut pow = Nat::zero();
    pow.l[bit / 64] = 1u64 << (bit % 64);
    let square = square_nat(&s);
    if cmp(&square, &pow) != core::cmp::Ordering::Less {
        return false;
    }
    let q = sub_raw(&pow, &square);
    // Re-encode through the same entry as every other test.
    let mut bytes = [0u8; LIMBS * 8];
    for (i, limb) in q.l.iter().enumerate() {
        bytes[i * 8..i * 8 + 8].copy_from_slice(&limb.to_le_bytes());
    }
    let len = q.bit_len().div_ceil(8).max(1);
    is_baillie_psw(&bytes[..len])
}

fn square_nat(value: &Nat) -> Nat {
    let mut wide = [0u64; LIMBS * 2];
    for i in 0..LIMBS {
        let mut carry = 0u128;
        for j in 0..LIMBS {
            carry += u128::from(wide[i + j]) + u128::from(value.l[i]) * u128::from(value.l[j]);
            wide[i + j] = carry as u64;
            carry >>= 64;
        }
        wide[i + LIMBS] = carry as u64;
    }
    let mut out = Nat::zero();
    if wide[LIMBS..].iter().any(|limb| *limb != 0) {
        // `s` is below `2^{e−1}` and `e ≤ 500`, so the square fits.
        out.l[LIMBS - 1] = u64::MAX;
        return out;
    }
    out.l.copy_from_slice(&wide[..LIMBS]);
    out
}

#[cfg(test)]
mod self_test {
    use super::{degree_is_prime, is_baillie_psw, miller_rabin_base2, strong_lucas_selfridge};

    #[test]
    fn small_values() {
        assert!(is_baillie_psw(&[2]));
        assert!(is_baillie_psw(&[251, 3]), "1019");
        assert!(!is_baillie_psw(&[1]));
        assert!(!is_baillie_psw(&[9]), "square");
        assert!(!is_baillie_psw(&[91]));
    }

    #[test]
    fn base2_pseudoprimes_fail_the_lucas_half() {
        for n in [2047u32, 3277, 4033, 4681, 8321] {
            let bytes = n.to_le_bytes();
            let bytes = &bytes[..n.to_le_bytes().iter().rposition(|b| *b != 0).unwrap() + 1];
            assert!(miller_rabin_base2(bytes), "{n}");
            assert!(!is_baillie_psw(bytes), "{n}");
        }
    }

    #[test]
    fn strong_lucas_pseudoprimes_fail_the_miller_half() {
        for n in [5459u32, 5777, 10877, 16109, 18971] {
            let bytes = n.to_le_bytes();
            let len = bytes.iter().rposition(|b| *b != 0).unwrap() + 1;
            assert!(strong_lucas_selfridge(&bytes[..len]), "{n}");
            assert!(!is_baillie_psw(&bytes[..len]), "{n}");
        }
        // The standard Lucas test accepts 323. The strong test does not.
        assert!(!strong_lucas_selfridge(&323u16.to_le_bytes()));
    }

    #[test]
    fn carmichael_561_is_refused() {
        assert!(!is_baillie_psw(&561u16.to_le_bytes()));
    }

    #[test]
    fn a_known_prime_degree_passes() {
        // q = 2^{17} − 3^2 = 131063, which is prime. e = 9 gives the exponent
        // 2e−1 = 17, and s = 3 is the odd integer below 2^{8}.
        assert!(degree_is_prime(9, &[3]));
        // 2^{17} − 2^2 is even.
        assert!(!degree_is_prime(9, &[2]));
    }
}
