// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// Jacobian doubling, addition, and the basis lift are a port of `DBL`, `ADD`,
// `jac_to_xz`, `jac_to_xz_add_components` in
// `external/PRISM_v2/src/ec/ref/lvlx/ec_jac.c` and `lift_basis` /
// `ec_recover_y` in `basis.c` (Apache-2.0; Luca De Feo, Francisco RH, and
// the SQIsign authors). See `NOTICE`.

//! Jacobian points on a Montgomery curve with `C = 1`.
//!
//! A point `(X : Y : Z)` is the affine point `(X/Z^2, Y/Z^3)`. The lift from
//! an x-only basis recovers `y(P)` by a square root and `y(Q)` by the
//! Okeya–Sakurai formula. The verifier's chains are built from the lifted
//! points.
//!
//! # What this arithmetic assumes
//!
//! Every function here that takes a [`Montgomery`] reads its `A` and
//! ignores its `C`: it is the arithmetic of `y^2 = x^3 + A x^2 + x`. None
//! tests that a point is on that curve. Off the curve the formulas still
//! return three field elements, and nothing marks them as meaningless.
//!
//! In the verifier the curves are built with `C = 1`, the signature's
//! points are tested on their curve first (`response_points` in
//! `prism-verify`), and the key's two lifted points are tested on theirs
//! by [`crate::pairing::weil`] before either chain is built.

use prism_field::Field;

use crate::montgomery::{xdbl_ac, Montgomery, Xz};

/// `(X : Y : Z)` on `y^2 = x^3 + A x^2 + x`: the affine point
/// `(X/Z^2, Y/Z^3)`, or the point at infinity when `Z = 0`.
///
/// Plain data with no invariant: the fields are public and nothing checks
/// that the point is on a curve. The arithmetic treats every value with
/// `Z = 0` as infinity, whatever its `X` and `Y`.
#[derive(Clone, Copy, Debug)]
pub struct Jac<F> {
    /// `X`.
    pub x: F,
    /// `Y`.
    pub y: F,
    /// `Z`.
    pub z: F,
}

impl<F: Field> Jac<F> {
    /// The point at infinity, as `(0 : 1 : 0)`.
    pub fn infinity() -> Self {
        Self {
            x: F::zero(),
            y: F::one(),
            z: F::zero(),
        }
    }

    /// Whether `Z = 0`.
    pub fn is_infinity(self) -> bool {
        self.z.is_zero()
    }
}

/// Drop the y-coordinate: `(X : Z^2)`. A point with `Z = 0` becomes
/// `(X : 0)`, and `(1 : 0)` when `X` is zero too, as `jac_to_xz` does.
/// The result is never `(0 : 0)`.
pub fn to_xz<F: Field>(point: Jac<F>) -> Xz<F> {
    let mut image = Xz {
        x: point.x,
        z: point.z.square(),
    };
    if image.x.is_zero() && image.z.is_zero() {
        image = Xz::infinity();
    }
    image
}

/// `[2] point` on the curve `y^2 = x^3 + A x^2 + x`.
///
/// A point with `Z = 0` gives a point with `Z = 0`, and so does a point
/// of order two (`Y = 0`). See the module for what is assumed of the
/// curve and the point.
pub fn dbl<F: Field>(curve: Montgomery<F>, point: Jac<F>) -> Jac<F> {
    let infinity = point.x.is_zero() && point.z.is_zero();
    let x2 = point.x.square();
    let three_x2 = x2.add(x2).add(x2);
    let z2 = point.z.square();
    let a_x = curve.a.mul(point.x);
    let two_ax = a_x.add(a_x);
    let alpha = three_x2.add(z2.mul(two_ax.add(z2)));
    let mut z = point.y.mul(point.z);
    z = z.add(z);
    let four_a_y2_z2 = z.square().mul(curve.a);
    let y2 = point.y.square();
    let two_y2 = y2.add(y2);
    let four_x_y2 = two_y2.mul(point.x.add(point.x));
    let mut x = alpha
        .square()
        .sub(four_a_y2_z2)
        .sub(four_x_y2)
        .sub(four_x_y2);
    let four_y4 = two_y2.square();
    let y = four_x_y2.sub(x).mul(alpha).sub(four_y4).sub(four_y4);
    if infinity {
        x = point.x;
        z = point.z;
    }
    Jac { x, y, z }
}

