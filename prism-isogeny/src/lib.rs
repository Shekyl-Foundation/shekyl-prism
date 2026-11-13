// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Montgomery curves and the 2-isogeny step a (2,2) chain is built from.
//!
//! A split (2,2)-isogeny is a pair of Vélu 2-isogenies, one on each factor.
//! That is the kernel which is already a product of torsion subgroups. The
//! glued theta chain in the C reference
//! (`theta_chain_compute_and_eval_verify`) is the differential target.
//! [`theta_step`] is one step of that chain. [`chain`] runs the steps and,
//! on the salt-PRISM path, splits the codomain. [`chain::chain_surface`]
//! stops before that split. [`dual_chain`] supplies the full-point adapted
//! chains and their duals used by prime-degree signature verification.

#![forbid(unsafe_code)]

pub mod chain;
pub mod dual_chain;
pub mod endomorphism;
pub mod gluing;
pub mod jac;
pub mod modular;
pub mod montgomery;
pub mod pairing;
pub mod splitting;
pub mod theta;
pub mod theta_base;
pub mod theta_step;
pub mod torsion;

use prism_field::Field;

/// Short Weierstrass `y^2 = x^3 + a4 x + a6`.
#[derive(Clone, Copy, Debug)]
pub struct Curve<F> {
    pub a4: F,
    pub a6: F,
}

/// Affine point. The point at infinity is `None` from the isogeny map.
#[derive(Clone, Copy, Debug)]
pub struct Point<F> {
    pub x: F,
    pub y: F,
}

impl<F: Field> Curve<F> {
    /// Montgomery `y^2 = x^3 + A x^2 + x`, translated by `X = x + A/3`.
    pub fn from_montgomery(coefficient: F) -> Option<Self> {
        let three = F::from_u64(3);
        let three_inv = three.inv()?;
        let shift = coefficient.mul(three_inv);
        let shift2 = shift.square();
        let shift3 = shift2.mul(shift);
        // a4 = 3 (A/3)^2 + A (-2) (A/3) + 1
        let a4 = three
            .mul(shift2)
            .add(coefficient.mul(F::from_u64(2).negate()).mul(shift))
            .add(F::one());
        // a6 = -(A/3)^3 + A (A/3)^2 - A/3
        let a6 = shift3.negate().add(coefficient.mul(shift2)).sub(shift);
        Some(Self { a4, a6 })
    }

    pub fn contains(self, point: Point<F>) -> bool {
        let y2 = point.y.square();
        let x2 = point.x.square();
        let rhs = point.x.mul(x2).add(self.a4.mul(point.x)).add(self.a6);
        y2.same(rhs)
    }

    /// The Montgomery point `(0, 0)` after the translation `X = x + A/3`.
    pub fn kernel_at_zero(coefficient: F) -> Option<Point<F>> {
        let three = F::from_u64(3).inv()?;
        Some(Point {
            x: coefficient.mul(three),
            y: F::zero(),
        })
    }
}

impl<F: Field> Point<F> {
    pub fn negated(self) -> Self {
        Self {
            x: self.x,
            y: self.y.negate(),
        }
    }
}

/// 2-isogeny with kernel a rational point of order 2 (`y = 0`).
///
/// Vélu: `t = 3 x0^2 + a4`, `a4' = a4 − 5 t`, `a6' = a6 − 7 x0 t`,
/// `x' = x + t / (x − x0)`, `y' = y (1 − t / (x − x0)^2)`.
/// The kernel is sent to the point at infinity.
pub fn two_isogeny<F: Field>(domain: Curve<F>, kernel: Point<F>) -> Option<Iso<F>> {
    if !kernel.y.is_zero() || !domain.contains(kernel) {
        return None;
    }
    let x0 = kernel.x;
    let t = F::from_u64(3).mul(x0.square()).add(domain.a4);
    let codomain = Curve {
        a4: domain.a4.sub(F::from_u64(5).mul(t)),
        a6: domain.a6.sub(F::from_u64(7).mul(x0).mul(t)),
    };
    Some(Iso {
        domain,
        codomain,
        x0,
        t,
    })
}

/// One evaluated 2-isogeny.
#[derive(Clone, Copy, Debug)]
pub struct Iso<F> {
    domain: Curve<F>,
    codomain: Curve<F>,
    x0: F,
    t: F,
}

impl<F: Field> Iso<F> {
    pub fn codomain(self) -> Curve<F> {
        self.codomain
    }

