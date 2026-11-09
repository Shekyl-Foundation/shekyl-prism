// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Algorithm 3 steps 3–8.
//!
//! One chain of length `e` on `K = ⟨(P_vk, −P_sig), (Q_vk, −Q_sig)⟩`, with
//! no splitting step. It is evaluated at `([s]X, 0)` and `([2^{e−1}]X, 0)`
//! for `X ∈ {R, T, R−T}`. The `[s]` images are the chain at step `e − 1`;
//! the `[2^{e−1}]` images are the chain at step `e`.
//!
//! The x-only kernel cannot tell `+P_sig` from `−P_sig`. Those kernels
//! differ by `ι(X, Y) = (X, −Y)`, and `ι` fixes every `(X, 0)`, so the
//! evaluated points do not see the sign. `docs/VERIFIER_SPEC.md` records this.
//!
//! The two readings lie on the same surface in two theta structures. The
//! transformation between them is not fixed, so both readings existing is
//! [`VerifyError::Unidentified`], not acceptance.

use prism_field::{Field, Level};
use prism_isogeny::chain::chain_surface;
use prism_isogeny::gluing::{Couple, Kernel};
use prism_isogeny::jac::{add, to_xz, Jac};
use prism_isogeny::montgomery::{a24, ladder, Montgomery, Xz};
use prism_isogeny::torsion::{
    double_times, even_parts_generate, rational_basis, starting_basis, RationalBasis,
};

use crate::basis::basis_hint;
use crate::challenge::ChallengeScalar;
use crate::identity::KeyIdentity;
use crate::signature::{AffinePoint, Response};
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
    let basis = if vk.is_zero() {
        starting_basis::<L>()
    } else {
        let hint = basis_hint::<L>(vk)?;
        rational_basis::<L>(vk, hint.index(), hint.coefficient_is_square())
    }
    .ok_or(VerifyError::Kernel)?;
    if !on_curve::<L>(sig, p_sig.x, p_sig.y) || !on_curve::<L>(sig, q_sig.x, q_sig.y) {
        return Err(VerifyError::Kernel);
    }
    let p_sig_x = Xz::affine(p_sig.x);
    let q_sig_x = Xz::affine(q_sig.x);
    if !order_two_pow::<L>(sig, p_sig_x) || !order_two_pow::<L>(sig, q_sig_x) {
        return Err(VerifyError::Kernel);
    }
    let signature_difference = difference::<L>(sig, p_sig.x, p_sig.y, q_sig.x, q_sig.y);
    let kernel = Kernel {
        t1: Couple {
            first: basis.even.p,
            second: p_sig_x,
        },
        t2: Couple {
            first: basis.even.q,
            second: q_sig_x,
        },
        difference: Couple {
            first: basis.even.difference,
            second: signature_difference,
        },
    };
    if !even_parts_generate::<L>(sig, p_sig_x, q_sig_x) {
        return Err(VerifyError::Kernel);
    }
    let curves = [Montgomery::normalized(vk), Montgomery::normalized(sig)];
    let points = pushed::<L>(vk, &basis, scalar);
    let readings =
        chain_surface(L::E, curves, kernel, false, &points, true).ok_or(VerifyError::Kernel)?;
    if readings.earlier.images.len() != points.len() || readings.last.images.len() != points.len() {
        return Err(VerifyError::Kernel);
    }
    // `readings.earlier` holds the step `e − 1` images of `([s]X, 0)`.
    // `readings.last` holds the step `e` images of `([2^{e−1}]X, 0)`.
    // Comparing them needs the transformation between the two theta
    // structures, which is open.
    Err(VerifyError::Unidentified)
}

fn order_two_pow<L: Level>(coefficient: L::Ext, point: Xz<L::Ext>) -> bool {
    let full = double_times::<L>(coefficient, point, L::E);
    let almost = double_times::<L>(coefficient, point, L::E - 1);
    full.z.is_zero() && !almost.z.is_zero()
}

fn pushed<L: Level>(
    vk: L::Ext,
    basis: &RationalBasis<L::Ext>,
    scalar: &[u8],
) -> [Couple<L::Ext>; 6] {
    let at = |point: Xz<L::Ext>| Couple {
        first: point,
        second: Xz::infinity(),
    };
    let bases = [basis.r, basis.t, basis.difference];
    [
        at(scale::<L>(vk, bases[0], scalar)),
        at(scale::<L>(vk, bases[1], scalar)),
        at(scale::<L>(vk, bases[2], scalar)),
        at(double_times::<L>(vk, bases[0], L::E - 1)),
        at(double_times::<L>(vk, bases[1], L::E - 1)),
        at(double_times::<L>(vk, bases[2], L::E - 1)),
    ]
}

