// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// Adapted from Theta_dim4 @ 4770b24, isogeny_chain_dim2.py,
// gluing_isogeny_dim2.py, isogeny_dim2.py and montgomery_theta.py.
// See NOTICE and LICENSES for the upstream MIT and Apache-2.0 notices.

//! Full-point, adapted-theta chains and evaluation of their duals.
//! All arithmetic here is on public verification inputs.

use crate::jac::{add, dbl_iter, to_xz, Jac};
use crate::montgomery::{Montgomery, Xz};
use crate::theta::{
    double_iter, hadamard, projectively_same, to_squared_theta, ThetaConstants, ThetaPoint,
};
use crate::theta_step::{
    apply_isomorphism, isogeny_compute, isogeny_eval, BasisChange, IsogenyStep,
};
use prism_field::Field;

pub type Couple<F> = [Jac<F>; 2];

fn map<F: Field>(matrix: [[F; 4]; 4], point: ThetaPoint<F>) -> ThetaPoint<F> {
    apply_isomorphism(&BasisChange { rows: matrix }, point, true)
}

fn inverse<F: Field>(mut a: [[F; 4]; 4]) -> Option<[[F; 4]; 4]> {
    let mut b = BasisChange::<F>::identity().rows;
    for col in 0..4 {
        let pivot = (col..4).find(|&row| !a[row][col].is_zero())?;
        a.swap(pivot, col);
        b.swap(pivot, col);
        let inv = a[col][col].inv()?;
        for j in 0..4 {
            a[col][j] = a[col][j].mul(inv);
            b[col][j] = b[col][j].mul(inv);
        }
        for row in 0..4 {
            if row == col {
                continue;
            }
            let factor = a[row][col];
            for j in 0..4 {
                a[row][j] = a[row][j].sub(factor.mul(a[col][j]));
                b[row][j] = b[row][j].sub(factor.mul(b[col][j]));
            }
        }
    }
    Some(b)
}

fn invert_coords<F: Field>(point: ThetaPoint<F>) -> Option<[F; 4]> {
    let c = point.coords();
    Some([c[0].inv()?, c[1].inv()?, c[2].inv()?, c[3].inv()?])
}

fn scaled<F: Field>(point: ThetaPoint<F>, factors: [F; 4]) -> ThetaPoint<F> {
    let c = point.coords();
    ThetaPoint::from_coords(core::array::from_fn(|i| c[i].mul(factors[i])))
}

fn valid<F: Field>(p: ThetaPoint<F>) -> Option<ThetaPoint<F>> {
    p.coords().iter().any(|v| !v.is_zero()).then_some(p)
}

fn product_coords<F: Field>(points: Couple<F>) -> ThetaPoint<F> {
    let [p, q] = points.map(to_xz);
    ThetaPoint::from_coords([p.x.mul(q.x), p.x.mul(q.z), p.z.mul(q.x), p.z.mul(q.z)])
}

/// Canonicalize an order-four basis, preserving its pairing. The returned
/// row matrix expresses the original basis in the canonical basis.
/// The bounded enumeration replaces the reference's order-four discrete logs.
pub fn canonical_four<F: Field>(
    curve: Montgomery<F>,
    p: Jac<F>,
    q: Jac<F>,
) -> Option<(Jac<F>, [[u8; 2]; 2])> {
    for swapped in [false, true] {
        let (first, second) = if swapped { (q, p) } else { (p, q) };
        let mut candidate = second;
        let neg_first = Jac {
            y: first.y.negate(),
            ..first
        };
        for d in 0..4 {
            let x = to_xz(candidate);
            if !x.z.is_zero() && x.x.same(x.z.negate()) {
                let matrix = if swapped {
                    [[d, 3], [1, 0]]
                } else {
                    [[1, 0], [d, 1]]
                };
                return Some((first, matrix));
            }
            candidate = add(curve, candidate, neg_first);
        }
    }
    None
}

