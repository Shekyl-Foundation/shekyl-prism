// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The Weil pairing of order `2^k` on a Montgomery curve.
//!
//! Miller's algorithm, doublings only. The pairing value lies in the
//! field the points are defined over. A zero line evaluation is `None`:
//! the two points are not in the position this formula evaluates, and a
//! caller that needed the value treats that as a failed check.

use prism_field::Field;

use crate::jac::{add, dbl, Jac};
use crate::montgomery::Montgomery;

/// Affine point. Infinity is not represented: a doubling that reaches it
/// returns the vertical-line contribution and stops the Miller loop.
#[derive(Clone, Copy)]
struct Aff<F> {
    x: F,
    y: F,
}

fn to_affine<F: Field>(point: Jac<F>) -> Option<Aff<F>> {
    let z_inv = point.z.inv()?;
    let z_inv2 = z_inv.square();
    Some(Aff {
        x: point.x.mul(z_inv2),
        y: point.y.mul(z_inv2).mul(z_inv),
    })
}

/// `f_{2^k, P}(Q)` as a fraction, numerator then denominator.
///
/// The running point `T` stays in Jacobian coordinates and no step inverts.
/// With `T = (X, Y, Z)`, `M = 3X² + Z²(2aX + Z²)` and `Z₃ = 2YZ`, the
/// tangent at `T` evaluated at the affine `Q = (x, y)` is
/// `(2Y(yZ³ − Y) − M(xZ² − X)) / (Z₃Z²)`, and the vertical through `[2]T`
/// is `(xZ₃² − X₃) / Z₃²`. Their quotient is the step's factor
/// `line · Z₃ / (Z² · vertical)`, which is the same field element the
/// affine loop multiplies in.
///
/// Every factor is tested for zero at the step it arises, as the affine
/// loop does: a product carried to the end would hide which step failed
/// and could turn a refusal into a value. `None` in exactly these cases:
/// a zero tangent value, a zero vertical value, `T` of order two before
/// the last step, and `T` not of order two at the last step.
fn miller<F: Field>(curve: Montgomery<F>, p: Jac<F>, q: Aff<F>, log_order: u32) -> Option<(F, F)> {
    if p.z.is_zero() {
        return None;
    }
    let mut t = p;
    let mut z2 = t.z.square();
    // `x(Q)·Z² − X`: the vertical through `T` at `Q`, up to `Z²`.
    let mut dx = q.x.mul(z2).sub(t.x);
    let mut numerator = F::one();
    let mut denominator = F::one();
    for step in 0..log_order {
        if t.y.is_zero() {
            // `[2]T = O`. The step's factor is the vertical through `T`.
            if dx.is_zero() || step + 1 != log_order {
                return None;
            }
            return Some((numerator.square().mul(dx), denominator.square().mul(z2)));
        }
        let a_x = curve.a.mul(t.x);
        let x2 = t.x.square();
        let m = x2.add(x2).add(x2).add(z2.mul(a_x.add(a_x).add(z2)));
        let dy = q.y.mul(z2).mul(t.z).sub(t.y);
        let line = dy.mul(t.y.add(t.y)).sub(m.mul(dx));
        if line.is_zero() {
            return None;
        }
        // `[2]T`, by the formulas of `jac::dbl`.
        let mut z3 = t.y.mul(t.z);
        z3 = z3.add(z3);
        let z3_2 = z3.square();
        let y2 = t.y.square();
        let two_y2 = y2.add(y2);
        let four_x_y2 = two_y2.mul(t.x.add(t.x));
        let x3 = m
            .square()
            .sub(z3_2.mul(curve.a))
            .sub(four_x_y2)
            .sub(four_x_y2);
        let four_y4 = two_y2.square();
        let y3 = four_x_y2.sub(x3).mul(m).sub(four_y4).sub(four_y4);
        let vertical = q.x.mul(z3_2).sub(x3);
        if vertical.is_zero() {
            return None;
        }
        numerator = numerator.square().mul(line).mul(z3);
        denominator = denominator.square().mul(z2).mul(vertical);
        t = Jac {
            x: x3,
            y: y3,
            z: z3,
        };
        z2 = z3_2;
        dx = vertical;
    }
    // If the final doubling never reached infinity, P is not of the
    // claimed order. A truncated Miller function is not a Weil pairing.
    None
}

