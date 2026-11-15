// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Curves, point orders, the torsion basis and the Weil pairing at `p324`,
//! against Sage.
//!
//! `testdata/p324_curves.tsv` is written by `scripts/sage/p324_fixture.sage`.
//! Each row is one supersingular Montgomery curve on a 2-isogeny walk from
//! `A = 0`, with points whose orders Sage computed, the basis the
//! verifier's index search names, and the pairing of that basis.
//!
//! There is no signer at this prime, so there is no signature here. These
//! tests reach everything the verifier does with a public key and with
//! points, and nothing that needs an accepted response.

use prism_field::{Field, ParameterSet, Salt, P324};
use prism_isogeny::jac::Jac;
use prism_isogeny::montgomery::{Montgomery, Xz};
use prism_isogeny::pairing::weil;
use prism_isogeny::torsion::{even_basis, has_order_p_plus_one};
use prism_verify::{basis_hint, ChallengeScalar, KeyIdentity};

type Set = P324;
type Ext = <Set as ParameterSet>::Ext;

const TABLE: &str = include_str!("../testdata/p324_curves.tsv");

fn hex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "odd hex length");
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

fn element(text: &str) -> Ext {
    Set::ext_decode(&hex(text)).expect("a canonical element")
}

/// One row of the file.
struct Curve {
    number: String,
    coefficient: Ext,
    key: Vec<u8>,
    index: u16,
    coefficient_is_square: bool,
    /// Abscissas of points of order `p + 1`, `(p + 1)/3`, `(p + 1)/2`, `3`.
    full: Ext,
    without_three: Ext,
    without_a_two: Ext,
    order_three: Ext,
    /// The basis the index names, as full points, and its pairing.
    p: Jac<Ext>,
    q: Jac<Ext>,
    pairing: Ext,
}

fn curves() -> Vec<Curve> {
    let found: Vec<Curve> = TABLE
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 14, "a CURVE row has fourteen fields");
            assert_eq!(fields[0], "CURVE");
            let point = |x: &str, y: &str| Jac {
                x: element(x),
                y: element(y),
                z: <Ext as Field>::one(),
            };
            Curve {
                number: fields[1].to_string(),
                coefficient: element(fields[2]),
                key: hex(fields[2]),
                index: fields[3].parse().expect("an index"),
                coefficient_is_square: match fields[4] {
                    "1" => true,
                    "0" => false,
                    other => panic!("square flag {other}"),
                },
                full: element(fields[5]),
                without_three: element(fields[6]),
                without_a_two: element(fields[7]),
                order_three: element(fields[8]),
                p: point(fields[9], fields[10]),
                q: point(fields[11], fields[12]),
                pairing: element(fields[13]),
            }
        })
        .collect();
    assert_eq!(found.len(), 8, "eight curves");
    found
}

fn same(left: Ext, right: Ext) -> bool {
    Field::same(left, right)
}

/// A point of order `p + 1` is full order. One missing the factor 3, one
/// missing one factor of 2, and one of order 3 are not. The orders are
/// Sage's. This is the check the cofactor's odd primes feed: with an empty
/// list of primes the second point would have passed.
#[test]
fn a_point_is_full_order_only_when_sage_says_its_order_is_p_plus_one() {
    for curve in curves() {
        let order = |x: Ext| has_order_p_plus_one::<Set>(curve.coefficient, Xz::affine(x));
        assert!(order(curve.full), "curve {}: order p + 1", curve.number);
        assert!(
            !order(curve.without_three),
            "curve {}: order (p + 1)/3 passed for full order",
            curve.number
        );
        assert!(
            !order(curve.without_a_two),
            "curve {}: order (p + 1)/2 passed for full order",
            curve.number
        );
        assert!(
            !order(curve.order_three),
            "curve {}: order 3 passed for full order",
            curve.number
        );
    }
}

