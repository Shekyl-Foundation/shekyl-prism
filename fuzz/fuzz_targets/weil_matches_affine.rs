// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The Weil pairing without inversions against the affine loop it
//! replaced: the same refusal or the same field element on every input.
//!
//! The input picks a level and a claimed order, then four scalars. The
//! pair is `([a]P + [b]Q, [c]P + [d]Q)` on a committed signature's curve,
//! so short inputs give the degenerate pairs (a zero point, dependent
//! points, points of low order) and long ones the generic pair.

#![no_main]

use libfuzzer_sys::fuzz_target;
use prism_field::{Field, Level, Level1, Level3, Level5};
use prism_isogeny::jac::{add, Jac};
use prism_isogeny::montgomery::Montgomery;
use prism_isogeny::pairing::{mul, oracle, weil};

fn element<L: Level>(text: &str, name: &str) -> L::Ext {
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

fn level<L: Level>(text: &str, order: u8, scalars: &[u8]) {
    let at = |x, y| Jac {
        x: element::<L>(text, x),
        y: element::<L>(text, y),
        z: L::Ext::one(),
    };
    let curve = Montgomery::normalized(element::<L>(text, "E "));
    let (p, q) = (at("Px ", "Py "), at("Qx ", "Qy "));
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
    match which % 3 {
        0 => level::<Level1>(
            include_str!("../../prism-verify/testdata/s5_lvl1.txt"),
            *order,
            scalars,
        ),
        1 => level::<Level3>(
            include_str!("../../prism-verify/testdata/s5_lvl3.txt"),
            *order,
            scalars,
        ),
        _ => level::<Level5>(
            include_str!("../../prism-verify/testdata/s5_lvl5.txt"),
            *order,
            scalars,
        ),
    }
});
