// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Curves, point orders, the torsion basis and the Weil pairing against
//! Sage, at a set that has no corpus.
//!
//! `testdata/<set>_curves.tsv` is written by
//! `scripts/sage/no_corpus_fixture.sage`. Each row is one supersingular
//! Montgomery curve on a 2-isogeny walk from `A = 0`, with points whose
//! orders Sage computed, the basis the verifier's index search names, and
//! the pairing of that basis.
//!
//! There is no signer at such a set, so there is no signature here. These
//! tests reach everything the verifier does with a public key and with
//! points, and nothing that needs an accepted response.
//!
//! The checks are generic over the set. Each set without a corpus runs
//! all of them, in a module of its own at the end of this file.

use prism_field::{Field, ParameterSet, Salt};
use prism_fixtures::curve_table::{CurveNumber, Point};
use prism_fixtures::no_corpus::{curve_table, WithoutCorpus};
use prism_isogeny::jac::Jac;
use prism_isogeny::montgomery::{Montgomery, Xz};
use prism_isogeny::pairing::weil;
use prism_isogeny::torsion::{even_basis, has_order_p_plus_one};
use prism_verify::{basis_hint, ChallengeScalar, KeyIdentity};

/// One row of the file, at this set.
struct Curve<L: ParameterSet> {
    number: CurveNumber,
    coefficient: L::Ext,
    key: Vec<u8>,
    index: u16,
    coefficient_is_square: bool,
    /// Abscissas of points of order `p + 1`, `(p + 1)/q` for the
    /// cofactor's odd prime `q`, `(p + 1)/2`, and of order the cofactor.
    full: L::Ext,
    without_odd_prime: L::Ext,
    without_a_two: L::Ext,
    order_odd_prime: L::Ext,
    /// The basis the index names, as full points, and its pairing.
    p: Jac<L::Ext>,
    q: Jac<L::Ext>,
    pairing: L::Ext,
}

fn curves<L: WithoutCorpus>() -> Vec<Curve<L>> {
    let point = |point: &Point| {
        let (x, y) = point.decode::<L>();
        Jac {
            x,
            y,
            z: <L::Ext as Field>::one(),
        }
    };
    curve_table::<L>()
        .rows()
        .iter()
        .map(|row| Curve {
            number: row.number,
            coefficient: row.coefficient.decode::<L>(),
            key: row.coefficient.bytes().to_vec(),
            index: row.index,
            coefficient_is_square: row.coefficient_is_square,
            full: row.order_full_x.decode::<L>(),
            without_odd_prime: row.order_without_odd_prime_x.decode::<L>(),
            without_a_two: row.order_without_two_x.decode::<L>(),
            order_odd_prime: row.order_odd_prime_x.decode::<L>(),
            p: point(&row.p),
            q: point(&row.q),
            pairing: row.pairing.decode::<L>(),
        })
        .collect()
}

/// How many messages `scripts/challenge_scalar.py` hashes under each key.
const MESSAGES_PER_KEY: usize = 2;

fn hex(text: &str) -> Vec<u8> {
    prism_fixtures::hex::decode(text).expect("hex")
}

fn same<F: Field>(left: F, right: F) -> bool {
    Field::same(left, right)
}

/// A point of order `p + 1` is full order. One missing the cofactor's odd
/// prime, one missing one factor of 2, and one of order the cofactor are
/// not. The orders are
/// Sage's. This is the check the cofactor's odd primes feed: with an empty
/// list of primes the second point would have passed.
fn a_point_is_full_order_only_when_sage_says_its_order_is_p_plus_one<L: WithoutCorpus>() {
    for curve in curves::<L>() {
        let order = |x: L::Ext| has_order_p_plus_one::<L>(curve.coefficient, Xz::affine(x));
        assert!(order(curve.full), "curve {}: order p + 1", curve.number);
        assert!(
            !order(curve.without_odd_prime),
            "curve {}: a point missing the cofactor's prime passed for full order",
            curve.number
        );
        assert!(
            !order(curve.without_a_two),
            "curve {}: order (p + 1)/2 passed for full order",
            curve.number
        );
        assert!(
            !order(curve.order_odd_prime),
            "curve {}: a point of order the cofactor passed for full order",
            curve.number
        );
    }
}

/// The verifier's index search names the index Sage's copy of the search
/// names, on both branches.
fn the_basis_index_is_the_one_the_search_in_sage_finds<L: WithoutCorpus>() {
    let mut squares = 0;
    for curve in curves::<L>() {
        let hint = basis_hint::<L>(curve.coefficient).expect("an index inside the bound");
        assert_eq!(
            (hint.index(), hint.coefficient_is_square()),
            (curve.index, curve.coefficient_is_square),
            "curve {}",
            curve.number
        );
        assert!(hint.index() >= 1 && hint.index() <= L::SEARCH_BOUND);
        squares += usize::from(curve.coefficient_is_square);
    }
    // Both branches of the search are exercised by the file.
    assert!(
        squares > 0 && squares < L::FIXTURES.curves,
        "{squares} of {} on the square branch",
        L::FIXTURES.curves
    );
}

