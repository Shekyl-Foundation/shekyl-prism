// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Research harness for fixed-width O0 constructors, both returned vectors,
//! and both Gram representations. Input files come from check_native.py.

use cqp_xcheck::native::{normalized, raw, reduce, Gram};
use cqp_xcheck::sets::{from_limbs_signed, from_limbs_unsigned, to_limbs, SetInfo, SETS};
use cqp_xcheck::with_set;
use cqp_xcheck::zi::Vec4;
use num_bigint::BigInt;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

struct Fixture<const C: usize> {
    kind: String,
    t: [u64; C],
    x: [u64; C],
    y: [u64; C],
    expected: Vec<BigInt>,
}

impl<const C: usize> Fixture<C> {
    fn parse(line: &str) -> Self {
        let mut fields = line.split_whitespace();
        let kind = fields.next().unwrap().to_owned();
        let mut values = fields.map(|s| BigInt::parse_bytes(s.as_bytes(), 16).unwrap());
        let t = to_limbs(&values.next().unwrap());
        let x = to_limbs(&values.next().unwrap());
        let y = to_limbs(&values.next().unwrap());
        let expected: Vec<_> = values.collect();
        assert_eq!(expected.len(), 12);
        Self {
            kind,
            t,
            x,
            y,
            expected,
        }
    }

    fn check<const G: usize>(&self, result: (Vec4<C>, Vec4<C>, Gram<G>), scale: &BigInt) {
        let (u, v, gram) = result;
        for (limbs, value) in u.iter().chain(&v).zip(&self.expected[..8]) {
            assert_eq!(from_limbs_signed(limbs), *value);
        }
        for (limbs, value) in gram.iter().zip(&self.expected[8..]) {
            assert_eq!(from_limbs_signed(limbs), scale * value);
        }
    }
}

fn check<const C: usize, const W: usize, const P: usize>(
    r: &Fixture<C>,
    p: &[u64; P],
    old: usize,
    new: usize,
) {
    let initial = normalized::<C, W, P>(&r.t, &r.x, &r.y, p);
    let [a, c, h, k] = initial.map(|x| from_limbs_signed(&x));
    assert_eq!(a * c - &h * &h - &k * &k, from_limbs_unsigned(p));
    let unscaled = raw::<C, W, P>(&r.t, &r.x, &r.y, p);
    let scale = from_limbs_signed(&r.t) * 2;
    for (small, large) in initial.iter().zip(&unscaled) {
        assert_eq!(from_limbs_signed(small) * &scale, from_limbs_signed(large));
    }
    r.check(reduce(&r.t, &r.x, &r.y, unscaled, old), &scale);
    for steps in [old, new] {
        r.check(reduce(&r.t, &r.x, &r.y, initial, steps), &BigInt::from(1));
    }
}

fn timed(f: impl FnOnce()) -> u128 {
    let start = Instant::now();
    f();
    start.elapsed().as_nanos()
}

fn bench<const C: usize, const W: usize, const P: usize>(
    rows: &[Fixture<C>],
    p: &[u64; P],
    old: usize,
    new: usize,
    name: &str,
) {
    let honest: Vec<_> = rows.iter().filter(|r| r.kind == "honest").collect();
    assert!(!honest.is_empty());
    let mut samples: [Vec<u128>; 4] = Default::default();
    for i in 0..51 {
        let r = honest[i % honest.len()];
        for offset in 0..4 {
            let variant = (i + offset) % 4;
            let ns = timed(|| {
                if variant == 0 {
                    let g = raw::<C, W, P>(black_box(&r.t), &r.x, &r.y, p);
                    black_box(reduce(&r.t, &r.x, &r.y, g, old));
                } else {
                    let g = normalized::<C, W, P>(black_box(&r.t), &r.x, &r.y, p);
                    if variant == 3 {
                        black_box(g);
                    } else {
                        black_box(reduce(
                            &r.t,
                            &r.x,
                            &r.y,
                            g,
                            if variant == 1 { old } else { new },
                        ));
                    }
                }
            });
            if i >= 10 {
                samples[variant].push(ns);
            }
        }
    }
    for (label, mut values) in ["raw", "normalized", "normalized_short", "setup_only"]
        .into_iter()
        .zip(samples)
    {
        values.sort_unstable();
        println!(
            "{name},{label},{},{}",
            if label == "normalized_short" {
                new
            } else if label == "setup_only" {
                0
            } else {
                old
            },
            values[values.len() / 2]
        );
    }
}

#[cfg(feature = "memcheck")]
fn probe<const C: usize, const W: usize, const P: usize>(
    rows: &mut [Fixture<C>],
    p: &[u64; P],
    steps: usize,
    control: bool,
) {
    unsafe extern "C" {
        fn rlc_secret(p: *mut u64, bytes: usize);
    }
    for kind in ["honest", "crafted_q5"] {
        let r = rows
            .iter_mut()
            .find(|r| r.kind == kind)
            .expect("missing input class");
        for v in [&mut r.t, &mut r.x, &mut r.y] {
            // SAFETY: the existing Memcheck helper annotates an initialized array.
            unsafe { rlc_secret(v.as_mut_ptr(), std::mem::size_of_val(v)) };
        }
        if control && black_box(r.t[0]) & 1 != 0 {
            eprintln!("positive control");
        }
        let g = normalized::<C, W, P>(&r.t, &r.x, &r.y, p);
        black_box(reduce(&r.t, &r.x, &r.y, g, steps));
    }
}

fn run<const C: usize, const W: usize, const P: usize>(si: &SetInfo, args: &[String]) {
    let p = to_limbs::<P>(&si.p());
    let input =
        std::fs::read_to_string(Path::new(&args[3]).join(format!("{}.txt", si.name))).unwrap();
    let rows: Vec<Fixture<C>> = input.lines().map(Fixture::parse).collect();
    assert!(!rows.is_empty());
    let old = args[4].parse().unwrap();
    let new = args[5].parse().unwrap();
    match args[1].as_str() {
        "check" => {
            for r in &rows {
                check::<C, W, P>(r, &p, old, new);
            }
            println!("{}: {} pairs and forms", si.name, rows.len());
        }
        "bench" => bench::<C, W, P>(&rows, &p, old, new, si.name),
        #[cfg(feature = "memcheck")]
        "memcheck" => {
            let mut rows = rows;
            probe::<C, W, P>(
                &mut rows,
                &p,
                new,
                args.get(6).is_some_and(|x| x == "control"),
            );
        }
        _ => panic!("expected check, bench or memcheck"),
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        args.len() >= 6,
        "native_gram MODE SET VECTOR_DIR OLD_STEPS NEW_STEPS"
    );
    let si = SETS.iter().find(|s| s.name == args[2]).unwrap();
    with_set!(si.name, run, si, &args);
}
