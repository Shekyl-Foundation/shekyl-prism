// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// The difference of two x-only points is the formula in
// `external/PRISM_v2/src/ec/ref/lvlx/basis.c` (`difference_point`).

//! A basis of `E[2^e]` from a Montgomery coefficient.
//!
//! The abscissa is the one the basis-index search names. Multiplying by the
//! odd cofactor `f` clears `p + 1 = f·2^e` down to order dividing `2^e`.
//! `R` is the point above `(0, 0)` before that multiplication,
//! `x(R) = −x(P) − A`. The canonical second point is not `R`: it is
//! `difference(P, R)` computed after the cofactor is cleared. Clearing the
//! difference of the uncleared points is a different point on about half
//! of keys, and it is not the C basis.

use prism_field::{Field, ParameterSet, MAX_ODD_PRIMES};

use crate::jac::xdbl_iter;
use crate::montgomery::{a24, ladder, Montgomery, Xz};

/// `(P, R, Q)` in x-only coordinates, each of order dividing `2^e`.
///
/// `Q` is [`difference`] of the cleared `P` and `R`. `full_difference` is
/// the difference of the points before the cofactor, and clearing that
/// point is not `Q`.
#[derive(Clone, Copy, Debug)]
pub struct EvenBasis<F> {
    /// Order dividing `2^e`, after multiplication by `f`.
    pub p: Xz<F>,
    /// Cleared point with full-point abscissa `−x(P) − A`.
    pub q: Xz<F>,
    /// `difference(p, q)`, the canonical second basis point.
    pub difference: Xz<F>,
    /// The same `P` and `R` before that multiplication, and their difference.
    pub full_p: Xz<F>,
    pub full_q: Xz<F>,
    pub full_difference: Xz<F>,
}

/// A basis `(R, T)` of `E[p+1]`, with `R − T`.
///
/// `[f]R`, `[f]T` and `[f](R−T)` are [`EvenBasis`]. Both points have order
/// `p+1`, their even parts generate `E[2^e]`, and their odd parts generate
/// `E[f]`.
#[derive(Clone, Copy, Debug)]
pub struct RationalBasis<F> {
    pub r: Xz<F>,
    pub t: Xz<F>,
    pub difference: Xz<F>,
    pub even: EvenBasis<F>,
}

fn conjugate<L: ParameterSet>(value: L::Ext) -> L::Ext {
    let (re, im) = L::ext_parts(value);
    L::ext_from_base(re).sub(L::ext_i().mul(L::ext_from_base(im)))
}

/// `x(P)` for the index the basis search settled on.
pub fn basis_abscissa<L: ParameterSet>(
    coefficient: L::Ext,
    index: u16,
    coefficient_is_square: bool,
) -> Option<L::Ext> {
    let index = u64::from(index);
    if coefficient_is_square {
        let z = L::Ext::one().add(L::ext_i().mul(L::Ext::from_u64(index)));
        Some(coefficient.mul(z.inv()?).negate())
    } else {
        Some(coefficient.mul(L::Ext::from_u64(index)))
    }
}

/// Deterministic `P − Q` on a normalized Montgomery curve, as `difference_point`.
pub fn difference<L: ParameterSet>(
    coefficient: L::Ext,
    p: Xz<L::Ext>,
    q: Xz<L::Ext>,
) -> Option<Xz<L::Ext>> {
    let curve = Montgomery::normalized(coefficient);
    let xx = p.x.mul(q.x);
    let zz = p.z.mul(q.z);
    let mut bxx = xx.sub(zz);
    bxx = bxx.square().mul(curve.c);
    let xz = xx.add(zz);
    let px_qz = p.x.mul(q.z);
    let pz_qx = p.z.mul(q.x);
    let mut bzz = px_qz.sub(pz_qx);
    bzz = bzz.square().mul(curve.c);
    let mut bxz = xz.mul(px_qz.add(pz_qx)).mul(curve.c);
    let cross = px_qz.mul(pz_qx).mul(curve.a);
    bxz = bxz.add(cross).add(cross);

    // Normalize by `C · conj(C)^2 · conj(P.z)^2 · conj(Q.z)^2`, which makes
    // the denominator a fourth power in `F_p`.
    let mut scale = conjugate::<L>(curve.c);
    scale = scale.square().mul(curve.c);
    let mut pz = conjugate::<L>(p.z);
    scale = scale.mul(pz.square());
    pz = conjugate::<L>(q.z);
    scale = scale.mul(pz.square());
    bxx = bxx.mul(scale);
    bxz = bxz.mul(scale);
    bzz = bzz.mul(scale);

    let disc = bxz.square().sub(bxx.mul(bzz));
    let root = disc.sqrt()?;
    Some(Xz {
        x: bxz.add(root),
        z: bzz,
    })
}

