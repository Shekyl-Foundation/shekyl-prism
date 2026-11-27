// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

use crate::reducer::{self, Basis, Bounds, Class, Reduced, ReductionSet, Scalar, Schedule, Vector};
use prism_int::{Int, SecretError, SecretInt};
use shekyl_prism::hazmat::ideal;
use std::hint::black_box;
use std::time::Instant;

#[cfg(test)]
#[path = "intersection_oracle.rs"]
mod intersection_oracle;

#[path = "../../tests/support/signer_fixtures.rs"]
mod signer_fixtures;
use signer_fixtures::check_bit;

fn check_invalid<const W: usize>(value: &Scalar<W>) {
    assert_eq!(value.declassify().unwrap_err(), SecretError::Overflowed);
}

fn magnitude(hex: &str) -> Vec<u8> {
    hex.trim_start_matches('-')
        .as_bytes()
        .rchunks(2)
        .map(|chunk| u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap())
        .collect()
}

fn scalar<const W: usize>(hex: &str) -> Scalar<W> {
    let x = SecretInt::from_le_bytes(&magnitude(hex));
    if hex.starts_with('-') {
        x.neg()
    } else {
        x
    }
}

struct Fixture<const W: usize> {
    class: Class,
    t: Scalar<W>,
    u: Vector<W>,
    v: Vector<W>,
    expected: Vec<Scalar<W>>,
}

impl<const W: usize> Fixture<W> {
    fn parse(line: &str) -> Self {
        let mut fields = line.split_whitespace();
        let token = fields.next().unwrap();
        let class = Class::from_token(token).unwrap_or_else(|| panic!("unknown class {token}"));
        let mut values = fields.map(scalar);
        let t = values.next().unwrap();
        let u = std::array::from_fn(|_| values.next().unwrap());
        let v = std::array::from_fn(|_| values.next().unwrap());
        let expected: Vec<_> = values.collect();
        assert_eq!(expected.len(), 12);
        Self {
            class,
            t,
            u,
            v,
            expected,
        }
    }

    fn input<const D: usize>(&self, bounds: &Bounds<W, D>) -> Basis<W> {
        Basis::from_intersection(&self.t, self.u.clone(), self.v.clone(), self.class, bounds)
    }

    fn check(&self, result: &Reduced<W>) {
        for (x, want) in result
            .u
            .iter()
            .chain(&result.v)
            .chain(&result.form)
            .zip(&self.expected)
        {
            check_bit(x.ct_eq(want), true);
        }
    }
}

