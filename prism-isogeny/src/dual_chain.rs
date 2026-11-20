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
//!
//! This is the chain the verifier runs. A [`DualChain`] is a chain of
//! (2,2)-isogenies from a product of two Montgomery curves, kept with
//! what is needed to evaluate it forward and to evaluate its dual
//! backward. The verifier builds two, of lengths `e − 2` and `e − 3`,
//! from the two halves of one kernel; requires that they end on the same
//! surface ([`DualChain::matches_dual`]); and carries three points
//! forward through the first and back through the second.
//!
//! "Adapted" is the reference's word: the product's theta structure is
//! first moved by the matrix of [`crate::theta_base::base_change_theta`]
//! so that the kernel is in the position the step formulas assume, and
//! every step is the 8-torsion formula.

use crate::jac::{add, dbl_iter, to_xz, Jac};
use crate::montgomery::{Montgomery, Xz};
use crate::theta::{
    double_iter, hadamard, projectively_same, to_squared_theta, ThetaConstants, ThetaPoint,
};
use crate::theta_step::{
    apply_isomorphism, isogeny_compute, isogeny_eval, BasisChange, IsogenyStep,
};
use prism_field::Field;

/// One full point on each factor of a product `E1 × E2`: `[P1, P2]`.
///
/// Not [`crate::gluing::Couple`], which is the same pair in x-only
/// coordinates. This chain needs the `y`.
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

/// The inverse of every element, by one inversion (Montgomery's trick).
/// `None` if any element is zero.
///
/// The zero test is on each element, not on their product: a zero theta
/// coordinate is what a crafted signature aims for, and its refusal must
/// not depend on how the inverses are computed. The results are the same
/// field elements as one inversion each would give.
fn invert_all<F: Field>(values: &[F]) -> Option<Vec<F>> {
    if values.iter().any(|v| v.is_zero()) {
        return None;
    }
    // `prefix[i]` is the product of the elements before `i`.
    let mut prefix = Vec::with_capacity(values.len());
    let mut product = F::one();
    for value in values {
        prefix.push(product);
        product = product.mul(*value);
    }
    // `suffix` is the inverse of the product of the elements up to `i`.
    let mut suffix = product.inv()?;
    let mut inverses = vec![F::zero(); values.len()];
    for i in (0..values.len()).rev() {
        inverses[i] = suffix.mul(prefix[i]);
        suffix = suffix.mul(values[i]);
    }
    Some(inverses)
}

