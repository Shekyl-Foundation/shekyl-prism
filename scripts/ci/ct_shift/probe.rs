// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Release measurement executable for the native Rust reducer.
#![forbid(unsafe_code)]
#[allow(dead_code)] // The fixture-only half-tie entry is exercised by the native example.
#[path = "../../../shekyl-prism/examples/stage2c/reducer.rs"]
mod reducer;

#[cfg(stage2c_set = "p248")]
use prism_field::P248 as Selected;
#[cfg(stage2c_set = "p324")]
use prism_field::P324 as Selected;
#[cfg(stage2c_set = "p376")]
use prism_field::P376 as Selected;
#[cfg(stage2c_set = "p500")]
use prism_field::P500 as Selected;
#[cfg(stage2c_set = "p664")]
use prism_field::P664 as Selected;
use prism_int::SecretInt;
use reducer::ReductionSet;
use std::hint::black_box;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

const LIMBS: usize = Selected::LIMBS;
type Scalar = SecretInt<LIMBS>;

#[inline(never)]
fn shift_probe(input: u64, count: u64) {
    let x = Scalar::from_u64(input);
    let wide = black_box(x.shl(64 * (LIMBS as u32 - 2)).add(&x.shl(63)).add(&x));
    let k = SecretInt::from_u64(count);
    let left = wide.shl_secret(&k);
    let right = wide.shr_secret(&k);
    black_box((left, right, wide.bit_length()));
    let divisor = SecretInt::from_u64(input | 1);
    black_box(wide.truncate::<2>().div_word(&divisor));
}

fn read_scalar(bytes: &[u8]) -> Scalar {
    let (magnitude, sign) = bytes.split_at(LIMBS * 8);
    let n = Scalar::from_le_bytes(magnitude);
    let negative = SecretInt::<1>::from_u64(u64::from(sign[0])).is_odd();
    Scalar::select(&n, &n.neg(), &negative)
}

fn reducer_probe(encoded: &[u8], class: reducer::Class) {
    let mut values = encoded.chunks_exact(LIMBS * 8 + 1).map(read_scalar);
    let norm = values.next().unwrap();
    let basis = reducer::Basis::from_intersection(
        &norm,
        std::array::from_fn(|_| values.next().unwrap()),
        std::array::from_fn(|_| values.next().unwrap()),
        class,
        &reducer::Bounds::new::<Selected>(),
    );
    let output = basis.reduce(reducer::Schedule::Reviewed);
    black_box((output.u, output.v, output.form));
}

#[inline(never)]
fn leaky_control(count: u64) {
    if count & 1 == 0 {
        black_box(1_u64);
    } else {
        black_box(2_u64);
        black_box(3_u64);
    }
}

#[inline(never)]
fn probe_end() {
    black_box(0_u64);
}

// Memcheck's external monitor marks the actual buffer undefined before the
// consumer reads it. The probe exchanges only its address, length and an ack.
fn prepare(bytes: &mut [u8], resume: Option<&Path>) {
    if let Some(resume) = resume {
        println!("{:p} {}", bytes.as_ptr(), bytes.len());
        io::stdout().flush().unwrap();
        // Stay schedulable so vgdb can deliver its monitor command without
        // ptrace privileges. The parent creates this file after marking.
        while !resume.exists() {
            std::thread::sleep(Duration::from_millis(1));
        }
        black_box(bytes);
    }
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let resume = args.get(3).map(Path::new);
    match args[0].as_str() {
        "control" | "probe" => {
            let mut bytes = [0; 16];
            for (chunk, arg) in bytes.chunks_exact_mut(8).zip(&args[1..3]) {
                let value = u64::from_str_radix(arg.trim_start_matches("0x"), 16).unwrap();
                chunk.copy_from_slice(&value.to_le_bytes());
            }
            prepare(&mut bytes, resume);
            let count = u64::from_le_bytes(bytes[8..].try_into().unwrap());
            if args[0] == "control" {
                leaky_control(count);
            } else {
                shift_probe(u64::from_le_bytes(bytes[..8].try_into().unwrap()), count);
            }
            probe_end();
        }
        "reduce" => {
            let class = match args[2].as_str() {
                "prered-guard256" => reducer::Class::Guarded,
                "sign-generic" => reducer::Class::Generic,
                _ => panic!("unknown public class"),
            };
            let mut encoded = std::fs::read(&args[1]).unwrap();
            assert_eq!(encoded.len(), 9 * (LIMBS * 8 + 1));
            prepare(&mut encoded, resume);
            reducer_probe(&encoded, class);
        }
        _ => panic!("unknown probe"),
    }
}
