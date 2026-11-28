// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Montgomery arithmetic modulo the public challenge degree.
use crate::geometry::inverse_odd;
use crate::{Gaussian, Vector};
use prism_int::{Int, SecretInt as Scalar};

// Montgomery arithmetic for the public odd challenge degree q. R=2^(64W).
// q fits below R/4. Inputs to multiply are in [0,q), so one subtraction
// suffices after the exact division by R. Products use the constructor width.
pub(crate) struct Modulus<const W: usize, const D: usize> {
    pub(crate) q: Scalar<W>,
    inverse: Scalar<W>,
    pub(crate) one: Scalar<W>,
    r2: Scalar<W>,
}
impl<const W: usize, const D: usize> Modulus<W, D> {
    pub(crate) fn new(q: &Int<W>) -> Self {
        let one = Int::<W>::one();
        let r = one.shl((64 * W - 1) as u32).unwrap().rem_euclid(q).unwrap();
        let r = r.shl(1).unwrap().rem_euclid(q).unwrap();
        let mut r2 = r.clone();
        for _ in 0..64 * W {
            r2 = r2.shl(1).unwrap();
            if &r2 >= q {
                r2 = r2.sub(q).unwrap();
            }
        }
        let q = Scalar::from_public(q);
        let inverse = inverse_odd(&q);
        Self {
            q,
            inverse,
            one: Scalar::from_public(&r),
            r2: Scalar::from_public(&r2),
        }
    }
    pub(crate) fn mul(&self, x: &Scalar<W>, y: &Scalar<W>) -> Scalar<W> {
        let t = x.resize::<D>().mul(&y.resize());
        let lo = t.truncate::<W>().mul_low(&self.inverse);
        // Subtractive REDC avoids materializing -inverse modulo R.
        let d = t.sub(&self.q.resize::<D>().mul(&lo.resize()));
        let z = d.shr((64 * W) as u32).resize::<W>();
        Scalar::select(&z, &z.add(&self.q), &z.is_negative())
    }
    pub(crate) fn add(&self, x: &Scalar<W>, y: &Scalar<W>) -> Scalar<W> {
        let z = x.add(y);
        Scalar::select(&z.sub(&self.q), &z, &z.ct_lt(&self.q))
    }
    pub(crate) fn sub(&self, x: &Scalar<W>, y: &Scalar<W>) -> Scalar<W> {
        let z = x.sub(y);
        Scalar::select(&z, &z.add(&self.q), &z.is_negative())
    }
    pub(crate) fn encode(&self, x: &Scalar<W>) -> Scalar<W> {
        // Encoded coordinates can be wider than q. Fold all W limbs before REDC.
        let mut r = Scalar::zero();
        let magnitude = x.abs();
        for bit in (0..(W * 64) as u32).rev() {
            r = self.add(&r, &r);
            r = Scalar::select(
                &r,
                &self.add(&r, &Scalar::one()),
                &magnitude.shr(bit).is_odd(),
            );
        }
        let r = Scalar::select(&r, &self.sub(&Scalar::zero(), &r), &x.is_negative());
        self.mul(&r, &self.r2)
    }
    pub(crate) fn centered(&self, r: &Scalar<W>) -> Scalar<W> {
        let x = self.mul(r, &Scalar::one());
        Scalar::select(&x, &x.sub(&self.q), &self.q.shr(1).ct_lt(&x))
    }
    pub(crate) fn residue(&self, w: &Vector<W>, root: &Gaussian<W>) -> Gaussian<W> {
        let [a, b, c, d] = w.each_ref().map(|v| self.encode(v));
        [
            self.add(
                &self.sub(&a, &self.mul(&c, &root[0])),
                &self.mul(&d, &root[1]),
            ),
            self.sub(
                &self.sub(&b, &self.mul(&c, &root[1])),
                &self.mul(&d, &root[0]),
            ),
        ]
    }
}