/// The verifier's index search names the index Sage's copy of the search
/// names, on both branches.
#[test]
fn the_basis_index_is_the_one_the_search_in_sage_finds() {
    let mut squares = 0;
    for curve in curves() {
        let hint = basis_hint::<Set>(curve.coefficient).expect("an index inside the bound");
        assert_eq!(
            (hint.index(), hint.coefficient_is_square()),
            (curve.index, curve.coefficient_is_square),
            "curve {}",
            curve.number
        );
        assert!(hint.index() >= 1 && hint.index() <= Set::SEARCH_BOUND);
        squares += usize::from(curve.coefficient_is_square);
    }
    // Both branches of the search are exercised by the file.
    assert!(
        squares > 0 && squares < 8,
        "{squares} of 8 on the square branch"
    );
}

/// `even_basis` gives the two points Sage built from that index, after the
/// cofactor 3 is cleared. Sage confirmed both have order `2^e`.
#[test]
fn the_even_basis_is_the_pair_of_order_two_to_the_e_that_sage_built() {
    for curve in curves() {
        let basis = even_basis::<Set>(curve.coefficient, curve.index, curve.coefficient_is_square)
            .expect("a basis");
        let p = basis.p.x_affine().expect("P is not the point at infinity");
        let q = basis.q.x_affine().expect("Q is not the point at infinity");
        assert!(same(p, curve.p.x), "curve {}: x(P)", curve.number);
        assert!(same(q, curve.q.x), "curve {}: x(Q)", curve.number);
    }
}

/// The Weil pairing of that basis is Sage's value, a primitive `2^e`-th
/// root of unity.
#[test]
fn the_weil_pairing_of_the_basis_is_the_value_sage_computed() {
    for curve in curves() {
        let montgomery = Montgomery::normalized(curve.coefficient);
        let value = weil(montgomery, curve.p, curve.q, Set::E).expect("a pairing value");
        assert!(same(value, curve.pairing), "curve {}", curve.number);
        // Order exactly 2^e: squaring e − 1 times gives −1.
        let mut power = value;
        for _ in 0..Set::E - 1 {
            power = Field::square(power);
        }
        assert!(
            same(power, Field::negate(<Ext as Field>::one())),
            "curve {}: the value is a primitive root",
            curve.number
        );
        // Swapping the arguments inverts the value.
        let swapped = weil(montgomery, curve.q, curve.p, Set::E).expect("a pairing value");
        assert!(
            same(Field::mul(value, swapped), <Ext as Field>::one()),
            "curve {}: e(Q, P) = 1/e(P, Q)",
            curve.number
        );
    }
}

/// Each coefficient is a public key the decoder takes.
#[test]
fn every_curve_of_the_file_decodes_as_a_public_key() {
    for curve in curves() {
        assert_eq!(curve.key.len(), Set::PUBLIC_KEY_LEN);
        let identity = KeyIdentity::<Set>::decode(&curve.key).expect("a key");
        assert!(same(identity.coefficient(), curve.coefficient));
    }
}

/// The challenge scalar for fixed keys, salts and messages is the one an
/// independent script computes. `scripts/challenge_scalar.py` has its own
/// Keccak and its own cSHAKE framing, and before it writes this file it
/// reproduces the `s` of all 113 committed vectors at the other sets.
#[test]
fn the_challenge_scalar_is_the_one_an_independent_script_computes() {
    let table = include_str!("../testdata/p324_challenge.tsv");
    let mut rows = 0;
    for line in table.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 4);
        let key = hex(fields[0]);
        let salt = <Set as ParameterSet>::Salt::from_bytes(&hex(fields[1])).expect("a salt");
        let message = if fields[2] == "-" {
            Vec::new()
        } else {
            hex(fields[2])
        };
        let identity = KeyIdentity::<Set>::decode(&key).expect("a key");
        let scalar = ChallengeScalar::derive(&identity, &message, salt);
        let bytes = scalar.as_bytes();
        assert_eq!(bytes, hex(fields[3]), "row {rows}");
        // 321 bits: 41 bytes with one bit in the last, and odd.
        assert_eq!(Set::CHALLENGE_BITS, 321);
        assert_eq!(bytes.len(), 41);
        assert_eq!(bytes[0] & 1, 1, "row {rows}: odd");
        assert_eq!(bytes[40] >> 1, 0, "row {rows}: 321 bits");
        rows += 1;
    }
    assert_eq!(rows, 16, "eight keys, two messages each");
}