fn clear_cofactor<L: ParameterSet>(coefficient: L::Ext, point: Xz<L::Ext>) -> Option<Xz<L::Ext>> {
    let constant = a24(coefficient)?;
    let width = usize::try_from(64 - L::ODD.leading_zeros()).unwrap_or(0);
    let bits = (0..width).rev().map(|shift| (L::ODD >> shift) & 1 == 1);
    Some(ladder(constant, bits, point))
}

/// The order-`2^e` basis named by `index`.
pub fn even_basis<L: ParameterSet>(
    coefficient: L::Ext,
    index: u16,
    coefficient_is_square: bool,
) -> Option<EvenBasis<L::Ext>> {
    let full_p = Xz::affine(basis_abscissa::<L>(
        coefficient,
        index,
        coefficient_is_square,
    )?);
    let full_q = Xz::affine(full_p.x.add(coefficient).negate());
    let full_difference = difference::<L>(coefficient, full_p, full_q)?;
    let p = clear_cofactor::<L>(coefficient, full_p)?;
    let q = clear_cofactor::<L>(coefficient, full_q)?;
    // After the cofactor, not before. Clearing `full_difference` matches
    // the C `Q` on only about half of keys.
    let difference = difference::<L>(coefficient, p, q)?;
    Some(EvenBasis {
        p,
        q,
        difference,
        full_p,
        full_q,
        full_difference,
    })
}

/// Multiply by `odd`, then double `doubles` times.
pub fn mul_odd_then_double<L: ParameterSet>(
    coefficient: L::Ext,
    point: Xz<L::Ext>,
    odd: u64,
    doubles: u32,
) -> Option<Xz<L::Ext>> {
    let constant = a24(coefficient)?;
    let width = usize::try_from(64 - odd.leading_zeros()).unwrap_or(0);
    let bits = (0..width).rev().map(|shift| (odd >> shift) & 1 == 1);
    Some(double_times::<L>(
        coefficient,
        ladder(constant, bits, point),
        doubles,
    ))
}

fn is_infinity<F: Field>(point: Xz<F>) -> bool {
    point.z.is_zero()
}

fn same_x<F: Field>(left: Xz<F>, right: Xz<F>) -> bool {
    left.x.mul(right.z).same(right.x.mul(left.z))
}

/// `[p+1]X = O`, and `[(p+1)/ℓ]X ≠ O` for `ℓ = 2` and every prime divisor of `f`.
pub fn has_order_p_plus_one<L: ParameterSet>(coefficient: L::Ext, point: Xz<L::Ext>) -> bool {
    let killed = |odd, doubles| {
        mul_odd_then_double::<L>(coefficient, point, odd, doubles).is_some_and(is_infinity)
    };
    if !killed(L::ODD, L::E) {
        return false;
    }
    if killed(L::ODD, L::E - 1) {
        return false;
    }
    L::ODD_PRIMES
        .iter()
        .all(|prime| !killed(L::ODD / prime, L::E))
}

/// `[2^{e−1}]` of two order-`2^e` points are distinct and nonzero.
///
/// `E[2^e] ≅ (Z/2^e)^2`, so `E[2]` has three nonzero points and they are the
/// nonzero vectors of `(Z/2)^2`. Two distinct ones are a basis of that
/// vector space, and therefore the two points generate `E[2^e]`.
pub fn even_parts_generate<L: ParameterSet>(
    coefficient: L::Ext,
    p: Xz<L::Ext>,
    q: Xz<L::Ext>,
) -> bool {
    let p2 = double_times::<L>(coefficient, p, L::E - 1);
    let q2 = double_times::<L>(coefficient, q, L::E - 1);
    !is_infinity(p2) && !is_infinity(q2) && !same_x(p2, q2)
}

