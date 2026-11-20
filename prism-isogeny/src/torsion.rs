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
//!
//! # What these functions do not test
//!
//! All of this is x-only arithmetic ([`crate::montgomery`]): it runs the
//! same on any coefficient and any abscissa, of the curve or of its
//! twist, and returns well-formed points. That the points are a basis is
//! true only when the curve has its `2^e`-torsion rational and the index
//! is the one the search finds for that curve. The functions that
//! *compute* points ([`even_basis`], [`difference`],
//! [`mul_odd_then_double`], [`double_times`]) test none of it. The
//! functions that *test* ([`has_order_p_plus_one`], and the two the
//! searches use) say so. The verifier tests what [`even_basis`] returns
//! with its own `basis_e` before using it, and holds the result in a type
//! that can be made no other way (`KeyBasis` in `prism-verify`).

use prism_field::{Cofactor, Field, ParameterSet};

use crate::jac::xdbl_iter;
use crate::montgomery::{a24, ladder, Montgomery, Xz};

/// `(P, R, Q)` in x-only coordinates: what [`even_basis`] computed.
///
/// `Q` is [`difference`] of the cleared `P` and `R`. `full_difference` is
/// the difference of the points before the cofactor, and clearing that
/// point is not `Q`.
///
/// Plain data with no invariant. The fields are public, and a value says
/// nothing about the orders of its points: on a curve whose group order
/// is not `(p+1)²` the "cleared" points need not have order dividing
/// `2^e`, and on any curve they need not be independent. The name is the
/// intent. Whoever uses one as a basis tests it.
#[derive(Clone, Copy, Debug)]
pub struct EvenBasis<F> {
    /// `[f]` of `full_p`.
    pub p: Xz<F>,
    /// `[f]` of `full_q`. This is `R`, not the second basis point.
    pub q: Xz<F>,
    /// `difference(p, q)`, the canonical second basis point `Q`.
    pub difference: Xz<F>,
    /// The point the basis index names, before multiplication by `f`.
    pub full_p: Xz<F>,
    /// The point with abscissa `−x(full_p) − A`.
    pub full_q: Xz<F>,
    /// `difference(full_p, full_q)`.
    pub full_difference: Xz<F>,
}

/// A basis `(R, T)` of `E[p+1]`, with `R − T`: what [`rational_basis`]
/// or [`starting_basis`] found.
///
/// The invariant is that both points have order `p+1`, that their even
/// parts generate `E[2^e]`, and that their odd parts generate `E[f]`,
/// on the curve the basis was found on. The fields are private and those
/// two functions are the only way to a value; each returns one only
/// after testing all three. The value does not carry its curve, so using
/// it on another is the caller's mistake to avoid.
///
/// Not on the verifier's path. The verifier's tests use it to build
/// inputs on the starting curve.
#[derive(Clone, Copy, Debug)]
pub struct RationalBasis<F> {
    r: Xz<F>,
    t: Xz<F>,
    difference: Xz<F>,
    even: EvenBasis<F>,
}

impl<F: Copy> RationalBasis<F> {
    /// `R`.
    pub fn r(&self) -> Xz<F> {
        self.r
    }

    /// `T`.
    pub fn t(&self) -> Xz<F> {
        self.t
    }

    /// [`difference`] of `R` and `T`.
    pub fn difference(&self) -> Xz<F> {
        self.difference
    }

    /// `[f]R` as `p`, `[f]T` as `q`, `[f]` of the difference as
    /// `difference`, and the three uncleared points as the `full_`
    /// fields. Unlike the value [`even_basis`] returns, `difference`
    /// here is the cleared difference, not the difference of the cleared
    /// points.
    pub fn even(&self) -> EvenBasis<F> {
        self.even
    }
}

fn conjugate<L: ParameterSet>(value: L::Ext) -> L::Ext {
    let (re, im) = L::ext_parts(value);
    L::ext_from_base(re).sub(L::ext_i().mul(L::ext_from_base(im)))
}

