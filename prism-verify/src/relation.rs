// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Order and span checks, the full-order Weil pairing, and the two-chain
//! accept path at exponent `e′ = e − 2`. Historical single-chain probes
//! remain in tests to document the former basis and structure failures.

use prism_field::{Field, ParameterSet};
#[cfg(test)]
use prism_isogeny::chain::{chain_surface, SurfaceStop};
#[cfg(test)]
use prism_isogeny::gluing::{Couple, Kernel};
#[cfg(test)]
use prism_isogeny::jac::Jac;
#[cfg(test)]
use prism_isogeny::jac::{add, to_xz};
#[cfg(test)]
use prism_isogeny::montgomery::Montgomery;
use prism_isogeny::montgomery::Xz;
#[cfg(test)]
use prism_isogeny::montgomery::{a24, ladder};
use prism_isogeny::pairing::{neg_square_mod_pow2, pow, weil};
#[cfg(test)]
use prism_isogeny::torsion::EvenBasis;
use prism_isogeny::torsion::{double_times, even_basis};

use crate::accept::Lifted;
use crate::basis::{basis_hint, BasisHint};
use crate::challenge::ChallengeScalar;
use crate::identity::KeyIdentity;
use crate::mark::Stage;
use crate::signature::{AffinePoint, Response};
use crate::VerifyError;

/// The isogeny relation for a key the public checks have already accepted.
pub fn relate<L: ParameterSet>(
    identity: &KeyIdentity<'_, L>,
    response: &Response<L>,
    scalar: &ChallengeScalar,
) -> Result<(), VerifyError> {
    relate_points::<L>(
        identity.coefficient(),
        response.curve,
        response.p,
        response.q,
        scalar.as_bytes(),
    )
}

/// The same relation, from the coefficient and the points. The starting
/// curve is no key here either: a zero `vk` is [`VerifyError::Origin`],
/// as it is in `verify`, which refuses it before it gets here.
///
/// The key's points are computed first and tested last of the checks
/// before the pairing, after the signature's: that is the order this
/// entry has always refused in, and its counts are the record.
pub fn relate_points<L: ParameterSet>(
    vk: L::Ext,
    sig: L::Ext,
    p_sig: AffinePoint<L>,
    q_sig: AffinePoint<L>,
    scalar: &[u8],
) -> Result<(), VerifyError> {
    Stage::Torsion.enter();
    if vk.is_zero() {
        return Err(VerifyError::Origin);
    }
    // A public key uses the canonical basis of `E[2^e]`: `Q` is
    // `difference(P, R)` after the cofactor.
    let named = NamedPoints::of_key(vk, basis_hint::<L>(vk)?).ok_or(VerifyError::Kernel)?;
    response_points::<L>(sig, p_sig, q_sig)?;
    let basis = named.checked().ok_or(VerifyError::Kernel)?;
    relate_bases::<L>(&basis, sig, p_sig, q_sig, scalar)
}

/// The relation for a key whose basis is already built and checked: the
/// entry of [`crate::verify_prepared`]. It differs from [`relate_points`]
/// in one thing, that it neither searches for the key's basis nor
/// computes nor tests it: `basis` is all three, done once.
pub(crate) fn relate_prepared<L: ParameterSet>(
    basis: &KeyBasis<L>,
    response: &Response<L>,
    scalar: &[u8],
) -> Result<(), VerifyError> {
    Stage::Torsion.enter();
    response_points::<L>(response.curve, response.p, response.q)?;
    relate_bases::<L>(basis, response.curve, response.p, response.q, scalar)
}

/// The points a basis index names on a key's curve, before anyone has
/// asked whether they are a basis.
///
/// The index search finds abscissas on the curve and does not show what
/// they generate: a key whose curve does not have its `2^e`-torsion
/// rational passes the search and names points that are no basis. The
/// one thing to do with a value of this type is [`Self::checked`].
struct NamedPoints<L: ParameterSet> {
    coefficient: L::Ext,
    p: Xz<L::Ext>,
    q: Xz<L::Ext>,
    difference: Xz<L::Ext>,
}

impl<L: ParameterSet> NamedPoints<L> {
    /// The points the index `hint` names on the curve of `coefficient`.
    /// `None` when the index names no point or the points have no
    /// difference.
    fn of_key(coefficient: L::Ext, hint: BasisHint) -> Option<Self> {
        let even = even_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square())?;
        Some(Self {
            coefficient,
            p: even.p,
            q: even.q,
            difference: even.difference,
        })
    }

    /// The test, and the only way to a [`KeyBasis`].
    fn checked(self) -> Option<KeyBasis<L>> {
        basis_e::<L>(self.coefficient, self.p, self.difference).then_some(KeyBasis {
            coefficient: self.coefficient,
            p: self.p,
            q: self.q,
            difference: self.difference,
        })
    }
}

/// A key's basis of `E[2^e]`, tested, with the curve it is a basis on:
/// `p` and `difference` each have order exactly `2^e` on the curve of
/// `coefficient` and together span its torsion.
///
/// The fields are private to this module and the one way to a value is
/// [`NamedPoints::checked`], so a `KeyBasis` is a basis by construction
/// and code that takes one tests nothing again. `q` is the third point
/// the lift needs, the one with `q = p − difference`.
///
/// It holds the coefficient the test was run with, and the relation and
/// the lift read the key's curve from here and take no other. So the
/// points cannot be used on a curve they were not tested on.
#[derive(Clone, Copy)]
pub(crate) struct KeyBasis<L: ParameterSet> {
    coefficient: L::Ext,
    p: Xz<L::Ext>,
    q: Xz<L::Ext>,
    difference: Xz<L::Ext>,
}

