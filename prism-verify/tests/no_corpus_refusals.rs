// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Every refusal `verify` can be driven to without a signer, at a set
//! that has no corpus.
//!
//! There is no honest signature at such a set, so nothing here is
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
//!
//! The checks are generic over the set. Each set without a corpus runs
//! all of them, in a module of its own at the end of this file, which
//! states the numbers pinned for that set.

#[path = "support/identity.rs"]
mod identity;

use identity::{first_prime_salt_counter, identity_response, salt_of, Form};
use prism_field::{Field, ParameterSet, Salt, P248, P324, P376, P500, P664};
use prism_fixtures::curve_table::{CurveNumber, CurveRow};
use prism_fixtures::no_corpus::{curve_table, Model, WithoutCorpus};
use prism_isogeny::jac::{dbl_iter, Jac};
use prism_isogeny::montgomery::Montgomery;
use prism_verify::signature::canonical_sign;
use prism_verify::{
    verify, verify_set, AffinePoint, ChallengeScalar, KeyIdentity, Response, Set as AnySet,
    VerifyError,
};

/// The numbers a set's test module pins: the first salt counter of prime
/// degree under the set's refusal message, for each curve used as a key.
/// A change to the hash, the projection or the primality test moves them.
#[derive(Clone, Copy)]
struct Pins {
    /// Under the square-branch curve, the key most tests verify under.
    prime_salt_under_key: u64,
    /// Under each permitted model of the set's fixtures, in their order.
    prime_salt_under_models: &'static [u64],
}

/// How many of the composite salts below the first prime one are run
/// through the whole verifier. The rest are held composite by the
/// primality test alone.
const COMPOSITE_SALTS_VERIFIED: u64 = 8;

/// The field elements of a signature: a curve and four coordinates.
fn signature_elements<L: ParameterSet>() -> usize {
    (L::SIGNATURE_LEN - L::SALT_LEN) / L::FP_LEN
}

/// The set's refusal message.
fn message<L: WithoutCorpus>() -> &'static [u8] {
    L::FIXTURES.refusal_message
}

fn curve<L: WithoutCorpus>(number: CurveNumber) -> CurveRow {
    curve_table::<L>().curve(number)
}

fn coefficient<L: WithoutCorpus>(number: CurveNumber) -> L::Ext {
    curve::<L>(number).coefficient.decode::<L>()
}

/// The key every test here verifies under, unless it says otherwise.
fn key<L: WithoutCorpus>() -> Vec<u8> {
    curve::<L>(L::FIXTURES.square_branch_curve)
        .coefficient
        .bytes()
        .to_vec()
}

/// A response whose curve and points are a real curve and a real basis of
/// its `2^e`-torsion, with `y(P)` of the canonical sign. The curve is
/// another one of the file, so it has nothing to do with the key.
///
/// Which curves of the file are the permitted model of their class is a
/// property of each coefficient. The fixtures' model curve is one; the
/// test below that counts them holds this choice in place.
fn unrelated<L: WithoutCorpus>(salt: L::Salt) -> Response<L> {
    let fixtures = L::FIXTURES;
    assert_ne!(
        fixtures.model_curve, fixtures.square_branch_curve,
        "the response curve is not the key"
    );
    let row = curve::<L>(fixtures.model_curve);
    // The file gives the points with whichever square root Sage took for
    // each `y`. The verifier wants `y(P)` of the canonical sign, and `−P`
    // is as good a basis point as `P`.
    let (x, y) = row.p.decode::<L>();
    let (q_x, q_y) = row.q.decode::<L>();
    let response = Response::<L> {
        curve: row.coefficient.decode::<L>(),
        p: AffinePoint {
            x,
            y: if canonical_sign::<L>(y) {
                y
            } else {
                Field::negate(y)
            },
        },
        q: AffinePoint { x: q_x, y: q_y },
        salt,
    };
    assert!(canonical_sign::<L>(response.p.y));
    response
}

fn refused<L: WithoutCorpus>(signature: &[u8]) -> Result<(), VerifyError> {
    verify::<L>(&key::<L>(), message::<L>(), signature)
}

/// The first salt whose challenge degree is prime under [`key`] and
/// the set's refusal message.
fn prime_salt<L: WithoutCorpus>(pins: Pins) -> L::Salt {
    let counter = first_prime_salt_counter::<L>(&key::<L>(), message::<L>());
    assert_eq!(counter, pins.prime_salt_under_key);
    salt_of::<L>(counter)
}

