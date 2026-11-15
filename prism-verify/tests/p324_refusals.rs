// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Every refusal `verify` can be driven to at `p324` without a signer.
//!
//! There is no honest signature at this prime, so nothing here is
//! accepted and nothing here shows that anything would be. What can be
//! built from public data alone is built: keys and signatures of wrong
//! length or with a field element out of range, the two forbidden keys, a
//! response with the wrong sign or the wrong curve model, a composite
//! challenge degree, and, with a salt whose degree is prime, response
//! points that are off their curve, of too small an order, dependent, or
//! a perfectly good basis that has nothing to do with the key.
//!
//! Reached: `Length`, `Canonical`, `Origin`, `Singular`, `Sign`, `Model`,
//! `Composite`, `Kernel`, `Pairing`. Not reached: `Basis` (it needs a key
//! with no basis index inside the bound, which nobody knows how to find),
//! and `Codomain` and `Image` (they lie past the pairing check, which only
//! a response bound to the key by the secret isogeny passes).

use prism_field::{Field, ParameterSet, Salt, P324};
use prism_isogeny::jac::{dbl_iter, Jac};
use prism_isogeny::montgomery::Montgomery;
use prism_verify::signature::canonical_sign;
use prism_verify::{
    verify, verify_set, AffinePoint, ChallengeScalar, KeyIdentity, Response, Set as AnySet,
    VerifyError,
};

type Set = P324;
type Ext = <Set as ParameterSet>::Ext;
type SaltOf = <Set as ParameterSet>::Salt;

const TABLE: &str = include_str!("../testdata/p324_curves.tsv");
const MESSAGE: &[u8] = b"shekyl-prism-p324-refusals";

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

fn element(text: &str) -> Ext {
    Set::ext_decode(&hex(text)).expect("a canonical element")
}

fn rows() -> Vec<Vec<&'static str>> {
    TABLE
        .lines()
        .filter(|line| line.starts_with("CURVE\t"))
        .map(|line| line.split('\t').collect())
        .collect()
}

/// The key every test here verifies under: the first curve of the file.
fn key() -> Vec<u8> {
    hex(rows()[0][2])
}

/// A response whose curve and points are a real curve and a real basis of
/// its `2^e`-torsion, with `y(P)` of the canonical sign. The curve is
/// another one of the file, so it has nothing to do with the key.
///
/// Which curves of the file are the permitted model of their class is a
/// property of each coefficient. The seventh is; the test below that
/// counts them holds this choice in place.
fn unrelated(salt: SaltOf) -> Response<Set> {
    let row = &rows()[6];
    // The file gives the points with whichever square root Sage took for
    // each `y`. The verifier wants `y(P)` of the canonical sign, and `−P`
    // is as good a basis point as `P`.
    let y = element(row[10]);
    let response = Response::<Set> {
        curve: element(row[2]),
        p: AffinePoint {
            x: element(row[9]),
            y: if canonical_sign::<Set>(y) {
                y
            } else {
                Field::negate(y)
            },
        },
        q: AffinePoint {
            x: element(row[11]),
            y: element(row[12]),
        },
        salt,
    };
    assert!(canonical_sign::<Set>(response.p.y));
    response
}

fn refused(signature: &[u8]) -> Result<(), VerifyError> {
    verify::<Set>(&key(), MESSAGE, signature)
}

/// The first salt counter whose challenge degree is prime under the key
/// and message of this file. About one degree in ninety is prime at this
/// size, so the search is short.
fn prime_salt() -> SaltOf {
    let key = key();
    let identity = KeyIdentity::<Set>::decode(&key).expect("a key");
    let found = (0..5000u64).find_map(|counter| {
        let salt = SaltOf::from_counter(counter).expect("a counter salt");
        ChallengeScalar::derive(&identity, MESSAGE, salt)
            .degree_is_prime::<Set>()
            .then_some((counter, salt))
    });
    let (counter, salt) = found.expect("a prime degree within 5,000 salts");
    // Pinned: a change to the hash, the projection or the primality test
    // moves this number.
    assert_eq!(counter, PRIME_SALT_COUNTER);
    salt
}

/// See [`prime_salt`].
const PRIME_SALT_COUNTER: u64 = 183;

