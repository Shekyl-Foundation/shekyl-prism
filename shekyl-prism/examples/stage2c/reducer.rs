// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Native Stage 2c reduction. Its caller supplies a spanning intersection from
//! an admitted key (or a generic HNF). This prototype has no key decoder.
use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
use prism_int::{Int, SecretBit, SecretInt};

pub type Scalar<const W: usize> = SecretInt<W>;
type Word = SecretInt<1>;
pub type Vector<const W: usize> = [Scalar<W>; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Guarded,
    Generic,
}

#[derive(Clone, Copy, Debug)]
pub enum Schedule {
    Reviewed,
    Pr45Baseline,
}

#[derive(Clone, Copy)]
pub struct Steps {
    reviewed: usize,
    baseline: usize,
}

impl Steps {
    pub fn count(self, schedule: Schedule) -> usize {
        match schedule {
            Schedule::Reviewed => self.reviewed,
            Schedule::Pr45Baseline => self.baseline,
        }
    }
}

/// Bind each public set to its proven coordinate and constructor widths.
pub trait ReductionSet<const W: usize, const D: usize>: ParameterSet {
    const LIMBS: usize = W;
    const GUARDED: Steps;
    const GENERIC: Steps;
}

macro_rules! reduction_set {
    ($set:ty, $width:literal, guarded: $g:literal / $gb:literal, generic: $h:literal / $hb:literal) => {
        impl ReductionSet<$width, { 2 * $width }> for $set {
            const GUARDED: Steps = Steps {
                reviewed: $g,
                baseline: $gb,
            };
            const GENERIC: Steps = Steps {
                reviewed: $h,
                baseline: $hb,
            };
        }
    };
}

reduction_set!(P248, 11, guarded: 267 / 297, generic: 280 / 312);
reduction_set!(P324, 14, guarded: 347 / 387, generic: 360 / 401);
reduction_set!(P376, 16, guarded: 402 / 448, generic: 415 / 463);
reduction_set!(P500, 21, guarded: 533 / 594, generic: 546 / 609);
reduction_set!(P664, 27, guarded: 706 / 787, generic: 719 / 803);

pub struct Bounds<const W: usize, const D: usize> {
    guarded_steps: Steps,
    generic_steps: Steps,
    p: SecretInt<D>,
    max_norm: Scalar<W>,
    guarded_diagonal: Scalar<W>,
    guarded_first_squared: SecretInt<D>,
    generic_diagonal: Scalar<W>,
}

impl<const W: usize, const D: usize> Bounds<W, D> {
    pub fn steps(&self, class: Class) -> Steps {
        match class {
            Class::Guarded => self.guarded_steps,
            Class::Generic => self.generic_steps,
        }
    }

    pub fn new<S: ReductionSet<W, D>>() -> Self {
        let one = Int::<W>::one();
        let p = Int::from_u64(S::COFACTOR.value())
            .shl(S::E)
            .unwrap()
            .sub(&one)
            .unwrap();
        let q = one.shl(2 * S::E - 5).unwrap();
        let max_norm = q.shl(p.bit_length().div_ceil(2) + 32).unwrap();
        let root = p.sqrt_floor().unwrap().add(&one).unwrap();
        let guarded = root
            .shl(9)
            .unwrap()
            .mul(&q.add(&Int::from_u64(3)).unwrap())
            .unwrap();
        let generic = max_norm
            .shl(2)
            .unwrap()
            .add(&p.shr(1))
            .unwrap()
            .add(&one)
            .unwrap();
        Self {
            guarded_steps: S::GUARDED,
            generic_steps: S::GENERIC,
            p: Scalar::<W>::from_public(&p).resize(),
            max_norm: Scalar::<W>::from_public(&max_norm),
            guarded_diagonal: Scalar::<W>::from_public(&guarded),
            // A' = 2 nrd(U)/T <= 512 sqrt(p) q on a guarded intersection.
            guarded_first_squared: Scalar::<W>::from_public(&q)
                .resize::<D>()
                .shl(9)
                .mul(&Scalar::<W>::from_public(&q).resize::<D>().shl(9))
                .mul(&Scalar::<W>::from_public(&p).resize()),
            generic_diagonal: Scalar::<W>::from_public(&generic),
        }
    }
}