/// `even_basis` gives the two points Sage built from that index, after the
/// cofactor is cleared. Sage confirmed both have order `2^e`.
fn the_even_basis_is_the_pair_of_order_two_to_the_e_that_sage_built<L: WithoutCorpus>() {
    for curve in curves::<L>() {
        let basis = even_basis::<L>(curve.coefficient, curve.index, curve.coefficient_is_square)
            .expect("a basis");
        let p = basis.p.x_affine().expect("P is not the point at infinity");
        let q = basis.q.x_affine().expect("Q is not the point at infinity");
        assert!(same(p, curve.p.x), "curve {}: x(P)", curve.number);
        assert!(same(q, curve.q.x), "curve {}: x(Q)", curve.number);
    }
}

/// The Weil pairing of that basis is Sage's value, a primitive `2^e`-th
/// root of unity.
fn the_weil_pairing_of_the_basis_is_the_value_sage_computed<L: WithoutCorpus>() {
    for curve in curves::<L>() {
        let montgomery = Montgomery::normalized(curve.coefficient);
        let value = weil(montgomery, curve.p, curve.q, L::E).expect("a pairing value");
        assert!(same(value, curve.pairing), "curve {}", curve.number);
        // Order exactly 2^e: squaring e − 1 times gives −1.
        let mut power = value;
        for _ in 0..L::E - 1 {
            power = Field::square(power);
        }
        assert!(
            same(power, Field::negate(<L::Ext as Field>::one())),
            "curve {}: the value is a primitive root",
            curve.number
        );
        // Swapping the arguments inverts the value.
        let swapped = weil(montgomery, curve.q, curve.p, L::E).expect("a pairing value");
        assert!(
            same(Field::mul(value, swapped), <L::Ext as Field>::one()),
            "curve {}: e(Q, P) = 1/e(P, Q)",
            curve.number
        );
    }
}

/// Each coefficient is a public key the decoder takes.
fn every_curve_of_the_file_decodes_as_a_public_key<L: WithoutCorpus>() {
    for curve in curves::<L>() {
        assert_eq!(curve.key.len(), L::PUBLIC_KEY_LEN);
        let identity = KeyIdentity::<L>::decode(&curve.key).expect("a key");
        assert!(same(identity.coefficient(), curve.coefficient));
    }
}

/// The challenge scalar for fixed keys, salts and messages is the one an
/// independent script computes. `scripts/challenge_scalar.py` has its own
/// Keccak and its own cSHAKE framing, and before it writes this file it
/// reproduces the `s` of all 113 committed vectors at the other sets.
fn the_challenge_scalar_is_the_one_an_independent_script_computes<L: WithoutCorpus>(
    challenge_bits: u32,
) {
    let table = L::FIXTURES.challenge_table;
    let mut rows = 0;
    for line in table.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 4);
        let key = hex(fields[0]);
        let salt = <L as ParameterSet>::Salt::from_bytes(&hex(fields[1])).expect("a salt");
        let message = if fields[2] == "-" {
            Vec::new()
        } else {
            hex(fields[2])
        };
        let identity = KeyIdentity::<L>::decode(&key).expect("a key");
        let scalar = ChallengeScalar::derive(&identity, &message, salt);
        let bytes = scalar.as_bytes();
        assert_eq!(bytes, hex(fields[3]), "row {rows}");
        // The width the set's test module states, in whole bytes with the
        // spare bits of the last one clear, and odd.
        assert_eq!(L::CHALLENGE_BITS, challenge_bits);
        let length = challenge_bits.div_ceil(u8::BITS) as usize;
        assert_eq!(bytes.len(), length);
        assert_eq!(bytes[0] & 1, 1, "row {rows}: odd");
        let spare = u8::BITS * length as u32 - challenge_bits;
        let top = *bytes.last().expect("a scalar is not empty");
        assert_eq!(
            u32::from(top) >> (u8::BITS - spare),
            0,
            "row {rows}: no bit above the width"
        );
        rows += 1;
    }
    assert_eq!(
        rows,
        MESSAGES_PER_KEY * L::FIXTURES.curves,
        "every curve as a key, each under the same number of messages"
    );
}

/// Every check of this file at one set. `challenge_bits` is stated here
/// and compared with the set's own constant: `e − 3`.
macro_rules! at_set {
    ($module:ident, $set:ty, challenge_bits = $challenge_bits:literal) => {
        mod $module {
            #[test]
            fn a_point_is_full_order_only_when_sage_says_its_order_is_p_plus_one() {
                super::a_point_is_full_order_only_when_sage_says_its_order_is_p_plus_one::<$set>();
            }

            #[test]
            fn the_basis_index_is_the_one_the_search_in_sage_finds() {
                super::the_basis_index_is_the_one_the_search_in_sage_finds::<$set>();
            }

            #[test]
            fn the_even_basis_is_the_pair_of_order_two_to_the_e_that_sage_built() {
                super::the_even_basis_is_the_pair_of_order_two_to_the_e_that_sage_built::<$set>();
            }

            #[test]
            fn the_weil_pairing_of_the_basis_is_the_value_sage_computed() {
                super::the_weil_pairing_of_the_basis_is_the_value_sage_computed::<$set>();
            }

            #[test]
            fn every_curve_of_the_file_decodes_as_a_public_key() {
                super::every_curve_of_the_file_decodes_as_a_public_key::<$set>();
            }

            #[test]
            fn the_challenge_scalar_is_the_one_an_independent_script_computes() {
                super::the_challenge_scalar_is_the_one_an_independent_script_computes::<$set>(
                    $challenge_bits,
                );
            }
        }
    };
}

at_set!(p324, prism_field::P324, challenge_bits = 321);
