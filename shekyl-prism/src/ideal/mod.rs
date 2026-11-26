// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Stage 2b/2d admission of the ideal component of a key, in O0 numerators.
mod geometry;
use geometry::{degree_bits, hermitian, inverse_odd, secret_norm_bits, GUARD_BITS};
use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
use prism_int::{Int, SecretBit, SecretInt as Scalar};
use prism_verify::prime::{challenge_degree, degree_is_prime};
use std::marker::PhantomData;

type Vector<const W: usize> = [Scalar<W>; 4];

/// Coordinate and constructor widths for ideal arithmetic at this public set.
pub trait IdealSet<const W: usize, const D: usize>: ParameterSet {}
impl IdealSet<11, 22> for P248 {}
impl IdealSet<14, 28> for P324 {}
impl IdealSet<16, 32> for P376 {}
impl IdealSet<21, 42> for P500 {}
impl IdealSet<27, 54> for P664 {}

/// A spanning intersection in O0 numerators ready for guarded Stage 2c reduction.
pub struct Intersection<const W: usize> {
    /// Carried norm Nq, marked if an invariant failed during signing.
    pub norm: Scalar<W>,
    /// First Gaussian generator.
    pub u: Vector<W>,
    /// Second Gaussian generator.
    pub v: Vector<W>,
}

/// An integral left ideal whose pre-reduced basis passed the factor-256 guard.
/// Integer fields are supplied by generation or by a key-format decoder.
pub struct GuardedIdeal<S: IdealSet<W, D>, const W: usize, const D: usize> {
    set: PhantomData<S>,
    p: Scalar<D>,
    norm: Scalar<W>,
    u: Vector<W>,
    v: Vector<W>,
}

type Gaussian<const W: usize> = [Scalar<W>; 2];

fn product<const W: usize>([a, b]: [&Scalar<W>; 2], [c, d]: [&Scalar<W>; 2]) -> Gaussian<W> {
    [a.mul(c).sub(&b.mul(d)), a.mul(d).add(&b.mul(c))]
}

fn difference<const W: usize>(a: Gaussian<W>, b: Gaussian<W>) -> Gaussian<W> {
    [a[0].sub(&b[0]), a[1].sub(&b[1])]
}

fn cross<const W: usize>(u: &Vector<W>, v: &Vector<W>) -> Gaussian<W> {
    let [a, b, c, d] = u;
    let [e, f, g, h] = v;
    difference(product([a, b], [g, h]), product([e, f], [c, d]))
}

fn abs_squared<const W: usize>(a: &Gaussian<W>) -> Scalar<W> {
    a[0].mul(&a[0]).add(&a[1].mul(&a[1]))
}

fn rotate<const W: usize>(v: &Vector<W>) -> Vector<W> {
    [v[1].neg(), v[0].clone(), v[3].neg(), v[2].clone()]
}

// Membership via Cramer's rule over Z[i]. Both determinants divided by
// det(u,v) must be Gaussian integers. Its squared magnitude is 4 N^2.
struct Span<const D: usize> {
    u: Vector<D>,
    v: Vector<D>,
    conjugate: Gaussian<D>,
    n2: Scalar<D>,
    inverse: Scalar<D>,
}

impl<const D: usize> Span<D> {
    fn contains(&self, w: &Vector<D>) -> SecretBit {
        let mut ok = self.n2.is_odd();
        for numerator in [cross(w, &self.v), cross(&self.u, w)] {
            for z in product(numerator.each_ref(), self.conjugate.each_ref()) {
                let mag = z.abs().shr(2);
                let q = mag.mul_low(&self.inverse);
                ok = ok
                    .and(&z.low_bits_zero(2))
                    .and(&q.mul(&self.n2).ct_eq(&mag));
            }
        }
        ok
    }
}

