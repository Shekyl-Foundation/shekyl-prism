// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The Weil pairing without inversions against the affine loop it
//! replaced: the same refusal or the same field element on every input.
//!
//! The input picks a set and a claimed order, then four scalars. The
//! pair is `([a]P + [b]Q, [c]P + [d]Q)` on a committed signature's curve,
//! or at `p324`, which has no signature, on a curve and basis Sage built,
//! so short inputs give the degenerate pairs (a zero point, dependent
//! points, points of low order) and long ones the generic pair.

#![no_main]

use libfuzzer_sys::fuzz_target;
use prism_field::{Field, ParameterSet, P248, P324, P376, P500};
use prism_fixtures::curve_table::Point;
use prism_fixtures::no_corpus::{curve_table, WithoutCorpus};
use prism_isogeny::jac::{add, Jac};
use prism_isogeny::montgomery::Montgomery;
use prism_isogeny::pairing::{mul, oracle, weil};

fn element<L: ParameterSet>(text: &str, name: &str) -> L::Ext {
    let hex = text
        .lines()
        .find_map(|line| line.strip_prefix(name))
        .expect("the vector has this row");
    let bytes: Vec<u8> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    L::ext_decode(&bytes).expect("a committed element is canonical")
}

/// The curve and basis of a committed signature.
fn committed<L: ParameterSet>(text: &str, order: u8, scalars: &[u8]) {
    let at = |x, y| Jac {
        x: element::<L>(text, x),
        y: element::<L>(text, y),
        z: L::Ext::one(),
    };
    let curve = Montgomery::normalized(element::<L>(text, "E "));
    pair::<L>(curve, at("Px ", "Py "), at("Qx ", "Qy "), order, scalars);
}

/// At a set without a corpus: the square-branch curve of its Sage curve
/// file and the order-`2^e` basis Sage built on it. `P` and `Q` are the
/// columns of those names.
fn sage_basis<L: WithoutCorpus>(order: u8, scalars: &[u8]) {
    let row = curve_table::<L>().curve(L::FIXTURES.square_branch_curve);
    let point = |point: &Point| {
        let (x, y) = point.decode::<L>();
        Jac {
            x,
            y,
            z: L::Ext::one(),
        }
    };
    let curve = Montgomery::normalized(row.coefficient.decode::<L>());
    pair::<L>(curve, point(&row.p), point(&row.q), order, scalars);
}

fn pair<L: ParameterSet>(
    curve: Montgomery<L::Ext>,
    p: Jac<L::Ext>,
    q: Jac<L::Ext>,
    order: u8,
    scalars: &[u8],
) {
    let width = scalars.len().div_ceil(4).max(1);
    let mut parts = scalars.chunks(width);
    let mut next = || parts.next().unwrap_or(&[]);
    let (a, b, c, d) = (next(), next(), next(), next());
    let r = add(curve, mul(curve, p, a), mul(curve, q, b));
    let t = add(curve, mul(curve, p, c), mul(curve, q, d));
    // The right order, one doubling short, or one too many.
    let k = L::E - 1 + u32::from(order % 3);
    match (weil(curve, r, t, k), oracle::weil(curve, r, t, k)) {
        (None, None) => {}
        (Some(new), Some(old)) => assert!(new.same(old)),
        (new, _) => panic!(
            "the loop without inversions {} where the affine loop did not",
            if new.is_some() {
                "gave a value"
            } else {
                "refused"
            }
        ),
    }
}

fuzz_target!(|data: &[u8]| {
    let [which, order, scalars @ ..] = data else {
        return;
    };
    // Four scalars of at most `e` bits each.
    if scalars.len() > 4 * 63 {
        return;
    }
    // One entry for each set. The input's first byte picks one; there is
    // no arm that an unlisted set would fall into.
    const SETS: [fn(u8, &[u8]); 4] = [
        |order, scalars| {
            committed::<P248>(
                include_str!("../../prism-verify/testdata/s5_lvl1.txt"),
                order,
                scalars,
            )
        },
        sage_basis::<P324>,
        |order, scalars| {
            committed::<P376>(
                include_str!("../../prism-verify/testdata/s5_lvl3.txt"),
                order,
                scalars,
            )
        },
        |order, scalars| {
            committed::<P500>(
                include_str!("../../prism-verify/testdata/s5_lvl5.txt"),
                order,
                scalars,
            )
        },
    ];
    SETS[usize::from(*which) % SETS.len()](*order, scalars);
});
