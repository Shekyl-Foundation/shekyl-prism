// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Field arithmetic at `p324` against Sage.
//!
//! `testdata/p324_field.tsv` is written by `scripts/sage/p324_fixture.sage`.
//! It holds every result in full for a grid of edge values, and for 10,000
//! random rows one SHA3-256 digest for each operation in each field. The
//! random inputs are not in the file: they come from a small generator
//! that the script and this test both implement, so the same 60,000 field
//! elements are drawn on both sides.
//!
//! The test is here and not in `prism-field` because it needs a hash, and
//! that crate depends on none.
//!
//! Every operation is called through the [`Field`] trait, by name. The
//! field types also have inherent methods of the same names with other
//! signatures, and the verifier uses the trait.

use prism_field::{Field, ParameterSet, P324};
use sha3::{Digest, Sha3_256};

type Set = P324;
type Base = <Set as ParameterSet>::Base;
type Ext = <Set as ParameterSet>::Ext;

const TABLE: &str = include_str!("../testdata/p324_field.tsv");
/// The script's seed.
const SEED: u64 = 0x5052_4953_4D33_3234;
const OPS: [&str; 7] = [
    "add",
    "sub",
    "mul",
    "square",
    "inv",
    "is_square",
    "sqrt_of_square",
];

fn hex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "odd hex length");
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

/// The generator of the script: SplitMix64.
struct Stream(u64);

impl Stream {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform residue, drawn as the script draws it: `FP_LEN` bytes with
    /// the spare bits of the top byte cleared, again while the decoder
    /// refuses the value as not below `p`.
    fn element(&mut self) -> Base {
        let spare = 8 * Set::FP_LEN as u32 - bits_of_p();
        loop {
            let mut raw = Vec::with_capacity(Set::FP_LEN + 8);
            while raw.len() < Set::FP_LEN {
                raw.extend_from_slice(&self.next().to_le_bytes());
            }
            raw.truncate(Set::FP_LEN);
            *raw.last_mut().expect("nonempty") &= u8::MAX >> spare;
            if let Some(value) = Set::base_decode(&raw) {
                return value;
            }
        }
    }

    fn ext(&mut self) -> Ext {
        let re = self.element();
        let im = self.element();
        ext(re, im)
    }
}

/// `p = f·2^e − 1` has `e` bits plus the bits of `f`, since `f·2^e` is not
/// a power of two for an odd `f > 1`.
fn bits_of_p() -> u32 {
    Set::E + Set::COFACTOR.bits()
}

fn ext(re: Base, im: Base) -> Ext {
    Field::add(
        Set::ext_from_base(re),
        Field::mul(Set::ext_i(), Set::ext_from_base(im)),
    )
}

/// Compare two little-endian encodings as integers.
fn less(left: &[u8], right: &[u8]) -> bool {
    left.iter().rev().lt(right.iter().rev())
}

/// What the script calls the canonical root of a square in `F_p`: of the
/// two roots, the smaller integer.
fn canonical_root(square: Base) -> Vec<u8> {
    let root = Field::sqrt(square).expect("a square has a root");
    assert!(
        Field::same(Field::square(root), square),
        "the root squares back"
    );
    let (plus, minus) = (
        Set::base_encode(root),
        Set::base_encode(Field::negate(root)),
    );
    if less(&minus, &plus) {
        minus
    } else {
        plus
    }
}

/// In `F_p²`: the root whose `(re, im)` is smaller, `re` compared first.
fn canonical_root_ext(square: Ext) -> Vec<u8> {
    let root = Field::sqrt(square).expect("a square has a root");
    assert!(
        Field::same(Field::square(root), square),
        "the root squares back"
    );
    let (plus, minus) = (Set::ext_encode(root), Set::ext_encode(Field::negate(root)));
    let n = Set::FP_LEN;
    let minus_first = if minus[..n] == plus[..n] {
        less(&minus[n..], &plus[n..])
    } else {
        less(&minus[..n], &plus[..n])
    };
    if minus_first {
        minus
    } else {
        plus
    }
}

/// The seven results of one row in `F_p`, in the order of [`OPS`].
fn base_results(a: Base, b: Base) -> [Vec<u8>; 7] {
    [
        Set::base_encode(Field::add(a, b)),
        Set::base_encode(Field::sub(a, b)),
        Set::base_encode(Field::mul(a, b)),
        Set::base_encode(Field::square(a)),
        Set::base_encode(Field::inv(a).unwrap_or_else(<Base as Field>::zero)),
        vec![u8::from(Field::is_square(a))],
        canonical_root(Field::square(a)),
    ]
}

