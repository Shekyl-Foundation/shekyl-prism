// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Native Stage 2c reduction. Its caller supplies a spanning intersection from
//! an admitted key (or a generic HNF). This prototype has no key decoder.
use crate::{self as ideal, geometry, GenericHnf, GuardedIdeal, IdealSet};
use prism_field::{P248, P324, P376, P500, P664};
use prism_int::{Int, SecretBit, SecretInt};

/// A secret integer of the set's coordinate width.
pub type Scalar<const W: usize> = SecretInt<W>;
type Word = SecretInt<1>;
/// Four O0 numerator coordinates, in order `(1, i, j, k)`.
pub type Vector<const W: usize> = [Scalar<W>; 4];

/// exp64, as section 2.3 states it. The divisor keeps the top
/// [`QUOTIENT_TOP_BITS`]. An exponent at or below [`QUOTIENT_WINDOW`] is one
/// half-up step; a larger exponent is the undershooting big step.
const QUOTIENT_TOP_BITS: u64 = 64;
const QUOTIENT_WINDOW: u64 = 60;

/// The reducer's diagonal stores `A' = 2 · nrd(U) / T` until the form is halved.
const DOUBLED_DIAGONAL_BITS: u32 = 1;

/// Slack on `q` in the guarded diagonal bound. The schedule's `C0` uses `q + 1`.
const GUARDED_DIAGONAL_SLACK: u64 = 3;

/// The public input class. It selects the starting bounds and the schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// A guarded intersection from an admitted key.
    Guarded,
    /// The generators of a generic normal form.
    Generic,
}

impl Class {
    /// Corpus spelling. Fixture files and measurement rows use this token.
    /// `Debug` is not a wire format.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Guarded => "prered-guard256",
            Self::Generic => "sign-generic",
        }
    }

    /// Inverse of [`Self::token`]. `None` is an unknown token, not a class.
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "prered-guard256" => Some(Self::Guarded),
            "sign-generic" => Some(Self::Generic),
            _ => None,
        }
    }
}

/// Which public step count the reducer runs.
#[derive(Clone, Copy, Debug)]
pub enum Schedule {
    /// The step table of `docs/SIGNER_SPEC.md` 2.3.
    Reviewed,
    /// The earlier, longer counts, kept for differential runs.
    Pr45Baseline,
}

impl Schedule {
    /// Measurement-row spelling of the schedule the reducer actually ran.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Reviewed => "reviewed",
            Self::Pr45Baseline => "pr45-baseline",
        }
    }
}

/// The step counts of one class at one set, one for each [`Schedule`].
#[derive(Clone, Copy)]
pub struct Steps {
    reviewed: usize,
    baseline: usize,
}

impl Steps {
    /// The count for `schedule`.
    pub fn count(self, schedule: Schedule) -> usize {
        match schedule {
            Schedule::Reviewed => self.reviewed,
            Schedule::Pr45Baseline => self.baseline,
        }
    }
}

/// Bind each public set to its [`IdealSet`] widths, schedules, and ideal corpus.
pub trait ReductionSet<const W: usize, const D: usize>: IdealSet<W, D> {
    /// The coordinate width in limbs.
    const LIMBS: usize = W;
    /// Step counts of the guarded class.
    const GUARDED: Steps;
    /// Step counts of the generic class.
    const GENERIC: Steps;
    /// Steps that pre-reduce a key-generation candidate.
    const SECRET_IDEAL_STEPS: usize;
    /// Recovered norm, root, basis, and challenge for this set's guarded rows.
    const IDEAL_CORPUS: &'static str;
}

