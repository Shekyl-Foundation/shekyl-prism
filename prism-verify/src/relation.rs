// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Order and span checks, the full-order Weil pairing, and the two-chain
//! accept path at exponent `e′ = e − 2`. Historical single-chain probes
//! remain in tests to document the former basis and structure failures.

use prism_field::{Field, Level};
#[cfg(test)]
use prism_isogeny::chain::{chain_surface, SurfaceStop};
#[cfg(test)]
use prism_isogeny::gluing::{Couple, Kernel};
#[cfg(test)]
use prism_isogeny::jac::{add, to_xz};
use prism_isogeny::jac::{lift_basis, Jac};
#[cfg(test)]
use prism_isogeny::montgomery::{a24, ladder};
use prism_isogeny::montgomery::{Montgomery, Xz};
use prism_isogeny::pairing::{mul, neg_square_mod_pow2, pow, weil};
use prism_isogeny::torsion::{double_times, even_basis, starting_basis, EvenBasis};

use crate::basis::basis_hint;
use crate::challenge::ChallengeScalar;
use crate::identity::KeyIdentity;
use crate::signature::{AffinePoint, Response};
use crate::stage::Stage;
use crate::VerifyError;

/// The isogeny relation for a key the public checks have already accepted.
pub fn relate<L: Level>(
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

/// The same relation, including the starting curve. `verify` never calls
/// this with a zero key: that key is [`VerifyError::Origin`] first. The
/// `E0` vectors call it directly.
pub fn relate_points<L: Level>(
    vk: L::Ext,
    sig: L::Ext,
    p_sig: AffinePoint<L>,
    q_sig: AffinePoint<L>,
    scalar: &[u8],
) -> Result<(), VerifyError> {
    // The starting curve keeps its `E[p+1]` basis: the degree-`q` witness
    // was built from that basis. A public key uses the canonical basis of
    // `E[2^e]`: `Q` is `difference(P, R)` after the cofactor.
    Stage::Torsion.enter();
    let (even, push) = if vk.is_zero() {
        let basis = starting_basis::<L>().ok_or(VerifyError::Kernel)?;
        (basis.even, [basis.r, basis.t, basis.difference])
    } else {
        let hint = basis_hint::<L>(vk)?;
        let even = even_basis::<L>(vk, hint.index(), hint.coefficient_is_square())
            .ok_or(VerifyError::Kernel)?;
        (even, [even.p, even.q, even.difference])
    };
    if !on_curve::<L>(sig, p_sig.x, p_sig.y) || !on_curve::<L>(sig, q_sig.x, q_sig.y) {
        return Err(VerifyError::Kernel);
    }
    let p_sig_x = Xz::affine(p_sig.x);
    let q_sig_x = Xz::affine(q_sig.x);
    if !order_e::<L>(sig, p_sig_x) || !order_e::<L>(sig, q_sig_x) {
        return Err(VerifyError::Kernel);
    }
    if !span_e::<L>(sig, p_sig_x, q_sig_x) {
        return Err(VerifyError::Kernel);
    }
    // The canonical search names abscissas; arbitrary public keys must
    // still prove that the resulting signature basis is full rational torsion.
    if !order_e::<L>(vk, even.p)
        || !order_e::<L>(vk, even.difference)
        || !span_e::<L>(vk, even.p, even.difference)
    {
        return Err(VerifyError::Kernel);
    }
    // Before either chain. A mismatch is `Pairing` on a public key and on
    // the starting curve alike; the degree-`q` witness calls the chain
    // helper itself and does not come through here.
    if !pairing_holds::<L>(vk, &even, sig, p_sig, q_sig, scalar) {
        return Err(VerifyError::Pairing);
    }
    let _ = push;
    crate::accept::check::<L>(vk, &even, sig, p_sig, q_sig, scalar)
}

/// `e_{2^e}([s]P_sig, [s]Q_sig) = e_{2^e}(P, Q)^q`.
///
/// `Q` is the cleared difference. `P − Q` for the lift is `R`, the cleared
/// point whose full-point abscissa is `−x(P) − A`. A global sign on the
/// lifted pair cancels in the pairing.
fn pairing_holds<L: Level>(
    vk: L::Ext,
    even: &EvenBasis<L::Ext>,
    sig: L::Ext,
    p_sig: AffinePoint<L>,
    q_sig: AffinePoint<L>,
    scalar: &[u8],
) -> bool {
    Stage::Pairing.enter();
    let Some((vk_curve, p_vk, q_vk)) =
        lift_basis(Montgomery::normalized(vk), even.p, even.difference, even.q)
    else {
        return false;
    };
    let sig_curve = Montgomery::normalized(sig);
    let one = L::Ext::one();
    let p = Jac {
        x: p_sig.x,
        y: p_sig.y,
        z: one,
    };
    let q = Jac {
        x: q_sig.x,
        y: q_sig.y,
        z: one,
    };
    Stage::Scale.enter();
    let (r1, r2) = (mul(sig_curve, p, scalar), mul(sig_curve, q, scalar));
    Stage::Pairing.enter();
    let (Some(left), Some(right)) = (
        weil(sig_curve, r1, r2, L::E),
        weil(vk_curve, p_vk, q_vk, L::E),
    ) else {
        return false;
    };
    left.same(pow(right, &neg_square_mod_pow2(scalar, L::E)))
}

/// The `e′` chain for a basis and signature that have already passed the
/// order and span checks.
#[cfg(test)]
fn e_prime_surface<L: Level>(
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

/// Order exactly `2^e`.
fn order_e<L: Level>(coefficient: L::Ext, point: Xz<L::Ext>) -> bool {
    let full = double_times::<L>(coefficient, point, L::E);
    let almost = double_times::<L>(coefficient, point, L::E - 1);
    full.z.is_zero() && !almost.z.is_zero()
}

/// `[2^{e−1}]` of two order-`2^e` points are distinct and nonzero.
fn span_e<L: Level>(coefficient: L::Ext, p: Xz<L::Ext>, q: Xz<L::Ext>) -> bool {
    let p2 = double_times::<L>(coefficient, p, L::E - 1);
    let q2 = double_times::<L>(coefficient, q, L::E - 1);
    !p2.is_infinity() && !q2.is_infinity() && !p2.x.mul(q2.z).same(q2.x.mul(p2.z))
}

#[cfg(test)]
fn pushed<L: Level>(vk: L::Ext, bases: [Xz<L::Ext>; 3], scalar: &[u8]) -> [Couple<L::Ext>; 6] {
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

fn on_curve<L: Level>(coefficient: L::Ext, x: L::Ext, y: L::Ext) -> bool {
    let square = x.square();
    y.square()
        .same(square.mul(x).add(coefficient.mul(square)).add(x))
}

#[cfg(test)]
fn difference<L: Level>(
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
fn scale<L: Level>(coefficient: L::Ext, point: Xz<L::Ext>, scalar: &[u8]) -> Xz<L::Ext> {
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
    use crate::VerifyError;
    use prism_field::{Field, Level, Level1};
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

    fn affine<L: Level>(point: Jac<L::Ext>) -> (L::Ext, L::Ext) {
        let z = point.z.inv().expect("finite");
        let z2 = z.square();
        (point.x.mul(z2), point.y.mul(z2.mul(z)))
    }

    fn signature<L: Level>(block: &str) -> (AffinePoint<L>, AffinePoint<L>, Vec<u8>) {
        let basis = starting_basis::<L>().expect("E0 basis");
        let (_, p, q) = lift_basis(
            Montgomery::normalized(L::Ext::zero()),
            basis.even.p,
            basis.even.q,
            basis.even.difference,
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

    fn exercise<L: Level>(block: &str) {
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
                &basis.even,
                [basis.r, basis.t, basis.difference],
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
        let text = include_str!("../testdata/e0_endomorphisms.txt");
        let block = text.split("NIST-").find(|block| !block.is_empty()).unwrap();
        exercise::<Level1>(block);
        for length in [260usize, 388, 516] {
            let err = verify::<Level1>(&[0u8; Level1::PUBLIC_KEY_LEN], b"m", &vec![0u8; length]);
            assert_eq!(err, Err(VerifyError::Length));
        }
    }
}

#[cfg(test)]
mod s5_vectors {
    use super::{e_prime_surface, on_curve, order_e, relate_points, span_e};
    use crate::basis::basis_hint;
    use crate::challenge::ChallengeScalar;
    use crate::identity::KeyIdentity;
    use crate::prime::degree_is_prime;
    use crate::signature::{canonical_sign, AffinePoint, Response};
    use crate::verify;
    use crate::VerifyError;
    use prism_field::{Field, Level, Level1, Level3, Level5, Salt};
    #[cfg(test)]
    use prism_isogeny::chain::SurfaceStop;
    use prism_isogeny::jac::Jac;
    use prism_isogeny::montgomery::{Montgomery, Xz};
    use prism_isogeny::pairing::{mul, weil};
    use prism_isogeny::torsion::{even_basis, has_order_p_plus_one, rational_basis};

    fn line<'a>(text: &'a str, label: &str) -> &'a str {
        text.lines()
            .find_map(|line| line.strip_prefix(label))
            .unwrap()
            .trim()
    }

    fn hex_bytes(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
            .collect()
    }

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

    struct Sample<L: Level> {
        key: Vec<u8>,
        response: Response<L>,
        scalar: Vec<u8>,
    }

    fn sample<L: Level>(text: &str) -> Sample<L> {
        let key = hex_bytes(line(text, "A "));
        let identity = KeyIdentity::<L>::decode(&key).expect("key");
        let salt = L::Salt::from_bytes(&hex_bytes(line(text, "salt "))).unwrap();
        let scalar = ChallengeScalar::derive::<L>(&identity, b"shekyl-prism-s5", salt);
        let mut from_file = decimal_le(line(text, "s "));
        from_file.resize(scalar.as_bytes().len(), 0);
        assert_eq!(from_file, scalar.as_bytes());
        let point = |x, y| AffinePoint::<L> {
            x: L::ext_decode(&hex_bytes(line(text, x))).unwrap(),
            y: L::ext_decode(&hex_bytes(line(text, y))).unwrap(),
        };
        Sample {
            key,
            response: Response {
                curve: L::ext_decode(&hex_bytes(line(text, "E "))).unwrap(),
                p: point("Px ", "Py "),
                q: point("Qx ", "Qy "),
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

    fn surface_stop<L: Level>(sample: &Sample<L>, q_sign: L::Ext) -> Stop {
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

    fn honest<L: Level>(text: &str, chain: Stop, negated_q: Stop, full_order: bool) {
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
        assert!(degree_is_prime(L::E - 2, &sample.scalar));
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
            verify::<L>(&sample.key, b"shekyl-prism-s5", &sample.response.encode()),
            Ok(())
        );

        let mut wrong_sign = sample.response;
        wrong_sign.p.y = wrong_sign.p.y.negate();
        wrong_sign.q.y = wrong_sign.q.y.negate();
        assert_eq!(
            verify::<L>(&sample.key, b"shekyl-prism-s5", &wrong_sign.encode()),
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
                let scalar = ChallengeScalar::derive(&identity, b"shekyl-prism-s5", salt);
                (!degree_is_prime(L::E - 2, scalar.as_bytes())).then_some(salt)
            })
            .expect("a composite salt exists");
        assert_eq!(
            verify::<L>(
                &sample.key,
                b"shekyl-prism-s5",
                &composite_response.encode()
            ),
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
                verify::<L>(&sample.key, b"shekyl-prism-s5", &swapped.encode()),
                Err(VerifyError::Sign)
            );
        }
    }

    fn assert_weil_values<L: Level>(text: &str, vk_hex: &str, sig_hex: &str) {
        let sample = sample::<L>(text);
        let one = L::Ext::one();
        let coefficient = KeyIdentity::<L>::decode(&sample.key)
            .expect("key")
            .coefficient();
        let vk = Montgomery::normalized(coefficient);
        let sig = Montgomery::normalized(sample.response.curve);
        let at = |x: &str, y: &str| Jac {
            x: L::ext_decode(&hex_bytes(line(text, x))).unwrap(),
            y: L::ext_decode(&hex_bytes(line(text, y))).unwrap(),
            z: one,
        };
        let public =
            weil(vk, at("VkPx ", "VkPy "), at("VkQx ", "VkQy "), L::E).expect("e(P_vk, Q_vk)");
        let signed = weil(
            sig,
            mul(sig, at("Px ", "Py "), &sample.scalar),
            mul(sig, at("Qx ", "Qy "), &sample.scalar),
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
        let table = include_str!("../testdata/weil_pairing.tsv");
        let mut rows = table.lines().filter(|line| !line.is_empty());
        assert_eq!(rows.next(), Some("file\tvk\tsig"));
        let files: [(&str, &str); 6] = [
            ("s5_lvl1.txt", include_str!("../testdata/s5_lvl1.txt")),
            ("s5_lvl3.txt", include_str!("../testdata/s5_lvl3.txt")),
            ("s5_lvl5.txt", include_str!("../testdata/s5_lvl5.txt")),
            ("s5_lvl1b.txt", include_str!("../testdata/s5_lvl1b.txt")),
            ("s5_lvl3b.txt", include_str!("../testdata/s5_lvl3b.txt")),
            ("s5_lvl5b.txt", include_str!("../testdata/s5_lvl5b.txt")),
        ];
        for (name, text) in files {
            let row = rows.next().expect(name);
            let mut parts = row.split('\t');
            assert_eq!(parts.next(), Some(name));
            let vk = parts.next().expect("vk");
            let sig = parts.next().expect("sig");
            match name {
                "s5_lvl1.txt" | "s5_lvl1b.txt" => assert_weil_values::<Level1>(text, vk, sig),
                "s5_lvl3.txt" | "s5_lvl3b.txt" => assert_weil_values::<Level3>(text, vk, sig),
                "s5_lvl5.txt" | "s5_lvl5b.txt" => assert_weil_values::<Level5>(text, vk, sig),
                _ => panic!("unexpected row {name}"),
            }
        }
        assert!(rows.next().is_none());
    }

    fn assert_vk_basis<L: Level>(text: &str) {
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
        let vk_px = L::ext_decode(&hex_bytes(line(text, "VkPx "))).unwrap();
        let vk_qx = L::ext_decode(&hex_bytes(line(text, "VkQx "))).unwrap();
        let vk_dx = L::ext_decode(&hex_bytes(line(text, "VkDx "))).unwrap();
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
        // cofactor of the uncleared difference, let levels I and III reach
        // both readings. The accept path replaces this chain.
        honest::<Level1>(
            include_str!("../testdata/s5_lvl1.txt"),
            Stop::Step(1),
            Stop::Readings,
            true,
        );
        honest::<Level3>(
            include_str!("../testdata/s5_lvl3.txt"),
            Stop::Step(1),
            Stop::Readings,
            true,
        );
        honest::<Level5>(
            include_str!("../testdata/s5_lvl5.txt"),
            Stop::Step(1),
            Stop::Readings,
            false,
        );
        assert_vk_basis::<Level1>(include_str!("../testdata/s5_lvl1.txt"));
        assert_vk_basis::<Level3>(include_str!("../testdata/s5_lvl3.txt"));
        assert_vk_basis::<Level5>(include_str!("../testdata/s5_lvl5.txt"));
        assert_vk_basis::<Level1>(include_str!("../testdata/s5_lvl1b.txt"));
        assert_vk_basis::<Level3>(include_str!("../testdata/s5_lvl3b.txt"));
        assert_vk_basis::<Level5>(include_str!("../testdata/s5_lvl5b.txt"));
        second_key::<Level1>(include_str!("../testdata/s5_lvl1b.txt"));
        second_key::<Level3>(include_str!("../testdata/s5_lvl3b.txt"));
        second_key::<Level5>(include_str!("../testdata/s5_lvl5b.txt"));
    }

    fn second_key<L: Level>(text: &str) {
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
