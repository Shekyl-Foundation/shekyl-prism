// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The committed vectors, `tests/data/vectors.tsv`, which
//! `scripts/int_vectors.py` writes from Python's integers: every operation
//! of both types at one and two limbs and at the five widths of the
//! parameter sets.
//!
//! The values here are test operands, so they are printed when a line
//! fails.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use prism_int::{Int, IntError, SecretError, SecretInt};

const VECTORS: &str = include_str!("data/vectors.tsv");
/// The widths the file has, and each operation must have lines at each.
const WIDTHS: [usize; 7] = [1, 2, 26, 33, 38, 49, 65];
/// The counts `unary` gives the low bits for, as the script's `LOW_COUNTS`.
const LOW_COUNTS: [u32; 6] = [0, 1, 2, 3, 63, 64];

/// An integer of the file: `0x…` after an optional `-`.
fn int<const N: usize>(text: &str) -> Int<N> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let digits = digits
        .strip_prefix("0x")
        .unwrap_or_else(|| panic!("not an integer: {text}"));
    let mut limbs = Vec::new();
    let mut end = digits.len();
    while end > 0 {
        let start = end.saturating_sub(16);
        limbs.push(u64::from_str_radix(&digits[start..end], 16).expect("hexadecimal"));
        end = start;
    }
    Int::from_le_limbs(&limbs, negative)
        .unwrap_or_else(|_| panic!("an operand of the file does not fit {N} limbs"))
}

/// What a refusal's name in the file is as an error.
fn refusal(name: &str) -> Option<IntError> {
    Some(match name {
        "overflow" => IntError::Overflow,
        "divzero" => IntError::DivisionByZero,
        "inexact" => IntError::Inexact,
        "negative" => IntError::Negative,
        "modulus" => IntError::Modulus,
        "count" => IntError::Count,
        _ => return None,
    })
}

/// `got` is what the file says: a value, or the refusal it names.
fn same<const N: usize>(got: Result<Int<N>, IntError>, want: &str, line: &str) {
    match (got, refusal(want)) {
        (Ok(value), None) => {
            assert!(
                value == int::<N>(want),
                "got {} on line: {line}",
                value.public_hex()
            );
        }
        (Err(error), Some(expected)) => assert_eq!(error, expected, "line: {line}"),
        (Ok(value), Some(expected)) => panic!(
            "got {}, want {expected:?}, on line: {line}",
            value.public_hex()
        ),
        (Err(error), None) => panic!("got {error:?}, want a value, on line: {line}"),
    }
}

/// The same for the secret type: a value that declassifies to the file's,
/// or, where the file says `overflow`, a mark that every exit refuses.
fn same_secret<const N: usize>(got: &SecretInt<N>, want: &str, line: &str) {
    match (got.declassify(), want) {
        (Err(error), "overflow") => {
            assert_eq!(error, SecretError::Overflowed, "line: {line}");
            let mut bytes = vec![0xa5u8; 8 * N];
            let mut negative = 0xa5u8;
            assert_eq!(
                got.export_le_bytes(&mut bytes, &mut negative),
                Err(SecretError::Overflowed)
            );
            assert!(
                bytes.iter().all(|&byte| byte == 0) && negative == 0,
                "line: {line}"
            );
            assert_eq!(got.ct_eq(got).declassify(), Err(SecretError::Overflowed));
        }
        (Ok(value), want) if refusal(want).is_none() => {
            // The line names the operands; the value computed from the
            // secret type is not printed, here as anywhere.
            assert!(
                value == int::<N>(want),
                "secret: a wrong value on line: {line}"
            );
        }
        (got, want) => panic!("secret: got {got:?}, want {want}, on line: {line}"),
    }
}

fn count(text: &str) -> u32 {
    text.parse().expect("a count")
}

fn secret<const N: usize>(value: &Int<N>) -> SecretInt<N> {
    SecretInt::from_public(value)
}

