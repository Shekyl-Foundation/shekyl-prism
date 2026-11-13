// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

use prism_field::{Field, Level, Level1, Level3, Level5, Salt};
use prism_isogeny::{
    jac::{add, dbl_iter, to_xz, Jac},
    montgomery::Montgomery,
    pairing::mul,
};
use prism_verify::{
    signature::canonical_sign, verify, AffinePoint, ChallengeScalar, KeyIdentity, Nist, Response,
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
fn vector<L: Level>(text: &str) -> (Vec<u8>, Vec<u8>, Response<L>) {
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
fn files() -> Vec<PathBuf> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let mut files: Vec<_> = fs::read_dir(base.join("accept"))
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    for n in ["1", "1b", "3", "3b", "5", "5b"] {
        files.push(base.join(format!("s5_lvl{n}.txt")));
    }
    files.sort();
    files
}
fn positive<L: Level>(text: &str) {
    let (pk, msg, response) = vector::<L>(text);
    assert_eq!(verify::<L>(&pk, &msg, &response.encode()), Ok(()));
}
#[test]
fn every_committed_honest_signature_is_accepted() {
    let files = files();
    assert_eq!(
        files.len(),
        113,
        "update the reviewed corpus count when adding fixtures"
    );
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        eprintln!("accept {}", path.display());
        match field(&text, "level ").parse::<Nist>().unwrap() {
            Nist::I => positive::<Level1>(&text),
            Nist::III => positive::<Level3>(&text),
            Nist::V => positive::<Level5>(&text),
        }
    }
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
    let inv2 = inv.mul(inv);
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
fn negative<L: Level>(text: &str) {
    let (pk, msg, r) = vector::<L>(text);
    let check = |r: Response<L>| verify::<L>(&pk, &msg, &r.encode());
    let curve = Montgomery::normalized(r.curve);
    let id = KeyIdentity::<L>::decode(&pk).unwrap();
    let mut other = r;
    other.p.y = other.p.y.negate();
    other.q.y = other.q.y.negate();
    assert_eq!(check(other), Err(VerifyError::Sign));
    other = r;
    other.q.y = other.q.y.negate();
    assert_eq!(check(other), Err(VerifyError::Pairing));
    other = r;
    std::mem::swap(&mut other.p, &mut other.q);
    assert_eq!(check(normalize(other)), Err(VerifyError::Pairing));
    // A determinant-one shear preserves the pairing, but changes the relation.
    other = r;
    other.q = affine::<L>(add(curve, point(r.q), point(r.p)));
    assert_eq!(check(normalize(other)), Err(VerifyError::Codomain));
    other = r;
    other.p = affine::<L>(mul(curve, point(r.p), &[3]));
    other.q = affine::<L>(mul(curve, point(r.q), &[3]));
    assert_eq!(check(normalize(other)), Err(VerifyError::Pairing));
    let mut scalar = vec![0u8; (L::E as usize).div_ceil(8)];
    scalar[0] = 1;
    scalar[((L::E - 1) / 8) as usize] |= 1 << ((L::E - 1) % 8);
    other = r;
    other.p = affine::<L>(mul(curve, point(r.p), &scalar));
    other.q = affine::<L>(mul(curve, point(r.q), &scalar));
    assert_eq!(check(normalize(other)), Err(VerifyError::Image));
    // All nonzero E[4] translates, in either point or both. Never accept.
    let p4 = dbl_iter(curve, point(r.p), L::E - 2);
    let q4 = dbl_iter(curve, point(r.q), L::E - 2);
    for a in 0..4 {
        for b in 0..4 {
            if a == 0 && b == 0 {
                continue;
            }
            let t = add(curve, mul(curve, p4, &[a]), mul(curve, q4, &[b]));
            for which in 0..3 {
                other = r;
                if which != 1 {
                    other.p = affine::<L>(add(curve, point(r.p), t));
                }
                if which != 0 {
                    other.q = affine::<L>(add(curve, point(r.q), t));
                }
                let verdict = check(normalize(other));
                assert!(
                    matches!(
                        verdict,
                        Err(VerifyError::Pairing | VerifyError::Codomain | VerifyError::Image)
                    ),
                    "translation {a},{b},{which}: {verdict:?}"
                );
            }
        }
    }
    // Alternate salt at a prime degree, and a different message/key.
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
    assert_eq!(check(other), Err(VerifyError::Image));
    let mut message = msg.clone();
    message.extend(b"-other");
    other = r;
    other.salt = find(&pk, &message);
    assert_eq!(
        verify::<L>(&pk, &message, &other.encode()),
        Err(VerifyError::Image)
    );
    let level = match L::E {
        248 => Nist::I,
        376 => Nist::III,
        500 => Nist::V,
        _ => unreachable!(),
    };
    let other_key = &prism_verify::reference_key(level).unwrap().public_key;
    other = r;
    other.salt = find(other_key, &msg);
    assert_eq!(
        verify::<L>(other_key, &msg, &other.encode()),
        Err(VerifyError::Pairing)
    );
    // A one-bit salt mutation either fails primality or the image relation.
    for bit in 0..L::SALT_LEN * 8 {
        other = r;
        let mut bytes = r.salt.as_bytes().to_vec();
        bytes[bit / 8] ^= 1 << (bit % 8);
        other.salt = L::Salt::from_bytes(&bytes).unwrap();
        let prime = ChallengeScalar::derive(&id, &msg, other.salt).degree_is_prime::<L>();
        assert_eq!(
            check(other),
            Err(if prime {
                VerifyError::Image
            } else {
                VerifyError::Composite
            })
        );
    }
    // A -> -A is an isomorphism, with (x,y) -> (-x, i*y).
    other = r;
    other.curve = other.curve.negate();
    other.p.x = other.p.x.negate();
    other.q.x = other.q.x.negate();
    other.p.y = other.p.y.mul(L::ext_i());
    other.q.y = other.q.y.mul(L::ext_i());
    assert_eq!(check(normalize(other)), Err(VerifyError::Model));
    // The four moves of the two other order-two points, with y transported.
    let root = r
        .curve
        .mul(r.curve)
        .sub(L::Ext::from_u64(4))
        .sqrt()
        .unwrap();
    let half = L::Ext::from_u64(2).inv().unwrap();
    for root in [root, root.negate()] {
        let alpha = r.curve.negate().add(root).mul(half);
        let lambda = alpha.mul(alpha).sub(L::Ext::one()).sqrt().unwrap();
        let inv = lambda.inv().unwrap();
        let scale = lambda.mul(lambda.sqrt().unwrap()).inv().unwrap();
        let mut moved = r;
        moved.curve = alpha.mul(L::Ext::from_u64(3)).add(r.curve).mul(inv);
        moved.p.x = r.p.x.sub(alpha).mul(inv);
        moved.q.x = r.q.x.sub(alpha).mul(inv);
        moved.p.y = r.p.y.mul(scale);
        moved.q.y = r.q.y.mul(scale);
        assert_eq!(check(normalize(moved)), Err(VerifyError::Model));
        moved.curve = moved.curve.negate();
        moved.p.x = moved.p.x.negate();
        moved.q.x = moved.q.x.negate();
        moved.p.y = moved.p.y.mul(L::ext_i());
        moved.q.y = moved.q.y.mul(L::ext_i());
        assert_eq!(check(normalize(moved)), Err(VerifyError::Model));
    }
    // Reduced-order points and an off-curve point cannot reach acceptance.
    other = r;
    other.p = affine::<L>(dbl_iter(curve, point(r.p), 2));
    other.q = affine::<L>(dbl_iter(curve, point(r.q), 2));
    assert_eq!(check(normalize(other)), Err(VerifyError::Kernel));
    other = r;
    other.q.y = other.q.y.add(L::Ext::one());
    assert_eq!(check(other), Err(VerifyError::Kernel));
    // Every singular or extra-automorphism response model is excluded.
    for a in [
        L::Ext::zero(),
        L::Ext::from_u64(2),
        L::Ext::from_u64(3).sqrt().unwrap(),
        L::Ext::from_u64(9)
            .mul(L::Ext::from_u64(2).inv().unwrap())
            .sqrt()
            .unwrap(),
    ] {
        other = r;
        other.curve = a;
        assert!(matches!(
            check(other),
            Err(VerifyError::Singular | VerifyError::Model)
        ));
    }
    // Sanity: our full-point arithmetic and infinity encoding agree.
    assert!(to_xz(mul(curve, p4, &[4])).is_infinity());
}
/// Prime and composite degrees among the one-bit salt mutations of a vector.
fn salt_flips<L: Level>(text: &str) -> (usize, usize) {
    let (pk, msg, r) = vector::<L>(text);
    let id = KeyIdentity::<L>::decode(&pk).unwrap();
    let mut prime = 0;
    for bit in 0..L::SALT_LEN * 8 {
        let mut bytes = r.salt.as_bytes().to_vec();
        bytes[bit / 8] ^= 1 << (bit % 8);
        let salt = L::Salt::from_bytes(&bytes).unwrap();
        prime += usize::from(ChallengeScalar::derive(&id, &msg, salt).degree_is_prime::<L>());
    }
    (prime, L::SALT_LEN * 8 - prime)
}
/// The split the salt-mutation negative records: over every committed
/// signature, how many one-bit salt changes land on a prime degree (refused
/// as `Image`) and how many on a composite one (refused as `Composite`).
/// `negative` checks the verdicts themselves on the six primary vectors.
#[test]
fn one_bit_salt_mutations_split_between_composite_and_prime() {
    let mut split = [(0usize, 0usize); 3];
    for path in files() {
        let text = fs::read_to_string(&path).unwrap();
        let (row, (prime, composite)) = match field(&text, "level ").parse::<Nist>().unwrap() {
            Nist::I => (0, salt_flips::<Level1>(&text)),
            Nist::III => (1, salt_flips::<Level3>(&text)),
            Nist::V => (2, salt_flips::<Level5>(&text)),
        };
        split[row].0 += prime;
        split[row].1 += composite;
    }
    for (name, (prime, composite)) in ["I", "III", "V"].iter().zip(split) {
        eprintln!("salt-bit-flips level={name} prime={prime} composite={composite}");
        // A salt bit that did not reach the hash would leave every degree
        // prime; a hash that ignored the salt entirely is the same failure.
        assert!(composite > prime, "level {name}");
    }
}
#[test]
fn malleability_and_message_mutations_are_refused() {
    for path in files()
        .into_iter()
        .filter(|p| p.parent().unwrap().file_name().unwrap() == "testdata")
    {
        let text = fs::read_to_string(path).unwrap();
        match field(&text, "level ").parse::<Nist>().unwrap() {
            Nist::I => negative::<Level1>(&text),
            Nist::III => negative::<Level3>(&text),
            Nist::V => negative::<Level5>(&text),
        }
    }
}
