// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Short-Weierstrass curves and the Vélu 2-isogeny, as a model of a
//! split (2,2)-isogeny.
//!
//! This was the crate's first content, and it is what the engine was
//! first imagined as: a (2,2)-isogeny of a product of two curves whose
//! kernel is itself a product, one point of order 2 on each curve, is a
//! pair of 2-isogenies, one on each factor ([`SplitTwoTwo`]), and a chain
//! of them repeats that step ([`split_chain`]). It is small enough to
//! check by hand, and it is tested over a field of 1,019 elements and at
//! `p248`.
//!
//! # What it is not
//!
//! It is not what the verifier computes, and no result of it is compared
//! with anything the verifier computes. The isogenies of a verification
//! are between products but their kernels are not products, which is why
//! they are computed in theta coordinates ([`crate::dual_chain`]). A
//! split isogeny tells nothing about a signature.
//!
//! Nothing outside this file calls it, in this crate or any other, so no
//! check in `prism-verify` establishes any assumption stated below: a
//! caller establishes each one itself.
//!
//! # Where it is compiled
//!
//! For this crate's tests and under the feature `oracle`, and not
//! otherwise. A default build does not contain it. It is kept for a
//! reader who wants to use or check it.
//! `scripts/ci/check_oracle_isolation.py` holds the feature out of the
//! default feature graph of the verifier and of the front crate, and out
//! of every tracked manifest but the fuzz crate's.
//!
//! # Where this is in the paper
//!
//! Nowhere. ePrint 2026/443 has no section this models. Its only mention
//! of Vélu's formulas is in section 4.4, as the cost of an attack that
//! computes an isogeny of large prime degree, and it has no section for
//! short-Weierstrass curves or for isogenies of degree 2. The formulas
//! here are those of the paper's reference \[65\] (Vélu, 1971) for a
//! kernel of order 2. The other papers in `docs/papers` were not
//! searched.
//!
//! # What nothing here tests
//!
//! That a curve is nonsingular (`4·a4³ + 27·a6² ≠ 0`), and that the
//! field's characteristic is not 2 or 3. On a singular cubic the
//! formulas still return values, and those values are not an isogeny of
//! elliptic curves: a wrong result that looks like a right one.
//!
//! Variable time, for public values only.

use prism_field::Field;

/// The short-Weierstrass curve `y² = x³ + a4·x + a6`.
///
/// Plain data with no invariant: the fields are public and nothing
/// checks that the cubic has three distinct roots, so a singular cubic
/// can be written.
#[derive(Clone, Copy, Debug)]
pub struct Curve<F> {
    /// The coefficient of `x`.
    pub a4: F,
    /// The constant term.
    pub a6: F,
}

/// An affine point `(x, y)`.
///
/// Plain data with no invariant: the fields are public and nothing
/// checks that the point is on any curve ([`Curve::contains`] does, when
/// asked). The point at infinity cannot be written; where a function
/// here would return it, it returns `None`.
#[derive(Clone, Copy, Debug)]
pub struct Point<F> {
    /// The abscissa.
    pub x: F,
    /// The ordinate.
    pub y: F,
}

impl<F: Field> Curve<F> {
    /// The Montgomery curve `y² = x³ + A·x² + x` in short-Weierstrass
    /// form, by the translation `X = x + A/3`:
    /// `a4 = 1 − A²/3`, `a6 = 2A³/27 − A/3`.
    ///
    /// `None` only where 3 has no inverse. That the Montgomery curve is
    /// nonsingular (`A² ≠ 4`) is not tested; a singular one gives a
    /// singular cubic.
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

    /// Whether `point` satisfies the curve's equation. [`two_isogeny`]
    /// makes this test of its kernel and [`Iso::image`] of its point, on
    /// every call.
    pub fn contains(self, point: Point<F>) -> bool {
        let y2 = point.y.square();
        let x2 = point.x.square();
        let rhs = point.x.mul(x2).add(self.a4.mul(point.x)).add(self.a6);
        y2.same(rhs)
    }

    /// The point `(0, 0)` of the Montgomery curve with coefficient
    /// `coefficient`, after the translation of [`Self::from_montgomery`]:
    /// `(A/3, 0)`, a point of order 2.
    ///
    /// `None` only where 3 has no inverse.
    pub fn kernel_at_zero(coefficient: F) -> Option<Point<F>> {
        let three = F::from_u64(3).inv()?;
        Some(Point {
            x: coefficient.mul(three),
            y: F::zero(),
        })
    }
}