fn ext_results(x: Ext, y: Ext) -> [Vec<u8>; 7] {
    [
        Set::ext_encode(Field::add(x, y)),
        Set::ext_encode(Field::sub(x, y)),
        Set::ext_encode(Field::mul(x, y)),
        Set::ext_encode(Field::square(x)),
        Set::ext_encode(Field::inv(x).unwrap_or_else(<Ext as Field>::zero)),
        vec![u8::from(Field::is_square(x))],
        canonical_root_ext(Field::square(x)),
    ]
}

fn rows(kind: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    TABLE
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .filter(move |fields| fields[0] == kind)
}

#[test]
fn the_edge_values_match_sage_in_the_base_field() {
    let mut seen = 0;
    for fields in rows("BASE") {
        assert_eq!(fields.len(), 10);
        let a = Set::base_decode(&hex(fields[1])).expect("a is canonical");
        let b = Set::base_decode(&hex(fields[2])).expect("b is canonical");
        let got = base_results(a, b);
        for (index, op) in OPS.iter().enumerate() {
            assert_eq!(
                got[index],
                hex(fields[3 + index]),
                "{op} of {} and {}",
                fields[1],
                fields[2]
            );
        }
        // Zero has no inverse, and the file writes zero there.
        assert_eq!(Field::inv(a).is_none(), Field::is_zero(a));
        seen += 1;
    }
    assert_eq!(seen, 121, "eleven edge values, every ordered pair");
}

#[test]
fn the_edge_values_match_sage_in_the_extension_field() {
    let mut seen = 0;
    for fields in rows("EXT") {
        assert_eq!(fields.len(), 10);
        let x = Set::ext_decode(&hex(fields[1])).expect("x is canonical");
        let y = Set::ext_decode(&hex(fields[2])).expect("y is canonical");
        let got = ext_results(x, y);
        for (index, op) in OPS.iter().enumerate() {
            assert_eq!(
                got[index],
                hex(fields[3 + index]),
                "{op} of {} and {}",
                fields[1],
                fields[2]
            );
        }
        seen += 1;
    }
    assert_eq!(seen, 63, "nine values against seven");
}

/// Ten thousand random rows in each field, one digest for each operation.
/// A wrong result anywhere changes the digest of that operation alone, so
/// a failure names the operation.
#[test]
fn ten_thousand_random_rows_match_sage_in_both_fields() {
    let expected: Vec<(String, String, usize, String)> = rows("RANDOM")
        .map(|fields| {
            assert_eq!(fields.len(), 5);
            (
                fields[1].to_string(),
                fields[2].to_string(),
                fields[3].parse().expect("a row count"),
                fields[4].to_string(),
            )
        })
        .collect();
    assert_eq!(expected.len(), 14, "seven operations in two fields");
    let count = expected[0].2;
    assert!(count >= 10_000, "at least ten thousand rows");
    assert!(expected.iter().all(|row| row.2 == count));

    let mut stream = Stream(SEED);
    let mut base: [Sha3_256; 7] = core::array::from_fn(|_| Sha3_256::new());
    let mut extension: [Sha3_256; 7] = core::array::from_fn(|_| Sha3_256::new());
    for _ in 0..count {
        let (a, b) = (stream.element(), stream.element());
        let (x, y) = (stream.ext(), stream.ext());
        for (hash, result) in base.iter_mut().zip(base_results(a, b)) {
            hash.update(&result);
        }
        for (hash, result) in extension.iter_mut().zip(ext_results(x, y)) {
            hash.update(&result);
        }
    }
    let digest = |hash: Sha3_256| -> String {
        hash.finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    };
    let got: Vec<(String, String, String)> = base
        .into_iter()
        .zip(OPS)
        .map(|(hash, op)| ("base".to_string(), op.to_string(), digest(hash)))
        .chain(
            extension
                .into_iter()
                .zip(OPS)
                .map(|(hash, op)| ("ext".to_string(), op.to_string(), digest(hash))),
        )
        .collect();
    for ((field, op, _, want), (got_field, got_op, got_digest)) in expected.iter().zip(&got) {
        assert_eq!((field, op), (got_field, got_op), "the file's row order");
        assert_eq!(got_digest, want, "{field} {op} over {count} rows");
    }
    // The generator is where the script left it, so both sides drew the
    // same number of values: no redraw happened on one side alone.
    let after: Vec<_> = rows("STREAM").collect();
    assert_eq!(after.len(), 1);
    assert_eq!(format!("{:016x}", stream.next()), after[0][2]);
}