fn prime_field_basis<L: ParameterSet>(
    coefficient: L::Ext,
    left: Xz<L::Ext>,
    right: Xz<L::Ext>,
    prime: u64,
) -> bool {
    if is_infinity(left) || is_infinity(right) {
        return false;
    }
    // Order dividing `prime` is the caller's. Order exactly `prime` is
    // nonzero, since `prime` is prime.
    (1..=prime / 2).all(|scalar| {
        mul_odd_then_double::<L>(coefficient, right, scalar, 0)
            .is_some_and(|image| !same_x(left, image))
    })
}

/// The images of `R` and `T` in `E[f]` generate `E[f]`.
///
/// `E[f]` splits into prime-power factors, one for each prime in
/// [`ParameterSet::ODD_PRIMES`]. Two
/// elements generate `(Z/p^k)^2` when their images generate `(Z/p)^2`, and
/// two elements generate `(Z/q)^2` for a prime `q` when neither is a scalar
/// multiple of the other. A scalar and its negative have the same
/// x-coordinate, so the multiples checked run only up to `q/2`.
pub fn odd_parts_generate<L: ParameterSet>(
    coefficient: L::Ext,
    r: Xz<L::Ext>,
    t: Xz<L::Ext>,
) -> bool {
    let u = double_times::<L>(coefficient, r, L::E);
    let v = double_times::<L>(coefficient, t, L::E);
    // The image of each point in `E[q]`, for each prime `q` of `f`:
    // multiplying by `f/q` kills the other primes and leaves order `q`.
    // Every projection is made before any is compared.
    let mut projected = [None; MAX_ODD_PRIMES];
    for (slot, &prime) in projected.iter_mut().zip(L::ODD_PRIMES) {
        let rest = L::ODD / prime;
        let project = |point| {
            if rest == 1 {
                Some(point)
            } else {
                mul_odd_then_double::<L>(coefficient, point, rest, 0)
            }
        };
        let Some(left) = project(u) else {
            return false;
        };
        let Some(right) = project(v) else {
            return false;
        };
        *slot = Some((left, right, prime));
    }
    projected
        .iter()
        .flatten()
        .all(|&(left, right, prime)| prime_field_basis::<L>(coefficient, left, right, prime))
}

/// `(R, T)` a basis of `E[p+1]`.
///
/// `R` is the point named by the basis index, before multiplication by `f`.
/// `T` is the point whose affine x-coordinate is the smallest positive
/// integer for which both points have order `p+1`, their even parts generate
/// `E[2^e]`, and their odd parts generate `E[f]`. The search stops at
/// [`ParameterSet::SEARCH_BOUND`]
/// candidates. `R − T` is [`difference`].
pub fn rational_basis<L: ParameterSet>(
    coefficient: L::Ext,
    index: u16,
    coefficient_is_square: bool,
) -> Option<RationalBasis<L::Ext>> {
    let even = even_basis::<L>(coefficient, index, coefficient_is_square)?;
    if !has_order_p_plus_one::<L>(coefficient, even.full_p) {
        return None;
    }
    let bound = u64::from(L::SEARCH_BOUND);
    for candidate in 1..=bound {
        let t = Xz::affine(L::Ext::from_u64(candidate));
        if same_x(t, even.full_p) || !has_order_p_plus_one::<L>(coefficient, t) {
            continue;
        }
        let t_even = clear_cofactor::<L>(coefficient, t)?;
        if !even_parts_generate::<L>(coefficient, even.p, t_even) {
            continue;
        }
        if !odd_parts_generate::<L>(coefficient, even.full_p, t) {
            continue;
        }
        let difference = difference::<L>(coefficient, even.full_p, t)?;
        let difference_even = clear_cofactor::<L>(coefficient, difference)?;
        return Some(RationalBasis {
            r: even.full_p,
            t,
            difference,
            even: EvenBasis {
                p: even.p,
                q: t_even,
                difference: difference_even,
                full_p: even.full_p,
                full_q: t,
                full_difference: difference,
            },
        });
    }
    None
}