/// The 2-isogeny from `domain` whose kernel is generated by `kernel`, a
/// point of order 2.
///
/// Vélu, for the kernel point `(x0, 0)`: `t = 3·x0² + a4`,
/// `a4' = a4 − 5t`, `a6' = a6 − 7·x0·t`, and a point maps by
/// `x' = x + t / (x − x0)`, `y' = y·(1 − t / (x − x0)²)`. The kernel
/// point goes to the point at infinity.
///
/// # What it assumes
///
/// `domain` is nonsingular. Not tested.
///
/// # When that fails
///
/// On a singular cubic the result is a well-formed [`Iso`] that is not
/// an isogeny, and nothing downstream shows it.
///
/// # What it tests
///
/// `None` unless `kernel` has `y = 0` and is on `domain`. On a
/// nonsingular curve those two together are exactly "a point of order
/// 2", so a kernel that is not one is refused.
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

/// One 2-isogeny: its domain, its codomain, and what [`Self::image`]
/// needs to evaluate it.
///
/// The fields are private and [`two_isogeny`] is the only way to build
/// one, so a value's kernel point was on its domain with `y = 0` and its
/// codomain is the one Vélu's formulas give for that kernel. The
/// invariant goes no further: the domain was not tested for
/// singularity.
#[derive(Clone, Copy, Debug)]
pub struct Iso<F> {
    domain: Curve<F>,
    codomain: Curve<F>,
    x0: F,
    t: F,
}

impl<F: Field> Iso<F> {
    /// The curve the isogeny maps to.
    pub fn codomain(self) -> Curve<F> {
        self.codomain
    }

    /// The image of `point`, a point of the codomain.
    ///
    /// `None` in two cases the caller cannot tell apart from the value:
    /// `point` is not on the domain, or `point` is the kernel point,
    /// whose image is the point at infinity. A caller that needs to know
    /// which tests [`Curve::contains`] on the domain first.
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

/// A split (2,2)-isogeny of a product of two curves: one 2-isogeny on
/// each factor.
///
/// Plain data with no invariant of its own: the fields are public, and
/// each is an [`Iso`] with that type's invariant. Nothing relates the
/// two.
pub struct SplitTwoTwo<F> {
    /// The 2-isogeny on the first factor.
    pub left: Iso<F>,
    /// The 2-isogeny on the second factor.
    pub right: Iso<F>,
}

impl<F: Field> SplitTwoTwo<F> {
    /// The pair of [`two_isogeny`] on `left` and on `right`. `None` when
    /// either refuses its kernel; the assumptions are those of
    /// [`two_isogeny`], once for each factor.
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

/// The codomain of `steps` successive 2-isogenies from the Montgomery
/// curve with coefficient `start`: the walk one factor of a chain of
/// split (2,2)-isogenies makes.
///
/// The first kernel is the Montgomery point `(0, 0)`. A 2-isogeny sends
/// the other two points of order 2 to one point of its codomain, the
/// kernel of its dual. Each later kernel is one of the two points of
/// order 2 of the current curve other than that one, so the walk never
/// steps back. With `steps = 0` the result is `start` itself in
/// short-Weierstrass form.
///
/// The loop runs `steps` times, a count the caller gives, and ends on
/// nothing computed.
///
/// # What it assumes
///
/// - The curve is nonsingular. Not tested.
/// - `siblings[0]` is the Montgomery x-coordinate of a point of order 2
///   of the starting curve other than `(0, 0)`. Only `siblings[0]` is
///   read, and it affects the result only when `steps` is 2 or more;
///   `siblings[1]` is never read.
///
/// # When that fails
///
/// A `siblings[0]` that is not a point of order 2 of the curve gives
/// `None` at the second step, where its image is asked for and it is
/// found off the curve. One that is `0` gives `None` there too, being
/// the kernel. A singular starting curve gives values with no meaning
/// and no sign of it.
///
/// # When it returns `None` on good inputs
///
/// When 2 or 3 has no inverse, and when a curve the walk reaches does
/// not have its three points of order 2 over `F`: the discriminant of
/// the remaining quadratic is then not a square. Over the extension
/// field of `p248`, from the starting curve `A = 0`, a test below walks
/// eight steps.
///
/// # The walk depends on the field's square root
///
/// Of the two candidates for the next kernel, the one taken is
/// `(−x + r) / 2` for the root `r` that [`Field::sqrt`] returns, where
/// `x` is the abscissa of the dual's kernel. With the other root the
/// walk takes the other candidate, and from the second step on it
/// returns another equation. Both are walks that do not step back. From
/// `A = 0` the second step gives `y² = x³ − 44x − 112` with the root 4 of
/// 16 and `y² = x³ − 44x + 112` with the root −4: two equations, of
/// curves that are isomorphic (by `x → −x`), so a different equation is
/// not always a different curve. A test below holds the first of them
/// at `p248`, where `prism-field` promises the root 4. For the extension
/// field of a parameter set the walk is determined by that promise; for
/// another implementation of [`Field`] it is whatever that
/// implementation returns. Nothing compares the walk with a reference.
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
    // The pair that leaves this function depends on which root this is:
    // the other one exchanges `left` and `right`, and `split_chain` then
    // takes the other of the two next kernels and reaches another curve.
    // Nothing compares the walk with a reference, so the choice is that
    // of the caller's `Field::sqrt`, as `split_chain` documents; the test
    // below holds it for the extension field of `p248`.
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
        // Either root: the two give the siblings in the other order, and a
        // 2-isogeny sends both siblings to the same point.
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
        // field; the walk at a parameter set is over F_{p^2}.
        let two = split_chain(coefficient, siblings, 2).expect("two steps");
        assert!(!(two.a4.same(one.a4) && two.a6.same(one.a6)));
    }

