// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The committed honest vectors and the negatives derived from them.
//!
//! One generator serves the acceptance tests and the operation-count
//! reporter (`examples/op_counts.rs`), so both see the same inputs under
//! the same names. Every case is deterministic in the vector's text.

use prism_field::{Field, Level, Salt};
use prism_isogeny::{
    jac::{add, dbl_iter, to_xz, Jac},
    montgomery::Montgomery,
    pairing::mul,
};
use prism_verify::{
    signature::canonical_sign, AffinePoint, ChallengeScalar, KeyIdentity, Nist, Response,
    VerifyError,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn field<'a>(text: &'a str, name: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(name))
        .unwrap_or_else(|| panic!("missing {name}"))
}
fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
/// Public key, message and decoded response of one vector file.
pub fn vector<L: Level>(text: &str) -> (Vec<u8>, Vec<u8>, Response<L>) {
    let pk = hex(field(text, "A "));
    let mut sig = Vec::new();
    for name in ["E ", "Px ", "Py ", "Qx ", "Qy "] {
        sig.extend(hex(field(text, name)));
    }
    sig.extend(hex(field(text, "salt ")));
    (
        pk,
        field(text, "message ").as_bytes().to_vec(),
        Response::decode(&sig).unwrap(),
    )
}
/// The level a vector file names.
pub fn level(text: &str) -> Nist {
    field(text, "level ").parse().unwrap()
}
/// Every committed honest vector, sorted by path.
pub fn files() -> Vec<PathBuf> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let mut files: Vec<_> = fs::read_dir(base.join("accept"))
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    files.extend(primary());
    files.sort();
    files
}
/// The six vectors the negatives are derived from, two keys per level.
pub fn primary() -> Vec<PathBuf> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    ["1", "1b", "3", "3b", "5", "5b"]
        .iter()
        .map(|n| base.join(format!("s5_lvl{n}.txt")))
        .collect()
}

/// One input that must be refused, and the refusals it may meet.
pub struct Case {
    pub name: String,
    pub key: Vec<u8>,
    pub message: Vec<u8>,
    pub signature: Vec<u8>,
    /// More than one where the outcome depends on the vector.
    pub allowed: &'static [VerifyError],
}

fn point<L: Level>(p: AffinePoint<L>) -> Jac<L::Ext> {
    Jac {
        x: p.x,
        y: p.y,
        z: L::Ext::one(),
    }
}
fn affine<L: Level>(p: Jac<L::Ext>) -> AffinePoint<L> {
    let inv = p.z.inv().unwrap();
    let inv2 = inv.square();
    AffinePoint {
        x: p.x.mul(inv2),
        y: p.y.mul(inv2).mul(inv),
    }
}
fn normalize<L: Level>(mut r: Response<L>) -> Response<L> {
    if !canonical_sign::<L>(r.p.y) {
        r.p.y = r.p.y.negate();
        r.q.y = r.q.y.negate();
    }
    r
}

const TRANSLATE: &[VerifyError] = &[
    VerifyError::Pairing,
    VerifyError::Codomain,
    VerifyError::Image,
];
const EXCLUDED_MODEL: &[VerifyError] = &[VerifyError::Singular, VerifyError::Model];

/// Every negative derived from one honest vector, in a fixed order.
pub fn negatives<L: Level>(text: &str) -> Vec<Case> {
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
    let other_key = prism_verify::reference_key(level(text))
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
