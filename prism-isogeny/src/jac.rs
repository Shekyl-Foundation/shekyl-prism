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
//! Okeya–Sakurai formula, which is what gluing reads.

use prism_field::Field;

use crate::montgomery::{xdbl_ac, Montgomery, Xz};

/// `(X : Y : Z)` on `y^2 = x^3 + A x^2 + x`.
#[derive(Clone, Copy, Debug)]
pub struct Jac<F> {
    pub x: F,
    pub y: F,
    pub z: F,
}

impl<F: Field> Jac<F> {
    pub fn infinity() -> Self {
        Self {
            x: F::zero(),
            y: F::one(),
            z: F::zero(),
        }
    }

    pub fn is_infinity(self) -> bool {
        self.z.is_zero()
    }
}

/// Drop the y-coordinate. Infinity becomes `(1 : 0)`, as `jac_to_xz` does.
pub fn to_xz<F: Field>(point: Jac<F>) -> Xz<F> {
    let mut image = Xz {
        x: point.x,
        z: point.z.mul(point.z),
    };
    if image.x.is_zero() && image.z.is_zero() {
        image = Xz::infinity();
    }
    image
}

/// `[2] point` on the normalized curve. Infinity stays infinity.
pub fn dbl<F: Field>(curve: Montgomery<F>, point: Jac<F>) -> Jac<F> {
    let infinity = point.x.is_zero() && point.z.is_zero();
    let x2 = point.x.mul(point.x);
    let three_x2 = x2.add(x2).add(x2);
    let z2 = point.z.mul(point.z);
    let two_ax = curve.a.mul(point.x).add(curve.a.mul(point.x));
    let alpha = three_x2.add(z2.mul(two_ax.add(z2)));
    let mut z = point.y.mul(point.z);
    z = z.add(z);
    let four_a_y2_z2 = z.mul(z).mul(curve.a);
    let two_y2 = point.y.mul(point.y).add(point.y.mul(point.y));
    let four_x_y2 = two_y2.mul(point.x.add(point.x));
    let mut x = alpha
        .mul(alpha)
        .sub(four_a_y2_z2)
        .sub(four_x_y2)
        .sub(four_x_y2);
    let four_y4 = two_y2.mul(two_y2);
    let y = four_x_y2.sub(x).mul(alpha).sub(four_y4).sub(four_y4);
    if infinity {
        x = point.x;
        z = point.z;
    }
    Jac { x, y, z }
}

/// `point + other` on the normalized curve, including the cases where either
/// point is infinity or the two points are equal or negatives.
pub fn add<F: Field>(curve: Montgomery<F>, point: Jac<F>, other: Jac<F>) -> Jac<F> {
    let point_inf = point.z.is_zero();
    let other_inf = other.z.is_zero();
    let z1_2 = point.z.mul(point.z);
    let z2_2 = other.z.mul(other.z);
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
    let mut dy_double = two_a.mul(point.x).add(z1_2);
    dy_double = dy_double.mul(z1_2).add(point.x.mul(point.x));
    dy_double = dy_double
        .add(point.x.mul(point.x))
        .add(point.x.mul(point.x));
    dy_double = dy_double.mul(other.z);
    if dx.is_zero() && dy.is_zero() {
        dx = dx_double;
        dy = dy_double;
    }

    let z_product = point.z.mul(other.z);
    let z_product_2 = z_product.mul(z_product);
    let dx2 = dx.mul(dx);
    let dy2 = dy.mul(dy);
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

/// `(u, v, w)` with `P+Q = (u−v : w)` and `P−Q = (u+v : w)`.
pub fn add_components<F: Field>(curve: Montgomery<F>, point: Jac<F>, other: Jac<F>) -> (F, F, F) {
    let z1_2 = point.z.mul(point.z);
    let z2_2 = other.z.mul(other.z);
    let x1_z2_2 = point.x.mul(z2_2);
    let z1_2_x2 = z1_2.mul(other.x);
    let mut y1_z2_3 = point.y.mul(other.z);
    y1_z2_3 = y1_z2_3.mul(z2_2);
    let mut z1_3_y2 = point.z.mul(other.y);
    z1_3_y2 = z1_3_y2.mul(z1_2);
    let z_product_2 = z1_2.mul(z2_2);
    let yy = y1_z2_3.mul(z1_3_y2);
    let v = yy.add(yy);
    let y_sum = y1_z2_3.mul(y1_z2_3).add(z1_3_y2.mul(z1_3_y2));
    let x_sum = x1_z2_2.add(z1_2_x2);
    let lambda = x1_z2_2.sub(z1_2_x2);
    let lambda2 = lambda.mul(lambda);
    let gamma = curve.a.mul(z_product_2).add(x_sum);
    let u = y_sum.sub(gamma.mul(lambda2));
    let w = lambda2.mul(z_product_2);
    (u, v, w)
}

/// A square root of `x^3 + A x^2 + x`, or `None` when the point is not on the
/// curve. The root is the one [`Field::sqrt`] returns, which uses the same
/// even-coordinate rule as the reference.
pub fn recover_y<F: Field>(curve: Montgomery<F>, x: F) -> Option<F> {
    let x2 = x.mul(x);
    let rhs = curve.a.mul(x2).add(x).add(x2.mul(x));
    rhs.sqrt()
}

/// Lift `(P, Q, P−Q)` to Jacobian points, normalizing the curve and `P`.
///
/// This mutates the basis the way the reference does: `P.z` becomes 1 and the
/// curve coefficient becomes `A/C`. The returned curve is that normalization.
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
    let v3 = v3.mul(v3).mul(pmq.x);
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
        y: q_y.mul(q_z.mul(q_z)),
        z: q_z,
    };
    Some((curve, p, q))
}

/// x-only doubling iterated `times` times.
pub fn xdbl_iter<F: Field>(curve: Montgomery<F>, mut point: Xz<F>, times: u32) -> Xz<F> {
    for _ in 0..times {
        point = xdbl_ac(curve, point);
    }
    point
}

/// Jacobian doubling iterated `times` times.
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
            let rhs = x0.mul(x0).mul(x0).add(curve.a.mul(x0.mul(x0))).add(x0);
            if rhs.is_square() {
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
}