/// The four inverse coordinates of each point, by one inversion for all of
/// them. `None` if any coordinate of any point is zero.
fn invert_coords<F: Field>(points: &[ThetaPoint<F>]) -> Option<Vec<[F; 4]>> {
    let flat: Vec<F> = points.iter().flat_map(|p| p.coords()).collect();
    let inverses = invert_all(&flat)?;
    Some(
        inverses
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect(),
    )
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
///
/// The canonical basis is `(T, U)` with `x(U) = −1`: `U` is a point of
/// order four above `(0, 0)`. The search tries `q − d·p` for `d` in
/// `0..4`, and then `p − d·q`, and takes the first with that abscissa.
/// The point returned is the `T` that goes with it: `p`, or `q` when
/// the roles were exchanged. Only `x(T)` is used by [`product_null`].
///
/// `None` when none of the eight candidates has `x = −1`.
///
/// # What it assumes
///
/// `p` and `q` are a basis of the 4-torsion of the curve
/// `y^2 = x^3 + A x^2 + x`. Not tested.
///
/// # When that fails
///
/// If the two do not generate the 4-torsion, the search usually finds
/// nothing and this is `None`. It can also find a candidate, and then
/// the pair returned is not a basis and nothing says so.
///
/// # Who establishes it in the verifier
///
/// `accept::build` in `prism-verify` passes `[2^(e−2)]` of two points
/// tested to have order exactly `2^e` and to be independent (`basis_e`),
/// on the key's curve and on the response's.
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

/// Product theta null induced by canonical order-four bases:
/// `((x₁+z₁)(x₂+z₂) : (x₁−z₁)(x₂+z₂) : (x₁+z₁)(x₂−z₂) : (x₁−z₁)(x₂−z₂))`
/// for `p = (x₁ : z₁)` on the first factor and `q = (x₂ : z₂)` on the
/// second.
///
/// # What it assumes
///
/// `p` and `q` are the first points [`canonical_four`] returned for the
/// two factors. It computes the formula for any two points; for others
/// the result is four field elements that are the null point of nothing,
/// with no sign of it. The verifier passes those two points and no
/// others.
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
            dual_pre: invert_coords(&[null_bc])?.pop()?,
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
///
/// Every field is private and [`Self::new`] is the only way to a value,
/// so a `DualChain` is a chain every step of which was computed and
/// passed the step's own checks.
pub struct DualChain<F> {
    gluing: Gluing<F>,
    steps: Vec<IsogenyStep<F>>,
    dual_pre: Vec<[F; 4]>,
    null: ThetaPoint<F>,
    nulls: Vec<ThetaPoint<F>>,
}

impl<F: Field> DualChain<F> {
    /// The chain of `length` steps from `curves[0] × curves[1]` with the
    /// given kernel.
    ///
    /// `kernel` is two generators, each a point on each curve. `product`
    /// is the product theta null of [`product_null`], and `n` the base
    /// change of [`crate::theta_base::base_change_theta`] for this
    /// kernel.
    ///
    /// `None` when `length < 2`, and wherever a step cannot be computed:
    /// the gluing does not find the zero coordinate its formula needs, a
    /// coordinate that must be inverted is zero, a step's kernel points
    /// have a zero where the 8-torsion formula needs none, a step's
    /// isotropy check fails, or a null point is `(0:0:0:0)`. All of
    /// these are the one `None`.
    ///
    /// # What it assumes
    ///
    /// The two generators have order `2^(length+2)`, generate a subgroup
    /// that is isotropic for the product pairing, and are adapted to
    /// `n`: in the theta structure `n` defines, their multiples of order
    /// eight are in the position the gluing formula assumes. The curves
    /// have `C = 1` and the points are on them.
    ///
    /// # When that fails
    ///
    /// Usually `None`, from one of the checks above. Those checks do not
    /// prove the assumption. A kernel that passes every one of them
    /// gives a chain, and a chain from the wrong kernel looks like any
    /// other.
    ///
    /// # Who establishes it in the verifier
    ///
    /// Nothing does before the call, and nothing needs to: the verifier
    /// does not trust the chain it gets. `accept::check` in
    /// `prism-verify` accepts only if the two chains end on the same
    /// surface and three points with known images are carried to them.
    /// That comparison is the protocol's test of the kernel, and so of
    /// the signature.
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
            for (pair, remaining) in &mut stack {
                *pair = pair.map(|p| isogeny_eval(&step, p));
                *remaining = remaining.checked_sub(1)?;
            }
            null = valid(step.codomain)?;
            nulls.push(null);
            steps.push(step);
        }
        // The dual of step `i` needs the inverse coordinates of the null
        // point it started from, `nulls[i]`. One inversion serves the whole
        // chain. A zero coordinate at any step is refused here and not at
        // its step; every refusal in this constructor is the same `None`,
        // and nothing outside it runs in between.
        let dual_pre = invert_coords(&nulls[..steps.len()])?;
        Some(Self {
            gluing,
            steps,
            dual_pre,
            null,
            nulls,
        })
    }

    /// The theta null point after the gluing and after each later step,
    /// in order: `length` of them, the last being the codomain's.
    pub fn nulls(&self) -> &[ThetaPoint<F>] {
        &self.nulls
    }

    /// The image of a point of the product, in the codomain's theta
    /// coordinates.
    ///
    /// `None` when the gluing cannot evaluate the point (a coordinate it
    /// must invert is zero) or an image is `(0:0:0:0)`. The point must
    /// be on the two curves; that is not tested, and for a point that is
    /// not, the result is a theta point like any other.
    pub fn eval(&self, point: Couple<F>) -> Option<ThetaPoint<F>> {
        let mut image = self.gluing.eval(point)?;
        for step in &self.steps {
            image = valid(isogeny_eval(step, image))?;
        }
        Some(image)
    }

    /// The image of a theta point of the codomain under the dual chain,
    /// as an x-only point on each of the two curves.
    ///
    /// Input uses the Hadamard of this chain's codomain structure. That
    /// is the structure the *other* chain of a matching pair ends in
    /// ([`Self::matches_dual`]), which is how the verifier uses it.
    ///
    /// `None` when an intermediate image is `(0:0:0:0)`, or when what
    /// comes back to the product is not a product point: its four
    /// coordinates must satisfy the rank-one relation, and the cases
    /// with a factor at infinity are told apart, never guessed. A point
    /// returned is never `(0 : 0)`.
    ///
    /// A theta point that is in some other structure is not refused for
    /// that, and gives a pair of abscissas that mean nothing. The
    /// verifier passes only what [`Self::eval`] of the matching chain
    /// returned, and compares the result with points it already knows.
    pub fn dual_eval(&self, mut point: ThetaPoint<F>) -> Option<[Xz<F>; 2]> {
        for pre in self.dual_pre.iter().rev() {
            point = valid(hadamard(scaled(to_squared_theta(point), *pre)))?;
        }
        self.gluing.dual_eval(point)
    }

    /// Whether this chain and `other` end on the same surface in dual
    /// theta structures: this chain's null point is, projectively, the
    /// Hadamard transform of the other's.
    ///
    /// This is the verifier's codomain check. When it holds, an image
    /// under [`Self::eval`] of this chain is a valid input to
    /// [`Self::dual_eval`] of `other`.
    pub fn matches_dual(&self, other: &Self) -> bool {
        projectively_same(self.null, hadamard(other.null))
    }
}