fn on_curve<F: Field>(curve: Montgomery<F>, p: Jac<F>) -> bool {
    if p.z.is_zero() {
        return false;
    }
    let z2 = p.z.square();
    let x2 = p.x.square();
    p.y.square().same(
        x2.mul(p.x)
            .add(curve.a.mul(x2).mul(z2))
            .add(p.x.mul(z2).mul(z2)),
    )
}

/// `e_{2^k}(P, Q)`. `log_order` is `k`. This verification routine requires
/// both points on the normalized curve with exact order `2^k`; it refuses
/// lower-order inputs, rather than implementing all degenerate pairings.
///
/// Three inversions whatever `k` is: one for each point's affine form and
/// one for the quotient of the two Miller values.
pub fn weil<F: Field>(curve: Montgomery<F>, p: Jac<F>, q: Jac<F>, log_order: u32) -> Option<F> {
    if !curve.c.same(F::one()) || !on_curve(curve, p) || !on_curve(curve, q) {
        return None;
    }
    let (p_affine, q_affine) = (to_affine(p)?, to_affine(q)?);
    let (n1, d1) = miller(curve, p, q_affine, log_order)?;
    let (n2, d2) = miller(curve, q, p_affine, log_order)?;
    Some(n1.mul(d2).mul(d1.mul(n2).inv()?))
}

/// The affine Miller loop this module used before its loop stopped
/// inverting: two inversions a step. It is the reference the loop above is
/// compared with, on values and on refusals, and nothing else calls it.
#[cfg(any(test, feature = "oracle"))]
pub mod oracle {
    use super::{on_curve, to_affine, Aff};
    use crate::jac::Jac;
    use crate::montgomery::Montgomery;
    use prism_field::Field;

    /// One Miller doubling. `None` in the point means `[2]T = O`.
    pub(super) fn double_step<F: Field>(
        curve: Montgomery<F>,
        t: Aff<F>,
        q: Aff<F>,
    ) -> Option<(F, Option<Aff<F>>)> {
        let two_y = t.y.add(t.y);
        if two_y.is_zero() {
            let vertical = q.x.sub(t.x);
            if vertical.is_zero() {
                return None;
            }
            return Some((vertical, None));
        }
        let slope_num =
            t.x.square()
                .mul(F::from_u64(3))
                .add(curve.a.mul(t.x).mul(F::from_u64(2)))
                .add(F::one());
        let slope = slope_num.mul(two_y.inv()?);
        let line = q.y.sub(t.y).sub(slope.mul(q.x.sub(t.x)));
        if line.is_zero() {
            return None;
        }
        let x3 = slope.square().sub(curve.a).sub(t.x).sub(t.x);
        let y3 = slope.mul(t.x.sub(x3)).sub(t.y);
        let denom = q.x.sub(x3);
        if denom.is_zero() {
            return None;
        }
        Some((line.mul(denom.inv()?), Some(Aff { x: x3, y: y3 })))
    }

    /// `f_{2^k, P}(Q)`, the Miller function.
    fn miller<F: Field>(curve: Montgomery<F>, p: Jac<F>, q: Jac<F>, log_order: u32) -> Option<F> {
        let mut t = to_affine(p)?;
        let target = to_affine(q)?;
        let mut value = F::one();
        for step in 0..log_order {
            let (contribution, next) = double_step(curve, t, target)?;
            value = value.square().mul(contribution);
            match next {
                Some(point) => t = point,
                None => {
                    if step + 1 != log_order {
                        return None;
                    }
                    return Some(value);
                }
            }
        }
        // If the final doubling never reached infinity, P is not of the
        // claimed order. A truncated Miller function is not a Weil pairing.
        None
    }

    /// `e_{2^k}(P, Q)` by the affine loop.
    pub fn weil<F: Field>(curve: Montgomery<F>, p: Jac<F>, q: Jac<F>, log_order: u32) -> Option<F> {
        if !curve.c.same(F::one()) || !on_curve(curve, p) || !on_curve(curve, q) {
            return None;
        }
        let numerator = miller(curve, p, q, log_order)?;
        let denominator = miller(curve, q, p, log_order)?;
        Some(numerator.mul(denominator.inv()?))
    }
}