/// The affine point of a Jacobian one, with `y` given the canonical sign.
/// Negating a point keeps its order and its curve.
fn affine(point: Jac<Ext>) -> AffinePoint<Set> {
    let inverse = Field::inv(point.z).expect("not the point at infinity");
    let square = Field::square(inverse);
    let x = Field::mul(point.x, square);
    let y = Field::mul(Field::mul(point.y, square), inverse);
    AffinePoint {
        x,
        y: if canonical_sign::<Set>(y) {
            y
        } else {
            Field::negate(y)
        },
    }
}

#[test]
fn a_wrong_length_is_refused_as_length() {
    let good = unrelated(SaltOf::zero()).encode();
    assert_eq!(good.len(), 458);
    assert_eq!(key().len(), 82);
    for length in [0, 81, 83, 64, 96, 128] {
        assert_eq!(
            verify::<Set>(&vec![1u8; length], MESSAGE, &good),
            Err(VerifyError::Length),
            "a key of {length} bytes"
        );
    }
    for length in [0, 457, 459, 352, 528, 704] {
        assert_eq!(
            refused(&vec![1u8; length]),
            Err(VerifyError::Length),
            "a signature of {length} bytes"
        );
    }
    // Both lengths are checked before either is decoded.
    assert_eq!(
        verify::<Set>(&[0u8; 82], MESSAGE, &[0u8; 457]),
        Err(VerifyError::Length)
    );
    // The same key and signature under another set are that set's wrong
    // lengths, not a different refusal.
    for other in [AnySet::P248, AnySet::P376, AnySet::P500] {
        assert_eq!(
            verify_set(other, &key(), MESSAGE, &good),
            Err(VerifyError::Length),
            "{other}"
        );
    }
}

#[test]
fn a_field_element_at_or_above_p_is_refused_as_canonical() {
    let good = unrelated(SaltOf::zero()).encode();
    // The key: each half in turn set to all ones, which is above p.
    for half in 0..2 {
        let mut bytes = key();
        bytes[41 * half..41 * (half + 1)].fill(0xff);
        assert_eq!(
            verify::<Set>(&bytes, MESSAGE, &good),
            Err(VerifyError::Canonical),
            "key half {half}"
        );
    }
    // The signature: each of its ten field elements in turn.
    for index in 0..10 {
        let mut bytes = good.clone();
        bytes[41 * index..41 * (index + 1)].fill(0xff);
        assert_eq!(
            refused(&bytes),
            Err(VerifyError::Canonical),
            "signature element {index}"
        );
    }
    // Exactly p, in the first element: all ones below a top byte of 0x2f.
    let mut exactly = good.clone();
    exactly[..40].fill(0xff);
    exactly[40] = 0x2f;
    assert_eq!(refused(&exactly), Err(VerifyError::Canonical));
    // One of the two spare bits of a top byte.
    let mut spare = good;
    spare[40] |= 0x80;
    assert_eq!(refused(&spare), Err(VerifyError::Canonical));
}

#[test]
fn the_two_forbidden_keys_are_refused() {
    let good = unrelated(SaltOf::zero()).encode();
    assert_eq!(
        verify::<Set>(&[0u8; 82], MESSAGE, &good),
        Err(VerifyError::Origin)
    );
    // The two square roots of 4.
    let two = <Ext as Field>::from_u64(2);
    for coefficient in [two, Field::negate(two)] {
        assert_eq!(
            verify::<Set>(&Set::ext_encode(coefficient), MESSAGE, &good),
            Err(VerifyError::Singular)
        );
    }
}

#[test]
fn the_wrong_sign_of_y_is_refused_as_sign() {
    let mut response = unrelated(SaltOf::zero());
    response.p.y = Field::negate(response.p.y);
    assert_eq!(refused(&response.encode()), Err(VerifyError::Sign));
    response.p.y = <Ext as Field>::zero();
    assert_eq!(refused(&response.encode()), Err(VerifyError::Sign));
}