fn ring<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let (a, b) = (int::<N>(operands[0]), int::<N>(operands[1]));
    same(a.add(&b), results[0], line);
    same(a.sub(&b), results[1], line);
    same(a.mul(&b), results[2], line);
    let order = match results[3] {
        "-1" => Ordering::Less,
        "0" => Ordering::Equal,
        "1" => Ordering::Greater,
        other => panic!("not an order: {other}"),
    };
    assert_eq!(a.cmp(&b), order, "line: {line}");
    assert_eq!(a.partial_cmp(&b), Some(order), "line: {line}");
    assert_eq!(a == b, order == Ordering::Equal, "line: {line}");

    let (sa, sb) = (secret(&a), secret(&b));
    same_secret(&sa.add(&sb), results[0], line);
    same_secret(&sa.sub(&sb), results[1], line);
    same_secret(&sa.mul(&sb), results[2], line);
    assert_eq!(
        sa.ct_eq(&sb).declassify(),
        Ok(order == Ordering::Equal),
        "line: {line}"
    );
    assert_eq!(
        sa.ct_lt(&sb).declassify(),
        Ok(order == Ordering::Less),
        "line: {line}"
    );
    assert_eq!(
        sb.ct_lt(&sa).declassify(),
        Ok(order == Ordering::Greater),
        "line: {line}"
    );
    // Selection and swap, by a bit this pair gives.
    let less = sa.ct_lt(&sb);
    let smaller = SecretInt::select(&sb, &sa, &less).declassify().unwrap();
    assert!(smaller == *(&a).min(&b), "line: {line}");
    let (mut low, mut high) = (sa.clone(), sb.clone());
    SecretInt::swap(&mut low, &mut high, &sb.ct_lt(&sa));
    assert!(low.declassify().unwrap() == *(&a).min(&b), "line: {line}");
    assert!(high.declassify().unwrap() == *(&a).max(&b), "line: {line}");
}

fn unary<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let a = int::<N>(operands[0]);
    same(Ok(a.neg()), results[0], line);
    same_secret(&secret(&a).neg(), results[0], line);
    assert_eq!(a.is_negative(), results[1] == "1", "line: {line}");
    assert_eq!(a.is_zero(), results[2] == "1", "line: {line}");
    assert_eq!(a.is_odd(), results[3] == "1", "line: {line}");
    assert_eq!(a.bit_length(), count(results[4]), "line: {line}");
    assert_eq!(a.trailing_zeros(), results[5].parse().ok(), "line: {line}");
    assert_eq!(
        results[5] == "none",
        a.trailing_zeros().is_none(),
        "line: {line}"
    );
    for (index, &bits) in LOW_COUNTS.iter().enumerate() {
        let low: u64 = results[6 + 2 * index].parse().expect("low bits");
        let residue: u64 = results[7 + 2 * index].parse().expect("a residue");
        assert_eq!(a.low_bits(bits), Ok(low), "low {bits} on line: {line}");
        assert_eq!(
            a.residue_pow2(bits),
            Ok(residue),
            "residue {bits} on line: {line}"
        );
    }
    assert_eq!(a.low_bits(65), Err(IntError::Count));
    assert_eq!(a.residue_pow2(65), Err(IntError::Count));
    assert_eq!(a.public_hex(), operands[0], "line: {line}");
    // The value survives its own encodings.
    assert!(secret(&a).declassify().unwrap() == a);
    assert!(a.clone() == a);
}

fn shift<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let a = int::<N>(operands[0]);
    let by = count(operands[1]);
    same(a.shl(by), results[0], line);
    same(Ok(a.shr(by)), results[1], line);
    same_secret(&secret(&a).shl(by), results[0], line);
    same_secret(&secret(&a).shr(by), results[1], line);
}

