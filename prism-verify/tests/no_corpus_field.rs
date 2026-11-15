// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Field arithmetic against Sage, at a set that has no corpus.
//!
//! `testdata/<set>_field.tsv` is written by
//! `scripts/sage/no_corpus_fixture.sage`. It holds every result in full
//! for a grid of edge values, and for 10,000 random rows one SHA3-256
//! digest for each operation in each field. The random inputs are not in
//! the file: they come from a small generator that the script and this
//! test both implement, so the same 60,000 field elements are drawn on
//! both sides.
//!
//! The test is here and not in `prism-field` because it needs a hash, and
//! that crate depends on none.
//!
//! Every operation is called through the [`Field`] trait, by name. The
//! field types also have inherent methods of the same names with other
//! signatures, and the verifier uses the trait.
//!
//! The checks are generic over the set. Each set without a corpus runs
//! all of them, in a module of its own at the end of this file.

use prism_field::{Field, ParameterSet};
use prism_fixtures::no_corpus::WithoutCorpus;
use sha3::{Digest, Sha3_256};

/// The operations of one row, in the order the file writes them.
const OPS: [&str; 7] = [
    "add",
    "sub",
    "mul",
    "square",
    "inv",
    "is_square",
    "sqrt_of_square",
];

/// The edge values of the base field, and so the rows of every ordered
/// pair of them.
const BASE_EDGE_VALUES: usize = 11;
/// The extension-field edge rows: nine values against seven.
const EXT_EDGE_ROWS: usize = 9 * 7;
/// The least number of random rows a file may hash.
const RANDOM_ROWS: usize = 10_000;

fn hex(text: &str) -> Vec<u8> {
    prism_fixtures::hex::decode(text).expect("hex")
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
    fn element<L: ParameterSet>(&mut self) -> L::Base {
        let spare = 8 * L::FP_LEN as u32 - bits_of_p::<L>();
        loop {
            let mut raw = Vec::with_capacity(L::FP_LEN + 8);
            while raw.len() < L::FP_LEN {
                raw.extend_from_slice(&self.next().to_le_bytes());
            }
            raw.truncate(L::FP_LEN);
            *raw.last_mut().expect("nonempty") &= u8::MAX >> spare;
            if let Some(value) = L::base_decode(&raw) {
                return value;
            }
        }
    }

    fn ext<L: ParameterSet>(&mut self) -> L::Ext {
        let re = self.element::<L>();
        let im = self.element::<L>();
        ext::<L>(re, im)
    }
}

/// `p = f·2^e − 1` has `e` bits plus the bits of `f`, since `f·2^e` is not
/// a power of two for an odd `f > 1`.
fn bits_of_p<L: ParameterSet>() -> u32 {
    L::E + L::COFACTOR.bits()
}

fn ext<L: ParameterSet>(re: L::Base, im: L::Base) -> L::Ext {
    Field::add(
        L::ext_from_base(re),
        Field::mul(L::ext_i(), L::ext_from_base(im)),
    )
}

/// Compare two little-endian encodings as integers.
fn less(left: &[u8], right: &[u8]) -> bool {
    left.iter().rev().lt(right.iter().rev())
}

/// What the script calls the canonical root of a square in `F_p`: of the
/// two roots, the smaller integer.
fn canonical_root<L: ParameterSet>(square: L::Base) -> Vec<u8> {
    let root = Field::sqrt(square).expect("a square has a root");
    assert!(
        Field::same(Field::square(root), square),
        "the root squares back"
    );
    let (plus, minus) = (L::base_encode(root), L::base_encode(Field::negate(root)));
    if less(&minus, &plus) {
        minus
    } else {
        plus
    }
}

