// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// Gluing is a port of `gluing_compute`, `gluing_eval_point`,
// `gluing_eval_point_special_case`, and `gluing_change_of_basis` in
// `external/PRISM_v2/src/hd/ref/lvlx/theta_isogenies.c` (Apache-2.0; Antonin
// Leroux and the SQIsign authors). See `NOTICE`.

//! The first (2,2)-step, from an elliptic product into theta coordinates.
//!
//! The kernel is a pair of points of order 8, one on each factor. A kernel
//! whose order is not 8, or whose squared theta image is not in the shape
//! the codomain formula assumes, is a refusal.

use prism_field::Field;

use crate::jac::{add_components, dbl, to_xz, Jac};
use crate::montgomery::{xdbl_ac, Montgomery, Xz};
use crate::theta::{hadamard, to_squared_theta, ThetaPoint};
use crate::theta_step::{apply_isomorphism, BasisChange};

/// One point on each factor, in x-only coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Couple<F> {
    pub first: Xz<F>,
    pub second: Xz<F>,
}

/// The same, with y-coordinates.
#[derive(Clone, Copy, Debug)]
pub struct CoupleJac<F> {
    pub first: Jac<F>,
    pub second: Jac<F>,
}

/// `(T1, T2, T1−T2)` on a product, the kernel the chain is given.
#[derive(Clone, Copy, Debug)]
pub struct Kernel<F> {
    pub t1: Couple<F>,
    pub t2: Couple<F>,
    pub difference: Couple<F>,
}

/// A gluing `E1 × E2 → A`.
#[derive(Clone, Copy, Debug)]
pub struct Gluing<F> {
    pub domain: [Montgomery<F>; 2],
    pub kernel: CoupleJac<F>,
    pub matrix: BasisChange<F>,
    pub image_x: F,
    pub image_y: F,
    pub precomputation: ThetaPoint<F>,
    pub codomain: ThetaPoint<F>,
}

#[derive(Clone, Copy)]
struct Translation<F> {
    g00: F,
    g01: F,
    g10: F,
    g11: F,
}

fn xz_equal<F: Field>(left: Xz<F>, right: Xz<F>) -> bool {
    left.x.mul(right.z).same(right.x.mul(left.z))
}

fn xz_zero<F: Field>(point: Xz<F>) -> bool {
    point.z.is_zero()
}

fn double_couple<F: Field>(curves: [Montgomery<F>; 2], point: Couple<F>) -> Couple<F> {
    Couple {
        first: xdbl_ac(curves[0], point.first),
        second: xdbl_ac(curves[1], point.second),
    }
}

fn double_jac<F: Field>(curves: [Montgomery<F>; 2], point: CoupleJac<F>) -> CoupleJac<F> {
    CoupleJac {
        first: dbl(curves[0], point.first),
        second: dbl(curves[1], point.second),
    }
}

fn order_two<F: Field>(curves: [Montgomery<F>; 2], first: Couple<F>, second: Couple<F>) -> bool {
    if xz_zero(first.first)
        || xz_zero(first.second)
        || xz_zero(second.first)
        || xz_zero(second.second)
    {
        return false;
    }
    if xz_equal(first.first, second.first) || xz_equal(first.second, second.second) {
        return false;
    }
    let doubled_first = double_couple(curves, first);
    let doubled_second = double_couple(curves, second);
    xz_zero(doubled_first.first)
        && xz_zero(doubled_first.second)
        && xz_zero(doubled_second.first)
        && xz_zero(doubled_second.second)
}

fn translation<F: Field>(
    order_four: Xz<F>,
    order_two: Xz<F>,
    z_inv: F,
    det_inv: F,
) -> Translation<F> {
    let tmp = order_four.x.mul(z_inv);
    let g10 = order_four.x.mul(order_two.x).mul(det_inv).sub(tmp);
    let g11 = order_two.x.mul(det_inv).mul(order_four.z);
    Translation {
        g00: g11.negate(),
        g01: order_two.z.mul(det_inv).mul(order_four.z).negate(),
        g10,
        g11,
    }
}

