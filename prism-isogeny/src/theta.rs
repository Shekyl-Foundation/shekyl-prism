// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// The theta arithmetic in this file is a port of
// `external/PRISM_v2/src/hd/ref/lvlx/theta_structure.c` and the inline
// transforms in `theta_structure.h` (Apache-2.0; Antonin Leroux and the
// SQIsign authors). See `NOTICE`.

//! Theta coordinates for a (2,2)-isogeny step.
//!
//! A theta point is four coordinates `(x : y : z : t)`. Doubling is the
//! squared-theta map of the C reference: square, Hadamard, scale by the
//! dual theta constants, Hadamard, scale by the primal constants.

use prism_field::Field;

/// One point of a theta structure.
#[derive(Clone, Copy, Debug)]
pub struct ThetaPoint<F> {
    pub x: F,
    pub y: F,
    pub z: F,
    pub t: F,
}

/// `(x+y+z+t, x−y+z−t, x+y−z−t, x−y−z+t)`.
pub fn hadamard<F: Field>(point: ThetaPoint<F>) -> ThetaPoint<F> {
    let sum_xy = point.x.add(point.y);
    let diff_xy = point.x.sub(point.y);
    let sum_zt = point.z.add(point.t);
    let diff_zt = point.z.sub(point.t);
    ThetaPoint {
        x: sum_xy.add(sum_zt),
        y: diff_xy.add(diff_zt),
        z: sum_xy.sub(sum_zt),
        t: diff_xy.sub(diff_zt),
    }
}

fn square_coords<F: Field>(point: ThetaPoint<F>) -> ThetaPoint<F> {
    ThetaPoint {
        x: point.x.square(),
        y: point.y.square(),
        z: point.z.square(),
        t: point.t.square(),
    }
}

/// Square each coordinate, then [`hadamard`].
pub fn to_squared_theta<F: Field>(point: ThetaPoint<F>) -> ThetaPoint<F> {
    hadamard(square_coords(point))
}

/// The eight scaling constants a doubling reads.
///
/// `dual_*` are the squared-theta constants of the null point. `primal_*`
/// are the same products of the null point itself. Names follow the C
/// structure: `YZT0` is the dual scale of `x`, `yzt0` the primal scale of `x`.
#[derive(Clone, Copy, Debug)]
pub struct ThetaConstants<F> {
    pub dual_x: F,
    pub dual_y: F,
    pub dual_z: F,
    pub dual_t: F,
    pub primal_x: F,
    pub primal_y: F,
    pub primal_z: F,
    pub primal_t: F,
}

impl<F: Field> ThetaPoint<F> {
    pub fn from_coords(coords: [F; 4]) -> Self {
        Self {
            x: coords[0],
            y: coords[1],
            z: coords[2],
            t: coords[3],
        }
    }

    pub fn coords(self) -> [F; 4] {
        [self.x, self.y, self.z, self.t]
    }

    /// Coordinate `index` modulo 4, the selection `choose_index_theta_point` makes.
    pub fn coord(self, index: usize) -> F {
        self.coords()[index % 4]
    }
}

/// Two theta points name the same point when one is a scalar multiple of the other.
pub fn projectively_same<F: Field>(left: ThetaPoint<F>, right: ThetaPoint<F>) -> bool {
    let left = left.coords();
    let right = right.coords();
    if left.iter().all(|v| v.is_zero()) || right.iter().all(|v| v.is_zero()) {
        return false;
    }
    (0..4).all(|i| (0..4).all(|j| left[i].mul(right[j]).same(left[j].mul(right[i]))))
}

impl<F: Field> ThetaConstants<F> {
    /// Precompute from the null point, as `theta_precomputation` does.
    pub fn from_null(null_point: ThetaPoint<F>) -> Self {
        let dual = to_squared_theta(null_point);
        let dual_xy = dual.x.mul(dual.y);
        let dual_zt = dual.z.mul(dual.t);
        let primal_xy = null_point.x.mul(null_point.y);
        let primal_zt = null_point.z.mul(null_point.t);
        Self {
            dual_x: dual_zt.mul(dual.y),
            dual_y: dual_zt.mul(dual.x),
            dual_z: dual_xy.mul(dual.t),
            dual_t: dual_xy.mul(dual.z),
            primal_x: primal_zt.mul(null_point.y),
            primal_y: primal_zt.mul(null_point.x),
            primal_z: primal_xy.mul(null_point.t),
            primal_t: primal_xy.mul(null_point.z),
        }
    }
}