/// `x(P)` for the index the basis search settled on: `−A / (1 + index·i)`
/// when `A` is a square, and `index·A` when it is not.
///
/// `None` only when `1 + index·i` is zero, which no 16-bit index makes
/// it at a parameter set. `coefficient_is_square` is trusted: with the
/// wrong flag this is the other formula's abscissa.
pub(crate) fn basis_abscissa<L: ParameterSet>(
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

/// One of `x(P − Q)` and `x(P + Q)` on the curve `y^2 = x^3 + A x^2 + x`,
/// chosen by a fixed rule: the reference's `difference_point`.
///
/// The two are the roots in `X/Z` of `Bzz·X² − 2·Bxz·XZ + Bxx·Z² = 0`,
/// where `Bxx`, `Bxz` and `Bzz` are the biquadratic forms of the curve at
/// the two points. The result is `(Bxz + √(Bxz² − Bxx·Bzz) : Bzz)`.
///
/// # The square root is part of the protocol
///
/// The other root gives the other of the two points. A signer and a
/// verifier that took different roots here would hold different second
/// basis points for the same key. The root taken is the one
/// [`Field::sqrt`] returns at a parameter set's extension field: real
/// part even, or imaginary part even when the real part is zero
/// (`prism_field::ParameterSet::Ext`, tested there at every set). That
/// is the rule of the reference's `fp2_sqrt`.
///
/// # The coordinates are part of it too
///
/// The forms are scaled before the root is taken, by
/// `C̄²·C·P̄z²·Q̄z²` with `C = 1` here. That scaling does not make the
/// choice independent of how the two points are written. The result is
/// a function of the four coordinates passed, and for the same two
/// points in another projective scaling it is the other of the two
/// candidates about half the time: the rule reads the parity of the
/// root, and multiplying the root by a factor from the base field
/// changes that parity as often as not. Measured on 1,980 rescalings of
/// 66 pairs of points at each of `p248` and `p500`, the point was the
/// same for 999 and for 972.
///
/// So "the difference of `P` and `Q`" is defined only together with the
/// coordinates of `P` and `Q`. [`even_basis`] passes the coordinates the
/// cofactor ladder returns, unnormalized, and a key's second basis point
/// is defined by that. The C reference does the same: its basis
/// functions set both points to `(x : 1)`, multiply each by the cofactor
/// with its Montgomery ladder, and pass the results to
/// `difference_point` with nothing between. That this crate's ladder
/// returns the reference's coordinates is observed, on the keys of the
/// committed vectors, and not proved.
///
/// # What it assumes
///
/// That `p` and `q` are finite points of the curve itself, with
/// different abscissas. Nothing is tested.
///
/// # When that fails
///
/// `Some` is not evidence of a point. A caller that needs one refuses a
/// result with `Z = 0`.
///
/// - An input with `Z = 0`, which is infinity or `(0 : 0)`: the scaling
///   is zero, the three forms vanish, and the result is `Some((0 : 0))`,
///   which is no point and reads as infinity.
/// - The same abscissa twice, `Q = ±P`: `Bzz` is zero and so is the
///   result's `Z`. By the root it is `(2·Bxz : 0)`, the point at infinity
///   that `P − P` is, or `(0 : 0)`.
/// - `None` when the discriminant is not a square, which does not happen
///   for two points of the curve itself.
///
/// # Who establishes it in the verifier
///
/// The verifier calls this only inside [`even_basis`], on coordinates
/// that function computed, and nothing there keeps them from being
/// degenerate: a point that the cofactor clears to infinity gives
/// `(0 : 0)` here. The verifier then tests the result's order and
/// independence (`basis_e` in `prism-verify`), and the order test
/// requires `[2^(e−1)]` of the point not to be infinity, which no result
/// with `Z = 0` passes.
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
    // The point that leaves this function depends on which root this is:
    // the other one gives `x(P + Q)` where this gives `x(P − Q)`, or the
    // reverse. The choice is part of the protocol and is the extension
    // field's rule: see "The square root is part of the protocol" above.
    let root = disc.sqrt()?;
    Some(Xz {
        x: bxz.add(root),
        z: bzz,
    })
}

fn clear_cofactor<L: ParameterSet>(coefficient: L::Ext, point: Xz<L::Ext>) -> Option<Xz<L::Ext>> {
    let constant = a24(coefficient)?;
    let bits = (0..L::COFACTOR.bits())
        .rev()
        .map(|shift| (L::COFACTOR.value() >> shift) & 1 == 1);
    Some(ladder(constant, bits, point))
}