impl<L: ParameterSet> KeyBasis<L> {
    /// Compute the points `hint` names on the curve of `coefficient` and
    /// test them.
    ///
    /// [`VerifyError::Basis`] when they are not a basis of `E[2^e]`: the
    /// key gives no usable basis, as it gives none when the search finds
    /// no index.
    pub(crate) fn of_key(coefficient: L::Ext, hint: BasisHint) -> Result<Self, VerifyError> {
        NamedPoints::of_key(coefficient, hint)
            .and_then(NamedPoints::checked)
            .ok_or(VerifyError::Basis)
    }

    /// The coefficient of the curve this is a basis on.
    pub(crate) fn coefficient(&self) -> L::Ext {
        self.coefficient
    }

    /// `P`, of order `2^e`.
    pub(crate) fn p(&self) -> Xz<L::Ext> {
        self.p
    }

    /// The cleared point with abscissa `−x(P) − A` before the cofactor.
    pub(crate) fn q(&self) -> Xz<L::Ext> {
        self.q
    }

    /// `difference(p, q)`, the canonical second basis point.
    pub(crate) fn difference(&self) -> Xz<L::Ext> {
        self.difference
    }
}

/// The signature's two points are on its curve and are a basis of its
/// `2^e`-torsion. `Kernel` otherwise.
fn response_points<L: ParameterSet>(
    sig: L::Ext,
    p_sig: AffinePoint<L>,
    q_sig: AffinePoint<L>,
) -> Result<(), VerifyError> {
    if !on_curve::<L>(sig, p_sig.x, p_sig.y) || !on_curve::<L>(sig, q_sig.x, q_sig.y) {
        return Err(VerifyError::Kernel);
    }
    if !basis_e::<L>(sig, Xz::affine(p_sig.x), Xz::affine(q_sig.x)) {
        return Err(VerifyError::Kernel);
    }
    Ok(())
}

/// Everything after both bases are known to be bases: the pairing, then
/// the chains.
fn relate_bases<L: ParameterSet>(
    basis: &KeyBasis<L>,
    sig: L::Ext,
    p_sig: AffinePoint<L>,
    q_sig: AffinePoint<L>,
    scalar: &[u8],
) -> Result<(), VerifyError> {
    // Before either chain. The degree-`q` witness on the starting curve
    // calls the chain helper itself and does not come through here.
    Stage::Pairing.enter();
    let Some(lifted) = crate::accept::lift::<L>(basis, sig, p_sig, q_sig, scalar) else {
        return Err(VerifyError::Pairing);
    };
    if !pairing_holds::<L>(&lifted, scalar) {
        return Err(VerifyError::Pairing);
    }
    crate::accept::check::<L>(&lifted, scalar)
}

/// `e_{2^e}([s]P_sig, [s]Q_sig) = e_{2^e}(P, Q)^q`.
///
/// `Q` is the cleared difference. `P − Q` for the lift is `R`, the cleared
/// point whose full-point abscissa is `−x(P) − A`. A global sign on the
/// lifted pair cancels in the pairing.
fn pairing_holds<L: ParameterSet>(lifted: &Lifted<L::Ext>, scalar: &[u8]) -> bool {
    Stage::Pairing.enter();
    let [r1, r2] = lifted.scaled;
    let [p_vk, q_vk] = lifted.vk;
    let (Some(left), Some(right)) = (
        weil(lifted.curves[1], r1, r2, L::E),
        weil(lifted.vk_curve, p_vk, q_vk, L::E),
    ) else {
        return false;
    };
    left.same(pow(right, &neg_square_mod_pow2(scalar, L::E)))
}

/// The `e′` chain for a basis and signature that have already passed the
/// order and span checks.
#[cfg(test)]
fn e_prime_surface<L: ParameterSet>(
    vk: L::Ext,
    even: &EvenBasis<L::Ext>,
    push: [Xz<L::Ext>; 3],
    sig: L::Ext,
    p_sig: AffinePoint<L>,
    q_sig: AffinePoint<L>,
    scalar: &[u8],
) -> Result<prism_isogeny::chain::SurfaceReadings<L::Ext>, SurfaceStop> {
    let p_sig_x = Xz::affine(p_sig.x);
    let q_sig_x = Xz::affine(q_sig.x);
    let diff = difference::<L>(sig, p_sig.x, p_sig.y, q_sig.x, q_sig.y);
    let kernel = Kernel {
        t1: Couple {
            first: double_times::<L>(vk, even.p, 2),
            second: double_times::<L>(sig, p_sig_x, 2),
        },
        t2: Couple {
            first: double_times::<L>(vk, even.q, 2),
            second: double_times::<L>(sig, q_sig_x, 2),
        },
        // `even.difference` is `difference` of the cleared points, which is
        // the C canonical `Q`. Clearing the uncleared difference is not.
        difference: Couple {
            first: double_times::<L>(vk, even.difference, 2),
            second: double_times::<L>(sig, diff, 2),
        },
    };
    let curves = [Montgomery::normalized(vk), Montgomery::normalized(sig)];
    let points = pushed::<L>(vk, push, scalar);
    chain_surface(L::E - 2, curves, kernel, false, &points, true)
}