macro_rules! reduction_set {
    ($set:ty, $width:literal, guarded: $g:literal / $gb:literal, generic: $h:literal / $hb:literal, secret_ideal: $k:literal, corpus: $corpus:literal) => {
        impl ReductionSet<$width, { 2 * $width }> for $set {
            const SECRET_IDEAL_STEPS: usize = $k;
            const IDEAL_CORPUS: &'static str = include_str!($corpus);
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

reduction_set!(P248, 11, guarded: 267 / 297, generic: 280 / 312, secret_ideal: 22, corpus: "../examples/stage2c/ideals/p248.txt");
reduction_set!(P324, 14, guarded: 347 / 387, generic: 360 / 401, secret_ideal: 22, corpus: "../examples/stage2c/ideals/p324.txt");
reduction_set!(P376, 16, guarded: 402 / 448, generic: 415 / 463, secret_ideal: 23, corpus: "../examples/stage2c/ideals/p376.txt");
reduction_set!(P500, 21, guarded: 533 / 594, generic: 546 / 609, secret_ideal: 22, corpus: "../examples/stage2c/ideals/p500.txt");
reduction_set!(P664, 27, guarded: 706 / 787, generic: 719 / 803, secret_ideal: 24, corpus: "../examples/stage2c/ideals/p664.txt");

/// The public bounds of a set: step counts, the prime, and the largest
/// norm and starting diagonal of each class.
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
    /// The step counts of `class`.
    pub fn steps(&self, class: Class) -> Steps {
        match class {
            Class::Guarded => self.guarded_steps,
            Class::Generic => self.generic_steps,
        }
    }

    /// The bounds of the set `S`.
    pub fn new<S: ReductionSet<W, D>>() -> Self {
        let one = Int::<W>::one();
        let p = ideal::field_prime::<S, W, D>().unwrap();
        let q = one.shl(geometry::degree_bits(S::E)).unwrap();
        let secret_norm_bits = geometry::secret_norm_bits(p.bit_length());
        let max_norm = q.shl(secret_norm_bits).unwrap();
        let root = p.sqrt_floor().unwrap().add(&one).unwrap();
        // A' ≤ 2^{DOUBLED_DIAGONAL_BITS + GUARD_BITS} · √p · (q + slack).
        let guard_shift = DOUBLED_DIAGONAL_BITS + geometry::GUARD_BITS;
        let guarded = root
            .shl(guard_shift)
            .unwrap()
            .mul(&q.add(&Int::from_u64(GUARDED_DIAGONAL_SLACK)).unwrap())
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
            // A' = 2 nrd(U)/T ≤ 2^{DOUBLED_DIAGONAL_BITS + GUARD_BITS} √p · q.
            guarded_first_squared: Scalar::<W>::from_public(&q)
                .resize::<D>()
                .shl(guard_shift)
                .mul(&Scalar::<W>::from_public(&q).resize::<D>().shl(guard_shift))
                .mul(&Scalar::<W>::from_public(&p).resize()),
            generic_diagonal: Scalar::<W>::from_public(&generic),
        }
    }
}

fn divide_form_entry<const W: usize, const D: usize>(
    n: &SecretInt<D>,
    t: &Scalar<W>,
    inv: &Scalar<W>,
    diagonal: bool,
) -> (Scalar<W>, SecretBit) {
    let bits = if diagonal {
        geometry::DENOMINATOR_SQUARE_BITS
    } else {
        geometry::ORDER_DENOMINATOR_BITS
    };
    let half = n.shr(bits);
    let mag = half.truncate::<W>().mul_low(inv);
    let quotient = Scalar::select(&mag, &mag.neg(), &half.is_negative());
    let exact = quotient.resize::<D>().mul(&t.resize()).ct_eq(&half);
    let result = if diagonal {
        quotient.shl(DOUBLED_DIAGONAL_BITS)
    } else {
        quotient
    };
    (result, exact.and(&n.low_bits_zero(bits)))
}

struct Gram<const W: usize> {
    a: Scalar<W>,
    c: Scalar<W>,
    h: [Scalar<W>; 2],
}

/// A Gaussian pair with its normalized Gram form, before or during
/// reduction, and the validity bit its constructor computed.
pub struct Basis<const W: usize> {
    steps: Steps,
    u: Vector<W>,
    v: Vector<W>,
    gram: Gram<W>,
    ok: SecretBit,
}

impl<const W: usize> Basis<W> {
    /// Fixture support: the four Gram entries after the initial ordering.
    #[doc(hidden)]
    pub fn ordered_gram(&self) -> [&Scalar<W>; 4] {
        [&self.gram.a, &self.gram.c, &self.gram.h[0], &self.gram.h[1]]
    }

    /// Build the normalized form of the pair `(u, v)` of an ideal of norm
    /// `t`, checking the class's shape, the exact divisions, the
    /// determinant and the starting bounds. A failed check is carried in
    /// the validity bit and marks every output of the reduction.
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
        let inv = geometry::inverse_odd(t);
        let (a, c, h) = match class {
            Class::Guarded => {
                for x in [&u, &v] {
                    ok = ok
                        .and(
                            &x[0]
                                .sub(&x[3])
                                .low_bits_zero(geometry::ORDER_DENOMINATOR_BITS),
                        )
                        .and(
                            &x[1]
                                .sub(&x[2])
                                .low_bits_zero(geometry::ORDER_DENOMINATOR_BITS),
                        );
                }
                let (h0, h1) = geometry::hermitian(&wide_v, &wide_u, &bounds.p);
                let mut entry = |n: &SecretInt<D>, diag| {
                    let (e, exact) = divide_form_entry(n, t, &inv, diag);
                    ok = ok.and(&exact);
                    e
                };
                (
                    entry(&geometry::hermitian(&wide_u, &wide_u, &bounds.p).0, true),
                    entry(&geometry::hermitian(&wide_v, &wide_v, &bounds.p).0, true),
                    [entry(&h0, false), entry(&h1, false)],
                )
            }
            Class::Generic => {
                let two_t = t.shl(geometry::ORDER_DENOMINATOR_BITS);
                ok = ok
                    .and(&u[0].ct_eq(&two_t))
                    .and(&v[0].low_bits_zero(geometry::ORDER_DENOMINATOR_BITS))
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

    /// Pre-reduce the generic HNF of a key-generation candidate and admit it.
    ///
    /// The column-HNF is [`GenericHnf::from_norm_and_root`]. The reduced basis
    /// is returned only together with the [`GuardedIdeal`] that
    /// [`GuardedIdeal::admit`] produced from that same HNF. Q4, the guard,
    /// and every other admission predicate are that one verdict.
    pub fn secret_candidate<S: ReductionSet<W, D>, const D: usize>(
        norm: &Scalar<W>,
        root: &ideal::Gaussian<W>,
        bounds: &Bounds<W, D>,
    ) -> Option<(GuardedIdeal<S, W, D>, Reduced<W>)> {
        let hnf = GenericHnf::from_norm_and_root(norm, root);
        let (u, v) = hnf.generators();
        let reduced = Self::from_intersection(norm, u, v, Class::Generic, bounds)
            .reduce_steps(S::SECRET_IDEAL_STEPS);
        let admitted = GuardedIdeal::<S, W, D>::admit(
            norm.clone(),
            hnf.columns(),
            reduced.u.clone(),
            reduced.v.clone(),
        )?;
        Some((admitted, reduced))
    }

    /// Run the class's step count for `schedule`.
    pub fn reduce(self, schedule: Schedule) -> Reduced<W> {
        let steps = self.steps.count(schedule);
        self.reduce_steps(steps)
    }

    fn reduce_steps(mut self, steps: usize) -> Reduced<W> {
        for _ in 0..steps {
            self.step();
        }
        let Gram { a, c, h: [h0, h1] } = self.gram;
        let mut form = [
            a.shr(DOUBLED_DIAGONAL_BITS),
            c.shr(DOUBLED_DIAGONAL_BITS),
            h0,
            h1,
        ];
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

/// A reduced pair and its norm form. Every value is marked if any
/// check or any arithmetic failed.
pub struct Reduced<const W: usize> {
    /// First vector of the reduced pair.
    pub u: Vector<W>,
    /// Second vector of the reduced pair.
    pub v: Vector<W>,
    /// F00, F22, F20, F21. The second column of each Gaussian pair is its i-rotation.
    pub form: [Scalar<W>; 4],
}

fn positive_part(x: &Word) -> Word {
    Word::select(x, &Word::zero(), &x.is_negative())
}

fn quotient<const W: usize>(g: &Gram<W>) -> ([Word; 2], Word) {
    let bits = g.a.bit_length();
    let k = positive_part(&bits.sub(&Word::from_u64(QUOTIENT_TOP_BITS)));
    let [b0, b1] = g.h.each_ref().map(Scalar::bit_length);
    let largest = Word::select(&b0, &b1, &b0.ct_lt(&b1));
    let shift = positive_part(&largest.sub(&bits).sub(&Word::from_u64(QUOTIENT_WINDOW)));
    let big = Word::zero().ct_lt(&shift);
    let divisor = g.a.shr_secret(&k).resize::<1>();
    let align = k.add(&shift);
    let m = g.h.each_ref().map(|h| {
        let top = h.abs().shr_secret(&align).resize::<2>().div_word(&divisor);
        let signed = |x: &Word| Word::select(x, &x.neg(), &h.is_negative());
        let estimate = signed(&top);
        // X = 2h + A - 2mA. Decrement when X < 0, increment when X >= 2A.
        let x = h.sub(&g.a.mul_word(&estimate)).shl(1).add(&g.a);
        let lower = x.is_negative();
        let upper = x.ct_lt(&g.a.shl(1)).not();
        let corrected = estimate
            .sub(&Word::select(&Word::zero(), &Word::one(), &lower))
            .add(&Word::select(&Word::zero(), &Word::one(), &upper));
        let undershoot = signed(&positive_part(&top.sub(&Word::one())));
        Word::select(&corrected, &undershoot, &big)
    });
    (m, shift)
}

/// Fixture support: the same constructor, step and form conversion on both half ties.
#[doc(hidden)]
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