/// `point + other` on the curve `y^2 = x^3 + A x^2 + x`.
///
/// Every case of the group law is handled: either point at infinity
/// (the other is returned as given), the two equal (the doubling
/// formula), and the two opposite (the result has `Z = 0`). See the
/// module for what is assumed of the curve and the points.
pub fn add<F: Field>(curve: Montgomery<F>, point: Jac<F>, other: Jac<F>) -> Jac<F> {
    let point_inf = point.z.is_zero();
    let other_inf = other.z.is_zero();
    let z1_2 = point.z.square();
    let z2_2 = other.z.square();
    let z2_3 = z2_2.mul(other.z);
    let z1_3 = z1_2.mul(point.z);
    let y1_z2_3 = z2_3.mul(point.y);
    let y2_z1_3 = z1_3.mul(other.y);
    let mut dy = y2_z1_3.sub(y1_z2_3);
    let u2 = z1_2.mul(other.x);
    let u1 = z2_2.mul(point.x);
    let mut dx = u2.sub(u1);

    let dx_double = point.y.add(point.y);
    let two_a = curve.a.add(curve.a);
    let x1_2 = point.x.square();
    let mut dy_double = two_a.mul(point.x).add(z1_2);
    dy_double = dy_double.mul(z1_2).add(x1_2);
    dy_double = dy_double.add(x1_2).add(x1_2);
    dy_double = dy_double.mul(other.z);
    if dx.is_zero() && dy.is_zero() {
        dx = dx_double;
        dy = dy_double;
    }

    let z_product = point.z.mul(other.z);
    let z_product_2 = z_product.square();
    let dx2 = dx.square();
    let dy2 = dy.square();
    let mut x = curve.a.mul(z_product_2).add(u1).add(u2);
    x = dy2.sub(x.mul(dx2));
    let mut y = u1.mul(dx2).sub(x).mul(dy);
    y = y.sub(y1_z2_3.mul(dx2.mul(dx)));
    let z = dx.mul(z_product);
    if point_inf {
        return other;
    }
    if other_inf {
        return point;
    }
    Jac { x, y, z }
}

/// `(u, v, w)` with `x(P+Q) = (u−v : w)` and `x(P−Q) = (u+v : w)`, for
/// two finite points on the curve. No case of the group law is handled
/// apart: with a point at infinity, or with `P = ±Q`, `w` is zero.
pub(crate) fn add_components<F: Field>(
    curve: Montgomery<F>,
    point: Jac<F>,
    other: Jac<F>,
) -> (F, F, F) {
    let z1_2 = point.z.square();
    let z2_2 = other.z.square();
    let x1_z2_2 = point.x.mul(z2_2);
    let z1_2_x2 = z1_2.mul(other.x);
    let mut y1_z2_3 = point.y.mul(other.z);
    y1_z2_3 = y1_z2_3.mul(z2_2);
    let mut z1_3_y2 = point.z.mul(other.y);
    z1_3_y2 = z1_3_y2.mul(z1_2);
    let z_product_2 = z1_2.mul(z2_2);
    let yy = y1_z2_3.mul(z1_3_y2);
    let v = yy.add(yy);
    let y_sum = y1_z2_3.square().add(z1_3_y2.square());
    let x_sum = x1_z2_2.add(z1_2_x2);
    let lambda = x1_z2_2.sub(z1_2_x2);
    let lambda2 = lambda.square();
    let gamma = curve.a.mul(z_product_2).add(x_sum);
    let u = y_sum.sub(gamma.mul(lambda2));
    let w = lambda2.mul(z_product_2);
    (u, v, w)
}

/// A `y` with `y^2 = x^3 + A x^2 + x`, or `None` when that value is not
/// a square in the field: then `x` is the abscissa of a point of the
/// twist and of no point of the curve.
///
/// Which of the two roots is returned is [`Field::sqrt`]'s choice. At a
/// parameter set's extension field that is the root whose real part is
/// even, or whose imaginary part is even when the real part is zero
/// (`prism_field::ParameterSet::Ext`), the rule of the reference's
/// `fp2_sqrt`.
pub fn recover_y<F: Field>(curve: Montgomery<F>, x: F) -> Option<F> {
    let x2 = x.square();
    let rhs = curve.a.mul(x2).add(x).add(x2.mul(x));
    // The value that leaves this function depends on which root this is:
    // the other one is the opposite point. The verifier's verdict does
    // not depend on it: see "The sign" at `lift_basis`.
    rhs.sqrt()
}

