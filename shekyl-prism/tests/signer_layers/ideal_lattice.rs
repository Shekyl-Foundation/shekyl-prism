// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Public, unbounded Z-lattice oracle for the specialized signing intersection.
//! The Euclidean column operations preserve the generated lattice. Adding
//! modulus columns here could hide an erroneous proper sublattice.

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use shekyl_prism::hazmat::field::{P248, P324, P376, P500, P664};
use shekyl_prism::hazmat::ideal::{geometry::degree_bits, GenericHnf, GuardedIdeal, IdealSet};
use shekyl_prism::hazmat::int::SecretInt;

#[path = "../support/lattice.rs"]
mod lattice;
use lattice::{column_hnf, gaussian_columns, public_fixture, subtract_multiple};

fn scalar<const W: usize>(value: &BigInt) -> SecretInt<W> {
    let (sign, bytes) = value.to_bytes_le();
    let x = SecretInt::from_le_bytes(&bytes);
    if sign == Sign::Minus {
        x.neg()
    } else {
        x
    }
}

fn compare_intersections<S: IdealSet<W, D>, const W: usize, const D: usize>(corpus: &str) {
    // Cross the six keys with all six challenges, including the 30 pairings
    // per set that were never used to derive the committed intersections.
    let cases: Vec<[BigInt; 14]> = corpus
        .lines()
        .map(|line| {
            let mut fields = line
                .split_whitespace()
                .map(|s| BigInt::parse_bytes(s.as_bytes(), 16).unwrap());
            let row = std::array::from_fn(|_| fields.next().unwrap());
            assert!(fields.next().is_none());
            row
        })
        .collect();
    assert_eq!(cases.len(), 6);
    for [n, xn, yn, a, b, c, d, e, f, g, h, ..] in &cases {
        let key_root = [scalar(xn), scalar(yn)];
        let u = [a, b, c, d].map(Clone::clone);
        let v = [e, f, g, h].map(Clone::clone);
        let hnf = GenericHnf::from_norm_and_root(&scalar(n), &key_root);
        let key = GuardedIdeal::<S, W, D>::admit(
            scalar(n),
            hnf.columns(),
            u.each_ref().map(scalar),
            v.each_ref().map(scalar),
        )
        .unwrap();
        let key_columns = gaussian_columns(u, v);
        for [.., s, x, y] in &cases {
            let mut challenge = s.to_bytes_le().1;
            assert!(challenge.len() <= S::S_LEN);
            challenge.resize(S::S_LEN, 0);
            let output = key
                .intersection(&challenge, &[scalar(x), scalar(y)])
                .unwrap();

            let q = (BigInt::from(1) << degree_bits(S::E)) - s * s;
            assert_eq!(n.gcd(&q), BigInt::from(1));
            assert_eq!(public_fixture(&output.norm), n * &q);

            // Q3: q times each key column and N times each challenge column.
            // The challenge lattice is generated over Z[i] by 2q and x+yi+j.
            let zero = BigInt::ZERO;
            let challenge_columns = gaussian_columns(
                [&q * 2, zero.clone(), zero.clone(), zero.clone()],
                [x.clone(), y.clone(), BigInt::from(1), zero],
            );
            let generators = key_columns
                .iter()
                .map(|c| c.each_ref().map(|x| x * &q))
                .chain(challenge_columns.into_iter().map(|c| c.map(|x| x * n)))
                .collect();
            let expected = column_hnf(generators);
            let actual = gaussian_columns(
                output.u.each_ref().map(public_fixture),
                output.v.each_ref().map(public_fixture),
            );
            assert_eq!(column_hnf(actual.to_vec()), expected);
        }
    }
}

#[test]
fn guarded_intersections_match_the_coprime_lattice_sum() {
    macro_rules! compare {
        ($set:ty, $file:literal) => {
            compare_intersections::<$set, _, _>(include_str!(concat!(
                "../../examples/stage2c/ideals/",
                $file
            )));
        };
    }
    compare!(P248, "p248.txt");
    compare!(P324, "p324.txt");
    compare!(P376, "p376.txt");
    compare!(P500, "p500.txt");
    compare!(P664, "p664.txt");
}

#[test]
fn canonical_form_distinguishes_span_from_index() {
    let basis =
        [[2, 0, 0, 0], [1, 3, 0, 0], [0, 2, 1, 0], [1, 0, 0, 1]].map(|c| c.map(BigInt::from));
    let mut generators = basis.to_vec();
    subtract_multiple(&mut generators[0], &BigInt::from(7), &basis[3]);
    generators.reverse();
    generators.push(basis[2].clone().map(|x| -x));
    assert_eq!(column_hnf(generators), basis);
    let mut other = basis.clone();
    other[3][0] = BigInt::ZERO;
    assert_ne!(column_hnf(other.to_vec()), basis);
    other = basis.clone();
    other[0] = other[0].clone().map(|x| x * 2);
    assert_ne!(column_hnf(other.to_vec()), basis);
}

#[test]
fn comaximal_ideals_can_have_equal_norms() {
    // At p=3 both 5i+j and 9i+j define roots of norm-7 ideals.
    // For the second residue map the first root maps to -4i, a unit mod 7.
    let ideal = |y| {
        gaussian_columns(
            [14, 0, 0, 0].map(BigInt::from),
            [0, y, 1, 0].map(BigInt::from),
        )
    };
    for y in [5, 9] {
        assert_eq!((y * y + 3) % 28, 0);
    }
    let sum = ideal(5).into_iter().chain(ideal(9)).collect();
    let order =
        [[2, 0, 0, 0], [0, 2, 0, 0], [0, 1, 1, 0], [1, 0, 0, 1]].map(|c| c.map(BigInt::from));
    assert_eq!(column_hnf(sum), order);
}
