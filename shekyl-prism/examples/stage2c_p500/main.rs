// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Native Stage 2c prototype on public oracle fixtures. No signing or key import API.
#![forbid(unsafe_code)]

mod reducer;
use prism_int::{SecretBit, SecretError, SecretInt};
use reducer::{Basis, Bounds, Class, Reduced, Scalar, Schedule, Vector};
use std::hint::black_box;
use std::time::Instant;

fn check_bit(bit: SecretBit, expected: bool) {
    assert_eq!(bit.declassify(), Ok(expected));
}

fn scalar(hex: &str) -> Scalar {
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

struct Fixture {
    class: Class,
    t: Scalar,
    u: Vector,
    v: Vector,
    expected: Vec<Scalar>,
}

impl Fixture {
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

    fn input(&self, bounds: &Bounds) -> Basis {
        Basis::from_intersection(&self.t, self.u.clone(), self.v.clone(), self.class, bounds)
    }

    fn check(&self, result: &Reduced) {
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

fn check_ties() {
    for steps in [1, 2, 3] {
        let result = reducer::tie_fixture(steps);
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

fn main() {
    check_ties();
    let bench = std::env::args().nth(1).as_deref() == Some("bench");
    let bounds = Bounds::p500();
    let fixtures: Vec<_> = include_str!("vectors.txt")
        .lines()
        .map(Fixture::parse)
        .collect();
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
    println!("{} P500 bases matched at both schedules. Both classes refused all five invalid inputs on every output.", fixtures.len());
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
                let steps = class.steps(schedule);
                samples.sort();
                println!(
                    "{class:?}, {steps} steps, median {:?} including setup and drop ({} samples)",
                    samples[samples.len() / 2],
                    samples.len()
                );
            }
        }
    }
}