fn small_abscissa<L: ParameterSet>(real: u64, imag: u64) -> L::Ext {
    L::Ext::from_u64(real).add(L::ext_i().mul(L::Ext::from_u64(imag)))
}

/// The basis of `E0[p+1]`, where `E0` is `y^2 = x^3 + x`.
///
/// `R` is the point with the smallest positive integer abscissa and order
/// `p+1`. Every point with an abscissa in `F_p` has the same order-2 image,
/// `(0, 0)`, because that is the only order-2 point fixed by Frobenius, so
/// no second point with a real abscissa completes the basis. `T` is the
/// least `a + b·i` with `0 ≤ a ≤ N` and `1 ≤ b ≤ N`, for `N` the set's
/// [`ParameterSet::SEARCH_BOUND`], ordered by `a` then
/// `b`, for which both points have order `p+1`, their even parts generate
/// `E[2^e]`, and their odd parts generate `E[f]`.
pub fn starting_basis<L: ParameterSet>() -> Option<RationalBasis<L::Ext>> {
    let coefficient = L::Ext::zero();
    let bound = u64::from(L::SEARCH_BOUND);
    let mut r = None;
    for candidate in 1..=bound {
        let point = Xz::affine(L::Ext::from_u64(candidate));
        if has_order_p_plus_one::<L>(coefficient, point) {
            r = Some(point);
            break;
        }
    }
    let r = r?;
    let r_even = clear_cofactor::<L>(coefficient, r)?;
    for real in 0..=bound {
        for imag in 1..=bound {
            let t = Xz::affine(small_abscissa::<L>(real, imag));
            if !has_order_p_plus_one::<L>(coefficient, t) {
                continue;
            }
            let t_even = clear_cofactor::<L>(coefficient, t)?;
            if !even_parts_generate::<L>(coefficient, r_even, t_even) {
                continue;
            }
            if !odd_parts_generate::<L>(coefficient, r, t) {
                continue;
            }
            let difference = difference::<L>(coefficient, r, t)?;
            let difference_even = clear_cofactor::<L>(coefficient, difference)?;
            return Some(RationalBasis {
                r,
                t,
                difference,
                even: EvenBasis {
                    p: r_even,
                    q: t_even,
                    difference: difference_even,
                    full_p: r,
                    full_q: t,
                    full_difference: difference,
                },
            });
        }
    }
    None
}

/// `[2^times] point` on the normalized curve. Infinity is `Z = 0`.
pub fn double_times<L: ParameterSet>(
    coefficient: L::Ext,
    point: Xz<L::Ext>,
    times: u32,
) -> Xz<L::Ext> {
    xdbl_iter(Montgomery::normalized(coefficient), point, times)
}

#[cfg(test)]
mod tests {
    use super::{even_basis, has_order_p_plus_one, mul_odd_then_double, odd_parts_generate};
    use crate::torsion::double_times;
    use prism_field::{Field, ParameterSet, P248, P324, P376, P500};

    fn key<L: ParameterSet>(hex: &str) -> L::Ext {
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
            .collect();
        L::ext_decode(&bytes).unwrap()
    }

    fn order_is_two_to_the_e<L: ParameterSet>(hex: &str, hint: u8) {
        let coefficient = key::<L>(hex);
        let basis =
            even_basis::<L>(coefficient, u16::from(hint >> 1), hint & 1 == 1).expect(L::NAME);
        let full = double_times::<L>(coefficient, basis.p, L::E);
        assert!(full.z.is_zero(), "{}", L::NAME);
        let almost = double_times::<L>(coefficient, basis.p, L::E - 1);
        assert!(!almost.z.is_zero(), "{}", L::NAME);
        let odd_part = double_times::<L>(coefficient, basis.full_p, L::E);
        assert!(!odd_part.z.is_zero(), "{}", L::NAME);
        let index = u16::from(hint >> 1);
        let square = hint & 1 == 1;
        let even = even_basis::<L>(coefficient, index, square).expect(L::NAME);
        assert!(
            super::has_order_p_plus_one::<L>(coefficient, even.full_p),
            "{} R order",
            L::NAME
        );
        assert!(
            super::has_order_p_plus_one::<L>(coefficient, even.full_q),
            "{} T order",
            L::NAME
        );
        assert!(
            super::even_parts_generate::<L>(coefficient, even.p, even.q),
            "{} even",
            L::NAME
        );
        assert!(
            super::rational_basis::<L>(coefficient, index, square).is_some(),
            "{}",
            L::NAME
        );
    }