/// The affine point of a Jacobian one, with `y` given the canonical sign.
/// Negating a point keeps its order and its curve.
fn affine<L: ParameterSet>(point: Jac<L::Ext>) -> AffinePoint<L> {
    let inverse = Field::inv(point.z).expect("not the point at infinity");
    let square = Field::square(inverse);
    let x = Field::mul(point.x, square);
    let y = Field::mul(Field::mul(point.y, square), inverse);
    AffinePoint {
        x,
        y: if canonical_sign::<L>(y) {
            y
        } else {
            Field::negate(y)
        },
    }
}

fn a_wrong_length_is_refused_as_length<L: WithoutCorpus>() {
    let good = unrelated::<L>(L::Salt::zero()).encode();
    assert_eq!(good.len(), L::SIGNATURE_LEN);
    assert_eq!(key::<L>().len(), L::PUBLIC_KEY_LEN);
    // Empty, one byte either side, and the lengths of the other sets.
    let others = other_sets::<L>();
    let mut key_lengths = vec![0, L::PUBLIC_KEY_LEN - 1, L::PUBLIC_KEY_LEN + 1];
    key_lengths.extend(others.iter().map(|other| other.public_key_len));
    for length in key_lengths {
        assert_eq!(
            verify::<L>(&vec![1u8; length], message::<L>(), &good),
            Err(VerifyError::Length),
            "a key of {length} bytes"
        );
    }
    let mut signature_lengths = vec![0, L::SIGNATURE_LEN - 1, L::SIGNATURE_LEN + 1];
    signature_lengths.extend(others.iter().map(|other| other.signature_len));
    for length in signature_lengths {
        assert_eq!(
            refused::<L>(&vec![1u8; length]),
            Err(VerifyError::Length),
            "a signature of {length} bytes"
        );
    }
    // Both lengths are checked before either is decoded.
    assert_eq!(
        verify::<L>(
            &vec![0u8; L::PUBLIC_KEY_LEN],
            message::<L>(),
            &vec![0u8; L::SIGNATURE_LEN - 1]
        ),
        Err(VerifyError::Length)
    );
    // The same key and signature under another set are that set's wrong
    // lengths, not a different refusal.
    assert!(!others.is_empty());
    for other in others {
        assert_eq!(
            verify_set(other.set, &key::<L>(), message::<L>(), &good),
            Err(VerifyError::Length),
            "{}",
            other.set
        );
    }
}

fn a_field_element_at_or_above_p_is_refused_as_canonical<L: WithoutCorpus>() {
    let good = unrelated::<L>(L::Salt::zero()).encode();
    // The key: each half in turn set to all ones, which is above p.
    for half in 0..2 {
        let mut bytes = key::<L>();
        bytes[L::FP_LEN * half..L::FP_LEN * (half + 1)].fill(0xff);
        assert_eq!(
            verify::<L>(&bytes, message::<L>(), &good),
            Err(VerifyError::Canonical),
            "key half {half}"
        );
    }
    // The signature: each of its ten field elements in turn.
    for index in 0..signature_elements::<L>() {
        let mut bytes = good.clone();
        bytes[L::FP_LEN * index..L::FP_LEN * (index + 1)].fill(0xff);
        assert_eq!(
            refused::<L>(&bytes),
            Err(VerifyError::Canonical),
            "signature element {index}"
        );
    }
    // Exactly p, in the real part of the first element: the encoding of
    // `−1` is `p − 1`, which ends in an even byte, and one more is `p`.
    let mut modulus = L::base_encode(Field::negate(<L::Base as Field>::one()));
    assert_eq!(modulus.len(), L::FP_LEN);
    assert_eq!(modulus[0] & 1, 0, "p − 1 is even");
    modulus[0] |= 1;
    let mut exactly = good.clone();
    exactly[..L::FP_LEN].copy_from_slice(&modulus);
    assert_eq!(refused::<L>(&exactly), Err(VerifyError::Canonical));
    // The highest bit of that top byte, which is spare at every set: no
    // prime here fills its last byte.
    let top = L::FP_LEN - 1;
    assert_eq!(
        modulus[top] & 0x80,
        0,
        "the top bit of the top byte is spare"
    );
    let mut spare = good;
    spare[top] |= 0x80;
    assert_eq!(refused::<L>(&spare), Err(VerifyError::Canonical));
}