/// `p` and `q` are a basis of the `2^e`-torsion of the curve: each has
/// order exactly `2^e`, and together they span it.
///
/// This is the whole of what the verifier asks of a pair of kernel
/// points before it uses them, and it asks it the same way of the
/// signature's pair and of the key's. The order test is not implied by
/// the span test: a point with a component of odd order passes
/// [`span_e`]. Nor is it implied by what follows: the two points are
/// used only through their multiples by the challenge scalar `s`, so a
/// component whose order divides `s` is gone before the pairing sees
/// it, and without the order test a signature carrying one is accepted.
fn basis_e<L: ParameterSet>(coefficient: L::Ext, p: Xz<L::Ext>, q: Xz<L::Ext>) -> bool {
    order_e::<L>(coefficient, p) && order_e::<L>(coefficient, q) && span_e::<L>(coefficient, p, q)
}

/// Order exactly `2^e`.
fn order_e<L: ParameterSet>(coefficient: L::Ext, point: Xz<L::Ext>) -> bool {
    let full = double_times::<L>(coefficient, point, L::E);
    let almost = double_times::<L>(coefficient, point, L::E - 1);
    full.z.is_zero() && !almost.z.is_zero()
}

/// `[2^{e−1}]` of two order-`2^e` points are distinct and nonzero.
fn span_e<L: ParameterSet>(coefficient: L::Ext, p: Xz<L::Ext>, q: Xz<L::Ext>) -> bool {
    let p2 = double_times::<L>(coefficient, p, L::E - 1);
    let q2 = double_times::<L>(coefficient, q, L::E - 1);
    !p2.is_infinity() && !q2.is_infinity() && !p2.x.mul(q2.z).same(q2.x.mul(p2.z))
}

#[cfg(test)]
fn pushed<L: ParameterSet>(
    vk: L::Ext,
    bases: [Xz<L::Ext>; 3],
    scalar: &[u8],
) -> [Couple<L::Ext>; 6] {
    let at = |point: Xz<L::Ext>| Couple {
        first: point,
        second: Xz::infinity(),
    };
    [
        at(scale::<L>(vk, bases[0], scalar)),
        at(scale::<L>(vk, bases[1], scalar)),
        at(scale::<L>(vk, bases[2], scalar)),
        at(double_times::<L>(vk, bases[0], L::E - 3)),
        at(double_times::<L>(vk, bases[1], L::E - 3)),
        at(double_times::<L>(vk, bases[2], L::E - 3)),
    ]
}

fn on_curve<L: ParameterSet>(coefficient: L::Ext, x: L::Ext, y: L::Ext) -> bool {
    let square = x.square();
    y.square()
        .same(square.mul(x).add(coefficient.mul(square)).add(x))
}

#[cfg(test)]
fn difference<L: ParameterSet>(
    coefficient: L::Ext,
    px: L::Ext,
    py: L::Ext,
    qx: L::Ext,
    qy: L::Ext,
) -> Xz<L::Ext> {
    let curve = Montgomery::normalized(coefficient);
    to_xz(add(
        curve,
        Jac {
            x: px,
            y: py,
            z: L::Ext::one(),
        },
        Jac {
            x: qx,
            y: qy.negate(),
            z: L::Ext::one(),
        },
    ))
}

#[cfg(test)]
fn scale<L: ParameterSet>(coefficient: L::Ext, point: Xz<L::Ext>, scalar: &[u8]) -> Xz<L::Ext> {
    let Some(constant) = a24(coefficient) else {
        return Xz::infinity();
    };
    let mut bits = Vec::new();
    for byte in scalar.iter().rev() {
        for shift in (0..8).rev() {
            bits.push((byte >> shift) & 1 == 1);
        }
    }
    while bits.first() == Some(&false) {
        bits.remove(0);
    }
    if bits.is_empty() {
        return Xz::infinity();
    }
    ladder(constant, bits.into_iter(), point)
}

#[cfg(test)]
mod e0_vectors {
    use super::e_prime_surface;
    use crate::signature::{canonical_sign, AffinePoint};
    use crate::verify;
    use crate::Committed;
    use crate::VerifyError;
    use prism_field::{Field, ParameterSet, P248};
    #[cfg(test)]
    use prism_isogeny::chain::SurfaceStop;
    use prism_isogeny::endomorphism::{apply, multiply};
    use prism_isogeny::jac::{lift_basis, Jac};
    use prism_isogeny::montgomery::Montgomery;
    use prism_isogeny::torsion::starting_basis;

    fn decimal(text: &str) -> Vec<u8> {
        let mut bytes = vec![0u8];
        for digit in text.bytes() {
            let mut carry = u16::from(digit - b'0');
            for byte in &mut bytes {
                carry += u16::from(*byte) * 10;
                *byte = carry as u8;
                carry >>= 8;
            }
            while carry > 0 {
                bytes.push(carry as u8);
                carry >>= 8;
            }
        }
        bytes
    }

