// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The committed signer constants, held to their definitions by this
//! crate's own arithmetic, at all five sets.
//!
//! `scripts/signer_constants.py` computes the starting basis of `E0[2^e]`
//! and the matrices of its endomorphisms in Python, with its own field
//! and its own curve. This test is a second implementation in a second
//! language: it reads the committed files and checks, with the field of
//! `prism-field` and the points and maps of this crate, that
//!
//! - both points are on `E0` and have order exactly `2^e`;
//! - they are independent, and `[2^(e−1)]Q0 = (0, 0)`;
//! - the third coordinate is `x(P0 − Q0)` and not `x(P0 + Q0)`: with it
//!   read the other way, the matrices below are not the actions;
//! - the matrices of `i`, of the Frobenius and of their product are the
//!   actions of [`prism_isogeny::endomorphism`]'s maps on the two points;
//! - `2·gen2 = 2·M_i`, `2·gen3 = M_i + M_j` and `2·gen4 = 1 + M_k`
//!   modulo `2^e`.
//!
//! **What this test cannot see:** the top bit of each entry of `gen3` and
//! `gen4`. Doubling loses it, and this crate has no arithmetic over
//! `F_{p⁴}`, where a half of a basis point lives. Those bits are tested
//! by the script, on points through a half of each basis point, and by
//! the comparison with the C reference (`p248`, `p376`, `p500`) and with
//! `sqisign-rs` (`p324`, `p500`, `p664`).
//!
//! **Why the helpers below are functions and not closures.** Written with
//! closures that take a point by value, this test failed at three sets in
//! the test profile and passed in release, on rustc 1.94.0. The cause was
//! not a constant: that compiler handed a closure the copy of a point that
//! an earlier closure's callee had changed (rust-lang/rust issue 155241;
//! ledger O31). Functions avoid the form that failed. They are not a
//! safeguard: the fault's class includes plain functions. The safeguard
//! is the toolchain, which is now a release with the fix, named as the
//! workspace's `rust-version`, with canaries of the fault in
//! `prism-field/tests/toolchain_canary.rs`.

use prism_field::{Field, ParameterSet, P248, P324, P376, P500, P664};
use prism_isogeny::endomorphism::{apply, multiply};
use prism_isogeny::jac::{add, dbl_iter, recover_y, Jac};
use prism_isogeny::montgomery::Montgomery;
use prism_vectors::signer::{self, Matrix};

/// `(x, y)` of a point, or `None` at infinity.
fn affine<F: Field>(point: Jac<F>) -> Option<(F, F)> {
    let z_inv = point.z.inv()?;
    let z_inv2 = z_inv.square();
    Some((point.x.mul(z_inv2), point.y.mul(z_inv2.mul(z_inv))))
}

fn same<F: Field>(left: Jac<F>, right: Jac<F>) -> bool {
    match (affine(left), affine(right)) {
        (None, None) => true,
        (Some((x1, y1)), Some((x2, y2))) => x1.same(x2) && y1.same(y2),
        _ => false,
    }
}

fn negated<F: Field>(point: Jac<F>) -> Jac<F> {
    Jac {
        y: point.y.negate(),
        ..point
    }
}

/// The point of `E0` with this `x`, as the file encodes it. The sign of
/// its `y` is the square root's choice.
fn lifted<L: ParameterSet>(curve: Montgomery<L::Ext>, encoded: &[u8]) -> Jac<L::Ext> {
    let x = L::ext_decode(encoded).expect("a canonical field element");
    let y = recover_y(curve, x).expect("a point of E0");
    assert!(
        y.square().same(x.square().mul(x).add(x)),
        "{}: on y^2 = x^3 + x",
        L::NAME
    );
    Jac {
        x,
        y,
        z: L::Ext::one(),
    }
}

/// `a + b` modulo `2^e`, little-endian bytes of the width of `a`.
fn sum_mod(a: &[u8], b: &[u8], e: u32) -> Vec<u8> {
    assert_eq!(a.len(), b.len());
    let mut carry = 0u16;
    let mut sum: Vec<u8> = a
        .iter()
        .zip(b)
        .map(|(x, y)| {
            let digit = u16::from(*x) + u16::from(*y) + carry;
            carry = digit >> 8;
            digit as u8
        })
        .collect();
    let spare = (8 * sum.len()) as u32 - e;
    if let Some(top) = sum.last_mut() {
        *top &= 0xff >> spare;
    }
    sum
}