fn on_curve<L: Level>(coefficient: L::Ext, x: L::Ext, y: L::Ext) -> bool {
    let square = x.mul(x);
    y.mul(y)
        .same(square.mul(x).add(coefficient.mul(square)).add(x))
}

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
    use super::relate_points;
    use crate::signature::{canonical_sign, AffinePoint};
    use crate::verify;
    use crate::VerifyError;
    use prism_field::{Field, Level, Level1};
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
        let z2 = z.mul(z);
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
        // The chain is the one-chain shape. This witness is the graph of one
        // endomorphism of E0. In the C tree that kernel is
        // `endomorphism_application_even_basis` on `BASIS_EVEN` of
        // `CURVE_E0`, and `theta_chain_compute_and_eval` stops the same way.
        // A theta coordinate of the step's kernel vanishes at step 3, which
        // the (2,2) step refuses, so both readings are not produced.
        assert_eq!(
            relate_points::<L>(zero, zero, p, q, &s),
            Err(VerifyError::Kernel)
        );
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
    use super::{on_curve, order_two_pow, relate_points};
    use crate::basis::basis_hint;
    use crate::challenge::ChallengeScalar;
    use crate::identity::KeyIdentity;
    use crate::prime::degree_is_prime;
    use crate::signature::{canonical_sign, AffinePoint, Response};
    use crate::verify;
    use crate::VerifyError;
    use prism_field::{ChallengeIndex, Field, Level, Level1, Level3, Level5};
    use prism_isogeny::montgomery::Xz;

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
        let index = ChallengeIndex::from_canonical_bytes(
            line(text, "j ").parse::<u16>().unwrap().to_le_bytes(),
        );
        let scalar =
            ChallengeScalar::derive_width::<L>(&identity, b"shekyl-prism-s5", index, L::E - 3);
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
                index,
            },
            scalar: scalar.as_bytes().to_vec(),
        }
    }

    fn honest<L: Level>(text: &str) {
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
        assert!(degree_is_prime(L::E - 2, &sample.scalar));
        // The points are a basis of E[2^{e'}], e' = e-2. The length-e
        // relation asks for order exactly 2^e and returns before the chain.
        let x = Xz::affine(sample.response.p.x);
        assert!(!order_two_pow::<L>(sample.response.curve, x));
        let coefficient = KeyIdentity::<L>::decode(&sample.key).unwrap().coefficient();
        assert_eq!(
            relate_points::<L>(
                coefficient,
                sample.response.curve,
                sample.response.p,
                sample.response.q,
                &sample.scalar,
            ),
            Err(VerifyError::Kernel)
        );
        // The instantiated verifier derives s at exponent e, so this
        // signature is composite there, still before any chain.
        assert_eq!(
            verify::<L>(&sample.key, b"shekyl-prism-s5", &sample.response.encode()),
            Err(VerifyError::Composite)
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

        let j = u16::from_le_bytes(sample.response.index.to_canonical_bytes());
        let next = ChallengeIndex::from_canonical_bytes(j.wrapping_add(1).to_le_bytes());
        let identity = KeyIdentity::<L>::decode(&sample.key).unwrap();
        let next_scalar =
            ChallengeScalar::derive_width::<L>(&identity, b"shekyl-prism-s5", next, L::E - 3);
        // j+1 is a different degree. When that degree is composite the
        // refusal, once the exponent is e', is Composite, before the chain.
        assert!(!degree_is_prime(L::E - 2, next_scalar.as_bytes()));

        // (P, −Q) keeps the normalized y(P). The length-e order check
        // refuses it with the same Kernel as the honest vector. Telling
        // them apart is the comparison, Unidentified once the chain is e'.
        let mut negated_q = sample.response;
        negated_q.q.y = negated_q.q.y.negate();
        assert!(canonical_sign::<L>(negated_q.p.y));
        assert_eq!(
            relate_points::<L>(
                coefficient,
                negated_q.curve,
                negated_q.p,
                negated_q.q,
                &sample.scalar,
            ),
            Err(VerifyError::Kernel)
        );

        let swapped = Response {
            p: sample.response.q,
            q: sample.response.p,
            ..sample.response
        };
        if canonical_sign::<L>(swapped.p.y) {
            // Same order check as the honest vector. The comparison is what
            // separates a swapped factor, and that is Unidentified at e'.
            assert_eq!(
                relate_points::<L>(
                    coefficient,
                    swapped.curve,
                    swapped.p,
                    swapped.q,
                    &sample.scalar,
                ),
                Err(VerifyError::Kernel)
            );
        } else {
            assert_eq!(
                verify::<L>(&sample.key, b"shekyl-prism-s5", &swapped.encode()),
                Err(VerifyError::Sign)
            );
        }
    }

    #[test]
    fn the_s5_vectors_are_prime_at_e_prime_and_refused_before_the_length_e_chain() {
        honest::<Level1>(include_str!("../testdata/s5_lvl1.txt"));
        honest::<Level3>(include_str!("../testdata/s5_lvl3.txt"));
        honest::<Level5>(include_str!("../testdata/s5_lvl5.txt"));
    }
}