    fn field<'a>(text: &'a str, label: &str) -> &'a str {
        text.lines()
            .find_map(|line| line.strip_prefix(label))
            .unwrap()
            .trim()
    }

    fn affine<L: ParameterSet>(point: Jac<L::Ext>) -> (L::Ext, L::Ext) {
        let z = point.z.inv().expect("finite");
        let z2 = z.square();
        (point.x.mul(z2), point.y.mul(z2.mul(z)))
    }

    fn signature<L: ParameterSet>(block: &str) -> (AffinePoint<L>, AffinePoint<L>, Vec<u8>) {
        let basis = starting_basis::<L>().expect("E0 basis");
        let (_, p, q) = lift_basis(
            Montgomery::normalized(L::Ext::zero()),
            basis.even().p,
            basis.even().q,
            basis.even().difference,
        )
        .expect("lift");
        let a = decimal(field(block, "a "));
        let b = decimal(field(block, "b "));
        let t = decimal(field(block, "t "));
        let c: i64 = field(block, "c ").parse().unwrap();
        let d: i64 = field(block, "d ").parse().unwrap();
        let p = multiply::<L>(apply::<L>(p, &a, &b, c, d), &t);
        let q = multiply::<L>(apply::<L>(q, &a, &b, c, d), &t);
        let (px, py) = affine::<L>(p);
        let (qx, qy) = affine::<L>(q);
        let mut p: AffinePoint<L> = AffinePoint { x: px, y: py };
        let mut q: AffinePoint<L> = AffinePoint { x: qx, y: qy };
        if !canonical_sign::<L>(p.y) {
            p.y = p.y.negate();
            q.y = q.y.negate();
        }
        (p, q, decimal(field(block, "s ")))
    }

    fn exercise<L: ParameterSet>(block: &str) {
        let (p, q, s) = signature::<L>(block);
        let zero = L::Ext::zero();
        let basis = starting_basis::<L>().expect("E0 basis");
        // The chain is the one-chain shape. This witness is the graph of one
        // endomorphism of E0. In the C tree that kernel is
        // `endomorphism_application_even_basis` on `BASIS_EVEN` of
        // `CURVE_E0`, and `theta_chain_compute_and_eval` stops the same way.
        // A theta coordinate of the step's kernel vanishes at step 3, which
        // the (2,2) step refuses, so both readings are not produced. The
        // witness calls the chain helper directly: `relate_points` now runs
        // the pairing check, which this graph is not.
        assert!(matches!(
            e_prime_surface::<L>(
                zero,
                &basis.even(),
                [basis.r(), basis.t(), basis.difference()],
                zero,
                p,
                q,
                &s,
            ),
            Err(SurfaceStop::Step(3))
        ));
    }

    #[test]
    fn the_degree_q_graph_stops_before_both_readings() {
        let text = prism_vectors::kat::E0_ENDOMORPHISMS;
        // The file's blocks are headed by the committed label; `p248`'s
        // runs from its heading to the next one.
        let heading = format!("{}\n", P248::LABEL);
        let rest = &text[text.find(&heading).unwrap() + heading.len()..];
        let block = &rest[..rest
            .find(<prism_field::P376 as Committed>::LABEL)
            .unwrap_or(rest.len())];
        exercise::<P248>(block);
        for length in [260usize, 388, 516] {
            let err = verify::<P248>(&[0u8; P248::PUBLIC_KEY_LEN], b"m", &vec![0u8; length]);
            assert_eq!(err, Err(VerifyError::Length));
        }
    }
}

#[cfg(test)]
mod s5_vectors {
    use super::{basis_e, e_prime_surface, on_curve, order_e, relate_points, span_e};
    use crate::basis::basis_hint;
    use crate::challenge::ChallengeScalar;
    use crate::identity::KeyIdentity;
    use crate::prepared::PreparedKey;
    use crate::prime::degree_is_prime;
    use crate::signature::{canonical_sign, AffinePoint, Response};
    use crate::verify;
    use crate::VerifyError;
    use prism_field::{Field, ParameterSet, Salt, P248, P376, P500};
    #[cfg(test)]
    use prism_isogeny::chain::SurfaceStop;
    use prism_isogeny::jac::{add, dbl, dbl_iter, recover_y, to_xz, Jac};
    use prism_isogeny::montgomery::{Montgomery, Xz};
    use prism_isogeny::pairing::{mul, weil};
    use prism_isogeny::torsion::{even_basis, has_order_p_plus_one, rational_basis};

    fn decimal_le(text: &str) -> Vec<u8> {
        let mut bytes = vec![0u8];
        for digit in text.bytes() {
            let mut carry = u16::from(digit - b'0');
            for byte in &mut bytes {
                carry += u16::from(*byte) * 10;
                *byte = carry as u8;
                carry >>= 8;
            }
            while carry > 0 {
                bytes.push(carry as u8);
                carry >>= 8;
            }
        }
        bytes
    }

    struct Sample<L: ParameterSet> {
        key: Vec<u8>,
        message: Vec<u8>,
        response: Response<L>,
        scalar: Vec<u8>,
    }

