// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The negatives derived from one honest vector.
//!
//! [`negatives`] takes the text of a committed vector and returns every
//! input the records hold for it that a verifier must refuse, in a fixed
//! order and under fixed names. The tests and the operation-count
//! reporter (`examples/op_counts.rs`) both call it, so both see the same
//! inputs under the same names. Every case is deterministic in the
//! vector's text.
//!
//! A [`Case`] carries the refusals it may meet as data. Nothing here asks
//! a verifier for a verdict: the cases are built with curve arithmetic and
//! the challenge hash, and the verifier is what a test then runs on them.

use crate::identity;
use crate::vectors::{set, vector};
use prism_field::{Field, ParameterSet, Salt};
use prism_isogeny::{
    jac::{add, dbl, dbl_iter, recover_y, to_xz, Jac},
    montgomery::Montgomery,
    pairing::mul,
};
use prism_verify::{
    signature::canonical_sign, AffinePoint, ChallengeScalar, KeyIdentity, Response, VerifyError,
};

/// One input that must be refused, and the refusals it may meet.
pub struct Case {
    /// The name of the case in the records, the same for every vector.
    pub name: String,
    /// The public key bytes.
    pub key: Vec<u8>,
    /// The message.
    pub message: Vec<u8>,
    /// The signature bytes.
    pub signature: Vec<u8>,
    /// The refusals a verifier may return. More than one where the outcome
    /// depends on the vector.
    pub allowed: &'static [VerifyError],
}

fn point<L: ParameterSet>(p: AffinePoint<L>) -> Jac<L::Ext> {
    Jac {
        x: p.x,
        y: p.y,
        z: L::Ext::one(),
    }
}
fn affine<L: ParameterSet>(p: Jac<L::Ext>) -> AffinePoint<L> {
    let inv = p.z.inv().unwrap();
    let inv2 = inv.square();
    AffinePoint {
        x: p.x.mul(inv2),
        y: p.y.mul(inv2).mul(inv),
    }
}
fn normalize<L: ParameterSet>(mut r: Response<L>) -> Response<L> {
    if !canonical_sign::<L>(r.p.y) {
        r.p.y = r.p.y.negate();
        r.q.y = r.q.y.negate();
    }
    r
}

/// A point of `curve` whose order is odd and above one: `[2^e]` of the
/// first point with a small integer abscissa that the doublings do not
/// kill. The curve's group is `(Z/(p+1))^2` and `p + 1 = f·2^e`, so what
/// is left has order dividing `f`.
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
        .expect("a response curve has a point of odd order with a small abscissa")
}

const TRANSLATE: &[VerifyError] = &[
    VerifyError::Pairing,
    VerifyError::Codomain,
    VerifyError::Image,
];
const EXCLUDED_MODEL: &[VerifyError] = &[VerifyError::Singular, VerifyError::Model];

