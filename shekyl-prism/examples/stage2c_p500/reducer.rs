// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The P500 reduction slice. Its caller supplies a spanning intersection from
//! an admitted key (or a generic HNF). This prototype has no key decoder.
use prism_field::{ParameterSet, P500};
use prism_int::{Int, SecretBit, SecretInt};

pub type Scalar = SecretInt<21>;
type Wide = SecretInt<42>;
type Word = SecretInt<1>;
pub type Vector = [Scalar; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Guarded,
    Generic,
}

impl Class {
    pub fn steps(self, schedule: Schedule) -> usize {
        match (self, schedule) {
            (Self::Guarded, Schedule::Reviewed) => 533,
            (Self::Generic, Schedule::Reviewed) => 546,
            (Self::Guarded, Schedule::Pr45Baseline) => 594,
            (Self::Generic, Schedule::Pr45Baseline) => 609,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Schedule {
    Reviewed,
    Pr45Baseline,
}

pub struct Bounds {
    p: Wide,
    max_norm: Scalar,
    guarded_diagonal: Scalar,
    guarded_first_squared: Wide,
    generic_diagonal: Scalar,
}

impl Bounds {
    pub fn p500() -> Self {
        let one = Int::<21>::one();
        let p = Int::from_u64(P500::COFACTOR.value())
            .shl(P500::E)
            .unwrap()
            .sub(&one)
            .unwrap();
        let q = one.shl(2 * P500::E - 5).unwrap();
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
            p: Scalar::from_public(&p).resize(),
            max_norm: Scalar::from_public(&max_norm),
            guarded_diagonal: Scalar::from_public(&guarded),
            // A' = 2 nrd(U)/T <= 512 sqrt(p) q on a guarded intersection.
            guarded_first_squared: Scalar::from_public(&q)
                .resize::<42>()
                .shl(9)
                .mul(&Scalar::from_public(&q).resize::<42>().shl(9))
                .mul(&Scalar::from_public(&p).resize()),
            generic_diagonal: Scalar::from_public(&generic),
        }
    }
}

fn hermitian(left: &[Wide; 4], right: &[Wide; 4], p: &Wide) -> (Wide, Wide) {
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

fn inverse_odd(t: &Scalar) -> Scalar {
    // Newton in Z/(2^(64W)). The low word starts correct modulo 2.
    let max = Scalar::from_le_bytes(&[0xff; 21 * 8]);
    let mut x = Scalar::one();
    for _ in 0..(21 * 64_usize).next_power_of_two().trailing_zeros() {
        let diff = Scalar::from_u64(2).sub(&t.mul_low(&x));
        let complement = max.sub(&diff.abs()).add(&Scalar::one());
        x = x.mul_low(&Scalar::select(&diff, &complement, &diff.is_negative()));
    }
    x
}

fn divide_form_entry(n: &Wide, t: &Scalar, inv: &Scalar, diagonal: bool) -> (Scalar, SecretBit) {
    let bits = if diagonal { 2 } else { 1 };
    let half = n.shr(bits);
    let mag = half.truncate::<21>().mul_low(inv);
    let quotient = Scalar::select(&mag, &mag.neg(), &half.is_negative());
    let exact = quotient.resize::<42>().mul(&t.resize()).ct_eq(&half);
    let result = if diagonal { quotient.shl(1) } else { quotient };
    (result, exact.and(&n.low_bits_zero(bits)))
}

struct Gram {
    a: Scalar,
    c: Scalar,
    h: [Scalar; 2],
}

pub struct Basis {
    class: Class,
    u: Vector,
    v: Vector,
    gram: Gram,
    ok: SecretBit,
}

impl Basis {
    pub fn from_intersection(
        t: &Scalar,
        u: Vector,
        v: Vector,
        class: Class,
        bounds: &Bounds,
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
                let mut entry = |n: &Wide, diag| {
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
            .resize::<42>()
            .mul(&c.resize())
            .sub(&h[0].resize::<42>().mul(&h[0].resize()))
            .sub(&h[1].resize::<42>().mul(&h[1].resize()));
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
            class,
            u,
            v,
            gram: Gram { a, c, h },
            ok,
        };
        result.order();
        if matches!(class, Class::Guarded) {
            let first = result.gram.a.resize::<42>();
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

    pub fn reduce(self, schedule: Schedule) -> Reduced {
        let steps = self.class.steps(schedule);
        self.reduce_steps(steps)
    }

    fn reduce_steps(mut self, steps: usize) -> Reduced {
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

pub struct Reduced {
    pub u: Vector,
    pub v: Vector,
    /// F00, F22, F20, F21. The second column of each Gaussian pair is its i-rotation.
    pub form: [Scalar; 4],
}

fn positive_part(x: &Word) -> Word {
    Word::select(x, &Word::zero(), &x.is_negative())
}

fn quotient(g: &Gram) -> ([Word; 2], Word) {
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
pub fn tie_fixture(steps: usize) -> Reduced {
    let mut bounds = Bounds::p500();
    bounds.p = Wide::from_u64(3);
    Basis::from_intersection(
        &Scalar::from_u64(5),
        [10, 0, 0, 0].map(Scalar::from_u64),
        [4, 9, 1, 0].map(Scalar::from_u64),
        Class::Generic,
        &bounds,
    )
    .reduce_steps(steps)
}