fn bytes<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let a = int::<N>(operands[0]);
    let length = operands[1].parse::<usize>().expect("a length");
    let mut out = vec![0xa5u8; length];
    let mut secret_out = vec![0xa5u8; length];
    let mut negative = 0xa5u8;
    let exported = secret(&a).export_le_bytes(&mut secret_out, &mut negative);
    if results[0] == "overflow" {
        assert_eq!(
            a.to_le_bytes(&mut out),
            Err(IntError::Overflow),
            "line: {line}"
        );
        assert!(out.iter().all(|&byte| byte == 0), "line: {line}");
        assert_eq!(exported, Err(SecretError::DoesNotFit), "line: {line}");
        assert!(
            secret_out.iter().all(|&byte| byte == 0) && negative == 0,
            "line: {line}"
        );
        return;
    }
    let want: Vec<u8> = match results[0] {
        "-" => Vec::new(),
        hex => (0..hex.len() / 2)
            .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
            .collect(),
    };
    assert_eq!(a.to_le_bytes(&mut out), Ok(()), "line: {line}");
    assert_eq!(out, want, "line: {line}");
    assert_eq!(exported, Ok(()), "line: {line}");
    assert_eq!(secret_out, want, "line: {line}");
    assert_eq!(negative, u8::from(a.is_negative()), "line: {line}");
    // And back. The bytes are of the magnitude.
    let magnitude = if a.is_negative() { a.neg() } else { a.clone() };
    assert!(
        Int::<N>::from_le_bytes(&want).unwrap() == magnitude,
        "line: {line}"
    );
    assert!(
        SecretInt::<N>::from_le_bytes(&want).declassify().unwrap() == magnitude,
        "line: {line}"
    );
}

fn div<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let (a, b) = (int::<N>(operands[0]), int::<N>(operands[1]));
    if results[0] == "divzero" {
        assert_eq!(
            a.div_rem_trunc(&b).unwrap_err(),
            IntError::DivisionByZero,
            "line: {line}"
        );
        assert_eq!(
            a.div_rem_floor(&b).unwrap_err(),
            IntError::DivisionByZero,
            "line: {line}"
        );
        assert_eq!(
            a.rem_euclid(&b).unwrap_err(),
            IntError::DivisionByZero,
            "line: {line}"
        );
        assert_eq!(
            a.div_round(&b).unwrap_err(),
            IntError::DivisionByZero,
            "line: {line}"
        );
        assert_eq!(
            a.div_exact(&b).unwrap_err(),
            IntError::DivisionByZero,
            "line: {line}"
        );
        assert_eq!(
            a.rem_centred(&b).unwrap_err(),
            IntError::Modulus,
            "line: {line}"
        );
        return;
    }
    let (quotient, remainder) = a.div_rem_trunc(&b).unwrap();
    same(Ok(quotient), results[0], line);
    same(Ok(remainder), results[1], line);
    let (quotient, remainder) = a.div_rem_floor(&b).unwrap();
    same(Ok(quotient), results[2], line);
    same(Ok(remainder), results[3], line);
    same(a.rem_euclid(&b), results[4], line);
    same(a.div_round(&b), results[5], line);
    same(a.rem_centred(&b), results[6], line);
    same(a.div_exact(&b), results[7], line);
}

fn xgcd<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let (a, b) = (int::<N>(operands[0]), int::<N>(operands[1]));
    same(a.gcd(&b), results[0], line);
    same(b.gcd(&a), results[0], line);
    let (g, u, v) = a.xgcd(&b).unwrap();
    same(Ok(g), results[0], line);
    same(Ok(u), results[1], line);
    same(Ok(v), results[2], line);
}

fn inv<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let (a, m) = (int::<N>(operands[0]), int::<N>(operands[1]));
    match (a.inv_mod(&m), results[0]) {
        (Ok(None), "none") => {}
        (Ok(Some(value)), want) if want != "none" => same(Ok(value), want, line),
        (Err(error), want) => assert_eq!(Some(error), refusal(want), "line: {line}"),
        (got, want) => panic!("got {got:?}, want {want}, on line: {line}"),
    }
}

fn sqrt<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let a = int::<N>(operands[0]);
    same(a.sqrt_floor(), results[0], line);
    match (a.sqrt_exact(), results[1]) {
        (Ok(None), "none") => {}
        (Ok(Some(root)), want) if want != "none" => same(Ok(root), want, line),
        (got, want) => panic!("got {got:?}, want {want}, on line: {line}"),
    }
}

