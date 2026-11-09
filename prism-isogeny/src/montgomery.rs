// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! x-only arithmetic on a Montgomery curve `y^2 = x^3 + A x^2 + x`.
//!
//! Points are projective `(X : Z)`. The curve constant the differential
//! formulas consume is `(A + 2) / 4`.

use prism_field::Field;

/// `(A + 2) / 4` for the Montgomery coefficient `A`.
pub fn a24<F: Field>(coefficient: F) -> Option<F> {
    F::from_u64(4)
        .inv()
        .map(|inv| coefficient.add(F::from_u64(2)).mul(inv))
}

/// A point in projective x-coordinates. `Z = 0` is the point at infinity.
#[derive(Clone, Copy, Debug)]
pub struct Xz<F> {
    pub x: F,
    pub z: F,
}

impl<F: Field> Xz<F> {
    pub fn affine(x: F) -> Self {
        Self { x, z: F::one() }
    }

    pub fn infinity() -> Self {
        Self {
            x: F::one(),
            z: F::zero(),
        }
    }

    pub fn is_infinity(self) -> bool {
        self.z.is_zero()
    }

    /// The affine x-coordinate, when the point is finite.
    pub fn x_affine(self) -> Option<F> {
        self.z.inv().map(|inv| self.x.mul(inv))
    }
}

/// Montgomery curve `C y^2 = C x^3 + A x^2 + C x`, stored as `(A : C)`.
#[derive(Clone, Copy, Debug)]
pub struct Montgomery<F> {
    pub a: F,
    pub c: F,
}

impl<F: Field> Montgomery<F> {
    pub fn normalized(coefficient: F) -> Self {
        Self {
            a: coefficient,
            c: F::one(),
        }
    }

    /// `(A/C : 1)`, the form Jacobian arithmetic assumes.
    pub fn affine_coefficient(self) -> Option<Self> {
        self.c.inv().map(|inv| Self {
            a: self.a.mul(inv),
            c: F::one(),
        })
    }
}

/// Doubling for a projective coefficient `(A : C)`, the reference `xDBL`.
pub fn xdbl_ac<F: Field>(curve: Montgomery<F>, point: Xz<F>) -> Xz<F> {
    let sum = point.x.add(point.z);
    let diff = point.x.sub(point.z);
    let sum2 = sum.mul(sum);
    let diff2 = diff.mul(diff);
    let four_xz = sum2.sub(diff2);
    let two_c = curve.c.add(curve.c);
    let scaled = diff2.mul(two_c);
    let scaled = scaled.add(scaled);
    let mixed = two_c.add(curve.a).mul(four_xz).add(scaled);
    Xz {
        x: sum2.mul(scaled),
        z: mixed.mul(four_xz),
    }
}

/// Doubling on the curve with constant [`a24`].
pub fn xdbl<F: Field>(a24: F, point: Xz<F>) -> Xz<F> {
    let sum = point.x.add(point.z);
    let diff = point.x.sub(point.z);
    let sum2 = sum.mul(sum);
    let diff2 = diff.mul(diff);
    let four_xz = sum2.sub(diff2);
    Xz {
        x: sum2.mul(diff2),
        z: four_xz.mul(diff2.add(a24.mul(four_xz))),
    }
}

/// Differential addition: `point + other`, given `difference = point - other`.
pub fn xadd<F: Field>(difference: Xz<F>, point: Xz<F>, other: Xz<F>) -> Xz<F> {
    let point_sum = point.x.add(point.z);
    let point_diff = point.x.sub(point.z);
    let other_sum = other.x.add(other.z);
    let other_diff = other.x.sub(other.z);
    let left = other_diff.mul(point_sum).add(other_sum.mul(point_diff));
    let right = other_diff.mul(point_sum).sub(other_sum.mul(point_diff));
    Xz {
        x: difference.z.mul(left.mul(left)),
        z: difference.x.mul(right.mul(right)),
    }
}

/// `[scalar] point`, scalar little-endian bits, high bit first.
pub fn ladder<F: Field>(
    a24: F,
    mut scalar_bits_msb_first: impl Iterator<Item = bool>,
    point: Xz<F>,
) -> Xz<F> {
    let mut x0 = Xz::infinity();
    let mut x1 = point;
    for bit in scalar_bits_msb_first.by_ref() {
        let (closer, farther) = if bit { (x1, x0) } else { (x0, x1) };
        let sum = xadd(point, closer, farther);
        let dbl = xdbl(a24, closer);
        if bit {
            x0 = sum;
            x1 = dbl;
        } else {
            x0 = dbl;
            x1 = sum;
        }
    }
    x0
}