fn hermitian<const D: usize>(
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

fn inverse_odd<const W: usize>(t: &Scalar<W>) -> Scalar<W> {
    // Newton in Z/(2^(64W)). The low word starts correct modulo 2.
    let max = Scalar::from_public(&Int::<W>::from_le_limbs(&[u64::MAX; W], false).unwrap());
    let mut x = Scalar::one();
    for _ in 0..(W * 64).next_power_of_two().trailing_zeros() {
        let diff = Scalar::from_u64(2).sub(&t.mul_low(&x));
        let complement = max.sub(&diff.abs()).add(&Scalar::one());
        x = x.mul_low(&Scalar::select(&diff, &complement, &diff.is_negative()));
    }
    x
}

fn divide_form_entry<const W: usize, const D: usize>(
    n: &SecretInt<D>,
    t: &Scalar<W>,
    inv: &Scalar<W>,
    diagonal: bool,
) -> (Scalar<W>, SecretBit) {
    let bits = if diagonal { 2 } else { 1 };
    let half = n.shr(bits);
    let mag = half.truncate::<W>().mul_low(inv);
    let quotient = Scalar::select(&mag, &mag.neg(), &half.is_negative());
    let exact = quotient.resize::<D>().mul(&t.resize()).ct_eq(&half);
    let result = if diagonal { quotient.shl(1) } else { quotient };
    (result, exact.and(&n.low_bits_zero(bits)))
}

struct Gram<const W: usize> {
    a: Scalar<W>,
    c: Scalar<W>,
    h: [Scalar<W>; 2],
}

pub struct Basis<const W: usize> {
    steps: Steps,
    u: Vector<W>,
    v: Vector<W>,
    gram: Gram<W>,
    ok: SecretBit,
}

impl<const W: usize> Basis<W> {
    pub fn from_intersection<const D: usize>(
        t: &Scalar<W>,
        u: Vector<W>,
        v: Vector<W>,
        class: Class,
        bounds: &Bounds<W, D>,
    ) -> Self {
        let mut ok = t
            .is_odd()
            .and(&t.is_negative().not())
            .and(&t.ct_lt(&bounds.max_norm));
        let wide_u = u.each_ref().map(Scalar::resize);
        let wide_v = v.each_ref().map(Scalar::resize);
        let inv = inverse_odd(t);
        let (a, c, h) = match class {
            Class::Guarded => {
                for x in [&u, &v] {
                    ok = ok
                        .and(&x[0].sub(&x[3]).low_bits_zero(1))
                        .and(&x[1].sub(&x[2]).low_bits_zero(1));
                }
                let (h0, h1) = hermitian(&wide_v, &wide_u, &bounds.p);
                let mut entry = |n: &SecretInt<D>, diag| {
                    let (e, exact) = divide_form_entry(n, t, &inv, diag);
                    ok = ok.and(&exact);
                    e
                };
                (
                    entry(&hermitian(&wide_u, &wide_u, &bounds.p).0, true),
                    entry(&hermitian(&wide_v, &wide_v, &bounds.p).0, true),
                    [entry(&h0, false), entry(&h1, false)],
                )
            }
            Class::Generic => {
                let two_t = t.shl(1);
                ok = ok
                    .and(&u[0].ct_eq(&two_t))
                    .and(&v[0].low_bits_zero(1))
                    .and(&v[1].is_odd());
                for x in &u[1..] {
                    ok = ok.and(&x.ct_eq(&Scalar::zero()));
                }
                for x in &v[..2] {
                    ok = ok.and(&x.is_negative().not()).and(&x.ct_lt(&two_t));
                }
                ok = ok
                    .and(&v[2].ct_eq(&Scalar::one()))
                    .and(&v[3].ct_eq(&Scalar::zero()));
                let n = wide_v[0]
                    .mul(&wide_v[0])
                    .add(&wide_v[1].mul(&wide_v[1]))
                    .add(&bounds.p);
                let (c, exact) = divide_form_entry(&n, t, &inv, true);
                ok = ok.and(&exact);
                (two_t, c, [v[0].clone(), v[1].clone()])
            }
        };
        let det = a
            .resize::<D>()
            .mul(&c.resize())
            .sub(&h[0].resize::<D>().mul(&h[0].resize()))
            .sub(&h[1].resize::<D>().mul(&h[1].resize()));
        let limit = match class {
            Class::Guarded => &bounds.guarded_diagonal,
            Class::Generic => &bounds.generic_diagonal,
        };
        ok = ok.and(&det.ct_eq(&bounds.p));
        for value in [&a, &c] {
            ok = ok
                .and(&Scalar::zero().ct_lt(value))
                .and(&limit.ct_lt(value).not());
        }
        let mut result = Self {
            steps: bounds.steps(class),
            u,
            v,
            gram: Gram { a, c, h },
            ok,
        };
        result.order();
        if matches!(class, Class::Guarded) {
            let first = result.gram.a.resize::<D>();
            result.ok = result
                .ok
                .and(&bounds.guarded_first_squared.ct_lt(&first.mul(&first)).not());
        }
        result
    }

    fn order(&mut self) {
        let g = &mut self.gram;
        let swap = g.c.ct_lt(&g.a);
        Scalar::swap(&mut g.a, &mut g.c, &swap);
        g.h[1] = Scalar::select(&g.h[1], &g.h[1].neg(), &swap);
        for (a, b) in self.u.iter_mut().zip(&mut self.v) {
            Scalar::swap(a, b, &swap);
        }
    }

    fn step(&mut self) {
        let (m, s) = quotient(&self.gram);
        for i in [0, 2] {
            let a = self.u[i]
                .mul_word(&m[0])
                .sub(&self.u[i + 1].mul_word(&m[1]));
            let b = self.u[i + 1]
                .mul_word(&m[0])
                .add(&self.u[i].mul_word(&m[1]));
            self.v[i] = self.v[i].sub(&a.shl_secret(&s));
            self.v[i + 1] = self.v[i + 1].sub(&b.shl_secret(&s));
        }
        for (h, digit) in self.gram.h.iter_mut().zip(&m) {
            let r = h.sub(&self.gram.a.mul_word(digit).shl_secret(&s));
            self.gram.c = self.gram.c.sub(&h.add(&r).mul_word(digit).shl_secret(&s));
            *h = r;
        }
        self.order();
    }

    pub fn reduce(self, schedule: Schedule) -> Reduced<W> {
        let steps = self.steps.count(schedule);
        self.reduce_steps(steps)
    }

    fn reduce_steps(mut self, steps: usize) -> Reduced<W> {
        for _ in 0..steps {
            self.step();
        }
        let Gram { a, c, h: [h0, h1] } = self.gram;
        let mut form = [a.shr(1), c.shr(1), h0, h1];
        // All consumers inherit admission and arithmetic failures, even if only
        // one coefficient or column will subsequently leave the secret domain.
        let mut valid = self.ok;
        for x in self.u.iter().chain(&self.v).chain(&form) {
            valid = valid.and(&x.ct_eq(x));
        }
        let invalid = valid.not();
        for x in self.u.iter_mut().chain(&mut self.v).chain(&mut form) {
            x.mark_if(&invalid);
        }
        Reduced {
            u: self.u,
            v: self.v,
            form,
        }
    }
}

pub struct Reduced<const W: usize> {
    pub u: Vector<W>,
    pub v: Vector<W>,
    /// F00, F22, F20, F21. The second column of each Gaussian pair is its i-rotation.
    pub form: [Scalar<W>; 4],
}

fn positive_part(x: &Word) -> Word {
    Word::select(x, &Word::zero(), &x.is_negative())
}

fn quotient<const W: usize>(g: &Gram<W>) -> ([Word; 2], Word) {
    let bits = g.a.bit_length();
    let k = positive_part(&bits.sub(&Word::from_u64(64)));
    let [b0, b1] = g.h.each_ref().map(Scalar::bit_length);
    let largest = Word::select(&b0, &b1, &b0.ct_lt(&b1));
    let shift = positive_part(&largest.sub(&bits).sub(&Word::from_u64(60)));
    let big = Word::zero().ct_lt(&shift);
    let divisor = g.a.shr_secret(&k).resize::<1>();
    let align = k.add(&shift);
    let m = g.h.each_ref().map(|h| {
        let top = h.abs().shr_secret(&align).resize::<2>().div_word(&divisor);
        let signed = |x: &Word| Word::select(x, &x.neg(), &h.is_negative());
        let estimate = signed(&top);
        let correction = h.sub(&g.a.mul_word(&estimate)).shl(1).add(&g.a);
        let lower = correction.is_negative();
        let upper = correction.ct_lt(&g.a.shl(1)).not();
        let corrected = estimate
            .sub(&Word::select(&Word::zero(), &Word::one(), &lower))
            .add(&Word::select(&Word::zero(), &Word::one(), &upper));
        let undershoot = signed(&positive_part(&top.sub(&Word::one())));
        Word::select(&corrected, &undershoot, &big)
    });
    (m, shift)
}

/// Exercise the same constructor, step and form conversion on both half ties.
pub fn tie_fixture<S: ReductionSet<W, D>, const W: usize, const D: usize>(
    steps: usize,
) -> Reduced<W> {
    let mut bounds = Bounds::new::<S>();
    bounds.p = SecretInt::<D>::from_u64(3);
    Basis::from_intersection(
        &Scalar::from_u64(5),
        [10, 0, 0, 0].map(Scalar::from_u64),
        [4, 9, 1, 0].map(Scalar::from_u64),
        Class::Generic,
        &bounds,
    )
    .reduce_steps(steps)
}