fn jacobi<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let (a, n) = (int::<N>(operands[0]), int::<N>(operands[1]));
    match a.jacobi(&n) {
        Ok(symbol) => assert_eq!(symbol.to_string(), results[0], "line: {line}"),
        Err(error) => assert_eq!(Some(error), refusal(results[0]), "line: {line}"),
    }
}

fn mulm<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let (a, b, m) = (
        int::<N>(operands[0]),
        int::<N>(operands[1]),
        int::<N>(operands[2]),
    );
    same(a.mul_mod(&b, &m), results[0], line);
}

fn powm<const N: usize>(operands: &[&str], results: &[&str], line: &str) {
    let (a, e, m) = (
        int::<N>(operands[0]),
        int::<N>(operands[1]),
        int::<N>(operands[2]),
    );
    same(a.pow_mod(&e, &m), results[0], line);
}

fn at_width<const N: usize>(operation: &str, operands: &[&str], results: &[&str], line: &str) {
    match operation {
        "ring" => ring::<N>(operands, results, line),
        "unary" => unary::<N>(operands, results, line),
        "shift" => shift::<N>(operands, results, line),
        "bytes" => bytes::<N>(operands, results, line),
        "div" => div::<N>(operands, results, line),
        "xgcd" => xgcd::<N>(operands, results, line),
        "inv" => inv::<N>(operands, results, line),
        "sqrt" => sqrt::<N>(operands, results, line),
        "jacobi" | "legendre" => jacobi::<N>(operands, results, line),
        "mulm" => mulm::<N>(operands, results, line),
        "powm" => powm::<N>(operands, results, line),
        other => panic!("the file has an operation this test does not know: {other}"),
    }
}

/// Runs every line of one operation, and requires lines at every width.
fn run(operation: &str) {
    let mut lines_at: BTreeMap<usize, usize> = BTreeMap::new();
    for line in VECTORS.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields[0] != operation {
            continue;
        }
        let width: usize = fields[1].parse().expect("a width");
        let equals = fields
            .iter()
            .position(|&field| field == "=")
            .expect("an equals sign");
        let (operands, results) = (&fields[2..equals], &fields[equals + 1..]);
        match width {
            1 => at_width::<1>(operation, operands, results, line),
            2 => at_width::<2>(operation, operands, results, line),
            26 => at_width::<26>(operation, operands, results, line),
            33 => at_width::<33>(operation, operands, results, line),
            38 => at_width::<38>(operation, operands, results, line),
            49 => at_width::<49>(operation, operands, results, line),
            65 => at_width::<65>(operation, operands, results, line),
            other => panic!("the file has a width this test does not know: {other}"),
        }
        *lines_at.entry(width).or_default() += 1;
    }
    for width in WIDTHS {
        assert!(
            lines_at.get(&width).copied().unwrap_or(0) > 0,
            "no line of {operation} at {width} limbs"
        );
    }
    // The file refuses at every width, in every operation that can
    // overflow: a sum, a difference, a product, a shift, the bytes.
    let refused_results: &[usize] = match operation {
        "ring" => &[0, 1, 2],
        "shift" | "bytes" => &[0],
        _ => &[],
    };
    for &index in refused_results {
        for width in WIDTHS {
            let refused = VECTORS
                .lines()
                .filter(|line| !line.starts_with('#'))
                .any(|line| {
                    let fields: Vec<&str> = line.split('\t').collect();
                    let equals = fields
                        .iter()
                        .position(|&field| field == "=")
                        .expect("an equals sign");
                    fields[0] == operation
                        && fields[1] == width.to_string()
                        && fields[equals + 1 + index] == "overflow"
                });
            assert!(
                refused,
                "no overflow in result {index} of {operation} at {width} limbs"
            );
        }
    }
}