/// Product theta null induced by canonical order-four bases.
pub fn product_null<F: Field>(p: Jac<F>, q: Jac<F>) -> ThetaPoint<F> {
    let p = to_xz(p);
    let q = to_xz(q);
    let a = p.x.add(p.z);
    let b = p.x.sub(p.z);
    let c = q.x.add(q.z);
    let d = q.x.sub(q.z);
    ThetaPoint::from_coords([a.mul(c), b.mul(c), a.mul(d), b.mul(d)])
}

struct Gluing<F> {
    curves: [Montgomery<F>; 2],
    matrix: [[F; 4]; 4],
    inverse: [[F; 4]; 4],
    shift: Couple<F>,
    pre: [F; 4],
    zero: usize,
    null: ThetaPoint<F>,
    dual_pre: [F; 4],
}

impl<F: Field> Gluing<F> {
    fn new(
        curves: [Montgomery<F>; 2],
        kernel: [Couple<F>; 2],
        product: ThetaPoint<F>,
        n: [[F; 4]; 4],
    ) -> Option<Self> {
        let z = product.coords();
        let matrix = core::array::from_fn(|i| {
            let m: [F; 4] = core::array::from_fn(|j| n[i][j].mul(z[j]));
            [
                m[0].add(m[1]).add(m[2]).add(m[3]),
                m[0].negate().sub(m[1]).add(m[2]).add(m[3]),
                m[0].negate().add(m[1]).sub(m[2]).add(m[3]),
                m[0].sub(m[1]).sub(m[2]).add(m[3]),
            ]
        });
        let null_bc = map(n, product);
        let first = to_squared_theta(map(matrix, product_coords(kernel[0]))).coords();
        let second = to_squared_theta(map(matrix, product_coords(kernel[1]))).coords();
        let zero = first.iter().position(|v| v.is_zero())?;
        if !second[zero].is_zero() {
            return None;
        }
        let a = second[1 ^ zero];
        let b = first[2 ^ zero];
        let c = second[3 ^ zero];
        let d = first[3 ^ zero];
        let mut abcd = [F::zero(); 4];
        abcd[1 ^ zero] = a.mul(c.inv()?);
        abcd[2 ^ zero] = b.mul(d.inv()?);
        abcd[3 ^ zero] = F::one();
        let mut pre = [F::zero(); 4];
        pre[1 ^ zero] = c.mul(a.inv()?);
        pre[2 ^ zero] = d.mul(b.inv()?);
        pre[3 ^ zero] = F::one();
        Some(Self {
            curves,
            matrix,
            inverse: inverse(matrix)?,
            shift: core::array::from_fn(|i| dbl_iter(curves[i], kernel[0][i], 1)),
            pre,
            zero,
            null: valid(hadamard(ThetaPoint::from_coords(abcd)))?,
            // H(H(null_bc)) is projectively null_bc.
            dual_pre: invert_coords(null_bc)?,
        })
    }

    fn eval(&self, point: Couple<F>) -> Option<ThetaPoint<F>> {
        let translate = core::array::from_fn(|i| add(self.curves[i], point[i], self.shift[i]));
        let a = to_squared_theta(map(self.matrix, product_coords(point))).coords();
        let b = to_squared_theta(map(self.matrix, product_coords(translate))).coords();
        let j = self.zero;
        let y = a[1 ^ j].mul(self.pre[1 ^ j]);
        let z = a[2 ^ j].mul(self.pre[2 ^ j]);
        let t = a[3 ^ j];
        let lambda = if !z.is_zero() {
            z.mul(b[3 ^ j].inv()?)
        } else {
            t.mul(b[2 ^ j].mul(self.pre[2 ^ j]).inv()?)
        };
        let x = b[1 ^ j].mul(self.pre[1 ^ j]).mul(lambda);
        let mut out = [F::zero(); 4];
        out[j] = x;
        out[1 ^ j] = y;
        out[2 ^ j] = z;
        out[3 ^ j] = t;
        valid(hadamard(ThetaPoint::from_coords(out)))
    }

