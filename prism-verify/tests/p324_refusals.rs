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
//! The pairing check is also passed from public data, with no secret: a
//! key's own curve as the response curve, and a pair of points whose
//! pairing is the inverse of the key basis's (`support/identity.rs`). Such
//! a response has no isogeny behind it, and the verifier refuses it in the
//! chain work that follows.
//!
//! Reached: `Length`, `Canonical`, `Origin`, `Singular`, `Sign`, `Model`,
//! `Composite`, `Kernel` (before the pairing and after it), `Pairing`,
//! `Codomain`. Not reached: `Basis` (it needs a key with no basis index
//! inside the bound, which nobody knows how to find) and `Image` (the
//! codomains must agree first, and no response known to be buildable from
//! public data gets them to).

#[path = "support/identity.rs"]
mod identity;

use identity::{first_prime_salt_counter, identity_response, salt_of, Form};
use prism_field::{Field, ParameterSet, Salt, P248, P324, P376, P500};
use prism_fixtures::curve_table::{CurveNumber, CurveRow};
use prism_fixtures::p324::{
    self, MODEL_CURVE, NEGATED_MODEL_CURVE, REFUSAL_MESSAGE as MESSAGE, SQUARE_BRANCH_CURVE,
};
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

/// The first salt counter of prime degree under [`MESSAGE`], for each
/// curve used as a key. Pinned: a change to the hash, the projection or
/// the primality test moves these numbers. About one degree in ninety is
/// prime at this size.
const PRIME_SALT_UNDER_SQUARE_BRANCH_CURVE: u64 = 183;
const PRIME_SALT_UNDER_MODEL_CURVE: u64 = 24;
const PRIME_SALT_UNDER_NEGATED_MODEL_CURVE: u64 = 16;

/// How many of the composite salts below the first prime one are run
/// through the whole verifier. The rest are held composite by the
/// primality test alone.
const COMPOSITE_SALTS_VERIFIED: u64 = 8;

/// The field elements of a signature: a curve and four coordinates.
const SIGNATURE_ELEMENTS: usize = (Set::SIGNATURE_LEN - Set::SALT_LEN) / Set::FP_LEN;

fn curve(number: CurveNumber) -> CurveRow {
    p324::curve_table().curve(number)
}

fn coefficient(number: CurveNumber) -> Ext {
    curve(number).coefficient.decode::<Set>()
}

/// The key every test here verifies under, unless it says otherwise.
fn key() -> Vec<u8> {
    curve(SQUARE_BRANCH_CURVE).coefficient.bytes().to_vec()
}

/// A response whose curve and points are a real curve and a real basis of
/// its `2^e`-torsion, with `y(P)` of the canonical sign. The curve is
/// another one of the file, so it has nothing to do with the key.
///
/// Which curves of the file are the permitted model of their class is a
/// property of each coefficient. [`MODEL_CURVE`] is one; the test below
/// that counts them holds this choice in place.
fn unrelated(salt: SaltOf) -> Response<Set> {
    let row = curve(MODEL_CURVE);
    // The file gives the points with whichever square root Sage took for
    // each `y`. The verifier wants `y(P)` of the canonical sign, and `−P`
    // is as good a basis point as `P`.
    let (x, y) = row.p.decode::<Set>();
    let (q_x, q_y) = row.q.decode::<Set>();
    let response = Response::<Set> {
        curve: row.coefficient.decode::<Set>(),
        p: AffinePoint {
            x,
            y: if canonical_sign::<Set>(y) {
                y
            } else {
                Field::negate(y)
            },
        },
        q: AffinePoint { x: q_x, y: q_y },
        salt,
    };
    assert!(canonical_sign::<Set>(response.p.y));
    response
}

fn refused(signature: &[u8]) -> Result<(), VerifyError> {
    verify::<Set>(&key(), MESSAGE, signature)
}