/// In `F_p²`: the root whose `(re, im)` is smaller, `re` compared first.
fn canonical_root_ext<L: ParameterSet>(square: L::Ext) -> Vec<u8> {
    let root = Field::sqrt(square).expect("a square has a root");
    assert!(
        Field::same(Field::square(root), square),
        "the root squares back"
    );
    let (plus, minus) = (L::ext_encode(root), L::ext_encode(Field::negate(root)));
    let n = L::FP_LEN;
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
fn base_results<L: ParameterSet>(a: L::Base, b: L::Base) -> [Vec<u8>; OPS.len()] {
    [
        L::base_encode(Field::add(a, b)),
        L::base_encode(Field::sub(a, b)),
        L::base_encode(Field::mul(a, b)),
        L::base_encode(Field::square(a)),
        L::base_encode(Field::inv(a).unwrap_or_else(<L::Base as Field>::zero)),
        vec![u8::from(Field::is_square(a))],
        canonical_root::<L>(Field::square(a)),
    ]
}

fn ext_results<L: ParameterSet>(x: L::Ext, y: L::Ext) -> [Vec<u8>; OPS.len()] {
    [
        L::ext_encode(Field::add(x, y)),
        L::ext_encode(Field::sub(x, y)),
        L::ext_encode(Field::mul(x, y)),
        L::ext_encode(Field::square(x)),
        L::ext_encode(Field::inv(x).unwrap_or_else(<L::Ext as Field>::zero)),
        vec![u8::from(Field::is_square(x))],
        canonical_root_ext::<L>(Field::square(x)),
    ]
}

fn rows<L: WithoutCorpus>(kind: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    L::FIXTURES
        .field_table
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .filter(move |fields| fields[0] == kind)
}

fn the_edge_values_match_sage_in_the_base_field<L: WithoutCorpus>() {
    let mut seen = 0;
    for fields in rows::<L>("BASE") {
        assert_eq!(fields.len(), 3 + OPS.len());
        let a = L::base_decode(&hex(fields[1])).expect("a is canonical");
        let b = L::base_decode(&hex(fields[2])).expect("b is canonical");
        let got = base_results::<L>(a, b);
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
    assert_eq!(
        seen,
        BASE_EDGE_VALUES * BASE_EDGE_VALUES,
        "every ordered pair of the edge values"
    );
}

fn the_edge_values_match_sage_in_the_extension_field<L: WithoutCorpus>() {
    let mut seen = 0;
    for fields in rows::<L>("EXT") {
        assert_eq!(fields.len(), 3 + OPS.len());
        let x = L::ext_decode(&hex(fields[1])).expect("x is canonical");
        let y = L::ext_decode(&hex(fields[2])).expect("y is canonical");
        let got = ext_results::<L>(x, y);
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
    assert_eq!(seen, EXT_EDGE_ROWS, "nine values against seven");
}

/// Ten thousand random rows in each field, one digest for each operation.
/// A wrong result anywhere changes the digest of that operation alone, so
/// a failure names the operation.
fn ten_thousand_random_rows_match_sage_in_both_fields<L: WithoutCorpus>() {
    let expected: Vec<(String, String, usize, String)> = rows::<L>("RANDOM")
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
    assert_eq!(
        expected.len(),
        2 * OPS.len(),
        "every operation in two fields"
    );
    let count = expected[0].2;
    assert!(count >= RANDOM_ROWS, "at least ten thousand rows");
    assert!(expected.iter().all(|row| row.2 == count));

    let mut stream = Stream(L::FIXTURES.field_seed);
    let mut base: [Sha3_256; OPS.len()] = core::array::from_fn(|_| Sha3_256::new());
    let mut extension: [Sha3_256; OPS.len()] = core::array::from_fn(|_| Sha3_256::new());
    for _ in 0..count {
        let (a, b) = (stream.element::<L>(), stream.element::<L>());
        let (x, y) = (stream.ext::<L>(), stream.ext::<L>());
        for (hash, result) in base.iter_mut().zip(base_results::<L>(a, b)) {
            hash.update(&result);
        }
        for (hash, result) in extension.iter_mut().zip(ext_results::<L>(x, y)) {
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
    let after: Vec<_> = rows::<L>("STREAM").collect();
    assert_eq!(after.len(), 1);
    assert_eq!(format!("{:016x}", stream.next()), after[0][2]);
}

/// Every check of this file at one set.
macro_rules! at_set {
    ($module:ident, $set:ty) => {
        mod $module {
            #[test]
            fn the_edge_values_match_sage_in_the_base_field() {
                super::the_edge_values_match_sage_in_the_base_field::<$set>();
            }

            #[test]
            fn the_edge_values_match_sage_in_the_extension_field() {
                super::the_edge_values_match_sage_in_the_extension_field::<$set>();
            }

            #[test]
            fn ten_thousand_random_rows_match_sage_in_both_fields() {
                super::ten_thousand_random_rows_match_sage_in_both_fields::<$set>();
            }
        }
    };
}

at_set!(p324, prism_field::P324);
