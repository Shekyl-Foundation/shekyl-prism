// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Variable-time exact baselines. NOT safe for secrets.
//! Cohen Algorithm 2.6.7 integral LLL (delta = 99/100), ported from
//! fsr.lll_exact. Only the Gram-Schmidt data (d, lambda) is kept, not the
//! transformation matrix H: the reported cost therefore slightly favours
//! this baseline compared with fsr.py, and the output is nrd(b1), not b1.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

pub fn gram(t: &BigInt, x: &BigInt, y: &BigInt, p: &BigInt) -> Vec<Vec<BigInt>> {
    let z = BigInt::zero();
    let o = BigInt::one();
    let b = [
        [t.clone(), z.clone(), z.clone(), z.clone()],
        [z.clone(), t.clone(), z.clone(), z.clone()],
        [x.clone(), y.clone(), o.clone(), z.clone()],
        [-y, x.clone(), z.clone(), o],
    ];
    (0..4)
        .map(|i| {
            (0..4)
                .map(|j| &b[i][0] * &b[j][0] + &b[i][1] * &b[j][1]
                    + p * (&b[i][2] * &b[j][2] + &b[i][3] * &b[j][3]))
                .collect()
        })
        .collect()
}

/// Returns (nrd(b1), number of Lovasz tests). Basis is LLL-reduced, not
/// necessarily shortest; cross-check vectors record lambda_1 separately.
#[inline(never)]
pub fn lll(g: &[Vec<BigInt>]) -> (BigInt, usize) {
    let n = g.len();
    let mut d = vec![BigInt::zero(); n + 1];
    d[0] = BigInt::one();
    let mut lam = vec![vec![BigInt::zero(); n]; n];
    for k in 0..n {
        for j in 0..=k {
            let mut u = g[k][j].clone();
            for i in 0..j {
                u = (&d[i + 1] * u - &lam[k][i] * &lam[j][i]) / &d[i];
            }
            if j < k {
                lam[k][j] = u;
            } else {
                d[k + 1] = u;
            }
        }
    }
    let redi = |k: usize, l: usize, lam: &mut Vec<Vec<BigInt>>, d: &Vec<BigInt>| {
        if 2 * lam[k][l].abs() > d[l + 1] {
            let q = (2i32 * &lam[k][l] + &d[l + 1]).div_floor(&(2i32 * &d[l + 1]));
            lam[k][l] -= &q * &d[l + 1];
            for i in 0..l {
                let t = &q * &lam[l][i];
                lam[k][i] -= t;
            }
        }
    };
    let mut k = 1;
    let mut steps = 0;
    while k < n {
        steps += 1;
        redi(k, k - 1, &mut lam, &d);
        if 100i32 * &d[k + 1] * &d[k - 1]
            < 99i32 * &d[k] * &d[k] - 100i32 * &lam[k][k - 1] * &lam[k][k - 1]
        {
            for j in 0..k - 1 {
                let t = lam[k][j].clone();
                lam[k][j] = lam[k - 1][j].clone();
                lam[k - 1][j] = t;
            }
            let lm = lam[k][k - 1].clone();
            let bv = (&d[k - 1] * &d[k + 1] + &lm * &lm) / &d[k];
            for i in k + 1..n {
                let t = lam[i][k].clone();
                let a = (&d[k + 1] * &lam[i][k - 1] - &lm * &t) / &d[k];
                let b = (&bv * &t + &lm * &a) / &d[k + 1];
                lam[i][k] = a;
                lam[i][k - 1] = b;
            }
            d[k] = bv;
            k = 1.max(k - 1);
        } else {
            for l in (0..k - 1).rev() {
                redi(k, l, &mut lam, &d);
            }
            k += 1;
        }
    }
    (d[1].clone(), steps)
}

/// Exact, variable-time Z[i]-Lagrange, for like-for-like comparison.
#[inline(never)]
pub fn gauss(t: &BigInt, x: &BigInt, y: &BigInt, p: &BigInt) -> (BigInt, usize) {
    let z = BigInt::zero();
    let o = BigInt::one();
    let mut u = [t.clone(), z.clone(), z.clone(), z.clone()];
    let mut v = [x.clone(), y.clone(), o, z];
    let norm = |a: &[BigInt; 4]| &a[0] * &a[0] + &a[1] * &a[1]
        + p * (&a[2] * &a[2] + &a[3] * &a[3]);
    let mut nu = norm(&u);
    let mut nv = norm(&v);
    if nv < nu {
        std::mem::swap(&mut u, &mut v);
        std::mem::swap(&mut nu, &mut nv);
    }
    let mut steps = 0;
    loop {
        steps += 1;
        let h0 = &v[0] * &u[0] + &v[1] * &u[1]
            + p * (&v[2] * &u[2] + &v[3] * &u[3]);
        let h1 = &v[1] * &u[0] - &v[0] * &u[1]
            + p * (&v[3] * &u[2] - &v[2] * &u[3]);
        let m0 = (2i32 * h0 + &nu).div_floor(&(2i32 * &nu));
        let m1 = (2i32 * h1 + &nu).div_floor(&(2i32 * &nu));
        if m0.is_zero() && m1.is_zero() {
            return (nu, steps);
        }
        v = [
            &v[0] - &m0 * &u[0] + &m1 * &u[1],
            &v[1] - &m0 * &u[1] - &m1 * &u[0],
            &v[2] - &m0 * &u[2] + &m1 * &u[3],
            &v[3] - &m0 * &u[3] - &m1 * &u[2],
        ];
        nv = norm(&v);
        if nv < nu {
            std::mem::swap(&mut u, &mut v);
            std::mem::swap(&mut nu, &mut nv);
        } else {
            return (nu, steps);
        }
    }
}