/// The first salt whose challenge degree is prime under [`key`] and
/// [`MESSAGE`].
fn prime_salt() -> SaltOf {
    let counter = first_prime_salt_counter::<Set>(&key(), MESSAGE);
    assert_eq!(counter, PRIME_SALT_UNDER_SQUARE_BRANCH_CURVE);
    salt_of::<Set>(counter)
}

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
    assert_eq!(good.len(), Set::SIGNATURE_LEN);
    assert_eq!(key().len(), Set::PUBLIC_KEY_LEN);
    // Empty, one byte either side, and the lengths of the other sets.
    for length in [
        0,
        Set::PUBLIC_KEY_LEN - 1,
        Set::PUBLIC_KEY_LEN + 1,
        P248::PUBLIC_KEY_LEN,
        P376::PUBLIC_KEY_LEN,
        P500::PUBLIC_KEY_LEN,
    ] {
        assert_eq!(
            verify::<Set>(&vec![1u8; length], MESSAGE, &good),
            Err(VerifyError::Length),
            "a key of {length} bytes"
        );
    }
    for length in [
        0,
        Set::SIGNATURE_LEN - 1,
        Set::SIGNATURE_LEN + 1,
        P248::SIGNATURE_LEN,
        P376::SIGNATURE_LEN,
        P500::SIGNATURE_LEN,
    ] {
        assert_eq!(
            refused(&vec![1u8; length]),
            Err(VerifyError::Length),
            "a signature of {length} bytes"
        );
    }
    // Both lengths are checked before either is decoded.
    assert_eq!(
        verify::<Set>(
            &[0u8; Set::PUBLIC_KEY_LEN],
            MESSAGE,
            &[0u8; Set::SIGNATURE_LEN - 1]
        ),
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
        bytes[Set::FP_LEN * half..Set::FP_LEN * (half + 1)].fill(0xff);
        assert_eq!(
            verify::<Set>(&bytes, MESSAGE, &good),
            Err(VerifyError::Canonical),
            "key half {half}"
        );
    }
    // The signature: each of its ten field elements in turn.
    for index in 0..SIGNATURE_ELEMENTS {
        let mut bytes = good.clone();
        bytes[Set::FP_LEN * index..Set::FP_LEN * (index + 1)].fill(0xff);
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
        verify::<Set>(&[0u8; Set::PUBLIC_KEY_LEN], MESSAGE, &good),
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
/// the sixteen pass: [`NEGATED_MODEL_CURVE`] negated and [`MODEL_CURVE`]
/// itself, which is what makes the latter usable above.
#[test]
fn a_curve_that_is_not_the_permitted_model_is_refused_as_model() {
    let mut past_model = Vec::new();
    for row in p324::curve_table().rows() {
        for negated in [false, true] {
            let mut response = unrelated(SaltOf::zero());
            let coefficient = row.coefficient.decode::<Set>();
            response.curve = if negated {
                Field::negate(coefficient)
            } else {
                coefficient
            };
            match refused(&response.encode()) {
                Err(VerifyError::Model) => {}
                Err(VerifyError::Composite) => past_model.push((row.number, negated)),
                other => panic!("curve {}, negated {negated}: {other:?}", row.number),
            }
        }
    }
    assert_eq!(
        past_model,
        vec![(NEGATED_MODEL_CURVE, true), (MODEL_CURVE, false)]
    );
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
    for counter in 0..PRIME_SALT_UNDER_SQUARE_BRANCH_CURVE {
        let salt = salt_of::<Set>(counter);
        assert!(!ChallengeScalar::derive(&identity, MESSAGE, salt).degree_is_prime::<Set>());
        if counter < COMPOSITE_SALTS_VERIFIED {
            assert_eq!(
                refused(&unrelated(salt).encode()),
                Err(VerifyError::Composite),
                "salt counter {counter}"
            );
        }
        composite += 1;
    }
    assert_eq!(composite, PRIME_SALT_UNDER_SQUARE_BRANCH_CURVE);
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
/// pairing. A response on the key's own curve gets further; see
/// `the_pairing_is_passed_from_public_data_and_the_chains_refuse`.
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

/// The pairing check is passed with no secret, and what follows it runs at
/// `p324`.
///
/// The key is [`MODEL_CURVE`] of the file (and, as a second key, the
/// negation of [`NEGATED_MODEL_CURVE`]: the two coefficients of the file
/// that are a permitted response model). The response is on the key's own curve. Its points
/// are the verifier's own basis `(P, Q)` of that key, answered as a pair
/// whose pairing is the inverse of `e(P, Q)`, which is all the pairing
/// check asks. The salt is the first counter with a prime degree.
///
/// - `(P, −Q)` passes the pairing and is refused as `Kernel`.
/// - `(P, P − Q)` passes the pairing and is refused as `Codomain`. To
///   return that, the verifier has built the kernel, run both theta
///   chains and compared the codomains. The operation counts of this
///   case are in `docs/benchmarks/op-counts.txt`.
/// - With the second point negated again, each is refused as `Pairing`:
///   the two verdicts above are not reached by a response that fails it.
///
/// This shows that the kernel construction, both chains and the codomain
/// comparison run at `p324` and refuse a response with no isogeny behind
/// it. It does not show acceptance: nothing here is accepted, and no
/// honest signature exists at this prime.
///
/// The fixture file's own pair for a curve is `(P, ±(P − Q))`, so the
/// second form is, up to that sign, the file's points under their own
/// curve as key.
#[test]
fn the_pairing_is_passed_from_public_data_and_the_chains_refuse() {
    let keys = [
        (
            Field::negate(coefficient(NEGATED_MODEL_CURVE)),
            PRIME_SALT_UNDER_NEGATED_MODEL_CURVE,
        ),
        (coefficient(MODEL_CURVE), PRIME_SALT_UNDER_MODEL_CURVE),
    ];
    // The verdict of each form, pinned here by the name it has in the
    // records, and not read from the helper.
    let verdicts = [
        ("identity-response-q-negated", VerifyError::Kernel),
        ("identity-response-q-difference", VerifyError::Codomain),
    ];
    for (coefficient, prime_salt_counter) in keys {
        for (form, (name, refusal)) in Form::ALL.into_iter().zip(verdicts) {
            assert_eq!(form.name(), name);
            assert_eq!(form.allowed(), [refusal], "{name}");
            let built = identity_response::<Set>(coefficient, MESSAGE, form);
            assert_eq!(
                first_prime_salt_counter::<Set>(&built.key, MESSAGE),
                prime_salt_counter
            );
            assert_eq!(built.response.salt, salt_of::<Set>(prime_salt_counter));
            assert!(Field::same(built.response.curve, coefficient));
            assert!(canonical_sign::<Set>(built.response.p.y));
            assert_eq!(
                verify::<Set>(&built.key, MESSAGE, &built.response.encode()),
                Err(refusal),
                "{name}"
            );
            let mut failing = built.response;
            failing.q.y = Field::negate(failing.q.y);
            assert_eq!(
                verify::<Set>(&built.key, MESSAGE, &failing.encode()),
                Err(VerifyError::Pairing),
                "{name} with the second point negated"
            );
        }
    }
    // The file's own points for the model curve, under that curve as
    // key, are the second form up to the sign of the second point.
    let built = identity_response::<Set>(coefficient(MODEL_CURVE), MESSAGE, Form::Difference);
    let file = unrelated(built.response.salt);
    assert!(Field::same(file.p.x, built.response.p.x) && Field::same(file.q.x, built.response.q.x));
    assert_eq!(
        verify::<Set>(&built.key, MESSAGE, &file.encode()),
        Err(VerifyError::Codomain)
    );
}