#[cfg(test)]
mod tests {
    use super::{invert_all, invert_coords};
    use crate::theta::ThetaPoint;
    use prism_field::{Field, ParameterSet, P248};

    type F = <P248 as ParameterSet>::Ext;

    fn element(n: u64) -> F {
        // Not in the base field, so the inverses are not either.
        F::from_u64(n).add(P248::ext_i().mul(F::from_u64(n + 7)))
    }

    #[test]
    fn the_shared_inversion_gives_each_element_its_own_inverse() {
        for len in [1usize, 2, 4, 5, 8, 100] {
            let values: Vec<F> = (0..len as u64).map(|n| element(3 * n + 1)).collect();
            let shared = invert_all(&values).expect("no element is zero");
            assert_eq!(shared.len(), len);
            for (value, inverse) in values.iter().zip(&shared) {
                let own = value.inv().expect("nonzero");
                assert!(inverse.same(own), "length {len}");
            }
        }
        assert_eq!(invert_all::<F>(&[]).map(|v| v.len()), Some(0));
    }

    #[test]
    fn a_zero_in_any_position_is_refused() {
        for len in [1usize, 4, 9] {
            for zero in 0..len {
                let mut values: Vec<F> = (0..len as u64).map(|n| element(n + 2)).collect();
                values[zero] = F::zero();
                assert!(invert_all(&values).is_none(), "zero at {zero} of {len}");
            }
        }
    }

    #[test]
    fn a_zero_theta_coordinate_is_refused_in_each_position_of_each_point() {
        let good = |n: u64| {
            ThetaPoint::from_coords([element(n), element(n + 1), element(n + 2), element(n + 3)])
        };
        let points = [good(1), good(10), good(20)];
        let inverses = invert_coords(&points).expect("no coordinate is zero");
        for (point, inverse) in points.iter().zip(&inverses) {
            for (c, i) in point.coords().iter().zip(inverse) {
                assert!(c.mul(*i).same(F::one()));
            }
        }
        for which in 0..points.len() {
            for coordinate in 0..4 {
                let mut bad = points;
                let mut coords = bad[which].coords();
                coords[coordinate] = F::zero();
                bad[which] = ThetaPoint::from_coords(coords);
                assert!(
                    invert_coords(&bad).is_none(),
                    "point {which}, coordinate {coordinate}"
                );
            }
        }
    }
}
