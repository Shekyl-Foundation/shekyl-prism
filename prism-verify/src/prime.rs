// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Baillie–PSW on a public little-endian integer.
//!
//! Trial division through 97, one strong Miller–Rabin test at base 2, then
//! the Lucas test with Selfridge's discriminant. No random bases. The
//! challenge prime is public, so this arithmetic is not constant-time.

/// `true` when `n` passes Baillie–PSW.
pub fn is_baillie_psw(n: &[u8]) -> bool {
    let n = trim(n);
    if cmp(&n, &[2]) == core::cmp::Ordering::Less {
        return false;
    }
    if eq(&n, &[2]) || eq(&n, &[3]) {
        return true;
    }
    if n[0] & 1 == 0 {
        return false;
    }
    for &p in SMALL {
        if eq(&n, &[p]) {
            return true;
        }
        if rem(&n, p) == 0 {
            return false;
        }
    }
    miller_rabin_base2(&n) && lucas(&n)
}

const SMALL: &[u8] = &[
    3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
];

pub(crate) fn miller_rabin_base2(n: &[u8]) -> bool {
    let n1 = sub(n, &[1]).expect("n > 2");
    let mut d = n1.clone();
    let mut s = 0u32;
    while d[0] & 1 == 0 {
        shr1(&mut d);
        s += 1;
    }
    let mut x = pow_mod(&[2], &d, n);
    if eq(&x, &[1]) || eq(&x, &n1) {
        return true;
    }
    for _ in 1..s {
        x = mul_mod(&x, &x, n);
        if eq(&x, &n1) {
            return true;
        }
    }
    false
}

pub(crate) fn lucas(n: &[u8]) -> bool {
    let Some(discriminant) = selfridge(n) else {
        return false;
    };
    lucas_u(n, discriminant) == [0]
}

pub(crate) fn selfridge(n: &[u8]) -> Option<i32> {
    let mut d = 5i32;
    for _ in 0..4096 {
        match jacobi(d, n) {
            -1 => return Some(d),
            0 => return None,
            _ => d = if d > 0 { -(d + 2) } else { -d + 2 },
        }
    }
    None
}

/// `U_{n+1} (mod n)` for `P = 1` and `Q = (1-D)/4`, by the binary chain.
pub(crate) fn lucas_u(n: &[u8], d: i32) -> Vec<u8> {
    let q = (1 - d) / 4;
    let k = add(n, &[1]);
    let mut u = vec![1u8];
    let mut v = vec![1u8];
    let mut qk = mod_i32(q, n);
    let bits = bit_len(&k);
    if bits <= 1 {
        return u;
    }
    for i in (0..bits - 1).rev() {
        let u2 = mul_mod(&u, &v, n);
        let v2 = sub_mod(&mul_mod(&v, &v, n), &add_mod(&qk, &qk, n), n);
        let q2 = mul_mod(&qk, &qk, n);
        u = u2;
        v = v2;
        qk = q2;
        if bit(&k, i) {
            let u_next = half(&add_mod(&u, &v, n), n);
            let d_u = mul_i32(&u, d, n);
            let v_next = half(&add_mod(&d_u, &v, n), n);
            u = u_next;
            v = v_next;
            qk = mul_i32(&qk, q, n);
        }
    }
    u
}

fn jacobi(a: i32, n: &[u8]) -> i32 {
    if a < 0 {
        let sign = if rem(n, 4) == 3 { -1 } else { 1 };
        return sign * jacobi(-a, n);
    }
    jacobi_bytes(&a.to_le_bytes(), n)
}

fn jacobi_bytes(a: &[u8], n: &[u8]) -> i32 {
    let mut a = trim(a);
    let mut n = trim(n);
    if eq(&n, &[0]) {
        return 0;
    }
    a = mod_reduce(&a, &n);
    let mut symbol = 1i32;
    while cmp(&a, &[0]) == core::cmp::Ordering::Greater {
        while a[0] & 1 == 0 {
            shr1(&mut a);
            let n8 = rem(&n, 8);
            if n8 == 3 || n8 == 5 {
                symbol = -symbol;
            }
        }
        if eq(&a, &[0]) {
            break;
        }
        if eq(&a, &[1]) {
            return symbol;
        }
        if rem(&a, 4) == 3 && rem(&n, 4) == 3 {
            symbol = -symbol;
        }
        let next_n = a.clone();
        a = mod_reduce(&n, &a);
        n = next_n;
    }
    if eq(&n, &[1]) {
        symbol
    } else {
        0
    }
}

