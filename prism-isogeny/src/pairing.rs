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
    let z_inv2 = z_inv.mul(z_inv);
    Some(Aff {
        x: point.x.mul(z_inv2),
        y: point.y.mul(z_inv2).mul(z_inv),
    })
}

/// One Miller doubling. `None` in the point means `[2]T = O`.
fn double_step<F: Field>(
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
        t.x.mul(t.x)
            .mul(F::from_u64(3))
            .add(curve.a.mul(t.x).mul(F::from_u64(2)))
            .add(F::one());
    let slope = slope_num.mul(two_y.inv()?);
    let line = q.y.sub(t.y).sub(slope.mul(q.x.sub(t.x)));
    if line.is_zero() {
        return None;
    }
    let x3 = slope.mul(slope).sub(curve.a).sub(t.x).sub(t.x);
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
        value = value.mul(value).mul(contribution);
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

fn on_curve<F: Field>(curve: Montgomery<F>, p: Jac<F>) -> bool {
    if p.z.is_zero() {
        return false;
    }
    let z2 = p.z.mul(p.z);
    let x2 = p.x.mul(p.x);
    p.y.mul(p.y).same(
        x2.mul(p.x)
            .add(curve.a.mul(x2).mul(z2))
            .add(p.x.mul(z2).mul(z2)),
    )
}

/// `e_{2^k}(P, Q)`. `log_order` is `k`. This verification routine requires
/// both points on the normalized curve with exact order `2^k`; it refuses
/// lower-order inputs, rather than implementing all degenerate pairings.
pub fn weil<F: Field>(curve: Montgomery<F>, p: Jac<F>, q: Jac<F>, log_order: u32) -> Option<F> {
    if !curve.c.same(F::one()) || !on_curve(curve, p) || !on_curve(curve, q) {
        return None;
    }
    let numerator = miller(curve, p, q, log_order)?;
    let denominator = miller(curve, q, p, log_order)?;
    Some(numerator.mul(denominator.inv()?))
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
            value = value.mul(value);
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
        let (_line, doubled) = super::double_step(curve, p, q).expect("double");
        let doubled = doubled.expect("order above 2");
        let y2 = doubled.y.mul(doubled.y);
        let x2 = doubled.x.mul(doubled.x);
        let rhs = doubled.x.mul(x2).add(doubled.x);
        assert!(y2.same(rhs));
    }
}