/// Of the two coefficients `A` and `−A` of each curve of the file, at most
/// one is the permitted model, and for most curves neither is: the least
/// coefficient of the class is some other Montgomery model. Exactly two of
/// the sixteen pass, which is what makes the seventh curve usable above.
#[test]
fn a_curve_that_is_not_the_permitted_model_is_refused_as_model() {
    let mut past_model = Vec::new();
    for (number, row) in rows().iter().enumerate() {
        for negated in [false, true] {
            let mut response = unrelated(SaltOf::zero());
            let coefficient = element(row[2]);
            response.curve = if negated {
                Field::negate(coefficient)
            } else {
                coefficient
            };
            match refused(&response.encode()) {
                Err(VerifyError::Model) => {}
                Err(VerifyError::Composite) => past_model.push((number, negated)),
                other => panic!("curve {number}, negated {negated}: {other:?}"),
            }
        }
    }
    assert_eq!(past_model, vec![(4, true), (6, false)]);
    // The starting curve and a singular curve are never a response curve.
    let mut response = unrelated(SaltOf::zero());
    response.curve = <Ext as Field>::zero();
    assert_eq!(refused(&response.encode()), Err(VerifyError::Model));
    response.curve = <Ext as Field>::from_u64(2);
    assert_eq!(refused(&response.encode()), Err(VerifyError::Singular));
}

#[test]
fn a_composite_challenge_degree_is_refused_as_composite() {
    let key = key();
    let identity = KeyIdentity::<Set>::decode(&key).expect("a key");
    let mut composite = 0;
    for counter in 0..PRIME_SALT_COUNTER {
        let salt = SaltOf::from_counter(counter).expect("a counter salt");
        assert!(!ChallengeScalar::derive(&identity, MESSAGE, salt).degree_is_prime::<Set>());
        if counter < 8 {
            assert_eq!(
                refused(&unrelated(salt).encode()),
                Err(VerifyError::Composite),
                "salt counter {counter}"
            );
        }
        composite += 1;
    }
    assert_eq!(composite, PRIME_SALT_COUNTER);
}

/// With a prime degree the verifier goes on to the response points. A
/// point off its curve, a point of order `2^(e−1)`, and a pair that is one
/// point twice, are each refused as `Kernel`, before any pairing.
#[test]
fn response_points_that_are_not_a_basis_are_refused_as_kernel() {
    let good = unrelated(prime_salt());
    let curve = Montgomery::normalized(good.curve);
    let jac = |point: AffinePoint<Set>| Jac {
        x: point.x,
        y: point.y,
        z: <Ext as Field>::one(),
    };
    let one = <Ext as Field>::one();
    let cases: Vec<(&str, AffinePoint<Set>, AffinePoint<Set>)> = vec![
        (
            "P off the curve",
            AffinePoint {
                x: Field::add(good.p.x, one),
                y: good.p.y,
            },
            good.q,
        ),
        (
            "Q off the curve",
            good.p,
            AffinePoint {
                x: Field::add(good.q.x, one),
                y: good.q.y,
            },
        ),
        (
            "P of order 2^(e-1)",
            affine(dbl_iter(curve, jac(good.p), 1)),
            good.q,
        ),
        (
            "Q of order 2^(e-1)",
            good.p,
            affine(dbl_iter(curve, jac(good.q), 1)),
        ),
        ("Q = P", good.p, good.p),
    ];
    for (name, p, q) in cases {
        let response = Response::<Set> { p, q, ..good };
        assert!(canonical_sign::<Set>(response.p.y), "{name}");
        assert_eq!(
            refused(&response.encode()),
            Err(VerifyError::Kernel),
            "{name}"
        );
    }
}

/// A real curve with a real basis of its `2^e`-torsion, a prime degree,
/// every public check passed, and no relation to the key: refused at the
/// pairing. This is as far as anything gets at `p324` without the secret.
#[test]
fn a_valid_basis_unrelated_to_the_key_is_refused_at_the_pairing() {
    let good = unrelated(prime_salt());
    assert_eq!(refused(&good.encode()), Err(VerifyError::Pairing));
    // The two points the other way round, with the sign rule kept.
    let swapped = Response::<Set> {
        p: AffinePoint {
            x: good.q.x,
            y: if canonical_sign::<Set>(good.q.y) {
                good.q.y
            } else {
                Field::negate(good.q.y)
            },
        },
        q: good.p,
        ..good
    };
    assert_eq!(refused(&swapped.encode()), Err(VerifyError::Pairing));
    // The same bytes under another message are a different challenge,
    // whose degree is composite.
    assert_eq!(
        verify::<Set>(&key(), b"another message", &good.encode()),
        Err(VerifyError::Composite)
    );
}