fn half(value: &[u8], n: &[u8]) -> Vec<u8> {
    let mut out = if value.first().is_some_and(|b| b & 1 == 1) {
        add(value, n)
    } else {
        value.to_vec()
    };
    shr1(&mut out);
    mod_reduce(&out, n)
}

fn mul_i32(value: &[u8], k: i32, n: &[u8]) -> Vec<u8> {
    let mag = mul_small(value, k.unsigned_abs());
    let mag = mod_reduce(&mag, n);
    if k >= 0 {
        mag
    } else if eq(&mag, &[0]) {
        vec![0]
    } else {
        sub(n, &mag).expect("magnitude is reduced")
    }
}

fn mod_i32(k: i32, n: &[u8]) -> Vec<u8> {
    mul_i32(&[1], k, n)
}

fn pow_mod(base: &[u8], exp: &[u8], n: &[u8]) -> Vec<u8> {
    let mut result = vec![1u8];
    let mut base = mod_reduce(base, n);
    let mut e = exp.to_vec();
    while cmp(&e, &[0]) == core::cmp::Ordering::Greater {
        if e[0] & 1 == 1 {
            result = mul_mod(&result, &base, n);
        }
        base = mul_mod(&base, &base, n);
        shr1(&mut e);
    }
    result
}

fn mul_mod(left: &[u8], right: &[u8], n: &[u8]) -> Vec<u8> {
    mod_reduce(&mul(left, right), n)
}

fn add_mod(left: &[u8], right: &[u8], n: &[u8]) -> Vec<u8> {
    mod_reduce(&add(left, right), n)
}

fn sub_mod(left: &[u8], right: &[u8], n: &[u8]) -> Vec<u8> {
    if cmp(left, right) != core::cmp::Ordering::Less {
        mod_reduce(&sub(left, right).expect("ordered"), n)
    } else {
        let gap = sub(right, left).expect("ordered");
        sub(n, &mod_reduce(&gap, n)).expect("reduced gap is smaller than n")
    }
}

fn mod_reduce(value: &[u8], n: &[u8]) -> Vec<u8> {
    let mut r = trim(value);
    if cmp(&r, n) == core::cmp::Ordering::Less {
        return if r.is_empty() { vec![0] } else { r };
    }
    let shift = bit_len(&r).saturating_sub(bit_len(n));
    let mut m = shl(n, shift);
    for i in (0..=shift).rev() {
        if cmp(&r, &m) != core::cmp::Ordering::Less {
            r = sub(&r, &m).expect("r >= m");
        }
        if i > 0 {
            shr1(&mut m);
        }
    }
    if r.is_empty() {
        vec![0]
    } else {
        r
    }
}

fn mul_small(value: &[u8], k: u32) -> Vec<u8> {
    let mut out = vec![0u8; value.len() + 4];
    let mut carry = 0u64;
    for (i, b) in value.iter().enumerate() {
        carry += u64::from(*b) * u64::from(k);
        out[i] = carry as u8;
        carry >>= 8;
    }
    let mut i = value.len();
    while carry > 0 {
        out[i] = carry as u8;
        carry >>= 8;
        i += 1;
    }
    trim(&out)
}

fn add(left: &[u8], right: &[u8]) -> Vec<u8> {
    let n = left.len().max(right.len());
    let mut out = vec![0u8; n + 1];
    let mut carry = 0u16;
    for (i, out_i) in out.iter_mut().enumerate().take(n) {
        let sum =
            u16::from(*left.get(i).unwrap_or(&0)) + u16::from(*right.get(i).unwrap_or(&0)) + carry;
        *out_i = sum as u8;
        carry = sum >> 8;
    }
    out[n] = carry as u8;
    trim(&out)
}

