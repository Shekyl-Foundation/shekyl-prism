// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! With the `signer` feature, the signer's layers are reachable through
//! `hazmat`. Cargo builds this file only when the feature is asked for
//! (`required-features` in the manifest), and CI's step that asks for it
//! requires the log to show each of these tests ran.

use shekyl_prism::hazmat::ideal::geometry::{
    degree_bits, secret_norm_bits, DEGREE_EXPONENT, DEGREE_SHIFT, ORDER_DENOMINATOR_BITS,
    Q4_SLACK_BITS,
};
#[path = "support/signer_fixtures.rs"]
mod signer_fixtures;
use shekyl_prism::hazmat::ideal::{GenericHnf, GuardedIdeal, IdealSet, Vector};
use shekyl_prism::hazmat::int::{Int, SecretBit, SecretInt};
use signer_fixtures::{check_bit, check_invalid, magnitude, scalar};

/// The integer crate is the one behind `hazmat::int`: its two types are
/// there and compute. No exit of the secret type is called: a call site
/// outside the integer crate needs a row in the specification's table.
#[test]
fn the_integers_are_under_hazmat() {
    let sum = Int::<4>::from_u64(40).add(&Int::from_u64(2)).unwrap();
    assert!(sum == Int::from_u64(42));
    let wrapped = SecretInt::<4>::from_public(&sum);
    let _: SecretBit = wrapped.ct_eq(&wrapped);
}

/// Ideal admission is the other signer layer behind `hazmat`. The generic
/// column-HNF is the norm-and-root constructor. No secret exit is called.
#[test]
fn the_ideal_layer_is_under_hazmat() {
    assert_eq!(degree_bits(8), DEGREE_EXPONENT * 8 - DEGREE_SHIFT);
    assert_eq!(secret_norm_bits(10), 10u32.div_ceil(2) + Q4_SLACK_BITS);
    let norm = SecretInt::<4>::from_u64(5);
    let root = [SecretInt::from_u64(2), SecretInt::from_u64(1)];
    let hnf = GenericHnf::from_norm_and_root(&norm, &root);
    let (pivot, root_column) = hnf.generators();
    check_bit(pivot[0].ct_eq(&norm.shl(ORDER_DENOMINATOR_BITS)), true);
    check_bit(root_column[2].ct_eq(&SecretInt::one()), true);
    check_bit(hnf.pivot().ct_eq(&pivot[0]), true);
}

fn check_intersection<S: IdealSet<W, D>, const W: usize, const D: usize>(
    ideals: &str,
    reductions: &str,
    first_failing_shear: u64,
    composite_scalar: u8,
) {
    let inputs: Vec<_> = ideals.lines().next().unwrap().split_whitespace().collect();
    let [norm, x, y, a, b, c, d, e, f, g, h, s, xq, yq] = inputs[..] else {
        panic!("ideal fixture schema")
    };
    let n = scalar(norm);
    let hnf = GenericHnf::from_norm_and_root(&n, &[scalar(x), scalar(y)]);
    let u = [a, b, c, d].map(scalar);
    let v = [e, f, g, h].map(scalar);
    let admit =
        |cols: &[Vector<W>; 4], v| GuardedIdeal::<S, W, D>::admit(n.clone(), cols, u.clone(), v);
    let key = admit(hnf.columns(), v.clone()).unwrap();
    let mut challenge = magnitude(s);
    challenge.resize(S::S_LEN, 0);
    let root = [scalar(xq), scalar(yq)];
    let intersection = key.intersection(&challenge, &root).unwrap();
    let mut expected = reductions
        .lines()
        .find_map(|l| l.strip_prefix("prered-guard256 "))
        .unwrap()
        .split_whitespace();
    for value in std::iter::once(&intersection.norm)
        .chain(&intersection.u)
        .chain(&intersection.v)
    {
        check_bit(value.ct_eq(&scalar(expected.next().unwrap())), true);
    }
    let marked = key
        .intersection(&challenge, &[SecretInt::zero(), SecretInt::zero()])
        .unwrap();
    for value in std::iter::once(&marked.norm)
        .chain(&marked.u)
        .chain(&marked.v)
    {
        check_invalid(value);
    }
    challenge.fill(0);
    challenge[0] = composite_scalar;
    assert!(key.intersection(&challenge, &root).is_none());

    // Unimodular column operations preserve the ideal but violate the two
    // canonical HNF conditions independently.
    let mut cols = hnf.columns().clone();
    cols[0][1] = cols[1][1].clone();
    assert!(admit(&cols, v.clone()).is_none());
    let mut cols = hnf.columns().clone();
    cols[2][0] = cols[2][0].add(hnf.pivot());
    assert!(admit(&cols, v.clone()).is_none());

    // The integer inequality N4(v+k*u)^2 <= p*(1024*N)^2 on the
    // first fixture yields the consecutive boundary cases below.
    let shear = |k| std::array::from_fn(|i| v[i].add(&u[i].mul(&SecretInt::from_u64(k))));
    assert!(admit(hnf.columns(), shear(first_failing_shear - 1)).is_some());
    assert!(admit(hnf.columns(), shear(first_failing_shear)).is_none());
}

#[test]
fn guarded_ideal_intersection_and_admission_boundaries() {
    use shekyl_prism::hazmat::field::{P248, P324, P376, P500, P664};
    // scripts/stage2d_fixtures.py derives one row per set from its first ideal.
    let mut rows = include_str!("../examples/stage2c/admission-boundaries.txt").lines();
    macro_rules! run {
        ($set:ty, $name:literal) => {{
            let fields: Vec<_> = rows.next().unwrap().split_whitespace().collect();
            let [name, shear, scalar, _factor] = fields[..] else {
                panic!("admission boundary fixture schema")
            };
            assert_eq!(name, $name);
            check_intersection::<$set, _, _>(
                include_str!(concat!("../examples/stage2c/ideals/", $name, ".txt")),
                include_str!(concat!("../examples/stage2c/vectors/", $name, ".txt")),
                shear.parse().unwrap(),
                scalar.parse().unwrap(),
            );
        }};
    }
    run!(P248, "p248");
    run!(P324, "p324");
    run!(P376, "p376");
    run!(P500, "p500");
    run!(P664, "p664");
    assert!(rows.next().is_none());
}