fn the_two_forbidden_keys_are_refused<L: WithoutCorpus>() {
    let good = unrelated::<L>(L::Salt::zero()).encode();
    assert_eq!(
        verify::<L>(&vec![0u8; L::PUBLIC_KEY_LEN], message::<L>(), &good),
        Err(VerifyError::Origin)
    );
    // The two square roots of 4.
    let two = <L::Ext as Field>::from_u64(2);
    for coefficient in [two, Field::negate(two)] {
        assert_eq!(
            verify::<L>(&L::ext_encode(coefficient), message::<L>(), &good),
            Err(VerifyError::Singular)
        );
    }
}

fn the_wrong_sign_of_y_is_refused_as_sign<L: WithoutCorpus>() {
    let mut response = unrelated::<L>(L::Salt::zero());
    response.p.y = Field::negate(response.p.y);
    assert_eq!(refused::<L>(&response.encode()), Err(VerifyError::Sign));
    response.p.y = <L::Ext as Field>::zero();
    assert_eq!(refused::<L>(&response.encode()), Err(VerifyError::Sign));
}

/// Of the two coefficients `A` and `−A` of each curve of the file, at most
/// one is the permitted model, and for most curves neither is: the least
/// coefficient of the class is some other Montgomery model. Which ones
/// pass is an accident of the set's walk, and the set's fixtures list
/// them: this test holds that the list is exactly the ones that pass. One
/// of them is the model curve itself, which is what makes it usable above.
fn a_curve_that_is_not_the_permitted_model_is_refused_as_model<L: WithoutCorpus>() {
    let mut past_model = Vec::new();
    for row in curve_table::<L>().rows() {
        for negated in [false, true] {
            let mut response = unrelated::<L>(L::Salt::zero());
            let coefficient = row.coefficient.decode::<L>();
            response.curve = if negated {
                Field::negate(coefficient)
            } else {
                coefficient
            };
            match refused::<L>(&response.encode()) {
                Err(VerifyError::Model) => {}
                Err(VerifyError::Composite) => past_model.push(Model {
                    curve: row.number,
                    negated,
                }),
                other => panic!("curve {}, negated {negated}: {other:?}", row.number),
            }
        }
    }
    assert_eq!(past_model, L::FIXTURES.models);
    // The starting curve and a singular curve are never a response curve.
    let mut response = unrelated::<L>(L::Salt::zero());
    response.curve = <L::Ext as Field>::zero();
    assert_eq!(refused::<L>(&response.encode()), Err(VerifyError::Model));
    response.curve = <L::Ext as Field>::from_u64(2);
    assert_eq!(refused::<L>(&response.encode()), Err(VerifyError::Singular));
}

fn a_composite_challenge_degree_is_refused_as_composite<L: WithoutCorpus>(pins: Pins) {
    let key = key::<L>();
    let identity = KeyIdentity::<L>::decode(&key).expect("a key");
    let mut composite = 0;
    for counter in 0..pins.prime_salt_under_key {
        let salt = salt_of::<L>(counter);
        assert!(!ChallengeScalar::derive(&identity, message::<L>(), salt).degree_is_prime::<L>());
        if counter < COMPOSITE_SALTS_VERIFIED {
            assert_eq!(
                refused::<L>(&unrelated::<L>(salt).encode()),
                Err(VerifyError::Composite),
                "salt counter {counter}"
            );
        }
        composite += 1;
    }
    assert_eq!(composite, pins.prime_salt_under_key);
}

/// With a prime degree the verifier goes on to the response points. A
/// point off its curve, a point of order `2^(e−1)`, and a pair that is one
/// point twice, are each refused as `Kernel`, before any pairing.
fn response_points_that_are_not_a_basis_are_refused_as_kernel<L: WithoutCorpus>(pins: Pins) {
    let good = unrelated::<L>(prime_salt::<L>(pins));
    let curve = Montgomery::normalized(good.curve);
    let jac = |point: AffinePoint<L>| Jac {
        x: point.x,
        y: point.y,
        z: <L::Ext as Field>::one(),
    };
    let one = <L::Ext as Field>::one();
    let cases: Vec<(&str, AffinePoint<L>, AffinePoint<L>)> = vec![
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
            affine::<L>(dbl_iter(curve, jac(good.p), 1)),
            good.q,
        ),
        (
            "Q of order 2^(e-1)",
            good.p,
            affine::<L>(dbl_iter(curve, jac(good.q), 1)),
        ),
        ("Q = P", good.p, good.p),
    ];
    for (name, p, q) in cases {
        let response = Response::<L> { p, q, ..good };
        assert!(canonical_sign::<L>(response.p.y), "{name}");
        assert_eq!(
            refused::<L>(&response.encode()),
            Err(VerifyError::Kernel),
            "{name}"
        );
    }
}

