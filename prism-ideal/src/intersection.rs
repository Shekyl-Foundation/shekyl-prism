// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Stage 2d: the guarded intersection with a challenge ideal.
use crate::admission::{rotate, GuardedIdeal};
use crate::geometry::degree_bits;
use crate::modulus::Modulus;
use crate::{Gaussian, IdealSet, Vector};
use prism_int::{Int, SecretInt as Scalar};
use prism_verify::prime::{challenge_degree, degree_is_prime};

/// A spanning intersection in O0 numerators ready for guarded Stage 2c reduction.
pub struct Intersection<const W: usize> {
    /// Carried norm Nq, marked if an invariant failed during signing.
    pub norm: Scalar<W>,
    /// First Gaussian generator.
    pub u: Vector<W>,
    /// Second Gaussian generator.
    pub v: Vector<W>,
}

impl<S: IdealSet<W, D>, const W: usize, const D: usize> GuardedIdeal<S, W, D> {
    /// The public challenge scalar is little endian as in the verifier. The ideal root
    /// is secret. Invalid roots or a failed inverse mark the reduction outputs.
    pub fn intersection(&self, scalar: &[u8], root: &Gaussian<W>) -> Option<Intersection<W>> {
        if scalar.len() != (S::CHALLENGE_BITS as usize).div_ceil(8)
            || !degree_is_prime::<S::Degree>(S::CHAIN_STEPS, scalar)
        {
            return None;
        }
        let degree =
            Int::from_le_bytes(&challenge_degree::<S::Degree>(S::CHAIN_STEPS, scalar)?).ok()?;
        if degree.bit_length() != degree_bits(S::E) {
            return None;
        }
        let ring = Modulus::<W, D>::new(&degree);
        let root = root.each_ref().map(|x| ring.encode(x));
        let p = ring.encode(&self.p.resize());
        let mut ok = ring
            .add(
                &p,
                &ring.add(&ring.mul(&root[0], &root[0]), &ring.mul(&root[1], &root[1])),
            )
            .ct_eq(&Scalar::zero());
        // The guard bounds nrd(v) below q at each set, so nonzero v cannot
        // belong to the challenge ideal. For prime q = 3 mod 4, its nonzero
        // Gaussian residue has invertible norm. Verify the inverse product
        // even though the public degree passed the probable-prime predicate.
        let u = &self.u;
        let v = &self.v;
        let ru = ring.residue(u, &root);
        let rv = ring.residue(v, &root);
        let norm = ring.add(&ring.mul(&rv[0], &rv[0]), &ring.mul(&rv[1], &rv[1]));
        let exponent = degree.sub(&Int::from_u64(2)).ok()?;
        let mut inv = ring.one.clone();
        for bit in (0..exponent.bit_length()).rev() {
            inv = ring.mul(&inv, &inv);
            if exponent.shr(bit).is_odd() {
                inv = ring.mul(&inv, &norm);
            }
        }
        ok = ok.and(&ring.mul(&inv, &norm).ct_eq(&ring.one));
        let re = ring.add(&ring.mul(&ru[0], &rv[0]), &ring.mul(&ru[1], &rv[1]));
        let im = ring.sub(&ring.mul(&ru[1], &rv[0]), &ring.mul(&ru[0], &rv[1]));
        let gamma = [re, im].map(|x| ring.centered(&ring.mul(&x, &inv)).neg());
        let jv = rotate(v);
        let mut second: Vector<W> =
            std::array::from_fn(|k| u[k].add(&gamma[0].mul(&v[k])).add(&gamma[1].mul(&jv[k])));
        let mut first = v.each_ref().map(|x| x.mul(&ring.q));
        let mut t = self.norm.mul(&ring.q);
        let invalid = ok.not();
        for entry in first.iter_mut().chain(&mut second).chain([&mut t]) {
            entry.mark_if(&invalid);
        }
        Some(Intersection {
            norm: t,
            u: first,
            v: second,
        })
    }
}