/// `[scalar] point`, `scalar` little-endian.
pub fn mul<F: Field>(curve: Montgomery<F>, point: Jac<F>, scalar: &[u8]) -> Jac<F> {
    let mut result = Jac::infinity();
    let mut addend = point;
    for byte in scalar {
        let mut bits = *byte;
        for _ in 0..8 {
            if bits & 1 == 1 {
                result = add(curve, result, addend);
            }
            addend = dbl(curve, addend);
            bits >>= 1;
        }
    }
    result
}

/// Low `bits` of `−s²`, little-endian. For `e ≥ 5` this is `q mod 2^e`,
/// because `q = 2^{2e−5} − s²` and `2^{2e−5}` is divisible by `2^e`.
pub fn neg_square_mod_pow2(scalar: &[u8], bits: u32) -> Vec<u8> {
    let width = (bits as usize).div_ceil(8);
    let mut column = vec![0u128; width];
    for (i, left) in scalar.iter().enumerate() {
        for (j, right) in scalar.iter().enumerate() {
            let place = i + j;
            if place >= width {
                continue;
            }
            column[place] += u128::from(*left) * u128::from(*right);
        }
    }
    let mut square = vec![0u8; width];
    let mut carry = 0u128;
    for (place, limb) in column.iter().enumerate() {
        let sum = limb + carry;
        square[place] = sum as u8;
        carry = sum >> 8;
    }
    let spare = width * 8 - bits as usize;
    if spare > 0 {
        let last = width - 1;
        square[last] &= 0xff >> spare;
    }
    let mut carry = 1u16;
    for limb in &mut square {
        let sum = u16::from(!*limb) + carry;
        *limb = sum as u8;
        carry = sum >> 8;
    }
    if spare > 0 {
        let last = width - 1;
        square[last] &= 0xff >> spare;
    }
    square
}