    fn sample<L: ParameterSet>(text: &str) -> Sample<L> {
        let vector = prism_vectors::corpus::Vector::new(text);
        let key = vector.public_key().expect("the vector's public key");
        let identity = KeyIdentity::<L>::decode(&key).expect("key");
        let message = vector.message().expect("the vector's message").to_vec();
        let salt = L::Salt::from_bytes(&vector.salt().expect("the vector's salt")).unwrap();
        let scalar = ChallengeScalar::derive::<L>(&identity, &message, salt);
        let mut from_file = decimal_le(vector.field("s").expect("the vector's scalar"));
        from_file.resize(scalar.as_bytes().len(), 0);
        assert_eq!(from_file, scalar.as_bytes());
        let element = |name| L::ext_decode(&vector.bytes(name).expect(name)).unwrap();
        Sample {
            key,
            message,
            response: Response {
                curve: element("E"),
                p: AffinePoint {
                    x: element("Px"),
                    y: element("Py"),
                },
                q: AffinePoint {
                    x: element("Qx"),
                    y: element("Qy"),
                },
                salt,
            },
            scalar: scalar.as_bytes().to_vec(),
        }
    }

    /// Both readings, or the isogeny step that stopped the chain.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Stop {
        Readings,
        Step(u32),
    }

    fn surface_stop<L: ParameterSet>(sample: &Sample<L>, q_sign: L::Ext) -> Stop {
        let coefficient = KeyIdentity::<L>::decode(&sample.key).unwrap().coefficient();
        let hint = basis_hint::<L>(coefficient).unwrap();
        let even = even_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square())
            .expect("the index names a point");
        assert!(even.full_q.x.same(even.full_p.x.add(coefficient).negate()));
        let mut q = sample.response.q;
        q.y = q_sign;
        match e_prime_surface::<L>(
            coefficient,
            &even,
            [even.p, even.q, even.difference],
            sample.response.curve,
            sample.response.p,
            q,
            &sample.scalar,
        )
        .err()
        {
            None => Stop::Readings,
            Some(SurfaceStop::Step(step)) => Stop::Step(step),
            Some(other) => panic!("chain stopped at {other:?}"),
        }
    }

    fn honest<L: ParameterSet>(text: &str, chain: Stop, negated_q: Stop, full_order: bool) {
        let sample = sample::<L>(text);
        assert!(
            basis_hint::<L>(KeyIdentity::<L>::decode(&sample.key).unwrap().coefficient()).is_ok()
        );
        assert!(canonical_sign::<L>(sample.response.p.y));
        assert!(on_curve::<L>(
            sample.response.curve,
            sample.response.p.x,
            sample.response.p.y
        ));
        assert!(on_curve::<L>(
            sample.response.curve,
            sample.response.q.x,
            sample.response.q.y
        ));
        assert!(degree_is_prime::<L::Degree>(L::E - 2, &sample.scalar));
        let coefficient = KeyIdentity::<L>::decode(&sample.key).unwrap().coefficient();
        let p = Xz::affine(sample.response.p.x);
        let q = Xz::affine(sample.response.q.x);
        assert!(order_e::<L>(sample.response.curve, p));
        assert!(order_e::<L>(sample.response.curve, q));
        assert!(span_e::<L>(sample.response.curve, p, q));
        let hint = basis_hint::<L>(coefficient).unwrap();
        assert_eq!(
            has_order_p_plus_one::<L>(
                coefficient,
                even_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square())
                    .unwrap()
                    .full_p,
            ),
            full_order
        );
        assert_eq!(
            rational_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square()).is_some(),
            full_order
        );
        assert_eq!(
            surface_stop::<L>(&sample, sample.response.q.y),
            chain,
            "{}",
            L::NAME
        );
        assert_eq!(
            relate_points::<L>(
                coefficient,
                sample.response.curve,
                sample.response.p,
                sample.response.q,
                &sample.scalar,
            ),
            Ok(())
        );
        // The degree is prime at e', so verify reaches the chain.
        assert_eq!(
            verify::<L>(&sample.key, &sample.message, &sample.response.encode()),
            Ok(())
        );

        let mut wrong_sign = sample.response;
        wrong_sign.p.y = wrong_sign.p.y.negate();
        wrong_sign.q.y = wrong_sign.q.y.negate();
        assert_eq!(
            verify::<L>(&sample.key, &sample.message, &wrong_sign.encode()),
            Err(VerifyError::Sign)
        );

        let mut flipped = sample.response;
        let mut curve = L::ext_encode(flipped.curve);
        curve[0] ^= 0x01;
        flipped.curve = L::ext_decode(&curve).expect("still canonical");
        assert!(!on_curve::<L>(flipped.curve, flipped.p.x, flipped.p.y));
        assert_eq!(
            relate_points::<L>(
                coefficient,
                flipped.curve,
                flipped.p,
                flipped.q,
                &sample.scalar,
            ),
            Err(VerifyError::Kernel)
        );

        let identity = KeyIdentity::<L>::decode(&sample.key).unwrap();
        let mut composite_response = sample.response;
        composite_response.salt = (0u64..65_536)
            .find_map(|n| {
                let salt = L::Salt::from_counter(n).unwrap();
                let scalar = ChallengeScalar::derive(&identity, &sample.message, salt);
                (!degree_is_prime::<L::Degree>(L::E - 2, scalar.as_bytes())).then_some(salt)
            })
            .expect("a composite salt exists");
        assert_eq!(
            verify::<L>(&sample.key, &sample.message, &composite_response.encode()),
            Err(VerifyError::Composite)
        );

        // (P, −Q) keeps the normalized y(P). The chain still reaches both
        // readings. The pairing check refuses it before that chain runs.
        let mut negated = sample.response;
        negated.q.y = negated.q.y.negate();
        assert!(canonical_sign::<L>(negated.p.y));
        assert_eq!(
            surface_stop::<L>(&sample, negated.q.y),
            negated_q,
            "{}",
            L::NAME
        );
        assert_eq!(
            relate_points::<L>(
                coefficient,
                negated.curve,
                negated.p,
                negated.q,
                &sample.scalar,
            ),
            Err(VerifyError::Pairing)
        );

        let swapped = Response {
            p: sample.response.q,
            q: sample.response.p,
            ..sample.response
        };
        if canonical_sign::<L>(swapped.p.y) {
            // A swap is determinant −1. The pairing refuses it before the chain.
            assert_eq!(
                relate_points::<L>(
                    coefficient,
                    swapped.curve,
                    swapped.p,
                    swapped.q,
                    &sample.scalar,
                ),
                Err(VerifyError::Pairing)
            );
        } else {
            assert_eq!(
                verify::<L>(&sample.key, &sample.message, &swapped.encode()),
                Err(VerifyError::Sign)
            );
        }
    }

    fn assert_weil_values<L: ParameterSet>(text: &str, vk_hex: &str, sig_hex: &str) {
        let sample = sample::<L>(text);
        let one = L::Ext::one();
        let coefficient = KeyIdentity::<L>::decode(&sample.key)
            .expect("key")
            .coefficient();
        let vk = Montgomery::normalized(coefficient);
        let sig = Montgomery::normalized(sample.response.curve);
        let vector = prism_vectors::corpus::Vector::new(text);
        let at = |x, y| Jac {
            x: L::ext_decode(&vector.bytes(x).expect(x)).unwrap(),
            y: L::ext_decode(&vector.bytes(y).expect(y)).unwrap(),
            z: one,
        };
        let public = weil(vk, at("VkPx", "VkPy"), at("VkQx", "VkQy"), L::E).expect("e(P_vk, Q_vk)");
        let signed = weil(
            sig,
            mul(sig, at("Px", "Py"), &sample.scalar),
            mul(sig, at("Qx", "Qy"), &sample.scalar),
            L::E,
        )
        .expect("e([s]P_sig, [s]Q_sig)");
        let encoded = |value: L::Ext| {
            L::ext_encode(value)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        assert_eq!(encoded(public), vk_hex, "{} public", L::NAME);
        assert_eq!(encoded(signed), sig_hex, "{} signature", L::NAME);
    }

    #[test]
    fn the_weil_values_match_sage() {
        use prism_vectors::corpus::{CorpusSet, Primary};
        let mut rows = prism_vectors::kat::WEIL_PAIRING
            .lines()
            .filter(|line| !line.is_empty());
        assert_eq!(rows.next(), Some("file\tvk\tsig"));
        let mut seen = Vec::new();
        for row in rows {
            let mut parts = row.split('\t');
            let name = parts.next().expect("file");
            let vk = parts.next().expect("vk");
            let sig = parts.next().expect("sig");
            let primary =
                Primary::by_file_name(name).unwrap_or_else(|| panic!("unexpected row {name}"));
            assert!(!seen.contains(&primary), "{name} is listed twice");
            seen.push(primary);
            match primary.corpus_set() {
                CorpusSet::P248 => assert_weil_values::<P248>(primary.text(), vk, sig),
                CorpusSet::P376 => assert_weil_values::<P376>(primary.text(), vk, sig),
                CorpusSet::P500 => assert_weil_values::<P500>(primary.text(), vk, sig),
            }
        }
        seen.sort();
        assert_eq!(seen, Primary::ALL.to_vec());
    }

    fn assert_vk_basis<L: ParameterSet>(text: &str) {
        // The exporter writes the C canonical pair (P, Q) of E[2^e] whose
        // t·σ image is the signature: P is the cleared index point and Q is
        // the C's difference_point(P, R), which in Rust terms is
        // difference(even.p, even.q). The exported VkDx is x(P − Q) = x(R),
        // the cleared −x−A point, which is even_basis.q.
        let sample = sample::<L>(text);
        let coefficient = KeyIdentity::<L>::decode(&sample.key).unwrap().coefficient();
        let hint = basis_hint::<L>(coefficient).unwrap();
        let even = even_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square())
            .expect("canonical basis");
        let vector = prism_vectors::corpus::Vector::new(text);
        let coordinate = |name| L::ext_decode(&vector.bytes(name).expect(name)).unwrap();
        let vk_px = coordinate("VkPx");
        let vk_qx = coordinate("VkQx");
        let vk_dx = coordinate("VkDx");
        let p = even.p.x_affine().expect("P");
        let r = even.q.x_affine().expect("R");
        let q = even
            .difference
            .x_affine()
            .expect("difference of the cleared points");
        assert!(
            p.same(vk_px),
            "{} VkPx disagrees with even_basis.p",
            L::NAME
        );
        assert!(
            q.same(vk_qx),
            "{} VkQx disagrees with difference(P, R)",
            L::NAME
        );
        assert!(
            r.same(vk_dx),
            "{} VkDx disagrees with even_basis.q (R)",
            L::NAME
        );
        // Cleared generators need not satisfy x(Q) = −x(P) − A as affine
        // values; the preimage pair does.
        assert!(
            even.full_q
                .x_affine()
                .unwrap()
                .same(even.full_p.x_affine().unwrap().add(coefficient).negate()),
            "{} full_q is not -full_p-A",
            L::NAME
        );
        let _ = sample;
    }

    #[test]
    fn honest_signatures_accept_and_the_old_chain_still_stops_at_step_one() {
        // The kernel difference is `difference` of the cleared points.
        // With that point, every committed honest vector stops at step 1
        // and `(P, −Q)` reaches both readings. The earlier field, the
        // cofactor of the uncleared difference, let `p248` and `p376` reach
        // both readings. The accept path replaces this chain.
        use prism_vectors::corpus::WithCorpus;
        // `(P, −Q)` reaches both readings at `p248` and `p376`, and stops
        // at `p500`. That is a measured fact of each set, so it stays written
        // on the set rather than folded into the loop below.
        honest::<P248>(
            <P248 as WithCorpus>::PRIMARY[0].text(),
            Stop::Step(1),
            Stop::Readings,
            true,
        );
        honest::<P376>(
            <P376 as WithCorpus>::PRIMARY[0].text(),
            Stop::Step(1),
            Stop::Readings,
            true,
        );
        honest::<P500>(
            <P500 as WithCorpus>::PRIMARY[0].text(),
            Stop::Step(1),
            Stop::Readings,
            false,
        );
        prism_vectors::for_each_corpus_set!(Set, {
            for primary in <Set as WithCorpus>::PRIMARY {
                assert_vk_basis::<Set>(primary.text());
            }
            second_key::<Set>(<Set as WithCorpus>::PRIMARY[1].text());
        });
    }

    /// The answer to an honest signature with its two kernel points
    /// replaced, the key, curve and scalar unchanged.
    fn with_kernel<L: ParameterSet>(
        sample: &Sample<L>,
        p: Jac<L::Ext>,
        q: Jac<L::Ext>,
    ) -> Result<(), VerifyError> {
        let at = |point: Jac<L::Ext>| {
            let z = point.z.inv().expect("a finite point");
            let z2 = z.square();
            AffinePoint {
                x: point.x.mul(z2),
                y: point.y.mul(z2.mul(z)),
            }
        };
        let coefficient = KeyIdentity::<L>::decode(&sample.key).unwrap().coefficient();
        relate_points::<L>(
            coefficient,
            sample.response.curve,
            at(p),
            at(q),
            &sample.scalar,
        )
    }

    /// A point of the signature's curve whose order is odd and above one:
    /// `[2^e]` of the first point with a small integer abscissa that the
    /// doublings do not kill.
    fn odd_order_point<L: ParameterSet>(curve: Montgomery<L::Ext>) -> Jac<L::Ext> {
        (2u64..64)
            .find_map(|x| {
                let x = L::Ext::from_u64(x);
                let point = Jac {
                    x,
                    y: recover_y(curve, x)?,
                    z: L::Ext::one(),
                };
                let odd = dbl_iter(curve, point, L::E);
                (!odd.is_infinity()).then_some(odd)
            })
            .expect("a point of odd order")
    }

    /// Six ways for the two points not to be a basis of `E[2^e]`, each
    /// with the key, the curve and the scalar of an honest signature, and
    /// each a pair of points that is on the curve.
    fn malformed_kernels<L: ParameterSet>(text: &str) {
        let sample = sample::<L>(text);
        let curve = Montgomery::normalized(sample.response.curve);
        let lift = |point: AffinePoint<L>| Jac {
            x: point.x,
            y: point.y,
            z: L::Ext::one(),
        };
        let (p, q) = (lift(sample.response.p), lift(sample.response.q));
        assert_eq!(with_kernel::<L>(&sample, p, q), Ok(()), "{}", L::NAME);
        let odd = odd_order_point::<L>(curve);
        let cases = [
            // Order `2^e` times an odd number. The span test passes these:
            // the order test alone refuses them.
            ("P plus a point of odd order", add(curve, p, odd), q),
            ("Q plus a point of odd order", p, add(curve, q, odd)),
            // Order `2^(e−1)`.
            ("[2]P for P", dbl(curve, p), q),
            ("[2]Q for Q", p, dbl(curve, q)),
            // Order `2^e` each, and the two do not span.
            ("P + [2]Q for Q", p, add(curve, p, dbl(curve, q))),
            ("P for Q", p, p),
        ];
        for (name, p, q) in cases {
            assert_eq!(
                with_kernel::<L>(&sample, p, q),
                Err(VerifyError::Kernel),
                "{}: {name}",
                L::NAME
            );
        }
        // What each test sees of the first, third and fifth, so the three
        // kinds above are the three kinds they are named as.
        let x = |point| to_xz(point);
        let sig = sample.response.curve;
        let with_odd = x(add(curve, p, odd));
        assert!(!order_e::<L>(sig, with_odd) && span_e::<L>(sig, with_odd, x(q)));
        assert!(!order_e::<L>(sig, x(dbl(curve, p))));
        let beside = x(add(curve, p, dbl(curve, q)));
        assert!(order_e::<L>(sig, beside) && !span_e::<L>(sig, x(p), beside));
    }

    #[test]
    fn a_signature_whose_points_are_not_a_basis_of_the_torsion_is_refused_as_kernel() {
        use prism_vectors::corpus::WithCorpus;
        prism_vectors::for_each_corpus_set!(Set, {
            malformed_kernels::<Set>(<Set as WithCorpus>::PRIMARY[0].text());
        });
    }

    /// How many of the coefficients `a + i`, `a` from 1 to 199, decode as
    /// a key, pass the index search, and name points that are no basis of
    /// `E[2^e]`: the curve of such a coefficient does not have its
    /// `2^e`-torsion rational. Each is refused by [`PreparedKey::prepare`]
    /// as `Basis`, and `prepare` accepts a coefficient of the range
    /// exactly when it is outside the family and has an index.
    fn keys_that_name_no_basis<L: ParameterSet>() -> usize {
        let mut family = 0;
        for real in 1u64..=199 {
            let coefficient = L::Ext::from_u64(real).add(L::ext_i());
            let bytes = L::ext_encode(coefficient);
            KeyIdentity::<L>::decode(&bytes).expect("a + i decodes");
            let named = basis_hint::<L>(coefficient).ok().map(|hint| {
                let even = even_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square())
                    .expect("the index names a point");
                basis_e::<L>(coefficient, even.p, even.difference)
            });
            let prepared = PreparedKey::<L>::prepare(&bytes).map(|_| ());
            match named {
                Some(true) => assert_eq!(prepared, Ok(()), "{}: {real} + i", L::NAME),
                Some(false) => {
                    family += 1;
                    assert_eq!(prepared, Err(VerifyError::Basis), "{}: {real} + i", L::NAME);
                }
                None => assert_eq!(prepared, Err(VerifyError::Basis), "{}: {real} + i", L::NAME),
            }
        }
        family
    }

    /// A key that names no basis is refused when it is prepared, at every
    /// set, so the split entry cannot reach the relation with one: there
    /// is no `PreparedKey` to hand it. All 199 coefficients are in the
    /// family at each of the five sets.
    #[test]
    fn a_key_whose_points_are_no_basis_is_refused_when_it_is_prepared() {
        use prism_field::{P324, P664};
        assert_eq!(keys_that_name_no_basis::<P248>(), 199);
        assert_eq!(keys_that_name_no_basis::<P324>(), 199);
        assert_eq!(keys_that_name_no_basis::<P376>(), 199);
        assert_eq!(keys_that_name_no_basis::<P500>(), 199);
        assert_eq!(keys_that_name_no_basis::<P664>(), 199);
    }

    /// The byte-level relation on such a key: `Kernel`, by the test of
    /// the key's own basis, given a signature that is good in itself:
    /// here the points and the scalar of an honest one. This is the one
    /// place where that test is what refuses on this entry. Through
    /// `verify` the challenge is hashed from the key's bytes, so most
    /// signatures are refused as `Composite` before the relation is
    /// reached, and one that is not meets this.
    fn a_curve_without_the_torsion<L: ParameterSet>(text: &str) {
        let sample = sample::<L>(text);
        let mut refused = 0;
        for real in 1u64..=24 {
            let coefficient = L::Ext::from_u64(real).add(L::ext_i());
            let Ok(hint) = basis_hint::<L>(coefficient) else {
                continue;
            };
            let even = even_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square())
                .expect("the index names a point");
            if basis_e::<L>(coefficient, even.p, even.difference) {
                // A supersingular curve among the small ones would be a key.
                continue;
            }
            assert_eq!(
                relate_points::<L>(
                    coefficient,
                    sample.response.curve,
                    sample.response.p,
                    sample.response.q,
                    &sample.scalar,
                ),
                Err(VerifyError::Kernel),
                "{}: A = {real} + i",
                L::NAME
            );
            refused += 1;
        }
        assert!(refused >= 20, "{}: {refused} of 24 curves", L::NAME);
    }

    /// The relation called directly with the starting curve for the key
    /// refuses it as `Origin`, as `verify` does, whatever the signature:
    /// here an honest one made under another key.
    #[test]
    fn the_relation_refuses_the_starting_curve_as_origin() {
        use prism_vectors::corpus::WithCorpus;
        prism_vectors::for_each_corpus_set!(Set, {
            let sample = sample::<Set>(<Set as WithCorpus>::PRIMARY[0].text());
            assert_eq!(
                relate_points::<Set>(
                    <Set as ParameterSet>::Ext::zero(),
                    sample.response.curve,
                    sample.response.p,
                    sample.response.q,
                    &sample.scalar,
                ),
                Err(VerifyError::Origin),
                "{}",
                Set::NAME
            );
        });
    }

    #[test]
    fn the_byte_level_relation_refuses_a_key_whose_points_are_no_basis_as_kernel() {
        use prism_vectors::corpus::WithCorpus;
        prism_vectors::for_each_corpus_set!(Set, {
            a_curve_without_the_torsion::<Set>(<Set as WithCorpus>::PRIMARY[0].text());
        });
    }

    fn second_key<L: ParameterSet>(text: &str) {
        let sample = sample::<L>(text);
        assert_eq!(
            surface_stop::<L>(&sample, sample.response.q.y),
            Stop::Step(1),
            "{}",
            L::NAME
        );
        assert_eq!(
            surface_stop::<L>(&sample, sample.response.q.y.negate()),
            Stop::Readings,
            "{}",
            L::NAME
        );
    }
}
