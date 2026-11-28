// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Release measurement executable for the native Rust reducer.
//!
//! One command per measurement. `constants` prints the instrument sizes
//! the Python runners must use. `probe` and `control` exercise secret
//! shifts. `reduce` is the Memcheck subject. `stack` and `canary` bracket
//! a call with the retention markers. `profile` reduces every input in a
//! concatenated basis file so one Callgrind run sees the whole class.
#![forbid(unsafe_code)]
// The reducer is the library's: the probe links `prism-ideal` and measures
// the code a signer would link.
use prism_ideal::reduce as reducer;

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
const LIMB_BYTES: usize = 8;
const SIGN_BYTES: usize = 1;
/// Norm, four coordinates of U, four of V. Matches `BASIS_INPUTS` in
/// `scripts/stage2c_fixtures.py`.
const BASIS_INPUTS: usize = 9;
const INPUT_STRIDE: usize = BASIS_INPUTS * (LIMBS * LIMB_BYTES + SIGN_BYTES);

/// Callee stack the retention scan paints.
///
/// The window is several times the deepest write of the five-set corpus.
/// A write that reaches the guard page fails the scan, so a wider set
/// cannot pass by overflowing the instrument silently.
const STACK_PAINT_BYTES: usize = 224 * 1024;
/// One target page. The scan treats a change in this suffix as an exhausted window.
const STACK_GUARD_BYTES: usize = 4 * 1024;
/// Fault the paint and the guard page before the markers. The guard page
/// is the probe's own slack for a few bytes of difference between the two
/// call sites, not a second budget.
const STACK_PRIME_BYTES: usize = STACK_PAINT_BYTES + STACK_GUARD_BYTES;
/// Leading bytes the canary copies into its frame. The Python canary plants
/// exactly these bytes; the scan must find every one of them.
const CANARY_BYTES: usize = 64;

const _: () = assert!(STACK_GUARD_BYTES < STACK_PAINT_BYTES);
const _: () = assert!(STACK_PRIME_BYTES == STACK_PAINT_BYTES + STACK_GUARD_BYTES);
const _: () = assert!(CANARY_BYTES % LIMB_BYTES == 0);
const _: () = assert!(CANARY_BYTES < INPUT_STRIDE);

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
    let (magnitude, sign) = bytes.split_at(LIMBS * LIMB_BYTES);
    let n = Scalar::from_le_bytes(magnitude);
    let negative = SecretInt::<1>::from_u64(u64::from(sign[0])).is_odd();
    Scalar::select(&n, &n.neg(), &negative)
}

#[inline(never)]
fn reducer_probe(
    encoded: &[u8],
    class: reducer::Class,
    bounds: &reducer::Bounds<LIMBS, { 2 * LIMBS }>,
) {
    let mut values = encoded
        .chunks_exact(LIMBS * LIMB_BYTES + SIGN_BYTES)
        .map(read_scalar);
    let norm = values.next().unwrap();
    let basis = reducer::Basis::from_intersection(
        &norm,
        std::array::from_fn(|_| values.next().unwrap()),
        std::array::from_fn(|_| values.next().unwrap()),
        class,
        bounds,
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

/// Touch each page of the paint window so the debugger's later write cannot
/// fault on a hole. The allocation dies with this frame; the pages stay mapped.
#[inline(never)]
fn prime_stack() {
    black_box(&mut [0u8; STACK_PRIME_BYTES]);
}

#[inline(never)]
fn stack_start() {
    black_box(1_u64);
}

/// Copy the probe's canary prefix onto this frame and return, leaving it there.
#[inline(never)]
fn retention_canary(bytes: &[u8]) {
    let mut copy = [0u8; CANARY_BYTES];
    copy.copy_from_slice(&bytes[..CANARY_BYTES]);
    black_box(&mut copy);
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

fn print_instruments() {
    println!("paint {STACK_PAINT_BYTES}");
    println!("guard {STACK_GUARD_BYTES}");
    println!("canary {CANARY_BYTES}");
    println!("stride {INPUT_STRIDE}");
}

struct BasisFile {
    class: reducer::Class,
    encoded: Vec<u8>,
}

fn class_from(token: &str) -> reducer::Class {
    reducer::Class::from_token(token).unwrap_or_else(|| panic!("unknown public class {token}"))
}

fn read_basis(path: &str, token: &str, count: usize) -> BasisFile {
    let encoded = std::fs::read(path).unwrap();
    assert_eq!(
        encoded.len(),
        count * INPUT_STRIDE,
        "basis file is not {count} inputs of {INPUT_STRIDE} bytes"
    );
    BasisFile {
        class: class_from(token),
        encoded,
    }
}

fn optional_resume(args: &[String]) -> Option<&Path> {
    args.get(3).map(Path::new)
}

fn run_shift(args: &[String]) {
    assert!(
        args.len() == 3 || args.len() == 4,
        "expected {mode} X K [RESUME]",
        mode = args[0]
    );
    let mut bytes = [0; 16];
    for (chunk, arg) in bytes.chunks_exact_mut(LIMB_BYTES).zip(&args[1..3]) {
        let value = u64::from_str_radix(arg.trim_start_matches("0x"), 16).unwrap();
        chunk.copy_from_slice(&value.to_le_bytes());
    }
    prepare(&mut bytes, optional_resume(args));
    let count = u64::from_le_bytes(bytes[LIMB_BYTES..].try_into().unwrap());
    if args[0] == "control" {
        leaky_control(count);
    } else {
        shift_probe(
            u64::from_le_bytes(bytes[..LIMB_BYTES].try_into().unwrap()),
            count,
        );
    }
    probe_end();
}

fn run_reduce(args: &[String]) {
    assert!(
        args.len() == 3 || args.len() == 4,
        "expected reduce FILE CLASS [RESUME]"
    );
    let mut basis = read_basis(&args[1], &args[2], 1);
    let bounds = reducer::Bounds::new::<Selected>();
    prepare(&mut basis.encoded, optional_resume(args));
    reducer_probe(&basis.encoded, basis.class, &bounds);
}

fn run_retain(args: &[String], canary: bool) {
    assert_eq!(args.len(), 3, "expected {} FILE CLASS", args[0]);
    let basis = read_basis(&args[1], &args[2], 1);
    // Bounds are public and live in this frame, above the painted window.
    let bounds = (!canary).then(|| reducer::Bounds::new::<Selected>());
    prime_stack();
    stack_start();
    if let Some(bounds) = bounds.as_ref() {
        reducer_probe(&basis.encoded, basis.class, bounds);
    } else {
        retention_canary(&basis.encoded);
    }
    probe_end();
}

fn run_profile(args: &[String]) {
    assert_eq!(args.len(), 4, "expected profile FILE CLASS COUNT");
    let count: usize = args[3].parse().expect("profile count");
    assert!(count > 0, "profile count");
    let basis = read_basis(&args[1], &args[2], count);
    let bounds = reducer::Bounds::new::<Selected>();
    for input in basis.encoded.chunks(INPUT_STRIDE) {
        reducer_probe(input, basis.class, &bounds);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        panic!("expected constants, control, probe, reduce, stack, canary, or profile");
    }
    match args[0].as_str() {
        "constants" => {
            assert_eq!(args.len(), 1, "constants takes no arguments");
            print_instruments();
        }
        "control" | "probe" => run_shift(&args),
        "reduce" => run_reduce(&args),
        "stack" => run_retain(&args, false),
        "canary" => run_retain(&args, true),
        "profile" => run_profile(&args),
        other => panic!("unknown probe {other}"),
    }
}
