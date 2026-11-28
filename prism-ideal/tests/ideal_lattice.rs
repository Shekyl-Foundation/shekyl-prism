// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Public, unbounded Z-lattice oracle for the specialized signing intersection.
//! The Euclidean column operations preserve the generated lattice. Adding
//! modulus columns here could hide an erroneous proper sublattice.

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use prism_field::{P248, P324, P376, P500, P664};
use prism_ideal::{geometry::degree_bits, GenericHnf, GuardedIdeal, IdealSet};
use prism_int::SecretInt;

/// One row of `examples/stage2c/admission-boundaries.txt`.
///
/// `scripts/stage2d_fixtures.py` derives it from the first ideal of the set:
/// the first shear past the factor-256 guard, and an odd byte whose
/// challenge degree is composite and divisible by `degree_factor`.
struct AdmissionBoundary<'a> {
    set_name: &'a str,
    first_failing_shear: u64,
    composite_scalar: u8,
    degree_factor: u32,
}

fn admission_boundary(line: &str) -> AdmissionBoundary<'_> {
    let mut fields = line.split_whitespace();
    let set_name = fields.next().expect("admission boundary set");
    let first_failing_shear = fields
        .next()
        .expect("admission boundary shear")
        .parse()
        .expect("admission boundary shear");
    let composite_scalar = fields
        .next()
        .expect("admission boundary scalar")
        .parse()
        .expect("admission boundary scalar");
    let degree_factor = fields
        .next()
        .expect("admission boundary factor")
        .parse()
        .expect("admission boundary factor");
    assert!(
        fields.next().is_none(),
        "admission boundary has trailing fields"
    );
    assert!(
        first_failing_shear > 0,
        "the failing shear has no predecessor"
    );
    assert!(degree_factor > 1, "a degree factor divides the degree");
    AdmissionBoundary {
        set_name,
        first_failing_shear,
        composite_scalar,
        degree_factor,
    }
}

/// `2^{degree_bits(e)} − s²`, the degree the verifier tests for primality.
fn public_degree(exponent: u32, scalar: &BigInt) -> BigInt {
    (BigInt::from(1) << degree_bits(exponent)) - scalar * scalar
}

#[path = "support/lattice.rs"]
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

/// Admission refusals on the first ideal: two non-canonical HNF columns,
/// the shears on either side of the factor-256 guard, and a composite degree
/// whose witness is `boundary.degree_factor`.
fn probe_admission_boundary<S: IdealSet<W, D>, const W: usize, const D: usize>(
    key: &GuardedIdeal<S, W, D>,
    hnf: &GenericHnf<W>,
    norm: &BigInt,
    u: &[BigInt; 4],
    v: &[BigInt; 4],
    challenge_root: [&BigInt; 2],
    boundary: &AdmissionBoundary<'_>,
) {
    let degree = public_degree(S::E, &BigInt::from(boundary.composite_scalar));
    let factor = BigInt::from(boundary.degree_factor);
    assert!(degree > factor, "{}", S::NAME);
    assert_eq!(&degree % &factor, BigInt::ZERO, "{}", S::NAME);
    let mut challenge = vec![0; S::S_LEN];
    challenge[0] = boundary.composite_scalar;
    assert!(
        key.intersection(&challenge, &challenge_root.map(scalar))
            .is_none(),
        "{}",
        S::NAME
    );

    let basis_u = u.each_ref().map(scalar);
    let basis_v = v.each_ref().map(scalar);
    let admit = |columns: &[_; 4], candidate_v| {
        GuardedIdeal::<S, W, D>::admit(scalar(norm), columns, basis_u.clone(), candidate_v)
    };
    // Each edit is unimodular on the ideal and breaks one HNF inequality.
    // The pivot column is first. The root column is third.
    let mut columns = hnf.columns().clone();
    columns[0][1] = columns[1][1].clone();
    assert!(admit(&columns, basis_v.clone()).is_none(), "{}", S::NAME);
    let mut columns = hnf.columns().clone();
    columns[2][0] = columns[2][0].add(hnf.pivot());
    assert!(admit(&columns, basis_v).is_none(), "{}", S::NAME);

    let sheared = |steps: u64| {
        let steps = BigInt::from(steps);
        std::array::from_fn(|coordinate| scalar(&(&v[coordinate] + &u[coordinate] * &steps)))
    };
    let admitted = boundary.first_failing_shear - 1;
    assert!(
        admit(hnf.columns(), sheared(admitted)).is_some(),
        "{}",
        S::NAME
    );
    assert!(
        admit(hnf.columns(), sheared(boundary.first_failing_shear)).is_none(),
        "{}",
        S::NAME
    );
}

fn compare_intersections<S: IdealSet<W, D>, const W: usize, const D: usize>(
    corpus: &str,
    boundary_row: &str,
) {
    let boundary = admission_boundary(boundary_row);
    assert_eq!(boundary.set_name, S::NAME);
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
    for (index, [n, xn, yn, a, b, c, d, e, f, g, h, _s, xq, yq]) in cases.iter().enumerate() {
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
        if index == 0 {
            probe_admission_boundary(&key, &hnf, n, &u, &v, [xq, yq], &boundary);
        }
        let key_columns = gaussian_columns(u, v);
        for [.., s, x, y] in &cases {
            let mut challenge = s.to_bytes_le().1;
            assert!(challenge.len() <= S::S_LEN);
            challenge.resize(S::S_LEN, 0);
            let output = key
                .intersection(&challenge, &[scalar(x), scalar(y)])
                .unwrap();

            let q = public_degree(S::E, s);
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
    let mut boundaries = include_str!("../examples/stage2c/admission-boundaries.txt").lines();
    macro_rules! compare {
        ($set:ty, $file:literal) => {
            compare_intersections::<$set, _, _>(
                include_str!(concat!("../examples/stage2c/ideals/", $file)),
                boundaries.next().expect("admission boundary row"),
            );
        };
    }
    compare!(P248, "p248.txt");
    compare!(P324, "p324.txt");
    compare!(P376, "p376.txt");
    compare!(P500, "p500.txt");
    compare!(P664, "p664.txt");
    assert!(boundaries.next().is_none());
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
