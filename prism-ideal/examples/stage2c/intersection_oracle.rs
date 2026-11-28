// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Replay the independently checked public corpus through import, intersection
//! and reduction. The four outcomes belong to distinct stages of the API.

use super::{check_bit, check_invalid, magnitude, scalar, ReductionSet};
use prism_field::{P248, P324, P376, P500, P664};
use prism_ideal::{GenericHnf, GuardedIdeal, Intersection};
use prism_vectors::Group;

fn fields<const N: usize>(text: &str) -> [&str; N] {
    let mut values = text.split_ascii_whitespace();
    let result = std::array::from_fn(|_| values.next().expect("incomplete fixture"));
    assert!(values.next().is_none(), "unexpected trailing field");
    result
}

#[path = "../../tests/support/lattice.rs"]
mod lattice;

fn intersection<S: ReductionSet<W, D>, const W: usize, const D: usize>(
    inputs: &str,
    verdict: &str,
) -> Option<Intersection<W>> {
    let [n, xn, yn, a, b, c, d, e, f, g, h, s, xq, yq] = fields(inputs);
    let n = scalar(n);
    let hnf = GenericHnf::from_norm_and_root(&n, &[scalar(xn), scalar(yn)]);
    let key = GuardedIdeal::<S, W, D>::admit(
        n,
        hnf.columns(),
        [a, b, c, d].map(scalar),
        [e, f, g, h].map(scalar),
    );
    if verdict == "reject-admission" {
        assert!(key.is_none());
        return None;
    }
    let mut challenge = magnitude(s);
    assert!(!s.starts_with('-') && challenge.len() <= S::S_LEN);
    challenge.resize(S::S_LEN, 0);
    let found = key
        .expect("expected admission")
        .intersection(&challenge, &[scalar(xq), scalar(yq)]);
    if verdict == "challenge-refused" {
        assert!(found.is_none());
        return None;
    }
    let found = found.expect("expected intersection");
    if verdict == "marked" {
        found
            .u
            .iter()
            .chain(&found.v)
            .chain([&found.norm])
            .for_each(check_invalid);
        return None;
    }
    assert_eq!(verdict, "ok");
    Some(found)
}

fn compare_oracle_hnfs<S: ReductionSet<W, D>, const W: usize, const D: usize>() {
    let path = Group::Signer
        .dir()
        .join("intersection/hnf")
        .join(format!("{}.tsv", S::NAME));
    let corpus = std::fs::read_to_string(path).unwrap();
    assert_eq!(corpus.lines().count(), 26);
    let mut accepted = 0;
    for line in corpus.lines() {
        let row: Vec<_> = line.split('\t').collect();
        let [name, verdict, inputs, norm, matrix, _note] = row.as_slice() else {
            panic!("expected six tab-separated columns");
        };
        eprintln!("{} {name}", S::NAME);
        let Some(found) = intersection::<S, W, D>(inputs, verdict) else {
            assert_eq!([*norm, *matrix], ["-"; 2]);
            continue;
        };
        check_bit(found.norm.ct_eq(&scalar(norm)), true);
        let columns = lattice::gaussian_columns(
            found.u.each_ref().map(lattice::public_fixture),
            found.v.each_ref().map(lattice::public_fixture),
        );
        let actual = lattice::column_hnf(columns.to_vec());
        // PARI writes this column HNF in row-major order in the fixture.
        for (index, expected) in fields::<16>(matrix).iter().enumerate() {
            let expected = num_bigint::BigInt::parse_bytes(expected.as_bytes(), 16).unwrap();
            assert_eq!(
                actual[index % 4][index / 4],
                expected,
                "{} {name}, entry {index}",
                S::NAME
            );
        }
        accepted += 1;
    }
    assert_eq!(accepted, 19);
}

#[test]
fn intersection_spans_match_oracle_hnfs_at_all_five_sets() {
    compare_oracle_hnfs::<P248, _, _>();
    compare_oracle_hnfs::<P324, _, _>();
    compare_oracle_hnfs::<P376, _, _>();
    compare_oracle_hnfs::<P500, _, _>();
    compare_oracle_hnfs::<P664, _, _>();
}