fn check_ideal<S: ReductionSet<W, D>, const W: usize, const D: usize>(
    line: &str,
    expected: &Fixture<W>,
    bounds: &Bounds<W, D>,
) {
    let mut fields = line.split_whitespace();
    let mut values = fields.by_ref().take(11).map(scalar);
    let n: Scalar<W> = values.next().unwrap();
    let ideal_root = [values.next().unwrap(), values.next().unwrap()];
    let u: Vector<W> = std::array::from_fn(|_| values.next().unwrap());
    let v: Vector<W> = std::array::from_fn(|_| values.next().unwrap());
    let mut challenge = magnitude(fields.next().unwrap());
    challenge.resize((S::CHALLENGE_BITS as usize).div_ceil(8), 0);
    let challenge_root = std::array::from_fn(|_| scalar(fields.next().unwrap()));
    assert!(fields.next().is_none());
    let z = Scalar::zero;
    let hnf = ideal::GenericHnf::from_norm_and_root(&n, &ideal_root);
    let (generated_key, generated) =
        Basis::secret_candidate::<S, D>(&n, &ideal_root, bounds).expect("admitted candidate");
    for (got, want) in generated
        .u
        .iter()
        .chain(&generated.v)
        .zip(u.iter().chain(&v))
    {
        check_bit(got.ct_eq(want), true);
    }
    let key = ideal::GuardedIdeal::<S, W, D>::admit(n.clone(), hnf.columns(), u.clone(), v.clone())
        .expect("imported ideal");
    let intersection = key.intersection(&challenge, &challenge_root).unwrap();
    let from_generation = generated_key
        .intersection(&challenge, &challenge_root)
        .unwrap();
    for (got, want) in from_generation
        .u
        .iter()
        .chain(&from_generation.v)
        .chain([&from_generation.norm])
        .zip(
            intersection
                .u
                .iter()
                .chain(&intersection.v)
                .chain([&intersection.norm]),
        )
    {
        check_bit(got.ct_eq(want), true);
    }
    for (got, want) in intersection
        .u
        .iter()
        .chain(&intersection.v)
        .chain([&intersection.norm])
        .zip(expected.u.iter().chain(&expected.v).chain([&expected.t]))
    {
        check_bit(got.ct_eq(want), true);
    }
    let result = Basis::from_intersection(
        &intersection.norm,
        intersection.u,
        intersection.v,
        Class::Guarded,
        bounds,
    )
    .reduce(Schedule::Reviewed);
    expected.check(&result);
    assert!(key.intersection(&[], &challenge_root).is_none());
    let invalid = key.intersection(&challenge, &[z(), z()]).unwrap();
    for value in invalid.u.iter().chain(&invalid.v).chain([&invalid.norm]) {
        check_invalid(value);
    }
    let admit = |columns: &[Vector<W>; 4], a, b| {
        ideal::GuardedIdeal::<S, W, D>::admit(n.clone(), columns, a, b)
    };
    // Unimodular shear within the same ideal preserves its HNF and index,
    // but takes the second generator beyond the guard.
    let past_guard = ideal::geometry::GUARD_BITS + 1;
    let long = std::array::from_fn(|i| v[i].add(&n.mul(&u[i]).shl(past_guard)));
    assert!(admit(hnf.columns(), u.clone(), long).is_none());
    // Shearing the ambient O0 numerator lattice by z1 += 2*z2 keeps its
    // index, parity and Gaussian span. The new span fails left closure.
    let step = ideal::geometry::ORDER_DENOMINATOR_BITS;
    let shear = |w: &Vector<W>| {
        [
            w[0].add(&w[2].shl(step)),
            w[1].add(&w[3].shl(step)),
            w[2].clone(),
            w[3].clone(),
        ]
    };
    let mut unstable = hnf.columns().clone();
    let pivot = hnf.pivot().clone();
    let bumped = unstable[2][0].add(&Scalar::from_u64(1u64 << step));
    let bumped = Scalar::select(&bumped.sub(&pivot), &bumped, &bumped.ct_lt(&pivot));
    unstable[2][0] = bumped.clone();
    unstable[3][1] = bumped;
    assert!(admit(&unstable, shear(&u), shear(&v)).is_none());
    let mut wrong = hnf.columns().clone();
    wrong[2][0] = wrong[2][0].add(&Scalar::one());
    assert!(admit(&wrong, u.clone(), v.clone()).is_none());
    assert!(admit(hnf.columns(), u.clone(), u).is_none());
    // One past the exclusive Q4 limit. Generation and import share the refusal.
    let prime = ideal::field_prime::<S, W, D>().expect("field prime");
    let excessive = Scalar::from_public(
        &Int::one()
            .shl(ideal::geometry::secret_norm_bits(prime.bit_length()))
            .unwrap()
            .add(&Int::one())
            .unwrap(),
    );
    let excessive_hnf = ideal::GenericHnf::from_norm_and_root(&excessive, &ideal_root);
    let (pivot_column, root_column) = excessive_hnf.generators();
    assert!(ideal::GuardedIdeal::<S, W, D>::admit(
        excessive.clone(),
        excessive_hnf.columns(),
        pivot_column,
        root_column,
    )
    .is_none());
    assert!(Basis::secret_candidate::<S, D>(&excessive, &ideal_root, bounds).is_none());
}