impl<S: IdealSet<W, D>, const W: usize, const D: usize> GuardedIdeal<S, W, D> {
    /// Shared by generation and import after the key's integer fields are decoded.
    /// HNF columns are in raw numerator coordinates, with positive pivots.
    pub fn admit(
        norm: Scalar<W>,
        hnf: &[Vector<W>; 4],
        u: Vector<W>,
        v: Vector<W>,
    ) -> Option<Self> {
        let prime = Int::<W>::from_u64(S::COFACTOR.value())
            .shl(S::E)
            .ok()?
            .sub(&Int::one())
            .ok()?;
        let limit =
            Scalar::from_public(&Int::one().shl(secret_norm_bits(prime.bit_length())).ok()?);
        let p = Scalar::<W>::from_public(&prime).resize::<D>();
        let n = norm.resize::<D>();
        let n2 = n.mul(&n);
        let u_wide = u.each_ref().map(Scalar::resize);
        let v_wide = v.each_ref().map(Scalar::resize);
        let det = cross(&u_wide, &v_wide);
        let index = n2.shl(2);
        let mut ok = norm
            .is_odd()
            .and(&norm.is_negative().not())
            .and(&norm.ct_lt(&limit))
            .and(&abs_squared(&det).ct_eq(&index));
        let span = Span {
            u: u_wide,
            v: v_wide,
            conjugate: [det[0].clone(), det[1].neg()],
            inverse: inverse_odd(&n2),
            n2,
        };
        let mut hnf_index = Scalar::<D>::one();
        for (j, col) in hnf.iter().enumerate() {
            ok = ok.and(&Scalar::zero().ct_lt(&col[j]));
            hnf_index = hnf_index.mul(&col[j].resize());
            for (i, x) in col.iter().enumerate() {
                if i < j {
                    ok = ok.and(&x.is_negative().not()).and(&x.ct_lt(&hnf[i][i]));
                }
                if i > j {
                    ok = ok.and(&x.ct_eq(&Scalar::zero()));
                }
            }
            ok = ok.and(&span.contains(&col.each_ref().map(Scalar::resize)));
        }
        ok = ok.and(&hnf_index.ct_eq(&index));
        // O0 = <1,i,(i+j)/2,(1+k)/2>; (1+k)/2 = 1 + i(i+j)/2.
        // i-stability is automatic for a Gaussian pair. Test the remaining
        // generator on *all four* integer columns, including iU and iV.
        for w in [&span.u, &span.v] {
            ok = ok
                .and(&w[0].sub(&w[3]).low_bits_zero(1))
                .and(&w[1].sub(&w[2]).low_bits_zero(1));
            for col in [w.clone(), rotate(w)] {
                let [a, b, c, d] = &col;
                let twice = [b.neg().sub(&p.mul(c)), a.add(&p.mul(d)), a.sub(d), c.sub(b)];
                for x in &twice {
                    ok = ok.and(&x.low_bits_zero(1));
                }
                ok = ok.and(&span.contains(&twice.map(|x| x.shr(1))));
            }
        }
        let weight = |w: &Vector<D>| hermitian(w, w, &p).0;
        let a = weight(&span.u);
        let c = weight(&span.v);
        let cap = n.shl(2 + GUARD_BITS);
        ok = ok
            .and(&Scalar::zero().ct_lt(&a))
            .and(&c.ct_lt(&a).not())
            .and(&p.mul(&cap.mul(&cap)).ct_lt(&c.mul(&c)).not());
        ok.declassify().unwrap_or(false).then_some(Self {
            norm,
            u,
            v,
            p,
            set: PhantomData,
        })
    }
}

// Montgomery arithmetic for the public odd challenge degree q. R=2^(64W).
// q fits below R/4. Inputs to multiply are in [0,q), so one subtraction
// suffices after the exact division by R. Products use the constructor width.
struct Modulus<const W: usize, const D: usize> {
    q: Scalar<W>,
    inverse: Scalar<W>,
    one: Scalar<W>,
    r2: Scalar<W>,
}
impl<const W: usize, const D: usize> Modulus<W, D> {
    fn new(q: &Int<W>) -> Self {
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
    fn mul(&self, x: &Scalar<W>, y: &Scalar<W>) -> Scalar<W> {
        let t = x.resize::<D>().mul(&y.resize());
        let lo = t.truncate::<W>().mul_low(&self.inverse);
        // Subtractive REDC avoids materializing -inverse modulo R.
        let d = t.sub(&self.q.resize::<D>().mul(&lo.resize()));
        let z = d.shr((64 * W) as u32).resize::<W>();
        Scalar::select(&z, &z.add(&self.q), &z.is_negative())
    }
    fn add(&self, x: &Scalar<W>, y: &Scalar<W>) -> Scalar<W> {
        let z = x.add(y);
        Scalar::select(&z.sub(&self.q), &z, &z.ct_lt(&self.q))
    }
    fn sub(&self, x: &Scalar<W>, y: &Scalar<W>) -> Scalar<W> {
        let z = x.sub(y);
        Scalar::select(&z, &z.add(&self.q), &z.is_negative())
    }
    fn encode(&self, x: &Scalar<W>) -> Scalar<W> {
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
    fn centered(&self, r: &Scalar<W>) -> Scalar<W> {
        let x = self.mul(r, &Scalar::one());
        Scalar::select(&x, &x.sub(&self.q), &self.q.shr(1).ct_lt(&x))
    }
    fn residue(&self, w: &Vector<W>, root: &Gaussian<W>) -> Gaussian<W> {
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
        // belong to the challenge ideal. With q = 3 mod 4 its nonzero Gaussian
        // residue has invertible norm. Check the inverse before using the result.
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