#[cfg(test)]
mod tests {
    use super::{a24, ladder, xdbl, Xz};
    use prism_field::Field;

    #[derive(Clone, Copy, Debug)]
    struct Toy(u64);
    const P: u64 = 1019;

    impl PartialEq for Toy {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }
    impl Eq for Toy {}

    impl Field for Toy {
        fn zero() -> Self {
            Self(0)
        }
        fn one() -> Self {
            Self(1)
        }
        fn from_u64(v: u64) -> Self {
            Self(v % P)
        }
        fn add(self, rhs: Self) -> Self {
            Self((self.0 + rhs.0) % P)
        }
        fn sub(self, rhs: Self) -> Self {
            Self((self.0 + P - rhs.0 % P) % P)
        }
        fn mul(self, rhs: Self) -> Self {
            Self(((self.0 as u128 * rhs.0 as u128) % P as u128) as u64)
        }
        fn negate(self) -> Self {
            Self((P - self.0 % P) % P)
        }
        fn inv(self) -> Option<Self> {
            if self.0.is_multiple_of(P) {
                None
            } else {
                Some(Self(pow(self.0, P - 2)))
            }
        }
        fn sqrt(self) -> Option<Self> {
            if !self.is_square() {
                return None;
            }
            Some(Self(pow(self.0, (P + 1) / 4)))
        }
        fn is_square(self) -> bool {
            self.0.is_multiple_of(P) || pow(self.0, (P - 1) / 2) == 1
        }
        fn is_zero(self) -> bool {
            self.0.is_multiple_of(P)
        }
        fn same(self, rhs: Self) -> bool {
            self.0 % P == rhs.0 % P
        }
    }

    fn pow(mut base: u64, mut exp: u64) -> u64 {
        let mut out = 1u64;
        base %= P;
        while exp > 0 {
            if exp & 1 == 1 {
                out = ((out as u128 * base as u128) % P as u128) as u64;
            }
            base = ((base as u128 * base as u128) % P as u128) as u64;
            exp >>= 1;
        }
        out
    }

    fn rhs(a: Toy, x: Toy) -> Toy {
        x.mul(x.mul(x).add(a.mul(x)).add(Toy::one()))
    }

    fn affine_double(a: Toy, x: Toy, y: Toy) -> Toy {
        let slope = x
            .mul(x)
            .mul(Toy::from_u64(3))
            .add(a.mul(x).mul(Toy::from_u64(2)))
            .add(Toy::one())
            .mul(y.mul(Toy::from_u64(2)).inv().unwrap());
        slope.mul(slope).sub(a).sub(x).sub(x)
    }

    #[test]
    fn doubling_matches_the_affine_group_law() {
        let a = Toy::from_u64(5);
        let constant = a24(a).unwrap();
        let mut saw = false;
        for x0 in 1..40u64 {
            let x = Toy::from_u64(x0);
            let right = rhs(a, x);
            let Some(y) = right.sqrt() else { continue };
            if y.is_zero() {
                continue;
            }
            let doubled = xdbl(constant, Xz::affine(x));
            let got = doubled.x_affine().unwrap();
            assert!(got.same(affine_double(a, x, y)), "x = {x0}");
            saw = true;
            break;
        }
        assert!(saw);
    }

    #[test]
    fn the_ladder_is_repeated_doubling() {
        let a = Toy::from_u64(5);
        let constant = a24(a).unwrap();
        let mut x = None;
        for x0 in 1..40u64 {
            let candidate = Toy::from_u64(x0);
            if rhs(a, candidate).is_square() {
                x = Some(candidate);
                break;
            }
        }
        let point = Xz::affine(x.expect("a point"));
        let acc = xdbl(constant, xdbl(constant, point));
        // 4 = 0b100.
        let scaled = ladder(constant, [true, false, false].into_iter(), point);
        assert!(acc.x_affine().unwrap().same(scaled.x_affine().unwrap()));
    }
}
