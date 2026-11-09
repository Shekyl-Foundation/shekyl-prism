// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The action of `a + b·i + c·π + d·i·π` on the starting curve `E0`.
//!
//! `i(x, y) = (−x, i·y)` and `π` is the Frobenius, which on `F_{p^2}` is
//! conjugation. `i^2 = −1` and `π^2 = [−p]`, so the norm of this element is
//! `a^2 + b^2 + p(c^2 + d^2)`.

use prism_field::{Field, Level};

use crate::jac::{add, Jac};
use crate::montgomery::Montgomery;

fn conjugate<L: Level>(value: L::Ext) -> L::Ext {
    let (re, im) = L::ext_parts(value);
    L::ext_from_base(re).sub(L::ext_i().mul(L::ext_from_base(im)))
}

fn i_map<L: Level>(point: Jac<L::Ext>) -> Jac<L::Ext> {
    Jac {
        x: point.x.negate(),
        y: point.y.mul(L::ext_i()),
        z: point.z,
    }
}

fn frobenius<L: Level>(point: Jac<L::Ext>) -> Jac<L::Ext> {
    Jac {
        x: conjugate::<L>(point.x),
        y: conjugate::<L>(point.y),
        z: conjugate::<L>(point.z),
    }
}

fn mul_le<L: Level>(curve: Montgomery<L::Ext>, point: Jac<L::Ext>, scalar: &[u8]) -> Jac<L::Ext> {
    let mut bits = Vec::new();
    for byte in scalar.iter().rev() {
        for shift in (0..8).rev() {
            bits.push((byte >> shift) & 1 == 1);
        }
    }
    while bits.first() == Some(&false) {
        bits.remove(0);
    }
    let mut acc = Jac::infinity();
    for bit in bits {
        acc = crate::jac::dbl(curve, acc);
        if bit {
            acc = add(curve, acc, point);
        }
    }
    acc
}

fn small_scalar<L: Level>(
    curve: Montgomery<L::Ext>,
    point: Jac<L::Ext>,
    scalar: i64,
) -> Jac<L::Ext> {
    let mut image = point;
    if scalar < 0 {
        image.y = image.y.negate();
    }
    let mut bytes = scalar.unsigned_abs().to_le_bytes().to_vec();
    bytes.truncate(8);
    mul_le::<L>(curve, image, &bytes)
}

/// `α(point)` for `α = a + bi + cπ + diπ`. `a` and `b` are little-endian.
pub fn apply<L: Level>(point: Jac<L::Ext>, a: &[u8], b: &[u8], c: i64, d: i64) -> Jac<L::Ext> {
    let curve = Montgomery::normalized(L::Ext::zero());
    let frobenius = frobenius::<L>(point);
    let terms = [
        mul_le::<L>(curve, point, a),
        mul_le::<L>(curve, i_map::<L>(point), b),
        small_scalar::<L>(curve, frobenius, c),
        small_scalar::<L>(curve, i_map::<L>(frobenius), d),
    ];
    terms
        .into_iter()
        .reduce(|sum, term| add(curve, sum, term))
        .unwrap_or_else(Jac::infinity)
}

/// `[scalar] point` on `E0`, scalar little-endian.
pub fn multiply<L: Level>(point: Jac<L::Ext>, scalar_le: &[u8]) -> Jac<L::Ext> {
    mul_le::<L>(Montgomery::normalized(L::Ext::zero()), point, scalar_le)
}