    /// `None` is the point at infinity, which is where the kernel lands.
    pub fn image(self, point: Point<F>) -> Option<Point<F>> {
        if !self.domain.contains(point) {
            return None;
        }
        let gap = point.x.sub(self.x0);
        let gap_inv = gap.inv()?;
        let x = point.x.add(self.t.mul(gap_inv));
        let gap2_inv = gap_inv.square();
        let scale = F::one().sub(self.t.mul(gap2_inv));
        Some(Point {
            x,
            y: point.y.mul(scale),
        })
    }
}

/// A split (2,2)-isogeny: one 2-isogeny on each factor.
pub struct SplitTwoTwo<F> {
    pub left: Iso<F>,
    pub right: Iso<F>,
}

impl<F: Field> SplitTwoTwo<F> {
    pub fn new(
        left: Curve<F>,
        left_kernel: Point<F>,
        right: Curve<F>,
        right_kernel: Point<F>,
    ) -> Option<Self> {
        Some(Self {
            left: two_isogeny(left, left_kernel)?,
            right: two_isogeny(right, right_kernel)?,
        })
    }
}

/// `steps` successive 2-isogenies, the factor a split (2,2) chain repeats.
///
/// The walk starts at the Montgomery curve with coefficient `start`. The first
/// kernel is `(0, 0)`. `siblings` are the other two 2-torsion x-coordinates,
/// in Montgomery `x`, used only to name a point the first step can push.
/// A 2-isogeny sends both of those points to the same codomain point, the
/// kernel of the dual. The next kernel is one of the other two roots of the
/// codomain's 2-division polynomial, so the walk does not step back.
pub fn split_chain<F: Field>(start: F, siblings: [F; 2], steps: u32) -> Option<Curve<F>> {
    let mut curve = Curve::from_montgomery(start)?;
    if steps == 0 {
        return Some(curve);
    }
    let shift = start.mul(F::from_u64(3).inv()?);
    let mut kernel = Point {
        x: shift,
        y: F::zero(),
    };
    let mut spare = Point {
        x: siblings[0].add(shift),
        y: F::zero(),
    };
    for i in 0..steps {
        if !curve.contains(kernel) {
            return None;
        }
        let iso = two_isogeny(curve, kernel)?;
        curve = iso.codomain();
        if i + 1 == steps {
            break;
        }
        let dual = iso.image(spare)?;
        let (left, _right) = other_torsion(curve, dual.x)?;
        kernel = Point {
            x: left,
            y: F::zero(),
        };
        spare = Point {
            x: _right,
            y: F::zero(),
        };
    }
    Some(curve)
}

/// The two 2-torsion x-coordinates other than `known` on a short-Weierstrass
/// curve whose cubic is `x^3 + a4 x + a6`.
fn other_torsion<F: Field>(curve: Curve<F>, known: F) -> Option<(F, F)> {
    let delta = F::from_u64(3)
        .mul(known.square())
        .negate()
        .sub(F::from_u64(4).mul(curve.a4));
    let root = delta.sqrt()?;
    let half = F::from_u64(2).inv()?;
    let base = known.negate();
    let left = base.add(root).mul(half);
    let right = base.sub(root).mul(half);
    if left.same(known) || right.same(known) || left.same(right) {
        return None;
    }
    let on = |x: F| curve.contains(Point { x, y: F::zero() });
    if !on(left) || !on(right) {
        return None;
    }
    Some((left, right))
}