/// The points the basis index names on the curve of `coefficient`: the
/// index point and the point above `(0, 0)`, each multiplied by the odd
/// cofactor `f`, and the [`difference`] of the two results.
///
/// `None` when [`difference`] finds no root, for the uncleared points or
/// the cleared ones.
///
/// # What it assumes
///
/// That the curve is one whose `2^e`-torsion is rational, and that
/// `index` and `coefficient_is_square` are what the basis search found
/// for it. Neither is tested here.
///
/// # When that fails
///
/// It still returns three points. On a curve without the torsion, or
/// with an index the search would not have chosen, they are not a basis
/// of anything, and nothing in the value says so.
///
/// # Who establishes it in the verifier
///
/// `prism-verify` computes the index and the flag itself (`basis_hint`),
/// takes the result as points that have only been *named*
/// (`NamedPoints`), and makes a basis of them only through `basis_e`:
/// `p` and `difference` each of order exactly `2^e`, and their
/// `[2^(e−1)]` multiples distinct.
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

/// `[odd · 2^doubles] point`, x-only: the ladder by `odd`, then
/// `doubles` doublings. `odd` may be any integer; the name is how the
/// callers use it.
///
/// `None` only where 4 has no inverse, which is in no field of a
/// parameter set. What [`crate::montgomery::ladder`] assumes of the point
/// is assumed here.
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
///
/// That is order exactly `p+1`, of the x-coordinate: the point is on the
/// curve or on its twist, and this does not say which. On a curve whose
/// group is `(Z/(p+1))²` the twist has no such point, because its group
/// is `(Z/(p−1))²` and `gcd(p+1, p−1) = 2`.
pub fn has_order_p_plus_one<L: ParameterSet>(coefficient: L::Ext, point: Xz<L::Ext>) -> bool {
    let f = L::COFACTOR.value();
    let killed = |odd, doubles| {
        mul_odd_then_double::<L>(coefficient, point, odd, doubles).is_some_and(is_infinity)
    };
    if !killed(f, L::E) {
        return false;
    }
    if killed(f, L::E - 1) {
        return false;
    }
    // Every prime of `f`, and no other: that is what a `Cofactor` is.
    L::COFACTOR
        .odd_primes()
        .iter()
        .all(|&prime| !killed(f / prime, L::E))
}