/// Lift `(P, Q, P−Q)` from x-coordinates to Jacobian points on the
/// normalized curve.
///
/// Returns the curve as `(A/C : 1)`, `P` as `(x : y : 1)` with `y` from
/// [`recover_y`], and `Q` with the `y` the Okeya–Sakurai formula derives
/// from `P`, `x(Q)` and `x(P−Q)`.
///
/// `None` when `C = 0`, when `P` is at infinity, or when `x(P)` is not
/// the abscissa of a point of the curve.
///
/// # What it assumes
///
/// `pmq` is the x-coordinate of `P − Q`, `Q` and `P − Q` are finite, and
/// `P` does not have order two. None of these is tested.
///
/// # When that fails
///
/// If `pmq` is not `x(P − Q)` or `x(P + Q)`, the `Q` returned is three
/// field elements that are not a point of the curve. If it is `x(P + Q)`, the `Q`
/// returned is `−Q`. If `Q` or `P − Q` is at infinity, or `y(P) = 0`,
/// the `Q` returned is `(0 : 0 : 0)`. All are well-formed values.
///
/// # The sign
///
/// The two square roots of [`recover_y`] give `(P, Q)` and `(−P, −Q)`.
/// Everything the verifier computes from the pair is the same for both:
/// the Weil pairing of the two, the x-coordinates of every combination
/// of them, and the chain a kernel built from them defines, which for
/// the second pair is the first chain after the automorphism `(−1, 1)`
/// of the product.
///
/// # Who establishes it in the verifier
///
/// `accept::lift` in `prism-verify` passes a key's tested basis: `P` and
/// `Q` of exact order `2^e` (`basis_e`), with `Q` the output of
/// [`crate::torsion::difference`] on `P` and the point passed as `pmq`,
/// so that `pmq` is `x(P − Q)` by construction. The pairing check that
/// follows refuses a lifted point that is not on the curve.
pub fn lift_basis<F: Field>(
    curve: Montgomery<F>,
    p: Xz<F>,
    q: Xz<F>,
    pmq: Xz<F>,
) -> Option<(Montgomery<F>, Jac<F>, Jac<F>)> {
    let curve = curve.affine_coefficient()?;
    let p_z_inv = p.z.inv()?;
    let p_x = p.x.mul(p_z_inv);
    let p_y = recover_y(curve, p_x)?;
    let p = Jac {
        x: p_x,
        y: p_y,
        z: F::one(),
    };

    // Okeya–Sakurai, then the conversion into Jacobian coordinates.
    let v1 = p.x.mul(q.z);
    let v2 = q.x.add(v1);
    let v3 = q.x.sub(v1);
    let v3 = v3.square().mul(pmq.x);
    let two_a = curve.a.add(curve.a);
    let v1 = two_a.mul(q.z);
    let v2 = v2.add(v1);
    let v4 = p.x.mul(q.x).add(q.z);
    let v2 = v2.mul(v4);
    let v1_qz = v1.mul(q.z);
    let v2 = v2.sub(v1_qz);
    let q_y = v3.sub(v2.mul(pmq.z));
    let scale = p.y.add(p.y).mul(q.z).mul(pmq.z);
    let q_x = q.x.mul(scale);
    let q_z = q.z.mul(scale);
    let q = Jac {
        x: q_x.mul(q_z),
        y: q_y.mul(q_z.square()),
        z: q_z,
    };
    Some((curve, p, q))
}

/// x-only doubling iterated `times` times.
pub(crate) fn xdbl_iter<F: Field>(curve: Montgomery<F>, mut point: Xz<F>, times: u32) -> Xz<F> {
    for _ in 0..times {
        point = xdbl_ac(curve, point);
    }
    point
}

/// `[2^times] point`: [`dbl`] iterated `times` times.
pub fn dbl_iter<F: Field>(curve: Montgomery<F>, mut point: Jac<F>, times: u32) -> Jac<F> {
    for _ in 0..times {
        point = dbl(curve, point);
    }
    point
}

#[cfg(test)]
mod tests {
    use super::{dbl, to_xz};
    use crate::montgomery::{a24, xdbl, Montgomery, Xz};
    use prism_field::Field;

    #[derive(Clone, Copy, Debug)]
    struct Toy(i64);
    const P: i64 = 1019;

    impl Toy {
        fn norm(self) -> i64 {
            self.0.rem_euclid(P)
        }
    }