macro_rules! operations {
    ($($name:ident),*) => {
        /// One test for each operation of the file.
        mod operation {
            $(
                #[test]
                fn $name() {
                    super::run(stringify!($name));
                }
            )*
        }

        /// Every operation the file has is one of the tests above.
        #[test]
        fn every_operation_of_the_file_is_run() {
            let known = ["stage", $(stringify!($name)),*];
            for line in VECTORS.lines().filter(|line| !line.starts_with('#')) {
                let operation = line.split('\t').next().unwrap();
                assert!(known.contains(&operation), "no test runs {operation}");
            }
        }
    };
}

operations!(ring, unary, shift, bytes, div, xgcd, inv, sqrt, jacobi, legendre, mulm, powm);

fn stage<const N: usize>(f: &[&str], line: &str) {
    let a = secret(&int::<N>(f[2]));
    let b = secret(&int::<N>(f[3]));
    let word = secret(&int::<1>(f[4]));
    let k = secret(&int::<1>(f[5]));
    same_secret(&a.shl_secret(&k), f[7], line);
    same_secret(&a.shr_secret(&k), f[8], line);
    same_secret(&a.bit_length(), f[9], line);
    same_secret(&a.mul_low(&b), f[10], line);
    same_secret(&a.mul_word(&word), f[11], line);
    same_secret(&a.truncate::<1>(), f[12], line);
    same_secret(&a.resize::<1>(), f[13], line);
}

#[test]
fn reduction_primitives_match_python() {
    let mut widths = std::collections::BTreeSet::new();
    for line in VECTORS.lines().filter(|s| s.starts_with("stage\t")) {
        let f: Vec<_> = line.split('\t').collect();
        let width: usize = f[1].parse().unwrap();
        widths.insert(width);
        match width {
            1 => stage::<1>(&f, line),
            2 => stage::<2>(&f, line),
            11 => stage::<11>(&f, line),
            14 => stage::<14>(&f, line),
            16 => stage::<16>(&f, line),
            21 => stage::<21>(&f, line),
            27 => stage::<27>(&f, line),
            22 => stage::<22>(&f, line),
            28 => stage::<28>(&f, line),
            32 => stage::<32>(&f, line),
            42 => stage::<42>(&f, line),
            54 => stage::<54>(&f, line),
            _ => panic!("unknown reducer width {width}"),
        }
    }
    assert_eq!(
        widths.into_iter().collect::<Vec<_>>(),
        [1, 2, 11, 14, 16, 21, 22, 27, 28, 32, 42, 54]
    );
}

#[test]
fn word_division_and_admission_marks() {
    for d in [0_u64, 1, 2, 17, 1 << 63, u64::MAX] {
        for hi in [0, 1, d.saturating_sub(1u64), d, u64::MAX] {
            for lo in [0, 1, 1 << 63, u64::MAX] {
                let n = u128::from(hi) << 64 | u128::from(lo);
                let a = SecretInt::<2>::from_le_bytes(&n.to_le_bytes());
                let b = SecretInt::from_u64(d);
                let expected = if hi >= d {
                    "overflow".into()
                } else {
                    format!("{:#x}", n / u128::from(d))
                };
                same_secret(&a.div_word(&b), &expected, "fixed division");
            }
        }
    }
    let one = SecretInt::<1>::one();
    let yes = one.is_odd();
    let no = yes.not();
    let mut invalid = one.clone();
    invalid.mark_if(&yes);
    let bad = invalid.is_negative();
    for bit in [bad.and(&no), yes.or(&bad), bad.not()] {
        let mut x = one.clone();
        x.mark_if(&bit);
        x.mark_if(&no);
        same_secret(&x, "overflow", "invalid predicate");
    }
    for x in [
        invalid.abs(),
        invalid.resize(),
        invalid.truncate(),
        invalid.bit_length(),
        one.mul_word(&invalid),
        invalid.mul_low(&one),
        one.shl_secret(&invalid),
        one.shr_secret(&invalid),
        invalid.resize::<2>().div_word(&one),
        one.resize::<2>().div_word(&one.neg()),
        one.neg().resize::<2>().div_word(&one),
    ] {
        same_secret(&x, "overflow", "sticky result");
    }
}