/// `[2^{e−1}]` of two order-`2^e` points are distinct and nonzero.
///
/// `E[2^e] ≅ (Z/2^e)^2`, so `E[2]` has three nonzero points and they are the
/// nonzero vectors of `(Z/2)^2`. Two distinct ones are a basis of that
/// vector space, and therefore the two points generate `E[2^e]`.
///
/// That the two points have order dividing `2^e` is the caller's: this
/// tests their `[2^{e−1}]` multiples and nothing else.
pub(crate) fn even_parts_generate<L: ParameterSet>(
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

/// The images of `R` and `T` in `E[q]`, for one odd prime `q` of `f`.
///
/// Held until every prime has been projected. Comparing inside the
/// projection loop would run a cofactor with two primes in a different
/// field-operation order.
#[derive(Clone, Copy)]
struct OddImage<F: Field> {
    left: Xz<F>,
    right: Xz<F>,
    prime: u64,
}

/// The images of `R` and `T` in `E[f]` generate `E[f]`.
///
/// `E[f]` splits into prime-power factors, one for each prime in
/// [`Cofactor::odd_primes`]. Two elements generate `(Z/p^k)^2` when
/// their images generate `(Z/p)^2`, and two elements generate `(Z/q)^2`
/// for a prime `q` when neither is a scalar multiple of the other. A
/// scalar and its negative have the same x-coordinate, so the multiples
/// checked run only up to `q/2`.
///
/// That both points have order dividing `p+1` is the caller's; the two
/// searches test it with [`has_order_p_plus_one`] first.
pub(crate) fn odd_parts_generate<L: ParameterSet>(
    coefficient: L::Ext,
    r: Xz<L::Ext>,
    t: Xz<L::Ext>,
) -> bool {
    let primes = L::COFACTOR.odd_primes();
    let u = double_times::<L>(coefficient, r, L::E);
    let v = double_times::<L>(coefficient, t, L::E);
    // The image of each point in `E[q]`, for each prime `q` of `f`:
    // multiplying by `f/q` kills the other primes and leaves order `q`.
    // Every projection that succeeds is made before any is compared. A
    // projection that fails refuses immediately.
    //
    // One slot for each prime a cofactor can have. A `Cofactor` never has
    // more, so indexing by position reaches every prime, and a list too
    // long to hold cannot be built to get here.
    let mut images: [Option<OddImage<L::Ext>>; Cofactor::MAX_ODD_PRIMES] =
        [None; Cofactor::MAX_ODD_PRIMES];
    for (index, &prime) in primes.iter().enumerate() {
        let rest = L::COFACTOR.value() / prime;
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
        images[index] = Some(OddImage { left, right, prime });
    }
    // The prefix of `primes.len()` slots was filled above. A hole refuses:
    // it is not a prime that was checked.
    images[..primes.len()].iter().all(|image| {
        image.as_ref().is_some_and(|image| {
            prime_field_basis::<L>(coefficient, image.left, image.right, image.prime)
        })
    })
}

/// `(R, T)` a basis of `E[p+1]`.
///
/// `R` is the point named by the basis index, before multiplication by `f`.
/// `T` is the point whose affine x-coordinate is the smallest positive
/// integer for which both points have order `p+1`, their even parts generate
/// `E[2^e]`, and their odd parts generate `E[f]`. The search stops at
/// [`ParameterSet::SEARCH_BOUND`]
/// candidates. `R − T` is [`difference`].
///
/// `None` when the index point does not have order `p+1`, when no
/// candidate within the bound completes a basis, or when a
/// [`difference`] has no root. Every property of the result is tested
/// here before it is returned.
///
/// Not on the verifier's path: the verifier needs the `2^e`-torsion
/// only. Its tests call this.
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
///
/// `None` when either search ends at the bound without a point. Every
/// property of the result is tested here before it is returned.
///
/// Not on the verifier's path, which refuses the starting curve as a
/// key. Its tests call this.
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

/// `[2^times] point` on the curve `y^2 = x^3 + coefficient·x^2 + x`,
/// x-only. Infinity is `Z = 0`.
///
/// Nothing is tested: the point is on the curve or its twist, and
/// `(0 : 0)` stays `(0 : 0)`. The verifier's order tests are built from
/// this (`order_e` and `span_e` in `prism-verify`): they ask for `Z = 0`
/// after `e` doublings and `Z ≠ 0` after `e − 1`, which no `(0 : 0)`
/// and no point of lower order passes.
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
    use prism_field::{Field, ParameterSet, P248, P324, P376, P500, P664};

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
            assert!(!L::COFACTOR.odd_primes().is_empty(), "{}", L::NAME);
            for &prime in L::COFACTOR.odd_primes() {
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

    /// `difference` returns `Some` with `Z = 0` where it has no point to
    /// return: `(0 : 0)` for an input at infinity, and a value with
    /// `Z = 0` for the same abscissa twice.
    #[test]
    fn the_difference_of_degenerate_inputs_has_z_zero() {
        use super::difference;
        use crate::montgomery::Xz;
        type Ext = <P248 as ParameterSet>::Ext;
        let coefficient = Ext::zero();
        let point = Xz::affine(Ext::from_u64(2));
        for (p, q) in [(Xz::infinity(), point), (point, Xz::infinity())] {
            let got = difference::<P248>(coefficient, p, q).expect("zero has a root");
            assert!(got.x.is_zero() && got.z.is_zero());
        }
        let twice = difference::<P248>(coefficient, point, point).expect("a square");
        assert!(twice.z.is_zero());
    }

    #[test]
    fn a_point_missing_a_prime_of_the_cofactor_is_not_full_order_at_any_set() {
        only_the_full_point_is_full_order::<P248>();
        only_the_full_point_is_full_order::<P324>();
        only_the_full_point_is_full_order::<P376>();
        only_the_full_point_is_full_order::<P500>();
        only_the_full_point_is_full_order::<P664>();
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
    fn the_starting_curve_has_a_basis_at_p664() {
        starting::<P664>();
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