fn check_ties<S: ReductionSet<W, D>, const W: usize, const D: usize>() {
    for steps in [1, 2, 3] {
        let result = reducer::tie_fixture::<S, W, D>(steps);
        let first = [4, -1, 1, 0];
        let (second, form) = if steps == 1 {
            ([10, 0, 0, 0], [1, 5, 4, 1])
        } else {
            ([1, -2, -2, -1], [1, 1, 0, -1])
        };
        for (got, expected) in result
            .u
            .iter()
            .chain(&result.v)
            .chain(&result.form)
            .zip(first.into_iter().chain(second).chain(form))
        {
            check_bit(got.ct_eq(&Scalar::from_i64(expected)), true);
        }
        let [a, c, real, imag] = &result.form;
        let mut terminal = c.ct_lt(a).not();
        for h in [real, imag] {
            terminal = terminal.and(&h.ct_lt(&a.neg()).not()).and(&h.ct_lt(a));
        }
        check_bit(terminal, steps >= 2);
    }
}

const SCHEDULES: [Schedule; 2] = [Schedule::Reviewed, Schedule::Pr45Baseline];

/// Smoke-bench samples per schedule. Odd, so each schedule leads the pair
/// once more than it follows across the whole run. This is not the leakage
/// measurement; that one is [`contrasts`].
const SMOKE_SAMPLES: usize = 41;

/// Repeats per fixture contrast. Two is the smallest sample whose standard
/// deviation is defined. The published run passes a larger count.
const MIN_CONTRAST_REPEATS: usize = 2;

/// First row of the class in corpus order. The same index as
/// `REFERENCE_INDEX` in `scripts/stage2c_fixtures.py`.
const REFERENCE_INDEX: usize = 0;

#[derive(Clone, Copy)]
pub enum Mode {
    Check,
    Bench,
    Timing { repeats: usize },
}

pub fn mode() -> Mode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => Mode::Check,
        [mode] if mode == "bench" => Mode::Bench,
        [mode, repeats] if mode == "timing" => {
            let repeats = repeats.parse::<usize>().expect("repeat count");
            assert!(
                repeats >= MIN_CONTRAST_REPEATS,
                "a contrast needs at least {MIN_CONTRAST_REPEATS} repeats"
            );
            Mode::Timing { repeats }
        }
        _ => panic!("expected no arguments, bench, or timing REPEATS"),
    }
}

/// Even repeats run the reference first. The summarizer rejects a row that
/// breaks this, so the file cannot claim an order the binary did not run.
fn reference_runs_first(repeat: usize) -> bool {
    repeat.is_multiple_of(2)
}

/// Nanoseconds to admit `case`, reduce it on `schedule`, and drop the inputs
/// and the result. Cloning into the black-boxed tuple happens before the
/// clock starts.
fn time_reduction<const W: usize, const D: usize>(
    case: &Fixture<W>,
    bounds: &Bounds<W, D>,
    schedule: Schedule,
) -> u128 {
    let input = black_box((case.t.clone(), case.u.clone(), case.v.clone()));
    let started = Instant::now();
    let (norm, u, v) = input;
    drop(black_box(
        Basis::from_intersection(&norm, u, v, case.class, bounds).reduce(schedule),
    ));
    drop(norm);
    started.elapsed().as_nanos()
}

fn of_class<const W: usize>(fixtures: &[Fixture<W>], class: Class) -> Vec<&Fixture<W>> {
    fixtures
        .iter()
        .filter(|fixture| fixture.class == class)
        .collect()
}

/// One paired difference per other fixture, repeated `repeats` times.
///
/// The pair is always the reference row against one later row of the same
/// class, schedule, and width. Repeating a four-fixture mixture and calling
/// the draw count an independent sample is a different experiment; this is
/// not that experiment.
fn contrasts<const W: usize, const D: usize>(
    name: &str,
    cases: &[&Fixture<W>],
    bounds: &Bounds<W, D>,
    repeats: usize,
) {
    assert!(
        cases.len() > REFERENCE_INDEX + 1,
        "a contrast needs the reference and another fixture"
    );
    let reference = cases[REFERENCE_INDEX];
    for case in cases {
        black_box(time_reduction(case, bounds, Schedule::Reviewed));
    }
    for (index, other) in cases.iter().enumerate().skip(REFERENCE_INDEX + 1) {
        for repeat in 0..repeats {
            let reference_first = reference_runs_first(repeat);
            let (first, second) = if reference_first {
                (reference, *other)
            } else {
                (*other, reference)
            };
            let first_ns = time_reduction(first, bounds, Schedule::Reviewed);
            let second_ns = time_reduction(second, bounds, Schedule::Reviewed);
            let (reference_ns, other_ns) = if reference_first {
                (first_ns, second_ns)
            } else {
                (second_ns, first_ns)
            };
            println!(
                "timing,{name},{},{},{index},{repeat},{reference_first},{reference_ns},{other_ns}",
                reference.class.token(),
                Schedule::Reviewed.token(),
            );
        }
    }
}