/// A real curve with a real basis of its `2^e`-torsion, a prime degree,
/// every public check passed, and no relation to the key: refused at the
/// pairing. A response on the key's own curve gets further; see
/// `the_pairing_is_passed_from_public_data_and_the_chains_refuse`.
fn a_valid_basis_unrelated_to_the_key_is_refused_at_the_pairing<L: WithoutCorpus>(pins: Pins) {
    let good = unrelated::<L>(prime_salt::<L>(pins));
    assert_eq!(refused::<L>(&good.encode()), Err(VerifyError::Pairing));
    // The two points the other way round, with the sign rule kept.
    let swapped = Response::<L> {
        p: AffinePoint {
            x: good.q.x,
            y: if canonical_sign::<L>(good.q.y) {
                good.q.y
            } else {
                Field::negate(good.q.y)
            },
        },
        q: good.p,
        ..good
    };
    assert_eq!(refused::<L>(&swapped.encode()), Err(VerifyError::Pairing));
    // The same bytes under another message are a different challenge,
    // whose degree is composite.
    assert_eq!(
        verify::<L>(&key::<L>(), b"another message", &good.encode()),
        Err(VerifyError::Composite)
    );
}

/// The pairing check is passed with no secret, and what follows it runs at
/// this set.
///
/// The key is each coefficient of the file that is a permitted response
/// model, in turn. The response is on the key's own curve. Its points
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
/// comparison run at this set and refuse a response with no isogeny behind
/// it. It does not show acceptance: nothing here is accepted, and no
/// honest signature exists at this prime.
///
/// The fixture file's own pair for a curve is `(P, ±(P − Q))`, so the
/// second form is, up to that sign, the file's points under their own
/// curve as key.
fn the_pairing_is_passed_from_public_data_and_the_chains_refuse<L: WithoutCorpus>(pins: Pins) {
    let fixtures = L::FIXTURES;
    assert_eq!(
        fixtures.models.len(),
        pins.prime_salt_under_models.len(),
        "one pinned salt counter for each model"
    );
    let keys = fixtures
        .models
        .iter()
        .zip(pins.prime_salt_under_models)
        .map(|(model, &prime_salt_counter)| {
            let coefficient = coefficient::<L>(model.curve);
            let coefficient = if model.negated {
                Field::negate(coefficient)
            } else {
                coefficient
            };
            (coefficient, prime_salt_counter)
        });
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
            let built = identity_response::<L>(coefficient, message::<L>(), form);
            assert_eq!(
                first_prime_salt_counter::<L>(&built.key, message::<L>()),
                prime_salt_counter
            );
            assert_eq!(built.response.salt, salt_of::<L>(prime_salt_counter));
            assert!(Field::same(built.response.curve, coefficient));
            assert!(canonical_sign::<L>(built.response.p.y));
            assert_eq!(
                verify::<L>(&built.key, message::<L>(), &built.response.encode()),
                Err(refusal),
                "{name}"
            );
            let mut failing = built.response;
            failing.q.y = Field::negate(failing.q.y);
            assert_eq!(
                verify::<L>(&built.key, message::<L>(), &failing.encode()),
                Err(VerifyError::Pairing),
                "{name} with the second point negated"
            );
        }
    }
    // The file's own points for the model curve, under that curve as key,
    // are the second form up to the sign of the second point. Sage took
    // whichever root it took for `y(Q)`, so one sign passes the pairing
    // and is refused as `Codomain`, and the other fails the pairing.
    let built = identity_response::<L>(
        coefficient::<L>(fixtures.model_curve),
        message::<L>(),
        Form::Difference,
    );
    let file = unrelated::<L>(built.response.salt);
    assert!(Field::same(file.p.x, built.response.p.x) && Field::same(file.q.x, built.response.q.x));
    let mut other_sign = file;
    other_sign.q.y = Field::negate(other_sign.q.y);
    let verdicts = [file, other_sign]
        .map(|response| verify::<L>(&built.key, message::<L>(), &response.encode()));
    assert!(
        verdicts == [Err(VerifyError::Codomain), Err(VerifyError::Pairing)]
            || verdicts == [Err(VerifyError::Pairing), Err(VerifyError::Codomain)],
        "{verdicts:?}"
    );
}