    fn dual_eval(&self, point: ThetaPoint<F>) -> Option<[Xz<F>; 2]> {
        let image = valid(map(
            self.inverse,
            scaled(to_squared_theta(point), self.dual_pre),
        ))?;
        let [xx, xz, zx, zz] = image.coords();
        if !xx.mul(zz).same(xz.mul(zx)) {
            return None;
        }
        if !zz.is_zero() {
            Some([Xz { x: xz, z: zz }, Xz { x: zx, z: zz }])
        } else if zx.is_zero() && !xz.is_zero() {
            Some([Xz::infinity(), Xz { x: xx, z: xz }])
        } else if !zx.is_zero() && xz.is_zero() {
            Some([Xz { x: xx, z: zx }, Xz::infinity()])
        } else if !xx.is_zero() && xz.is_zero() && zx.is_zero() {
            Some([Xz::infinity(); 2])
        } else {
            None
        }
    }
}

/// A chain retaining its forward maps and the data for evaluating its dual.
pub struct DualChain<F> {
    gluing: Gluing<F>,
    steps: Vec<IsogenyStep<F>>,
    dual_pre: Vec<[F; 4]>,
    pub null: ThetaPoint<F>,
    pub nulls: Vec<ThetaPoint<F>>,
}

impl<F: Field> DualChain<F> {
    /// `kernel` has order `2^(length+2)` and is adapted to `n`.
    pub fn new(
        length: u32,
        curves: [Montgomery<F>; 2],
        kernel: [Couple<F>; 2],
        product: ThetaPoint<F>,
        n: [[F; 4]; 4],
    ) -> Option<Self> {
        if length < 2 {
            return None;
        }
        // Balanced stack: saved generators cost O(log length) storage and
        // O(length log length) doublings, rather than redoubling from the root.
        let mut jac = vec![(kernel, length - 1)];
        while jac.last()?.1 != 0 {
            let (pair, remaining) = *jac.last()?;
            let times = remaining.div_ceil(2);
            let child = pair.map(|p| core::array::from_fn(|i| dbl_iter(curves[i], p[i], times)));
            jac.push((child, remaining - times));
        }
        let (eight, _) = jac.pop()?;
        let gluing = Gluing::new(curves, eight, product, n)?;
        let mut stack = Vec::new();
        for (pair, remaining) in jac {
            stack.push((
                [gluing.eval(pair[0])?, gluing.eval(pair[1])?],
                remaining - 1,
            ));
        }
        let mut null = gluing.null;
        let mut nulls = vec![null];
        let mut steps = Vec::new();
        let mut dual_pre = Vec::new();
        for _ in 1..length {
            while stack.last()?.1 != 0 {
                let (pair, remaining) = *stack.last()?;
                let times = remaining.div_ceil(2);
                let constants = ThetaConstants::from_null(null);
                let child = pair.map(|p| double_iter(constants, p, times));
                stack.push((child, remaining - times));
            }
            let (pair, _) = stack.pop()?;
            let step = isogeny_compute(pair[0], pair[1], false, true, true)?;
            dual_pre.push(invert_coords(null)?);
            for (pair, remaining) in &mut stack {
                *pair = pair.map(|p| isogeny_eval(&step, p));
                *remaining = remaining.checked_sub(1)?;
            }
            null = valid(step.codomain)?;
            nulls.push(null);
            steps.push(step);
        }
        Some(Self {
            gluing,
            steps,
            dual_pre,
            null,
            nulls,
        })
    }

    pub fn eval(&self, point: Couple<F>) -> Option<ThetaPoint<F>> {
        let mut image = self.gluing.eval(point)?;
        for step in &self.steps {
            image = valid(isogeny_eval(step, image))?;
        }
        Some(image)
    }

    /// Input uses the Hadamard of this chain's codomain structure.
    pub fn dual_eval(&self, mut point: ThetaPoint<F>) -> Option<[Xz<F>; 2]> {
        for pre in self.dual_pre.iter().rev() {
            point = valid(hadamard(scaled(to_squared_theta(point), *pre)))?;
        }
        self.gluing.dual_eval(point)
    }

    pub fn matches_dual(&self, other: &Self) -> bool {
        projectively_same(self.null, hadamard(other.null))
    }
}