fn change_of_basis<F: Field>(
    curves: [Montgomery<F>; 2],
    k1_4: Couple<F>,
    k2_4: Couple<F>,
) -> Option<BasisChange<F>> {
    let k1_2 = double_couple(curves, k1_4);
    let k2_2 = double_couple(curves, k2_4);
    if !order_two(curves, k1_2, k2_2) {
        return None;
    }
    let pairs = [
        (k1_4.first, k1_2.first),
        (k1_4.second, k1_2.second),
        (k2_4.first, k2_2.first),
        (k2_4.second, k2_2.second),
    ];
    let mut pending = [(F::zero(), F::zero()); 4];
    for (slot, (order_four, order_two)) in pending.iter_mut().zip(pairs) {
        let det = order_four
            .x
            .mul(order_two.z)
            .sub(order_four.z.mul(order_two.x));
        *slot = (order_four.z, det);
    }
    let mut inverse = [F::zero(); 8];
    for (index, (z, det)) in pending.iter().enumerate() {
        inverse[index] = z.inv()?;
        inverse[index + 4] = det.inv()?;
    }
    let matrices = [
        translation(pairs[0].0, pairs[0].1, inverse[0], inverse[4]),
        translation(pairs[1].0, pairs[1].1, inverse[1], inverse[5]),
        translation(pairs[2].0, pairs[2].1, inverse[2], inverse[6]),
        translation(pairs[3].0, pairs[3].1, inverse[3], inverse[7]),
    ];
    Some(basis_matrix(matrices))
}

fn basis_matrix<F: Field>(g: [Translation<F>; 4]) -> BasisChange<F> {
    let t001 = g[0].g00.mul(g[2].g00).add(g[0].g01.mul(g[2].g10));
    let t101 = g[0].g10.mul(g[2].g00).add(g[0].g11.mul(g[2].g10));
    let t002 = g[1].g00.mul(g[3].g00).add(g[1].g01.mul(g[3].g10));
    let t102 = g[1].g10.mul(g[3].g00).add(g[1].g11.mul(g[3].g10));
    let mut rows = [[F::zero(); 4]; 4];
    rows[0][0] = F::one()
        .add(t001.mul(t002))
        .add(g[2].g00.mul(g[3].g00))
        .add(g[0].g00.mul(g[1].g00));
    rows[0][1] = t001
        .mul(t102)
        .add(g[2].g00.mul(g[3].g10))
        .add(g[0].g00.mul(g[1].g10));
    rows[0][2] = t101
        .mul(t002)
        .add(g[2].g10.mul(g[3].g00))
        .add(g[0].g10.mul(g[1].g00));
    rows[0][3] = t101
        .mul(t102)
        .add(g[2].g10.mul(g[3].g10))
        .add(g[0].g10.mul(g[1].g10));
    // Action of (0, K2_4.P2): row 1 from row 0 and G3.
    rows[1][0] = g[3].g00.mul(rows[0][0]).add(g[3].g01.mul(rows[0][1]));
    rows[1][1] = g[3].g10.mul(rows[0][0]).add(g[3].g11.mul(rows[0][1]));
    rows[1][2] = g[3].g00.mul(rows[0][2]).add(g[3].g01.mul(rows[0][3]));
    rows[1][3] = g[3].g10.mul(rows[0][2]).add(g[3].g11.mul(rows[0][3]));
    // Action of (K1_4.P1, 0): row 2 from row 0 and G0.
    rows[2][0] = g[0].g00.mul(rows[0][0]).add(g[0].g01.mul(rows[0][2]));
    rows[2][1] = g[0].g00.mul(rows[0][1]).add(g[0].g01.mul(rows[0][3]));
    rows[2][2] = g[0].g10.mul(rows[0][0]).add(g[0].g11.mul(rows[0][2]));
    rows[2][3] = g[0].g10.mul(rows[0][1]).add(g[0].g11.mul(rows[0][3]));
    // Action of (K1_4.P1, K2_4.P2): row 3 from row 1 and G0.
    rows[3][0] = g[0].g00.mul(rows[1][0]).add(g[0].g01.mul(rows[1][2]));
    rows[3][1] = g[0].g00.mul(rows[1][1]).add(g[0].g01.mul(rows[1][3]));
    rows[3][2] = g[0].g10.mul(rows[1][0]).add(g[0].g11.mul(rows[1][2]));
    rows[3][3] = g[0].g10.mul(rows[1][1]).add(g[0].g11.mul(rows[1][3]));
    BasisChange { rows }
}

fn product_theta<F: Field>(point: Couple<F>) -> ThetaPoint<F> {
    ThetaPoint {
        x: point.first.x.mul(point.second.x),
        y: point.first.x.mul(point.second.z),
        z: point.second.x.mul(point.first.z),
        t: point.first.z.mul(point.second.z),
    }
}