fn sub(left: &[u8], right: &[u8]) -> Option<Vec<u8>> {
    if cmp(left, right) == core::cmp::Ordering::Less {
        return None;
    }
    let mut out = left.to_vec();
    let mut borrow = 0i16;
    for (i, out_i) in out.iter_mut().enumerate() {
        let v = i16::from(*out_i) - i16::from(*right.get(i).unwrap_or(&0)) - borrow;
        if v < 0 {
            *out_i = (v + 256) as u8;
            borrow = 1;
        } else {
            *out_i = v as u8;
            borrow = 0;
        }
    }
    Some(trim(&out))
}

fn mul(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; left.len() + right.len() + 1];
    for (i, a) in left.iter().enumerate() {
        let mut carry = 0u16;
        for (j, b) in right.iter().enumerate() {
            let sum = u16::from(out[i + j]) + u16::from(*a) * u16::from(*b) + carry;
            out[i + j] = sum as u8;
            carry = sum >> 8;
        }
        let mut k = i + right.len();
        while carry > 0 {
            let sum = u16::from(out[k]) + carry;
            out[k] = sum as u8;
            carry = sum >> 8;
            k += 1;
        }
    }
    trim(&out)
}

fn shl(n: &[u8], bits: u32) -> Vec<u8> {
    if bits == 0 {
        return trim(n);
    }
    let byte = (bits / 8) as usize;
    let bit = bits % 8;
    let mut out = vec![0u8; n.len() + byte + 1];
    for (i, b) in n.iter().enumerate() {
        let value = u16::from(*b) << bit;
        out[i + byte] |= value as u8;
        out[i + byte + 1] |= (value >> 8) as u8;
    }
    trim(&out)
}

fn shr1(n: &mut Vec<u8>) {
    let mut carry = 0u8;
    for b in n.iter_mut().rev() {
        let next = *b & 1;
        *b = (*b >> 1) | (carry << 7);
        carry = next;
    }
    while n.last() == Some(&0) && n.len() > 1 {
        n.pop();
    }
    if n.is_empty() {
        n.push(0);
    }
}

fn bit_len(n: &[u8]) -> u32 {
    for (i, b) in n.iter().enumerate().rev() {
        if *b != 0 {
            return (i as u32) * 8 + (8 - b.leading_zeros());
        }
    }
    0
}

fn bit(n: &[u8], index: u32) -> bool {
    n.get((index / 8) as usize)
        .is_some_and(|b| b & (1u8 << (index % 8)) != 0)
}

fn rem(n: &[u8], p: u8) -> u8 {
    if p == 0 {
        return 0;
    }
    let mut acc = 0u16;
    for b in n.iter().rev() {
        acc = ((acc << 8) + u16::from(*b)) % u16::from(p);
    }
    acc as u8
}

fn eq(left: &[u8], right: &[u8]) -> bool {
    cmp(left, right) == core::cmp::Ordering::Equal
}

fn cmp(left: &[u8], right: &[u8]) -> core::cmp::Ordering {
    let n = left.len().max(right.len());
    for i in (0..n).rev() {
        let l = left.get(i).copied().unwrap_or(0);
        let r = right.get(i).copied().unwrap_or(0);
        if l != r {
            return l.cmp(&r);
        }
    }
    core::cmp::Ordering::Equal
}

fn trim(n: &[u8]) -> Vec<u8> {
    let mut out = n.to_vec();
    while out.last() == Some(&0) && out.len() > 1 {
        out.pop();
    }
    if out.is_empty() {
        vec![0]
    } else {
        out
    }
}

#[cfg(test)]
mod self_test {
    use super::is_baillie_psw;

    #[test]
    fn known_values() {
        assert!(is_baillie_psw(&[2]));
        assert!(is_baillie_psw(&[251, 3]), "1019");
        assert!(!is_baillie_psw(&[255, 7]), "2047");
    }
}