fn matrix_sum(a: &Matrix, b: &Matrix, e: u32) -> Matrix {
    Matrix {
        m11: sum_mod(&a.m11, &b.m11, e),
        m12: sum_mod(&a.m12, &b.m12, e),
        m21: sum_mod(&a.m21, &b.m21, e),
        m22: sum_mod(&a.m22, &b.m22, e),
    }
}

fn identity(width: usize) -> Matrix {
    let mut one = vec![0u8; width];
    one[0] = 1;
    Matrix {
        m11: one.clone(),
        m12: vec![0; width],
        m21: vec![0; width],
        m22: one,
    }
}

/// `[2^(e−1)] point`.
fn top<F: Field>(curve: Montgomery<F>, point: Jac<F>, e: u32) -> Jac<F> {
    dbl_iter(curve, point, e - 1)
}

/// `x(p − q)`. The two are not equal or opposite here.
fn difference_x<F: Field>(curve: Montgomery<F>, p: Jac<F>, q: Jac<F>) -> F {
    affine(add(curve, p, negated(q)))
        .expect("the points are not equal")
        .0
}

/// The images of `p` and `q` under `a + b·i + c·π + d·i·π`.
fn images<L: ParameterSet>(
    (p, q): (Jac<L::Ext>, Jac<L::Ext>),
    (a, b, c, d): (u8, u8, i64, i64),
) -> (Jac<L::Ext>, Jac<L::Ext>) {
    (
        apply::<L>(p, &[a], &[b], c, d),
        apply::<L>(q, &[a], &[b], c, d),
    )
}

/// `[a] p + [b] q`, the scalars little-endian.
fn combine<L: ParameterSet>(
    curve: Montgomery<L::Ext>,
    (p, q): (Jac<L::Ext>, Jac<L::Ext>),
    a: &[u8],
    b: &[u8],
) -> Jac<L::Ext> {
    add(curve, multiply::<L>(p, a), multiply::<L>(q, b))
}

/// Whether `matrix` sends the basis to `(on_p, on_q)`: columns are images.
fn acts_as<L: ParameterSet>(
    curve: Montgomery<L::Ext>,
    matrix: &Matrix,
    basis: (Jac<L::Ext>, Jac<L::Ext>),
    (on_p, on_q): (Jac<L::Ext>, Jac<L::Ext>),
) -> bool {
    same(combine::<L>(curve, basis, &matrix.m11, &matrix.m21), on_p)
        && same(combine::<L>(curve, basis, &matrix.m12, &matrix.m22), on_q)
}

