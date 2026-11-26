// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

use crate::reducer::{self, Basis, Bounds, Class, Reduced, ReductionSet, Scalar, Schedule, Vector};
use prism_int::{SecretBit, SecretError, SecretInt};
use std::hint::black_box;
use std::time::Instant;

fn check_bit(bit: SecretBit, expected: bool) {
    assert_eq!(bit.declassify(), Ok(expected));
}

fn scalar<const W: usize>(hex: &str) -> Scalar<W> {
    let digits = hex.trim_start_matches('-').as_bytes();
    let bytes: Vec<_> = digits
        .rchunks(2)
        .map(|chunk| u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap())
        .collect();
    let x = SecretInt::from_le_bytes(&bytes);
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
        let class = match fields.next().unwrap() {
            "prered-guard256" => Class::Guarded,
            "sign-generic" => Class::Generic,
            other => panic!("unknown class {other}"),
        };
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

pub fn run<S: ReductionSet<W, D>, const W: usize, const D: usize>(corpus: &str, bench: bool) {
    check_ties::<S, W, D>();
    let bounds = Bounds::new::<S>();
    let fixtures: Vec<_> = corpus.lines().map(Fixture::parse).collect();
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
                assert_eq!(x.declassify().unwrap_err(), SecretError::Overflowed);
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
    if bench {
        // Paired smoke measurement on committed inputs. Alternate the order
        // so a warm cache does not always favor the shorter schedule.
        for class in [Class::Guarded, Class::Generic] {
            let cases: Vec<_> = fixtures.iter().filter(|f| f.class == class).collect();
            let mut times = [Vec::new(), Vec::new()];
            for i in 0..41 {
                let f = cases[i % cases.len()];
                for j in [i % 2, 1 - i % 2] {
                    let schedule = SCHEDULES[j];
                    let start = Instant::now();
                    drop(black_box(f.input(&bounds).reduce(schedule)));
                    times[j].push(start.elapsed());
                }
            }
            for (schedule, mut samples) in SCHEDULES.into_iter().zip(times) {
                let steps = bounds.steps(class).count(schedule);
                samples.sort();
                println!(
                    "{} {class:?} {schedule:?}, {steps} steps, median {:?} including setup and drop ({} samples)",
                    S::NAME, samples[samples.len() / 2],
                    samples.len()
                );
            }
        }
    }
}