fn scale_with<F: Field>(point: ThetaPoint<F>, x: F, y: F, z: F, t: F) -> ThetaPoint<F> {
    ThetaPoint {
        x: point.x.mul(x),
        y: point.y.mul(y),
        z: point.z.mul(z),
        t: point.t.mul(t),
    }
}

/// `[2] point` on the theta structure with these constants.
pub fn double_point<F: Field>(constants: ThetaConstants<F>, point: ThetaPoint<F>) -> ThetaPoint<F> {
    let squared = square_coords(to_squared_theta(point));
    let scaled = scale_with(
        squared,
        constants.dual_x,
        constants.dual_y,
        constants.dual_z,
        constants.dual_t,
    );
    scale_with(
        hadamard(scaled),
        constants.primal_x,
        constants.primal_y,
        constants.primal_z,
        constants.primal_t,
    )
}

/// `[2^exp] point`.
pub fn double_iter<F: Field>(
    constants: ThetaConstants<F>,
    mut point: ThetaPoint<F>,
    exp: u32,
) -> ThetaPoint<F> {
    for _ in 0..exp {
        point = double_point(constants, point);
    }
    point
}

/// A product theta point satisfies `x t = y z`.
pub fn is_product<F: Field>(point: ThetaPoint<F>) -> bool {
    point.x.mul(point.t).same(point.y.mul(point.z))
}

#[cfg(test)]
mod tests {
    use super::{double_iter, hadamard, is_product, ThetaConstants, ThetaPoint};
    use prism_field::Field;

    #[derive(Clone, Copy, Debug)]
    struct Toy(i64);
    const P: i64 = 1019;

    impl PartialEq for Toy {
        fn eq(&self, other: &Self) -> bool {
            self.norm() == other.norm()
        }
    }
    impl Eq for Toy {}

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
            if self.norm() == 0 {
                None
            } else {
                Some(Self(mod_pow(self.norm(), P - 2)))
            }
        }
        fn sqrt(self) -> Option<Self> {
            if !self.is_square() {
                return None;
            }
            Some(Self(mod_pow(self.norm(), (P + 1) / 4)))
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
        let mut out = 1i64;
        base = base.rem_euclid(P);
        while exp > 0 {
            if exp & 1 == 1 {
                out = (out * base).rem_euclid(P);
            }
            base = (base * base).rem_euclid(P);
            exp >>= 1;
        }
        out
    }

    #[test]
    fn zero_coordinates_are_not_a_projective_point() {
        let invalid = ThetaPoint::from_coords([Toy::zero(); 4]);
        let valid = ThetaPoint::from_coords([Toy::one(); 4]);
        assert!(!super::projectively_same(invalid, valid));
        assert!(!super::projectively_same(valid, invalid));
        assert!(!super::projectively_same(invalid, invalid));
    }

    #[test]
    fn hadamard_twice_scales_by_four() {
        let point = ThetaPoint {
            x: Toy::from_u64(3),
            y: Toy::from_u64(5),
            z: Toy::from_u64(7),
            t: Toy::from_u64(11),
        };
        let twice = hadamard(hadamard(point));
        let four = Toy::from_u64(4);
        assert!(twice.x.same(point.x.mul(four)));
        assert!(twice.y.same(point.y.mul(four)));
        assert!(twice.z.same(point.z.mul(four)));
        assert!(twice.t.same(point.t.mul(four)));
    }

    #[test]
    fn a_product_point_has_equal_cross_ratios() {
        let point = ThetaPoint {
            x: Toy::from_u64(2),
            y: Toy::from_u64(3),
            z: Toy::from_u64(4),
            t: Toy::from_u64(6),
        };
        assert!(is_product(point));
        assert!(!is_product(ThetaPoint {
            t: Toy::from_u64(7),
            ..point
        }));
    }

    #[test]
    fn doubling_zero_times_is_the_point() {
        let null = ThetaPoint {
            x: Toy::one(),
            y: Toy::from_u64(2),
            z: Toy::from_u64(3),
            t: Toy::from_u64(4),
        };
        let constants = ThetaConstants::from_null(null);
        let point = ThetaPoint {
            x: Toy::from_u64(8),
            y: Toy::from_u64(9),
            z: Toy::from_u64(10),
            t: Toy::from_u64(11),
        };
        let same = double_iter(constants, point, 0);
        assert!(same.x.same(point.x));
        assert!(same.t.same(point.t));
    }
}
