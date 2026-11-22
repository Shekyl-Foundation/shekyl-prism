// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Canaries for a fault of the compiler, not of this crate.
//!
//! rustc from 1.77.0 to 1.96 can pass a `Copy` value to a callee through
//! a hidden pointer to the caller's own copy, without copying it first.
//! A callee that changes its by-value argument then changes the caller's
//! value, and a later call with the same variable receives the changed
//! one. It is rust-lang/rust issue 155241, fixed in 1.97.0 by pull
//! request 155343; ledger O31 has what was measured here. This workspace
//! pins a release with the fix and names it as `rust-version`, so these
//! tests are expected to pass. They are kept so that a toolchain change
//! that brings the fault back fails here, in the test profile and in
//! release, on each architecture CI builds.
//!
//! They live in the lowest crate of the workspace because they need
//! nothing from any crate: `cargo test --workspace` runs them in both
//! profiles on x86_64 and aarch64, first, and the fault is in the
//! compiler that builds every crate above.
//!
//! Two devices keep the optimizer from deciding the question before it
//! reaches the code that has the fault. The value is made behind
//! [`std::hint::black_box`], so the compiler cannot see its contents and
//! fold the comparison to a constant. The callee is `#[inline(never)]`,
//! so the value is passed to a real call, which is where the hidden
//! pointer is; inlined, there is no argument to pass and no fault to
//! show.

use std::hint::black_box;

/// Larger than two registers, so it is passed by a hidden pointer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Limbs {
    x: [u64; 12],
    y: [u64; 12],
    z: [u64; 12],
}

/// A value the compiler cannot see into.
fn make() -> Limbs {
    let mut limbs = Limbs {
        x: [0; 12],
        y: [0; 12],
        z: [0; 12],
    };
    for (k, ((x, y), z)) in limbs
        .x
        .iter_mut()
        .zip(limbs.y.iter_mut())
        .zip(limbs.z.iter_mut())
        .enumerate()
    {
        *x = 7 + k as u64;
        *y = 11;
        *z = 1;
    }
    black_box(limbs)
}

/// Changes its own copy of the argument, as it may, and returns it.
#[inline(never)]
fn churn(mut limbs: Limbs, rounds: u32) -> Limbs {
    for round in 0..rounds {
        for k in 0..12 {
            limbs.x[k] = limbs.x[k]
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(u64::from(round) + k as u64);
            limbs.y[k] ^= limbs.x[k].rotate_left(7);
            limbs.z[k] = limbs.z[k].wrapping_add(limbs.y[k]);
        }
    }
    limbs
}

/// The closure form: the program that showed the fault on rustc 1.94.0
/// (NOTEBOOK 2026-11-21, signer stage 1b). A closure passes the value to
/// a callee that changes its copy; a second closure is then given the
/// same variable, and must receive the value as it was made. On rustc
/// 1.94.0, x86_64, it failed at `opt-level` 1, 2 and 3 and passed at 0
/// and in a plain release build (ledger O31).
#[test]
fn a_closure_receives_the_value_and_not_an_earlier_callees_copy() {
    let made = make();
    let first = |limbs| churn(limbs, 3);
    let changed = first(made);
    let identity = |limbs: Limbs| limbs;
    let received = identity(made);
    assert_eq!(received, made);
    assert_eq!(received.x[0], 7);
    assert_ne!(changed, made);
}

/// The plain-function form: one function, called twice with the same
/// value, whose body changes its own copy. The two calls must return the
/// same thing, and the caller's value must be unchanged. This is the
/// first case of the regression test upstream added for the fault,
/// written here from its description. Its result on rustc 1.94.0 is in
/// ledger O31.
#[test]
fn a_function_called_twice_with_one_value_receives_it_twice() {
    let made = make();
    let first = churn(made, 3);
    let second = churn(made, 3);
    assert_eq!(first, second);
    assert_eq!(made.x[0], 7);
    assert_ne!(first, made);
}