/// Glue `curves` along the order-8 kernel `(k1, k2)`.
pub fn gluing_compute<F: Field>(
    curves: [Montgomery<F>; 2],
    k1: CoupleJac<F>,
    k2: CoupleJac<F>,
    check_isotropy: bool,
) -> Option<Gluing<F>> {
    let k1_4 = double_jac(curves, k1);
    let k2_4 = double_jac(curves, k2);
    let xz = |point: CoupleJac<F>| Couple {
        first: to_xz(point.first),
        second: to_xz(point.second),
    };
    let matrix = change_of_basis(curves, xz(k1_4), xz(k2_4))?;
    let first = to_squared_theta(apply_isomorphism(&matrix, product_theta(xz(k1)), true));
    let second = to_squared_theta(apply_isomorphism(&matrix, product_theta(xz(k2)), true));
    if !first.t.is_zero() || !second.t.is_zero() {
        return None;
    }
    if first.x.is_zero()
        || second.x.is_zero()
        || first.y.is_zero()
        || second.z.is_zero()
        || first.z.is_zero()
    {
        return None;
    }
    let codomain = ThetaPoint {
        x: first.x.mul(second.x),
        y: first.y.mul(second.x),
        z: first.x.mul(second.z),
        t: F::zero(),
    };
    let precomputation = ThetaPoint {
        x: first.y.mul(second.z),
        y: codomain.z,
        z: codomain.y,
        t: F::zero(),
    };
    let image_x = first.x.mul(precomputation.x);
    let image_y = first.z.mul(precomputation.z);
    if check_isotropy {
        let left = first.y.mul(precomputation.y);
        if !image_x.same(left) {
            return None;
        }
        let right = second.x.mul(precomputation.x);
        let left = second.z.mul(precomputation.z);
        if !left.same(right) {
            return None;
        }
    }
    let codomain = hadamard(codomain);
    Some(Gluing {
        domain: curves,
        kernel: k1,
        matrix,
        image_x,
        image_y,
        precomputation,
        codomain,
    })
}

/// Image of a general Jacobian point. The chain uses this for kernel points.
pub fn gluing_eval<F: Field>(gluing: &Gluing<F>, point: CoupleJac<F>) -> ThetaPoint<F> {
    let (u1, v1, w1) = add_components(gluing.domain[0], point.first, gluing.kernel.first);
    let (u2, v2, w2) = add_components(gluing.domain[1], point.second, gluing.kernel.second);
    let t1x = u1.mul(u2).add(v1.mul(v2));
    let t1 = ThetaPoint {
        x: t1x,
        y: u1.mul(w2),
        z: w1.mul(u2),
        t: w1.mul(w2),
    };
    let t2 = ThetaPoint {
        x: u1.add(v1).mul(u2.add(v2)).sub(t1x),
        y: v1.mul(w2),
        z: w1.mul(v2),
        t: F::zero(),
    };
    let t1 = apply_isomorphism(&gluing.matrix, t1, true);
    let t2 = apply_isomorphism(&gluing.matrix, t2, false);
    let t1 = ThetaPoint::from_coords(t1.coords().map(|coord| coord.square()));
    let t2 = ThetaPoint::from_coords(t2.coords().map(|coord| coord.square()));
    let difference = ThetaPoint {
        x: t1.x.sub(t2.x),
        y: t1.y.sub(t2.y),
        z: t1.z.sub(t2.z),
        t: t1.t.sub(t2.t),
    };
    let mixed = hadamard(difference);
    hadamard(ThetaPoint {
        x: mixed.x.mul(gluing.image_y),
        y: mixed.y.mul(gluing.image_y),
        z: mixed.z.mul(gluing.image_x),
        t: mixed.t.mul(gluing.image_x),
    })
}

/// Image of a point with a zero factor, `(P, 0)` or `(0, P)`.
pub fn gluing_eval_special<F: Field>(
    gluing: &Gluing<F>,
    point: Couple<F>,
) -> Option<ThetaPoint<F>> {
    let transformed = to_squared_theta(apply_isomorphism(
        &gluing.matrix,
        product_theta(point),
        true,
    ));
    if !transformed.t.is_zero() {
        return None;
    }
    Some(hadamard(ThetaPoint {
        x: transformed.x.mul(gluing.precomputation.x),
        y: transformed.y.mul(gluing.precomputation.y),
        z: transformed.z.mul(gluing.precomputation.z),
        t: F::zero(),
    }))
}