pub fn run<S: ReductionSet<W, D>, const W: usize, const D: usize>(corpus: &str, mode: Mode) {
    check_ties::<S, W, D>();
    let bounds = Bounds::new::<S>();
    let fixtures: Vec<_> = corpus.lines().map(Fixture::parse).collect();
    let ideals = S::IDEAL_CORPUS;
    let guarded: Vec<_> = fixtures
        .iter()
        .filter(|f| f.class == Class::Guarded)
        .collect();
    assert_eq!(guarded.len(), 6);
    assert_eq!(ideals.lines().count(), guarded.len());
    for (line, expected) in ideals.lines().zip(guarded) {
        check_ideal::<S, _, _>(line, expected, &bounds);
    }
    println!(
        "stage2d {} secret_steps={} guarded_intersections={}",
        S::NAME,
        S::SECRET_IDEAL_STEPS,
        ideals.lines().count()
    );
    for f in &fixtures {
        for schedule in SCHEDULES {
            f.check(&f.input(&bounds).reduce(schedule));
        }
    }
    for class in [Class::Guarded, Class::Generic] {
        let f = fixtures.iter().find(|f| f.class == class).unwrap();
        let mut wrong_parity = f.v.clone();
        wrong_parity[1] = wrong_parity[1].add(&Scalar::one());
        for (norm, v) in [
            (Scalar::zero(), f.v.clone()),
            (f.t.neg(), f.v.clone()),
            (f.t.shl(1), f.v.clone()),
            (f.t.clone(), wrong_parity),
            (f.t.clone(), f.v.each_ref().map(|x| x.shl(1))),
        ] {
            let invalid = Basis::from_intersection(&norm, f.u.clone(), v, class, &bounds)
                .reduce(Schedule::Reviewed);
            for x in invalid.u.iter().chain(&invalid.v).chain(&invalid.form) {
                check_invalid(x);
            }
        }
    }
    // CI compares the schedules exercised above to the independent count oracle.
    println!(
        "stage2c {} limbs={} guarded={} generic={} bases={}",
        S::NAME,
        S::LIMBS,
        bounds.steps(Class::Guarded).count(Schedule::Reviewed),
        bounds.steps(Class::Generic).count(Schedule::Reviewed),
        fixtures.len()
    );
    match mode {
        Mode::Check => {}
        Mode::Timing { repeats } => {
            for class in [Class::Guarded, Class::Generic] {
                contrasts(S::NAME, &of_class(&fixtures, class), &bounds, repeats);
            }
        }
        Mode::Bench => {
            // Paired smoke measurement on committed inputs. Alternate which
            // schedule runs first so a warm cache does not always favor the
            // shorter one. The interval is the same `time_reduction` as `contrasts`.
            for class in [Class::Guarded, Class::Generic] {
                let cases = of_class(&fixtures, class);
                let mut times = [Vec::new(), Vec::new()];
                for i in 0..SMOKE_SAMPLES {
                    let fixture = cases[i % cases.len()];
                    for j in [i % 2, 1 - i % 2] {
                        times[j].push(time_reduction(fixture, &bounds, SCHEDULES[j]));
                    }
                }
                for (schedule, mut samples) in SCHEDULES.into_iter().zip(times) {
                    let steps = bounds.steps(class).count(schedule);
                    samples.sort();
                    let median = std::time::Duration::from_nanos(
                        u64::try_from(samples[samples.len() / 2]).expect("sample fits in u64"),
                    );
                    println!(
                        "{} {} {}, {steps} steps, median {median:?} including setup and drop ({} samples)",
                        S::NAME,
                        class.token(),
                        schedule.token(),
                        samples.len()
                    );
                }
            }
        }
    }
}
