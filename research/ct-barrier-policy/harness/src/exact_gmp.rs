// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The same variable-time exact baselines as `exact`, on GMP (rug), with
//! in-place arithmetic: the stronger baseline for the cycle ratio. NOT
//! safe for secrets.

use num_bigint::BigInt;
use rug::ops::DivRounding;
use rug::{Assign, Integer};

pub fn conv(a: &BigInt) -> Integer {
    Integer::from_str_radix(&a.to_str_radix(16), 16).unwrap()
}

pub fn gram(t: &Integer, x: &Integer, y: &Integer, p: &Integer) -> Vec<Vec<Integer>> {
    let z = Integer::new();
    let o = Integer::from(1);
    let b = [
        [t.clone(), z.clone(), z.clone(), z.clone()],
        [z.clone(), t.clone(), z.clone(), z.clone()],
        [x.clone(), y.clone(), o.clone(), z.clone()],
        [Integer::from(-y), x.clone(), z.clone(), o],
    ];
    (0..4)
        .map(|i| {
            (0..4)
                .map(|j| {
                    let mut s = Integer::from(&b[i][2] * &b[j][2]);
                    s += &b[i][3] * &b[j][3];
                    s *= p;
                    s += &b[i][0] * &b[j][0];
                    s += &b[i][1] * &b[j][1];
                    s
                })
                .collect()
        })
        .collect()
}

/// Cohen 2.6.7 integral LLL, delta = 99/100. Returns (nrd(b1), Lovasz tests).
#[inline(never)]
pub fn lll(g: &[Vec<Integer>]) -> (Integer, usize) {
    let n = g.len();
    let mut d = vec![Integer::new(); n + 1];
    d[0].assign(1);
    let mut lam = vec![vec![Integer::new(); n]; n];
    let (mut t1, mut t2, mut q) = (Integer::new(), Integer::new(), Integer::new());
    for k in 0..n {
        for j in 0..=k {
            let mut u = g[k][j].clone();
            for i in 0..j {
                t1.assign(&d[i + 1] * &u);
                t2.assign(&lam[k][i] * &lam[j][i]);
                t1 -= &t2;
                u.assign(t1.div_exact_ref(&d[i]));
            }
            if j < k {
                lam[k][j] = u;
            } else {
                d[k + 1] = u;
            }
        }
    }
    fn redi(k: usize, l: usize, lam: &mut [Vec<Integer>], d: &[Integer], q: &mut Integer, t: &mut Integer) {
        t.assign(lam[k][l].abs_ref());
        *t <<= 1;
        if *t > d[l + 1] {
            q.assign(&lam[k][l] << 1);
            *q += &d[l + 1];
            t.assign(&d[l + 1] << 1);
            let qq = Integer::from((&*q).div_floor(&*t));
            q.assign(qq);
            t.assign(&*q * &d[l + 1]);
            lam[k][l] -= &*t;
            for i in 0..l {
                t.assign(&*q * &lam[l][i]);
                lam[k][i] -= &*t;
            }
        }
    }
    let mut k = 1;
    let mut steps = 0;
    let (mut lhs, mut rhs) = (Integer::new(), Integer::new());
    while k < n {
        steps += 1;
        redi(k, k - 1, &mut lam, &d, &mut q, &mut t1);
        lhs.assign(&d[k + 1] * &d[k - 1]);
        lhs *= 100;
        rhs.assign(&d[k] * &d[k]);
        rhs *= 99;
        t1.assign(lam[k][k - 1].square_ref());
        t1 *= 100;
        rhs -= &t1;
        if lhs < rhs {
            for j in 0..k - 1 {
                let (a, b) = lam.split_at_mut(k);
                std::mem::swap(&mut a[k - 1][j], &mut b[0][j]);
            }
            let lm = lam[k][k - 1].clone();
            let mut bv = Integer::from(&d[k - 1] * &d[k + 1]);
            bv += lm.square_ref();
            bv.div_exact_mut(&d[k]);
            for i in k + 1..n {
                let t = lam[i][k].clone();
                let mut a = Integer::from(&d[k + 1] * &lam[i][k - 1]);
                a -= &lm * &t;
                a.div_exact_mut(&d[k]);
                let mut b = Integer::from(&bv * &t);
                b += &lm * &a;
                b.div_exact_mut(&d[k + 1]);
                lam[i][k] = a;
                lam[i][k - 1] = b;
            }
            d[k] = bv;
            k = 1.max(k - 1);
        } else {
            for l in (0..k - 1).rev() {
                redi(k, l, &mut lam, &d, &mut q, &mut t1);
            }
            k += 1;
        }
    }
    (d[1].clone(), steps)
}

/// Exact variable-time Z[i]-Lagrange on GMP.
#[inline(never)]
pub fn gauss(t: &Integer, x: &Integer, y: &Integer, p: &Integer) -> (Integer, usize) {
    let mut u = [t.clone(), Integer::new(), Integer::new(), Integer::new()];
    let mut v = [x.clone(), y.clone(), Integer::from(1), Integer::new()];
    fn norm(a: &[Integer; 4], p: &Integer, out: &mut Integer, tmp: &mut Integer) {
        out.assign(a[2].square_ref());
        *out += a[3].square_ref();
        *out *= p;
        tmp.assign(a[0].square_ref());
        *out += &*tmp;
        tmp.assign(a[1].square_ref());
        *out += &*tmp;
    }
    let (mut nu, mut nv, mut tmp) = (Integer::new(), Integer::new(), Integer::new());
    norm(&u, p, &mut nu, &mut tmp);
    norm(&v, p, &mut nv, &mut tmp);
    if nv < nu {
        std::mem::swap(&mut u, &mut v);
        std::mem::swap(&mut nu, &mut nv);
    }
    let (mut h0, mut h1, mut den, mut m0, mut m1) =
        (Integer::new(), Integer::new(), Integer::new(), Integer::new(), Integer::new());
    let mut steps = 0;
    loop {
        steps += 1;
        h0.assign(&v[2] * &u[2]);
        h0 += &v[3] * &u[3];
        h0 *= p;
        h0 += &v[0] * &u[0];
        h0 += &v[1] * &u[1];
        h1.assign(&v[3] * &u[2]);
        h1 -= &v[2] * &u[3];
        h1 *= p;
        h1 += &v[1] * &u[0];
        h1 -= &v[0] * &u[1];
        den.assign(&nu << 1);
        m0.assign(&h0 << 1);
        m0 += &nu;
        let t0 = Integer::from((&m0).div_floor(&den));
        m0.assign(t0);
        m1.assign(&h1 << 1);
        m1 += &nu;
        let t1 = Integer::from((&m1).div_floor(&den));
        m1.assign(t1);
        if m0 == 0 && m1 == 0 {
            return (nu, steps);
        }
        v[0] -= &m0 * &u[0];
        v[0] += &m1 * &u[1];
        v[1] -= &m0 * &u[1];
        v[1] -= &m1 * &u[0];
        v[2] -= &m0 * &u[2];
        v[2] += &m1 * &u[3];
        v[3] -= &m0 * &u[3];
        v[3] -= &m1 * &u[2];
        norm(&v, p, &mut nv, &mut tmp);
        if nv < nu {
            std::mem::swap(&mut u, &mut v);
            std::mem::swap(&mut nu, &mut nv);
        } else {
            return (nu, steps);
        }
    }
}