/// Every negative derived from one honest vector, in a fixed order.
///
/// `text` is a committed vector file of the set `L`. The number of cases
/// grows with the salt, one for each of its bits: 329 at `p248`, and
/// 2,742 over the six primary vectors.
///
/// # Panics
///
/// If `text` is not an honest vector of `L`: the construction decodes the
/// key and the signature and takes square roots that exist on a response
/// curve.
pub fn negatives<L: ParameterSet>(text: &str) -> Vec<Case> {
    let (pk, msg, r) = vector::<L>(text);
    let curve = Montgomery::normalized(r.curve);
    let id = KeyIdentity::<L>::decode(&pk).unwrap();
    let mut cases = Vec::new();
    let mut push = |name: String, r: Response<L>, allowed: &'static [VerifyError]| {
        cases.push(Case {
            name,
            key: pk.clone(),
            message: msg.clone(),
            signature: r.encode(),
            allowed,
        });
    };

    let mut other = r;
    other.p.y = other.p.y.negate();
    other.q.y = other.q.y.negate();
    push("both-y-negated".into(), other, &[VerifyError::Sign]);
    other = r;
    other.q.y = other.q.y.negate();
    push("q-negated".into(), other, &[VerifyError::Pairing]);
    other = r;
    std::mem::swap(&mut other.p, &mut other.q);
    push(
        "p-q-swapped".into(),
        normalize(other),
        &[VerifyError::Pairing],
    );
    // A determinant-one shear preserves the pairing, but changes the relation.
    other = r;
    other.q = affine::<L>(add(curve, point(r.q), point(r.p)));
    push(
        "shear-q-plus-p".into(),
        normalize(other),
        &[VerifyError::Codomain],
    );
    other = r;
    other.p = affine::<L>(mul(curve, point(r.p), &[3]));
    other.q = affine::<L>(mul(curve, point(r.q), &[3]));
    push(
        "both-tripled".into(),
        normalize(other),
        &[VerifyError::Pairing],
    );
    let mut scalar = vec![0u8; (L::E as usize).div_ceil(8)];
    scalar[0] = 1;
    scalar[((L::E - 1) / 8) as usize] |= 1 << ((L::E - 1) % 8);
    other = r;
    other.p = affine::<L>(mul(curve, point(r.p), &scalar));
    other.q = affine::<L>(mul(curve, point(r.q), &scalar));
    push(
        "both-top-bit-scaled".into(),
        normalize(other),
        &[VerifyError::Image],
    );
    // All nonzero E[4] translates, in either point or both. Never accept.
    let p4 = dbl_iter(curve, point(r.p), L::E - 2);
    let q4 = dbl_iter(curve, point(r.q), L::E - 2);
    // Sanity: our full-point arithmetic and infinity encoding agree.
    assert!(to_xz(mul(curve, p4, &[4])).is_infinity());
    for a in 0..4 {
        for b in 0..4 {
            if a == 0 && b == 0 {
                continue;
            }
            let t = add(curve, mul(curve, p4, &[a]), mul(curve, q4, &[b]));
            for (which, moved) in ["p", "q", "both"].iter().enumerate() {
                other = r;
                if which != 1 {
                    other.p = affine::<L>(add(curve, point(r.p), t));
                }
                if which != 0 {
                    other.q = affine::<L>(add(curve, point(r.q), t));
                }
                push(
                    format!("translate-{moved}-by-{a}P4+{b}Q4"),
                    normalize(other),
                    TRANSLATE,
                );
            }
        }
    }
    // A one-bit salt mutation either fails primality or the image relation.
    for bit in 0..L::SALT_LEN * 8 {
        other = r;
        let mut bytes = r.salt.as_bytes().to_vec();
        bytes[bit / 8] ^= 1 << (bit % 8);
        other.salt = L::Salt::from_bytes(&bytes).unwrap();
        let prime = ChallengeScalar::derive(&id, &msg, other.salt).degree_is_prime::<L>();
        push(
            format!("salt-bit-{bit}"),
            other,
            if prime {
                &[VerifyError::Image]
            } else {
                &[VerifyError::Composite]
            },
        );
    }
    // A -> -A is an isomorphism, with (x,y) -> (-x, i*y).
    other = r;
    other.curve = other.curve.negate();
    other.p.x = other.p.x.negate();
    other.q.x = other.q.x.negate();
    other.p.y = other.p.y.mul(L::ext_i());
    other.q.y = other.q.y.mul(L::ext_i());
    push(
        "model-negated-a".into(),
        normalize(other),
        &[VerifyError::Model],
    );
    // The four moves of the two other order-two points, with y transported.
    let root = r
        .curve
        .mul(r.curve)
        .sub(L::Ext::from_u64(4))
        .sqrt()
        .unwrap();
    let half = L::Ext::from_u64(2).inv().unwrap();
    for (n, root) in [root, root.negate()].into_iter().enumerate() {
        let alpha = r.curve.negate().add(root).mul(half);
        let lambda = alpha.square().sub(L::Ext::one()).sqrt().unwrap();
        let inv = lambda.inv().unwrap();
        let scale = lambda.mul(lambda.sqrt().unwrap()).inv().unwrap();
        let mut moved = r;
        moved.curve = alpha.mul(L::Ext::from_u64(3)).add(r.curve).mul(inv);
        moved.p.x = r.p.x.sub(alpha).mul(inv);
        moved.q.x = r.q.x.sub(alpha).mul(inv);
        moved.p.y = r.p.y.mul(scale);
        moved.q.y = r.q.y.mul(scale);
        push(
            format!("model-moved-{n}"),
            normalize(moved),
            &[VerifyError::Model],
        );
        moved.curve = moved.curve.negate();
        moved.p.x = moved.p.x.negate();
        moved.q.x = moved.q.x.negate();
        moved.p.y = moved.p.y.mul(L::ext_i());
        moved.q.y = moved.q.y.mul(L::ext_i());
        push(
            format!("model-moved-{n}-negated"),
            normalize(moved),
            &[VerifyError::Model],
        );
    }
    // Reduced-order points and an off-curve point cannot reach acceptance.
    other = r;
    other.p = affine::<L>(dbl_iter(curve, point(r.p), 2));
    other.q = affine::<L>(dbl_iter(curve, point(r.q), 2));
    push(
        "both-doubled-twice".into(),
        normalize(other),
        &[VerifyError::Kernel],
    );
    other = r;
    other.q.y = other.q.y.add(L::Ext::one());
    push("q-off-curve".into(), other, &[VerifyError::Kernel]);
    // On the curve, and not a basis of `E[2^e]`. A point of odd order
    // added to `P` or to `Q` leaves the pair spanning the `2^e`-torsion
    // and gives the point an order that is not `2^e`: the order test is
    // the one check that refuses it, and without that test the first of
    // these is accepted at `p248`. The third has order `2^e` twice and
    // does not span.
    let odd = odd_order_point::<L>(curve);
    other = r;
    other.p = affine::<L>(add(curve, point(r.p), odd));
    push(
        "p-plus-odd-order".into(),
        normalize(other),
        &[VerifyError::Kernel],
    );
    other = r;
    other.q = affine::<L>(add(curve, point(r.q), odd));
    push("q-plus-odd-order".into(), other, &[VerifyError::Kernel]);
    other = r;
    other.q = affine::<L>(add(curve, point(r.p), dbl(curve, point(r.q))));
    push("q-beside-p".into(), other, &[VerifyError::Kernel]);
    // Every singular or extra-automorphism response model is excluded.
    for (name, a) in [
        ("zero", L::Ext::zero()),
        ("two", L::Ext::from_u64(2)),
        ("root-three", L::Ext::from_u64(3).sqrt().unwrap()),
        (
            "root-nine-halves",
            L::Ext::from_u64(9)
                .mul(L::Ext::from_u64(2).inv().unwrap())
                .sqrt()
                .unwrap(),
        ),
    ] {
        other = r;
        other.curve = a;
        push(format!("curve-{name}"), other, EXCLUDED_MODEL);
    }

    // Alternate salt at a prime degree, and a different message or key.
    let find = |key: &[u8], message: &[u8]| {
        let identity = KeyIdentity::<L>::decode(key).unwrap();
        (0u64..65_536)
            .find_map(|n| {
                let salt = L::Salt::from_counter(n).unwrap();
                let scalar = ChallengeScalar::derive(&identity, message, salt);
                (salt != r.salt && scalar.degree_is_prime::<L>()).then_some(salt)
            })
            .unwrap()
    };
    other = r;
    other.salt = find(&pk, &msg);
    push("other-prime-salt".into(), other, &[VerifyError::Image]);
    let mut message = msg.clone();
    message.extend(b"-other");
    other = r;
    other.salt = find(&pk, &message);
    cases.push(Case {
        name: "other-message".into(),
        key: pk.clone(),
        message,
        signature: other.encode(),
        allowed: &[VerifyError::Image],
    });
    let other_key = prism_vectors::hints::row(set(text).label())
        .unwrap()
        .public_key
        .clone();
    other = r;
    other.salt = find(&other_key, &msg);
    cases.push(Case {
        name: "other-key".into(),
        key: other_key,
        message: msg.clone(),
        signature: other.encode(),
        allowed: &[VerifyError::Pairing],
    });
    // The pairing check passed with no secret: the response curve of the
    // honest signature taken as a key, and the verifier's own basis of it
    // answered with a pair of inverse pairing. See `identity.rs`.
    for form in identity::Form::ALL {
        let built = identity::identity_response::<L>(r.curve, &msg, form);
        cases.push(Case {
            name: form.name().into(),
            key: built.key,
            message: msg.clone(),
            signature: built.response.encode(),
            allowed: form.allowed(),
        });
    }
    // The refusals that need no curve arithmetic to produce.
    let honest = r.encode();
    cases.push(Case {
        name: "signature-one-byte-short".into(),
        key: pk.clone(),
        message: msg.clone(),
        signature: honest[..honest.len() - 1].to_vec(),
        allowed: &[VerifyError::Length],
    });
    let mut unreduced = honest.clone();
    unreduced[..L::FP2_LEN].fill(0xff);
    cases.push(Case {
        name: "curve-bytes-all-ones".into(),
        key: pk.clone(),
        message: msg.clone(),
        signature: unreduced,
        allowed: &[VerifyError::Canonical],
    });
    cases.push(Case {
        name: "key-zero".into(),
        key: vec![0; L::PUBLIC_KEY_LEN],
        message: msg,
        signature: honest,
        allowed: &[VerifyError::Origin],
    });
    cases
}