    impl Field for Toy {
        fn zero() -> Self {
            Self(0)
        }
        fn one() -> Self {
            Self(1)
        }
        fn from_u64(v: u64) -> Self {
            Self((v % P as u64) as i64)
        }
        fn add(self, rhs: Self) -> Self {
            Self(self.norm() + rhs.norm())
        }
        fn sub(self, rhs: Self) -> Self {
            Self(self.norm() - rhs.norm())
        }
        fn square(self) -> Self {
            self.mul(self)
        }
        fn mul(self, rhs: Self) -> Self {
            Self(self.norm() * rhs.norm())
        }
        fn negate(self) -> Self {
            Self(-self.norm())
        }
        fn inv(self) -> Option<Self> {
            (self.norm() != 0).then(|| Self(mod_pow(self.norm(), P - 2)))
        }
        fn sqrt(self) -> Option<Self> {
            self.is_square()
                .then(|| Self(mod_pow(self.norm(), (P + 1) / 4)))
        }
        fn is_square(self) -> bool {
            let n = self.norm();
            n == 0 || mod_pow(n, (P - 1) / 2) == 1
        }
        fn is_zero(self) -> bool {
            self.norm() == 0
        }
        fn same(self, rhs: Self) -> bool {
            self.norm() == rhs.norm()
        }
    }

    fn mod_pow(mut base: i64, mut exp: i64) -> i64 {
        let mut acc = 1i64;
        base = base.rem_euclid(P);
        while exp > 0 {
            if exp % 2 == 1 {
                acc = (acc * base).rem_euclid(P);
            }
            base = (base * base).rem_euclid(P);
            exp /= 2;
        }
        acc
    }

    #[test]
    fn jacobian_doubling_matches_x_only_doubling() {
        let curve = Montgomery::normalized(Toy::from_u64(5));
        let constant = a24(curve.a).unwrap();
        let mut x = None;
        for candidate in 1..40u64 {
            let x0 = Toy::from_u64(candidate);
            let rhs = x0.square().mul(x0).add(curve.a.mul(x0.square())).add(x0);
            if rhs.is_square() {
                // Either root: the doubling below is compared in x alone.
                x = Some((x0, rhs.sqrt().unwrap()));
                break;
            }
        }
        let (x, y) = x.unwrap();
        let jac = super::Jac {
            x,
            y,
            z: Toy::one(),
        };
        let doubled = to_xz(dbl(curve, jac));
        let x_only = xdbl(constant, Xz::affine(x));
        let left = doubled.x.mul(x_only.z);
        let right = doubled.z.mul(x_only.x);
        assert!(left.same(right));
    }

    /// The `y` that [`super::recover_y`] returns is the root the
    /// extension field promises, and the opposite point's `y` is not.
    /// With the other root this fails at every point it meets.
    fn recover_y_returns_the_promised_root<L: prism_field::ParameterSet>() {
        let curve = Montgomery::normalized(L::Ext::from_u64(6));
        let even = |value: L::Base| L::base_encode(value)[0] & 1 == 0;
        let promised = |y: L::Ext| {
            let (re, im) = L::ext_parts(y);
            if re.is_zero() {
                even(im)
            } else {
                even(re)
            }
        };
        let (mut on_the_curve, mut on_the_twist) = (0, 0);
        for v in 2..66u64 {
            let x = L::Ext::from_u64(v).add(L::ext_i());
            let rhs = x.square().mul(x).add(curve.a.mul(x.square())).add(x);
            match super::recover_y(curve, x) {
                Some(y) => {
                    assert!(y.square().same(rhs), "{}: {v}", L::NAME);
                    assert!(promised(y), "{}: {v}", L::NAME);
                    assert!(!promised(y.negate()), "{}: {v}", L::NAME);
                    on_the_curve += 1;
                }
                None => {
                    assert!(!rhs.is_square(), "{}: {v}", L::NAME);
                    on_the_twist += 1;
                }
            }
        }
        assert!(on_the_curve > 0 && on_the_twist > 0, "{}", L::NAME);
    }

    #[test]
    fn recover_y_returns_the_promised_root_at_every_set() {
        use prism_field::{P248, P324, P376, P500, P664};
        recover_y_returns_the_promised_root::<P248>();
        recover_y_returns_the_promised_root::<P324>();
        recover_y_returns_the_promised_root::<P376>();
        recover_y_returns_the_promised_root::<P500>();
        recover_y_returns_the_promised_root::<P664>();
    }
}
