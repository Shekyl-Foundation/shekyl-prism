// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! With the `signer` feature, the signer's layers are reachable through
//! `hazmat`. Cargo builds this file only when the feature is asked for
//! (`required-features` in the manifest), and CI's step that asks for it
//! requires the log to show this test ran.

use shekyl_prism::hazmat::int::{Int, SecretInt};

/// The integer crate is the one behind `hazmat::int`: its two types are
/// there and compute. No exit of the secret type is called: a call site
/// outside the integer crate needs a row in the specification's table.
#[test]
fn the_integers_are_under_hazmat() {
    let sum = Int::<4>::from_u64(40).add(&Int::from_u64(2)).unwrap();
    assert!(sum == Int::from_u64(42));
    let secret = SecretInt::<4>::from_public(&sum);
    assert_eq!(format!("{:?}", secret.ct_eq(&secret)), "SecretBit");
}