    fn starting<L: ParameterSet>() {
        assert!(super::starting_basis::<L>().is_some(), "{}", L::NAME);
    }

    /// A point is full order only when no prime of `p + 1` can be divided
    /// out of its order. On the starting curve, `R` of the basis has order
    /// `p + 1`; `[q]R` for each odd prime `q` of the cofactor, and `[2]R`,
    /// do not, and each must be reported as not full order. A set whose
    /// list of odd primes were empty or short would pass `[q]R`.
    fn only_the_full_point_is_full_order<L: ParameterSet>() {
        let coefficient = L::Ext::zero();
        let basis = super::starting_basis::<L>().expect(L::NAME);
        for point in [basis.r, basis.t] {
            assert!(has_order_p_plus_one::<L>(coefficient, point), "{}", L::NAME);
            let doubled = mul_odd_then_double::<L>(coefficient, point, 1, 1).expect(L::NAME);
            assert!(
                !has_order_p_plus_one::<L>(coefficient, doubled),
                "{}: [2]R passed for full order",
                L::NAME
            );
            assert!(!L::ODD_PRIMES.is_empty(), "{}", L::NAME);
            for &prime in L::ODD_PRIMES {
                let reduced =
                    mul_odd_then_double::<L>(coefficient, point, prime, 0).expect(L::NAME);
                assert!(
                    !has_order_p_plus_one::<L>(coefficient, reduced),
                    "{}: [{prime}]R passed for full order",
                    L::NAME
                );
            }
        }
        // The two points of the basis generate both parts of the torsion.
        assert!(
            odd_parts_generate::<L>(coefficient, basis.r, basis.t),
            "{}",
            L::NAME
        );
        // And a point against itself does not.
        assert!(
            !odd_parts_generate::<L>(coefficient, basis.r, basis.r),
            "{}",
            L::NAME
        );
    }

    #[test]
    fn a_point_missing_a_prime_of_the_cofactor_is_not_full_order_at_any_set() {
        only_the_full_point_is_full_order::<P248>();
        only_the_full_point_is_full_order::<P324>();
        only_the_full_point_is_full_order::<P376>();
        only_the_full_point_is_full_order::<P500>();
    }

    #[test]
    fn the_starting_curve_has_a_basis_at_p248() {
        starting::<P248>();
    }

    #[test]
    fn the_starting_curve_has_a_basis_at_p324() {
        starting::<P324>();
    }

    #[test]
    fn the_starting_curve_has_a_basis_at_p376() {
        starting::<P376>();
    }

    #[test]
    fn the_starting_curve_has_a_basis_at_p500() {
        starting::<P500>();
    }

    #[test]
    fn the_reference_keys_have_a_full_order_basis() {
        order_is_two_to_the_e::<P248>(
            "e1bea27fb0d6c4f6ee3e8453aa04de40159a4504ccdcd20297e961a683f30003c9318e7f04701121a0d291ee5b1f2a65af8a9c8c514194517b402d4877f7bf01",
            11,
        );
        order_is_two_to_the_e::<P376>(
            "6d8bff5340b0a6c2e120b56435381403147276e74ccc89fbfead5477453b8aba577d66678b65c320a6f89dab7a9ef3303990b85b491234294c5a5dc9c52396dcde9fa60e735339f2fd50c7df930fee6b313b6c85a7bba7c5da8bf4d90ee3eb2f",
            21,
        );
        order_is_two_to_the_e::<P500>(
            "9e70ccc3c05b879b170848e55c9811a4146ef12a3c39fcac82a06dc1b4524badf0f8f07b569e0edb63835109563058271e3a92724195121fc00ce08354523801685d9220043cbe9565303c9e8dc6da07cffbddc23a8ccd0a221199fc62d0e7d30577671fb73fa36fb9e7641587c2a0d86213b16c038038e0389c51f647bc0200",
            2,
        );
    }
}