/// `value` raised to a little-endian exponent.
pub fn pow<F: Field>(mut value: F, exponent: &[u8]) -> F {
    let mut result = F::one();
    for byte in exponent {
        let mut bits = *byte;
        for _ in 0..8 {
            if bits & 1 == 1 {
                result = result.mul(value);
            }
            value = value.square();
            bits >>= 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{mul, neg_square_mod_pow2, pow, weil, Aff};
    use crate::jac::Jac;
    use crate::montgomery::Montgomery;
    use prism_field::Field;

    const P: u64 = 10_007;

    #[derive(Clone, Copy, Debug)]
    struct Fp2 {
        re: u64,
        im: u64,
    }

    impl Fp2 {
        fn new(re: u64, im: u64) -> Self {
            Self {
                re: re % P,
                im: im % P,
            }
        }
    }

    impl PartialEq for Fp2 {
        fn eq(&self, other: &Self) -> bool {
            self.re == other.re && self.im == other.im
        }
    }

    impl Field for Fp2 {
        fn zero() -> Self {
            Self::new(0, 0)
        }
        fn one() -> Self {
            Self::new(1, 0)
        }
        fn from_u64(v: u64) -> Self {
            Self::new(v, 0)
        }
        fn add(self, rhs: Self) -> Self {
            Self::new(self.re + rhs.re, self.im + rhs.im)
        }
        fn sub(self, rhs: Self) -> Self {
            Self::new(self.re + P - rhs.re, self.im + P - rhs.im)
        }
        fn square(self) -> Self {
            self.mul(self)
        }
        fn mul(self, rhs: Self) -> Self {
            let re = self.re * rhs.re + (P - (self.im * rhs.im % P));
            let im = self.re * rhs.im + self.im * rhs.re;
            Self::new(re % P, im % P)
        }
        fn negate(self) -> Self {
            Self::new(P - self.re, P - self.im)
        }
        fn inv(self) -> Option<Self> {
            let norm = (self.re * self.re + self.im * self.im) % P;
            if norm == 0 {
                return None;
            }
            let inv = mod_pow(norm, P - 2);
            Some(Self::new(self.re * inv, (P - self.im) * inv))
        }
        fn sqrt(self) -> Option<Self> {
            let _ = self;
            None
        }
        fn is_square(self) -> bool {
            let _ = self;
            true
        }
        fn is_zero(self) -> bool {
            self.re == 0 && self.im == 0
        }
        fn same(self, rhs: Self) -> bool {
            self == rhs
        }
    }

    fn mod_pow(mut base: u64, mut exp: u64) -> u64 {
        let mut result = 1u64;
        base %= P;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result * base % P;
            }
            base = base * base % P;
            exp >>= 1;
        }
        result
    }

    fn point(x: Fp2, y: Fp2) -> Jac<Fp2> {
        Jac {
            x,
            y,
            z: Fp2::one(),
        }
    }

    #[test]
    fn the_order_four_pairing_matches_the_curve() {
        // Sage `P.weil_pairing(Q, 4)` on `y^2 = x^3 + x` over `F_{10007^2}`.
        let curve = Montgomery::normalized(Fp2::zero());
        let p = point(Fp2::new(1, 0), Fp2::new(2641, 0));
        let q = point(Fp2::new(0, 2640), Fp2::new(2640, 2640));
        let value = weil(curve, p, q, 2).expect("pairing");
        assert!(value.same(Fp2::new(0, 10006)));
        let triple = mul(curve, p, &[3]);
        let scaled = weil(curve, triple, q, 2).expect("scaled pairing");
        assert!(scaled.same(pow(value, &[3])));
        assert!(scaled.same(Fp2::new(0, 1)));
    }

    #[test]
    fn a_truncated_miller_loop_is_not_a_pairing() {
        let curve = Montgomery::normalized(Fp2::zero());
        let p = point(Fp2::new(1, 0), Fp2::new(2641, 0));
        let q = point(Fp2::new(0, 2640), Fp2::new(2640, 2640));
        assert!(
            weil(curve, p, q, 1).is_none(),
            "order-four points are not order-two inputs"
        );
        let off_curve = Jac {
            y: p.y.add(Fp2::one()),
            ..p
        };
        assert!(weil(curve, off_curve, q, 2).is_none());
    }

    #[test]
    fn negating_a_square_modulo_a_power_of_two_is_the_low_bits() {
        // s = 3, s^2 = 9, -9 mod 2^5 = 23.
        assert_eq!(neg_square_mod_pow2(&[3], 5), vec![23]);
    }

    #[test]
    fn an_affine_double_stays_on_the_curve() {
        let curve = Montgomery::normalized(Fp2::zero());
        let p = Aff {
            x: Fp2::new(1, 0),
            y: Fp2::new(2641, 0),
        };
        let q = Aff {
            x: Fp2::new(0, 2640),
            y: Fp2::new(2640, 2640),
        };
        let (_line, doubled) = super::oracle::double_step(curve, p, q).expect("double");
        let doubled = doubled.expect("order above 2");
        let y2 = doubled.y.square();
        let x2 = doubled.x.square();
        let rhs = doubled.x.mul(x2).add(doubled.x);
        assert!(y2.same(rhs));
    }

    /// The loop that does not invert against the affine loop it replaced,
    /// on the committed signatures' own curves and points.
    mod differential {
        use super::super::{mul, oracle, weil};
        use crate::jac::{add, dbl_iter, Jac};
        use crate::montgomery::Montgomery;
        use prism_field::{Field, Level, Level1, Level3, Level5};

        const VECTORS: [&str; 6] = [
            include_str!("../../prism-verify/testdata/s5_lvl1.txt"),
            include_str!("../../prism-verify/testdata/s5_lvl1b.txt"),
            include_str!("../../prism-verify/testdata/s5_lvl3.txt"),
            include_str!("../../prism-verify/testdata/s5_lvl3b.txt"),
            include_str!("../../prism-verify/testdata/s5_lvl5.txt"),
            include_str!("../../prism-verify/testdata/s5_lvl5b.txt"),
        ];

        fn element<L: Level>(text: &str, name: &str) -> L::Ext {
            let hex = text
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .expect("the vector has this row");
            let bytes: Vec<u8> = hex
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            L::ext_decode(&bytes).expect("a committed element is canonical")
        }

        type Point<L> = Jac<<L as Level>::Ext>;
        type Curve<L> = Montgomery<<L as Level>::Ext>;

        /// The response curve and its two points, a basis of the `2^e`-torsion.
        fn basis<L: Level>(text: &str) -> (Curve<L>, Point<L>, Point<L>) {
            let at = |x, y| Jac {
                x: element::<L>(text, x),
                y: element::<L>(text, y),
                z: L::Ext::one(),
            };
            (
                Montgomery::normalized(element::<L>(text, "E ")),
                at("Px ", "Py "),
                at("Qx ", "Qy "),
            )
        }

        /// xorshift64*: a fixed stream, so a failure names its case.
        struct Stream(u64);

        impl Stream {
            fn next(&mut self) -> u64 {
                self.0 ^= self.0 >> 12;
                self.0 ^= self.0 << 25;
                self.0 ^= self.0 >> 27;
                self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
            }

            /// A scalar below `2^e`, with its low bit as asked.
            fn scalar<L: Level>(&mut self, odd: Option<bool>) -> Vec<u8> {
                let mut bytes = vec![0u8; (L::E as usize).div_ceil(8)];
                for chunk in bytes.chunks_mut(8) {
                    let word = self.next().to_le_bytes();
                    chunk.copy_from_slice(&word[..chunk.len()]);
                }
                match odd {
                    Some(true) => bytes[0] |= 1,
                    Some(false) => bytes[0] &= !1,
                    None => {}
                }
                bytes
            }
        }

        /// Both loops on one input: the same refusal, or the same element.
        fn agree<F: Field>(curve: Montgomery<F>, p: Jac<F>, q: Jac<F>, k: u32, case: &str) -> bool {
            let new = weil(curve, p, q, k);
            let old = oracle::weil(curve, p, q, k);
            match (new, old) {
                (None, None) => false,
                (Some(a), Some(b)) => {
                    assert!(a.same(b), "{case}: the two loops give different values");
                    true
                }
                (new, _) => panic!(
                    "{case}: the loop without inversions {} where the affine loop did not",
                    if new.is_some() {
                        "gave a value"
                    } else {
                        "refused"
                    }
                ),
            }
        }

        fn combine<F: Field>(
            curve: Montgomery<F>,
            p: Jac<F>,
            q: Jac<F>,
            a: &[u8],
            b: &[u8],
        ) -> Jac<F> {
            add(curve, mul(curve, p, a), mul(curve, q, b))
        }

        /// `pairs` full-order independent pairs, which must have a value,
        /// then `pairs / 4` unconstrained pairs, which may be refused.
        fn random<L: Level>(text: &str, seed: u64, pairs: u32) {
            let (curve, p, q) = basis::<L>(text);
            let mut stream = Stream(seed);
            for n in 0..pairs {
                // Odd on the diagonal and even off it: determinant odd, so
                // the images are again a basis of the full torsion.
                let (a, b) = (
                    stream.scalar::<L>(Some(true)),
                    stream.scalar::<L>(Some(false)),
                );
                let (c, d) = (
                    stream.scalar::<L>(Some(false)),
                    stream.scalar::<L>(Some(true)),
                );
                let (r, t) = (combine(curve, p, q, &a, &b), combine(curve, p, q, &c, &d));
                let case = format!("{} basis pair {n}", L::NAME);
                assert!(
                    agree(curve, r, t, L::E, &case),
                    "{case}: a basis has a pairing"
                );
            }
            for n in 0..pairs / 4 {
                let (a, b) = (stream.scalar::<L>(None), stream.scalar::<L>(None));
                let (c, d) = (stream.scalar::<L>(None), stream.scalar::<L>(None));
                let (r, t) = (combine(curve, p, q, &a, &b), combine(curve, p, q, &c, &d));
                agree(curve, r, t, L::E, &format!("{} free pair {n}", L::NAME));
            }
        }

        /// Inputs a forger would choose. Each must be treated alike by both
        /// loops; `values` counts those that have a pairing value at all.
        fn adversarial<L: Level>(text: &str, seed: u64) -> u32 {
            let (curve, p, q) = basis::<L>(text);
            let e = L::E;
            let negate = |point: Jac<L::Ext>| Jac {
                y: point.y.negate(),
                ..point
            };
            let mut stream = Stream(seed);
            let mut cases: Vec<(String, Point<L>, Point<L>)> = vec![
                ("the basis".into(), p, q),
                ("Q = P".into(), p, p),
                ("Q = -P".into(), p, negate(p)),
                ("P = infinity".into(), Jac::infinity(), q),
                ("Q = infinity".into(), p, Jac::infinity()),
                (
                    "P off the curve".into(),
                    Jac {
                        y: p.y.add(L::Ext::one()),
                        ..p
                    },
                    q,
                ),
                (
                    "Q off the curve".into(),
                    p,
                    Jac {
                        x: q.x.add(L::Ext::one()),
                        ..q
                    },
                ),
            ];
            for k in [2u8, 3, 4, 5, 255] {
                cases.push((format!("Q = [{k}]P"), p, mul(curve, p, &[k])));
            }
            for n in 0..4 {
                let k = stream.scalar::<L>(None);
                cases.push((format!("Q = [random {n}]P"), p, mul(curve, p, &k)));
            }
            // Either point of lower order, down to order two and one.
            for down in [1, 2, e - 2, e - 1, e] {
                cases.push((
                    format!("P doubled {down} times"),
                    dbl_iter(curve, p, down),
                    q,
                ));
                cases.push((
                    format!("Q doubled {down} times"),
                    p,
                    dbl_iter(curve, q, down),
                ));
            }
            // `Q = [2^j]P + T` for each point `T` of order two.
            let two_torsion = [
                dbl_iter(curve, p, e - 1),
                dbl_iter(curve, q, e - 1),
                add(curve, dbl_iter(curve, p, e - 1), dbl_iter(curve, q, e - 1)),
            ];
            for (name, torsion) in ["P", "Q", "P+Q"].iter().zip(two_torsion) {
                for j in [0, 1, e - 2, e - 1] {
                    cases.push((
                        format!("Q = [2^{j}]P + [2^(e-1)]({name})"),
                        p,
                        add(curve, dbl_iter(curve, p, j), torsion),
                    ));
                    cases.push((
                        format!("P = [2^{j}]Q + [2^(e-1)]({name})"),
                        add(curve, dbl_iter(curve, q, j), torsion),
                        q,
                    ));
                }
            }
            let mut values = 0;
            for (name, r, t) in cases {
                let case = format!("{} {name}", L::NAME);
                values += u32::from(agree(curve, r, t, e, &case));
                // The same pair at one doubling too few and one too many.
                agree(
                    curve,
                    r,
                    t,
                    e - 1,
                    &format!("{case}, order 2^(e-1) claimed"),
                );
                agree(
                    curve,
                    r,
                    t,
                    e + 1,
                    &format!("{case}, order 2^(e+1) claimed"),
                );
            }
            // A curve constant other than one is refused before any loop.
            // The comparison tells values apart: `e(Q, P)` is the inverse of
            // `e(P, Q)`, and a pairing of full order is not its own inverse.
            let forward = weil(curve, p, q, e).expect("the basis has a pairing");
            let backward = oracle::weil(curve, q, p, e).expect("and so does its swap");
            assert!(!forward.same(backward) && forward.mul(backward).same(L::Ext::one()));
            let scaled = Montgomery {
                c: L::Ext::from_u64(2),
                ..curve
            };
            assert!(!agree(scaled, p, q, e, "c = 2"));
            values
        }

        fn every_level(run: impl Fn(usize, &str)) {
            for (n, text) in VECTORS.iter().enumerate() {
                run(n, text);
            }
        }

        #[test]
        fn adversarial_inputs_are_refused_or_valued_alike() {
            every_level(|n, text| {
                let seed = 0x5052_4953_4d00_0000 + n as u64;
                let values = match n / 2 {
                    0 => adversarial::<Level1>(text, seed),
                    1 => adversarial::<Level3>(text, seed),
                    _ => adversarial::<Level5>(text, seed),
                };
                // The honest basis has a value, so the set is not all
                // refusals; most of the rest must be refused.
                assert!(values >= 1, "vector {n}: the basis itself was refused");
            });
        }

        #[test]
        fn a_sample_of_random_pairs_agrees() {
            every_level(|n, text| match n / 2 {
                0 => random::<Level1>(text, 11 + n as u64, 8),
                1 => random::<Level3>(text, 11 + n as u64, 8),
                _ => random::<Level5>(text, 11 + n as u64, 8),
            });
        }

        /// 1,000 full-order pairs and 250 unconstrained ones per level.
        /// Minutes in a debug build, so CI runs it in release by name.
        #[test]
        #[ignore = "long: run in release with --ignored"]
        fn a_thousand_random_pairs_per_level_agree() {
            every_level(|n, text| match n / 2 {
                0 => random::<Level1>(text, 1000 + n as u64, 500),
                1 => random::<Level3>(text, 1000 + n as u64, 500),
                _ => random::<Level5>(text, 1000 + n as u64, 500),
            });
        }
    }
}