    #[test]
    fn p248_chain_walks_eight_steps() {
        use prism_field::{ParameterSet, P248};
        let start = <P248 as ParameterSet>::Ext::zero();
        let i = <P248 as ParameterSet>::ext_i();
        let walked = split_chain(start, [i, i.negate()], 8).expect("eight steps");
        assert!(!(walked.a4.is_zero() && walked.a6.is_zero()));
    }

    /// From `y^2 = x^3 + x` the first step, with kernel `(0, 0)`, gives
    /// `y^2 = x^3 - 4x`, and both `(±i, 0)` land on `(0, 0)` there. The
    /// other two points of order 2 are `(±2, 0)`, from the roots `±4` of
    /// 16. The extension field's rule names 4, whose real part is even,
    /// so the second kernel is `(2, 0)` and the second curve is
    /// `y^2 = x^3 - 44x - 112`. With the other root the constant term is
    /// `+112` and this fails.
    #[test]
    fn two_steps_from_the_starting_curve_follow_the_promised_root() {
        use prism_field::{ParameterSet, P248};
        type Ext = <P248 as ParameterSet>::Ext;
        let i = P248::ext_i();
        let one = split_chain(Ext::zero(), [i, i.negate()], 1).expect("one step");
        assert!(one.a4.same(Ext::from_u64(4).negate()));
        assert!(one.a6.is_zero());
        let two = split_chain(Ext::zero(), [i, i.negate()], 2).expect("two steps");
        assert!(two.a4.same(Ext::from_u64(44).negate()));
        assert!(two.a6.same(Ext::from_u64(112).negate()));
    }

    #[test]
    fn a_split_two_two_is_a_pair_of_two_isogenies() {
        let (curve, kernel) = point_on_curve();
        let split = SplitTwoTwo::new(curve, kernel, curve, kernel).unwrap();
        let alone = two_isogeny(curve, kernel).unwrap();
        assert!(split.left.codomain().a4.same(alone.codomain().a4));
        assert!(split.right.codomain().a6.same(alone.codomain().a6));
    }

    /// On `y^2 = x^3 + x` the 2-torsion other than `x = 0` is `x = ±i`,
    /// from the roots `±2i` of `−4`. The extension field's rule names
    /// `2i`, whose imaginary part is even, so the pair is `(i, −i)` in
    /// that order. With the other root this fails.
    #[test]
    fn the_other_torsion_is_ordered_by_the_promised_root() {
        use prism_field::{ParameterSet, P248};
        type Ext = <P248 as ParameterSet>::Ext;
        let curve = Curve::from_montgomery(Ext::zero()).unwrap();
        let (left, right) = super::other_torsion(curve, Ext::zero()).expect("E0 has its 2-torsion");
        let i = P248::ext_i();
        assert!(left.same(i));
        assert!(right.same(i.negate()));
    }
}