/// What the length tests need of a set other than the one under test.
struct OtherSet {
    set: AnySet,
    public_key_len: usize,
    signature_len: usize,
}

/// Every set but `L`. One entry for each set of the crate: a new set is
/// added here, and none is reached by a default.
fn other_sets<L: ParameterSet>() -> Vec<OtherSet> {
    fn entry<M: ParameterSet>(set: AnySet) -> OtherSet {
        assert_eq!(set.name(), M::NAME);
        OtherSet {
            set,
            public_key_len: M::PUBLIC_KEY_LEN,
            signature_len: M::SIGNATURE_LEN,
        }
    }
    let all = [
        entry::<P248>(AnySet::P248),
        entry::<P324>(AnySet::P324),
        entry::<P376>(AnySet::P376),
        entry::<P500>(AnySet::P500),
        entry::<P664>(AnySet::P664),
    ];
    assert_eq!(all.len(), AnySet::ALL.len());
    all.into_iter()
        .filter(|other| other.set.name() != L::NAME)
        .collect()
}

/// Every check of this file at one set, with the numbers pinned for it.
macro_rules! at_set {
    ($module:ident, $set:ty, $pins:expr) => {
        mod $module {
            use super::Pins;

            const PINS: Pins = $pins;

            #[test]
            fn a_wrong_length_is_refused_as_length() {
                super::a_wrong_length_is_refused_as_length::<$set>();
            }

            #[test]
            fn a_field_element_at_or_above_p_is_refused_as_canonical() {
                super::a_field_element_at_or_above_p_is_refused_as_canonical::<$set>();
            }

            #[test]
            fn the_two_forbidden_keys_are_refused() {
                super::the_two_forbidden_keys_are_refused::<$set>();
            }

            #[test]
            fn the_wrong_sign_of_y_is_refused_as_sign() {
                super::the_wrong_sign_of_y_is_refused_as_sign::<$set>();
            }

            #[test]
            fn a_curve_that_is_not_the_permitted_model_is_refused_as_model() {
                super::a_curve_that_is_not_the_permitted_model_is_refused_as_model::<$set>();
            }

            #[test]
            fn a_composite_challenge_degree_is_refused_as_composite() {
                super::a_composite_challenge_degree_is_refused_as_composite::<$set>(PINS);
            }

            #[test]
            fn response_points_that_are_not_a_basis_are_refused_as_kernel() {
                super::response_points_that_are_not_a_basis_are_refused_as_kernel::<$set>(PINS);
            }

            #[test]
            fn a_valid_basis_unrelated_to_the_key_is_refused_at_the_pairing() {
                super::a_valid_basis_unrelated_to_the_key_is_refused_at_the_pairing::<$set>(PINS);
            }

            #[test]
            fn the_pairing_is_passed_from_public_data_and_the_chains_refuse() {
                super::the_pairing_is_passed_from_public_data_and_the_chains_refuse::<$set>(PINS);
            }
        }
    };
}

// At `p324` about one degree in a hundred is prime: 93 and 79 of 10,000
// in two runs of the primality differential.
at_set!(
    p324,
    prism_field::P324,
    Pins {
        prime_salt_under_key: 183,
        prime_salt_under_models: &[16, 24],
    }
);

// At `p664` about one degree in 200 to 250 is prime; the heuristic for
// `q = 2^k − s²` with `k` odd predicts one in 248 (a constant of about
// 3.70 over `ln q`). Three samples: 59 of 10,000 and 18 of 4,000 in runs
// of the primality differential, and 89 of 20,000 in an independent
// simulation. Such a degree is never divisible by 3, so it is prime more
// often than an odd integer of its size.
at_set!(
    p664,
    prism_field::P664,
    Pins {
        prime_salt_under_key: 520,
        prime_salt_under_models: &[413, 78],
    }
);