#[cfg(test)]
mod tests {
    use super::{split_chain, two_isogeny, Curve, Point, SplitTwoTwo};
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
            Self((self.0 + P - rhs.0) % P)
        }
        fn square(self) -> Self {
            self.mul(self)
        }
        fn mul(self, rhs: Self) -> Self {
            Self(((self.0 as u128 * rhs.0 as u128) % P as u128) as u64)
        }
        fn negate(self) -> Self {
            Self((P - self.0) % P)
        }
        fn inv(self) -> Option<Self> {
            if self.0 == 0 {
                None
            } else {
                Some(Self(mod_pow(self.0, P - 2)))
            }
        }
        fn sqrt(self) -> Option<Self> {
            if self.0 == 0 {
                return Some(Self(0));
            }
            if !self.is_square() {
                return None;
            }
            Some(Self(mod_pow(self.0, (P + 1) / 4)))
        }
        fn is_square(self) -> bool {
            self.0 == 0 || mod_pow(self.0, (P - 1) / 2) == 1
        }
        fn is_zero(self) -> bool {
            self.0 == 0
        }
        fn same(self, rhs: Self) -> bool {
            self.0 == rhs.0
        }
    }

    fn mod_pow(mut base: u64, mut exp: u64) -> u64 {
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

    fn point_on_curve() -> (Curve<Toy>, Point<Toy>) {
        // alpha = 3, A = -(alpha + 1/alpha), the setup checked by hand.
        let alpha = Toy::from_u64(3);
        let coefficient = alpha.add(alpha.inv().unwrap()).negate();
        let curve = Curve::from_montgomery(coefficient).unwrap();
        let kernel = Curve::kernel_at_zero(coefficient).unwrap();
        // Use the order-2 point at x = alpha, translated, which was the
        // hand check. kernel_at_zero is (0, 0). Either is valid; this test
        // uses alpha so it matches the independent Vélu check.
        let kernel = Point {
            x: alpha.add(kernel.x),
            y: Toy::zero(),
        };
        assert!(curve.contains(kernel));
        (curve, kernel)
    }

    #[test]
    fn kernel_lands_at_infinity_and_a_point_lands_on_the_codomain() {
        let (curve, kernel) = point_on_curve();
        let iso = two_isogeny(curve, kernel).unwrap();
        assert!(iso.image(kernel).is_none());
        let source = Point {
            x: Toy::from_u64(2).add(Curve::kernel_at_zero(Toy::zero()).unwrap().x),
            y: Toy::zero(),
        };
        // (2, y) may not lie on this curve. Search a small x that does.
        let mut found = None;
        for x in 1..80u64 {
            let point = Point {
                x: Toy::from_u64(x),
                y: Toy::zero(),
            };
            if curve.contains(point) && !point.x.same(kernel.x) {
                found = Some(point);
                break;
            }
            let y2 = {
                let x = Toy::from_u64(x);
                x.square().mul(x).add(curve.a4.mul(x)).add(curve.a6)
            };
            let leg = mod_pow(y2.0, (P - 1) / 2);
            if leg == 1 {
                let y = Toy(mod_pow(y2.0, (P + 1) / 4));
                let point = Point {
                    x: Toy::from_u64(x),
                    y,
                };
                if curve.contains(point) {
                    found = Some(point);
                    break;
                }
            }
        }
        let point = found.expect("a rational point");
        let image = iso.image(point).expect("not the kernel");
        assert!(iso.codomain().contains(image));
        let _ = source;
    }

    #[test]
    fn a_chain_walks_past_the_dual_kernel() {
        let alpha = Toy::from_u64(3);
        let coefficient = alpha.add(alpha.inv().unwrap()).negate();
        let delta = coefficient.square().sub(Toy::from_u64(4));
        let root = delta.sqrt().expect("2-torsion discriminant");
        let half = Toy::from_u64(2).inv().unwrap();
        let siblings = [
            coefficient.negate().add(root).mul(half),
            coefficient.negate().sub(root).mul(half),
        ];
        let one = split_chain(coefficient, siblings, 1).expect("one step");
        let kernel = Curve::kernel_at_zero(coefficient).unwrap();
        let iso = two_isogeny(Curve::from_montgomery(coefficient).unwrap(), kernel).unwrap();
        assert!(one.a4.same(iso.codomain().a4));
        assert!(one.a6.same(iso.codomain().a6));
        // Two steps is the one that used to die: both siblings land on the
        // dual kernel. Later discriminants need not be squares in this prime
        // field; the NIST walk is over F_{p^2}.
        let two = split_chain(coefficient, siblings, 2).expect("two steps");
        assert!(!(two.a4.same(one.a4) && two.a6.same(one.a6)));
    }

    #[test]
    fn level_i_chain_walks_eight_steps() {
        use prism_field::{Level, Level1};
        let start = <Level1 as Level>::ext_from_base(<Level1 as Level>::fp_zero());
        let i = <Level1 as Level>::ext_i();
        let walked = split_chain(start, [i, i.negate()], 8).expect("eight steps");
        assert!(!(walked.a4.is_zero() && walked.a6.is_zero()));
    }

    #[test]
    fn a_split_two_two_is_a_pair_of_two_isogenies() {
        let (curve, kernel) = point_on_curve();
        let split = SplitTwoTwo::new(curve, kernel, curve, kernel).unwrap();
        let alone = two_isogeny(curve, kernel).unwrap();
        assert!(split.left.codomain().a4.same(alone.codomain().a4));
        assert!(split.right.codomain().a6.same(alone.codomain().a6));
    }
}
