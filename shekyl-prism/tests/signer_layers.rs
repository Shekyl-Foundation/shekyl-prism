// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! With the `signer` feature, the signer's layers are reachable through
//! `hazmat`. Cargo builds this file only when the feature is asked for
//! (`required-features` in the manifest), and CI's step that asks for it
//! requires the log to show each of these tests ran.

#[path = "signer_layers/ideal_lattice.rs"]
mod ideal_lattice;

use shekyl_prism::hazmat::ideal::geometry::{
    degree_bits, secret_norm_bits, DEGREE_EXPONENT, DEGREE_SHIFT, ORDER_DENOMINATOR_BITS,
    Q4_SLACK_BITS,
};
#[path = "support/signer_fixtures.rs"]
mod signer_fixtures;
use shekyl_prism::hazmat::ideal::GenericHnf;
use shekyl_prism::hazmat::int::{Int, SecretBit, SecretInt};
use signer_fixtures::check_bit;

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
/// column-HNF is the norm-and-root constructor. The comparisons leave
/// through `check_bit`, the fixture exit the treatment table counts.
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