fn the_constants_are_what_the_specification_defines<L: ParameterSet>() {
    let set = signer::set(L::NAME).expect("the set's committed file");
    let curve = Montgomery::normalized(L::Ext::zero());
    let e = L::E;
    assert_eq!(set.e, e);

    // On E0, of order exactly 2^e, independent, and Q0 above (0, 0).
    let p = lifted::<L>(curve, &set.basis_px);
    let q_either = lifted::<L>(curve, &set.basis_qx);
    for point in [p, q_either] {
        let above = top(curve, point, e);
        assert!(!above.is_infinity(), "{}: order below 2^e", L::NAME);
        assert!(
            dbl_iter(curve, above, 1).is_infinity(),
            "{}: order above 2^e",
            L::NAME
        );
    }
    let (p_top, _) = affine(top(curve, p, e)).unwrap();
    let (q_top, q_top_y) = affine(top(curve, q_either, e)).unwrap();
    assert!(!p_top.same(q_top), "{}: dependent", L::NAME);
    assert!(
        q_top.is_zero() && q_top_y.is_zero(),
        "{}: [2^(e-1)]Q0 is not (0, 0)",
        L::NAME
    );

    // The third coordinate fixes the sign of Q0 against P0: exactly one
    // of the two signs has x(P0 - Q0) equal to it.
    let third = L::ext_decode(&set.basis_pmqx).expect("a canonical field element");
    let q = if difference_x(curve, p, q_either).same(third) {
        q_either
    } else {
        negated(q_either)
    };
    assert!(
        difference_x(curve, p, q).same(third),
        "{}: x(P0 - Q0)",
        L::NAME
    );
    assert!(
        !difference_x(curve, p, negated(q)).same(third),
        "{}: the third coordinate is also x(P0 + Q0)",
        L::NAME
    );

    // The maps of this crate's `endomorphism` module on the two points.
    let maps = [
        ("i", &set.action_i, (0, 1, 0, 0)),
        ("the Frobenius", &set.action_j, (0, 0, 1, 0)),
        ("k", &set.action_k, (0, 0, 0, 1)),
        ("gen2", &set.action_gen2, (0, 1, 0, 0)),
    ];
    for (name, matrix, map) in maps {
        let (on_p, on_q) = images::<L>((p, q), map);
        assert!(
            acts_as::<L>(curve, matrix, (p, q), (on_p, on_q)),
            "{}: the matrix of {name} is not its action",
            L::NAME
        );
        // With the third coordinate read as x(P0 + Q0), Q0 is the other
        // point of the two, and the matrix is not the action.
        assert!(
            !acts_as::<L>(curve, matrix, (p, negated(q)), (on_p, negated(on_q))),
            "{}: the matrix of {name} does not depend on the sign of Q0",
            L::NAME
        );
    }

    // The halved generators, as far as doubling shows them.
    let width = set.action_i.m11.len();
    let twice_gen3 = matrix_sum(&set.action_gen3, &set.action_gen3, e);
    let twice_gen4 = matrix_sum(&set.action_gen4, &set.action_gen4, e);
    assert_eq!(
        twice_gen3,
        matrix_sum(&set.action_i, &set.action_j, e),
        "{}: 2 gen3 = i + j",
        L::NAME
    );
    assert_eq!(
        twice_gen4,
        matrix_sum(&identity(width), &set.action_k, e),
        "{}: 2 gen4 = 1 + k",
        L::NAME
    );
    // And on points, twice each: 2·gen3 acts as i + j, 2·gen4 as 1 + k.
    assert!(
        acts_as::<L>(
            curve,
            &twice_gen3,
            (p, q),
            images::<L>((p, q), (0, 1, 1, 0))
        ),
        "{}: 2 gen3 on points",
        L::NAME
    );
    assert!(
        acts_as::<L>(
            curve,
            &twice_gen4,
            (p, q),
            images::<L>((p, q), (1, 0, 0, 1))
        ),
        "{}: 2 gen4 on points",
        L::NAME
    );
}

#[test]
fn the_signer_constants_of_p248_are_what_the_specification_defines() {
    the_constants_are_what_the_specification_defines::<P248>();
}

#[test]
fn the_signer_constants_of_p324_are_what_the_specification_defines() {
    the_constants_are_what_the_specification_defines::<P324>();
}

#[test]
fn the_signer_constants_of_p376_are_what_the_specification_defines() {
    the_constants_are_what_the_specification_defines::<P376>();
}

#[test]
fn the_signer_constants_of_p500_are_what_the_specification_defines() {
    the_constants_are_what_the_specification_defines::<P500>();
}

#[test]
fn the_signer_constants_of_p664_are_what_the_specification_defines() {
    the_constants_are_what_the_specification_defines::<P664>();
}

#[test]
fn sums_modulo_a_power_of_two_carry_and_drop_the_top() {
    assert_eq!(sum_mod(&[0xff, 0x00], &[0x01, 0x00], 16), [0x00, 0x01]);
    assert_eq!(sum_mod(&[0xff, 0xff], &[0x01, 0x00], 16), [0x00, 0x00]);
    assert_eq!(sum_mod(&[0xff, 0x0f], &[0x01, 0x00], 12), [0x00, 0x00]);
    assert_eq!(sum_mod(&[0x00, 0x08], &[0x00, 0x08], 12), [0x00, 0x00]);
    assert_eq!(sum_mod(&[0x00, 0x04], &[0x00, 0x04], 12), [0x00, 0x08]);
}
