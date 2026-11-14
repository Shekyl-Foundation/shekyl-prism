// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// Splitting and the product-to-Montgomery maps are a port of
// `splitting_compute`, `theta_product_structure_to_elliptic_product`, and
// `theta_point_to_montgomery_point` in
// `external/PRISM_v2/src/hd/ref/lvlx/theta_isogenies.c`, with the matrix
// table from `src/precomp/ref/lvl1/hd_splitting_transforms.c` (Apache-2.0;
// Antonin Leroux and the SQIsign authors). See `NOTICE`.
//
// The ten matrices and the two index tables are identical at every NIST
// level in that tree. Verification calls splitting with `randomize` false,
// so the six signing-only normalisation matrices are not in this port.

//! Identify a theta codomain with an elliptic product.
//!
//! A theta null point splits when exactly one of the ten even theta
//! structures makes a linear combination of its coordinates vanish. The
//! matrix of that structure is the isomorphism the comparison in Algorithm 3
//! reads through. The verification chain also requires that structure to be
//! index 8, the value the pinned verifier passes as `zero_index`.

use prism_field::Field;

use crate::montgomery::Xz;
use crate::theta::{is_product, ThetaPoint};
use crate::theta_step::{apply_isomorphism, BasisChange};

/// `(a : b)` for a Montgomery coefficient, the pair `(A, C)` the C curve stores.
#[derive(Clone, Copy, Debug)]
pub struct ProjectiveCoefficient<F> {
    pub a: F,
    pub c: F,
}

/// The two Montgomery curves a split theta null point determines.
#[derive(Clone, Copy, Debug)]
pub struct EllipticProduct<F> {
    pub first: ProjectiveCoefficient<F>,
    pub second: ProjectiveCoefficient<F>,
}

/// A point of `E1 × E2` in x-only coordinates.
#[derive(Clone, Copy, Debug)]
pub struct CouplePoint<F> {
    pub first: Xz<F>,
    pub second: Xz<F>,
}

/// The splitting isomorphism and the null point written in its coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Splitting<F> {
    /// Which of the ten even structures vanished. Verification expects 8.
    pub index: usize,
    pub matrix: BasisChange<F>,
    pub null_point: ThetaPoint<F>,
}

/// `0, 1, i, −1, −i`, the codes in `FP2_CONSTANTS`.
const ZERO: u8 = 0;
const ONE: u8 = 1;
const I: u8 = 2;
const MINUS_ONE: u8 = 3;
const MINUS_I: u8 = 4;

/// Copied from `SPLITTING_TRANSFORMS`. Each entry is a row.
const SPLITTING_TRANSFORMS: [[[u8; 4]; 4]; 10] = [
    [
        [ONE, I, ONE, I],
        [ONE, MINUS_I, MINUS_ONE, I],
        [ONE, I, MINUS_ONE, MINUS_I],
        [MINUS_ONE, I, MINUS_ONE, I],
    ],
    [
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ZERO, ZERO, ONE],
        [ZERO, ZERO, ONE, ZERO],
        [ZERO, MINUS_ONE, ZERO, ZERO],
    ],
    [
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, ZERO, ONE],
        [ZERO, ZERO, MINUS_ONE, ZERO],
    ],
    [
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, ONE, ZERO],
        [ZERO, ZERO, ZERO, MINUS_ONE],
    ],
    [
        [ONE, ONE, ONE, ONE],
        [ONE, MINUS_ONE, MINUS_ONE, ONE],
        [ONE, ONE, MINUS_ONE, MINUS_ONE],
        [MINUS_ONE, ONE, MINUS_ONE, ONE],
    ],
    [
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, ZERO, ONE],
        [ZERO, ZERO, ONE, ZERO],
    ],
    [
        [ONE, ONE, ONE, ONE],
        [ONE, MINUS_ONE, ONE, MINUS_ONE],
        [ONE, MINUS_ONE, MINUS_ONE, ONE],
        [MINUS_ONE, MINUS_ONE, ONE, ONE],
    ],
    [
        [ONE, ONE, ONE, ONE],
        [ONE, MINUS_ONE, ONE, MINUS_ONE],
        [ONE, MINUS_ONE, MINUS_ONE, ONE],
        [ONE, ONE, MINUS_ONE, MINUS_ONE],
    ],
    [
        [ONE, ONE, ONE, ONE],
        [ONE, MINUS_ONE, ONE, MINUS_ONE],
        [ONE, ONE, MINUS_ONE, MINUS_ONE],
        [MINUS_ONE, ONE, ONE, MINUS_ONE],
    ],
    [
        [ONE, ZERO, ZERO, ZERO],
        [ZERO, ONE, ZERO, ZERO],
        [ZERO, ZERO, ONE, ZERO],
        [ZERO, ZERO, ZERO, ONE],
    ],
];

/// `(character index, coordinate xor)` from `EVEN_INDEX`.
const EVEN_INDEX: [[usize; 2]; 10] = [
    [0, 0],
    [0, 1],
    [0, 2],
    [0, 3],
    [1, 0],
    [1, 2],
    [2, 0],
    [2, 1],
    [3, 0],
    [3, 3],
];

/// `CHI_EVAL`. `+1` keeps the product, `−1` negates it.
const CHI_EVAL: [[i8; 4]; 4] = [[1, 1, 1, 1], [1, -1, 1, -1], [1, 1, -1, -1], [1, -1, -1, 1]];

fn matrix_entry<F: Field>(code: u8, i: F) -> F {
    match code {
        ZERO => F::zero(),
        ONE => F::one(),
        I => i,
        MINUS_ONE => F::one().negate(),
        MINUS_I => i.negate(),
        _ => panic!("splitting table code {code} is not one of the five constants"),
    }
}

fn splitting_matrix<F: Field>(index: usize, i: F) -> BasisChange<F> {
    let mut rows = [[F::zero(); 4]; 4];
    for (row, codes) in rows.iter_mut().zip(SPLITTING_TRANSFORMS[index]) {
        for (entry, code) in row.iter_mut().zip(codes) {
            *entry = matrix_entry(code, i);
        }
    }
    BasisChange { rows }
}

/// Split `null_point`. `i` is the imaginary unit of `F_{p^2}`.
///
/// `expected` is the C `zero_index`: verification passes 8, and a null point
/// whose vanishing structure is some other index is a refusal even when
/// exactly one structure vanishes. `None` accepts whichever single index
/// vanishes.
pub fn splitting_compute<F: Field>(
    null_point: ThetaPoint<F>,
    i: F,
    expected: Option<usize>,
) -> Option<Splitting<F>> {
    let mut found: Option<usize> = None;
    for (index, even) in EVEN_INDEX.iter().enumerate() {
        let mut sum = F::zero();
        let character = even[0];
        let shift = even[1];
        for (coordinate, sign) in CHI_EVAL[character].iter().enumerate() {
            let mut term = null_point
                .coord(coordinate ^ shift)
                .mul(null_point.coord(coordinate));
            if *sign < 0 {
                term = term.negate();
            }
            sum = sum.add(term);
        }
        if sum.is_zero() {
            if found.is_some() {
                return None;
            }
            found = Some(index);
        } else if expected == Some(index) {
            return None;
        }
    }
    let index = found?;
    if expected.is_some_and(|expected| expected != index) {
        return None;
    }
    let matrix = splitting_matrix(index, i);
    let null_point = apply_isomorphism(&matrix, null_point, true);
    Some(Splitting {
        index,
        matrix,
        null_point,
    })
}

fn fourth_ratio<F: Field>(x: F, y: F) -> Option<ProjectiveCoefficient<F>> {
    let x4 = x.square().square();
    let y4 = y.square().square();
    let sum = x4.add(y4);
    let coefficient = ProjectiveCoefficient {
        a: sum.add(sum).negate(),
        c: x4.sub(y4),
    };
    if coefficient.c.is_zero() {
        None
    } else {
        Some(coefficient)
    }
}

/// The Montgomery product of a theta null point that is already a product
/// point (`x t = y z`), after splitting's isomorphism.
pub fn elliptic_product<F: Field>(null_point: ThetaPoint<F>) -> Option<EllipticProduct<F>> {
    if !is_product(null_point)
        || null_point.x.is_zero()
        || null_point.y.is_zero()
        || null_point.z.is_zero()
    {
        return None;
    }
    Some(EllipticProduct {
        first: fourth_ratio(null_point.x, null_point.z)?,
        second: fourth_ratio(null_point.x, null_point.y)?,
    })
}

/// `(P1, P2)` on that product. A point that is not itself a product point,
/// or the zero point, is a refusal.
pub fn montgomery_couple<F: Field>(
    structure: ThetaPoint<F>,
    point: ThetaPoint<F>,
) -> Option<CouplePoint<F>> {
    if !is_product(point) {
        return None;
    }
    let (second_x, second_z) = if point.x.is_zero() && point.y.is_zero() {
        (point.z, point.t)
    } else {
        (point.x, point.y)
    };
    if second_x.is_zero() && second_z.is_zero() {
        return None;
    }
    let (first_x, first_z) = if point.x.is_zero() && point.z.is_zero() {
        (point.y, point.t)
    } else {
        (point.x, point.z)
    };
    let second = couple_coordinate(structure.y, structure.x, second_x, second_z);
    let first = couple_coordinate(structure.z, structure.x, first_x, first_z);
    Some(CouplePoint { first, second })
}

/// `X = n·u + m·v`, `Z = −n·u + m·v`.
fn couple_coordinate<F: Field>(null_n: F, null_m: F, point_u: F, point_v: F) -> Xz<F> {
    let nu = null_n.mul(point_u);
    let mv = null_m.mul(point_v);
    Xz {
        x: nu.add(mv),
        z: mv.sub(nu),
    }
}

#[cfg(test)]
mod tests {
    use super::{elliptic_product, montgomery_couple, splitting_compute};
    use crate::theta::ThetaPoint;
    use prism_field::{Field, Level, Level1};

    type Ext = <Level1 as Level>::Ext;

    fn real(value: u64) -> Ext {
        Level1::ext_from_base(<Level1 as Level>::Base::from_u64(value))
    }

    #[test]
    fn a_product_null_point_splits_at_the_identity_and_recovers_its_curves() {
        // xt = yz, and the identity is the only even structure that vanishes.
        let null_point = ThetaPoint {
            x: real(1),
            y: real(2),
            z: real(3),
            t: real(6),
        };
        let splitting =
            splitting_compute(null_point, Level1::ext_i(), None).expect("one structure");
        assert_eq!(splitting.index, 9);
        let product = elliptic_product(splitting.null_point).expect("product");
        // A/C = −2(x^4 + y^4)/(x^4 − y^4), read on the split null point.
        let x = splitting.null_point.x;
        let y = splitting.null_point.y;
        let x4 = x.square().square();
        let y4 = y.square().square();
        let sum = x4.add(y4);
        let a = sum.add(sum).negate();
        let c = x4.sub(y4);
        assert!(product.second.a.mul(c).same(a.mul(product.second.c)));

        let image = montgomery_couple(splitting.null_point, splitting.null_point).expect("point");
        assert!(!image.first.z.is_zero() || !image.first.x.is_zero());
    }

    #[test]
    fn a_point_that_is_not_a_product_does_not_split_uniquely() {
        let null_point = ThetaPoint {
            x: real(1),
            y: real(2),
            z: real(3),
            t: real(4),
        };
        assert!(splitting_compute(null_point, Level1::ext_i(), None).is_none());
    }

    #[test]
    fn verification_requires_structure_eight() {
        let null_point = ThetaPoint {
            x: real(1),
            y: real(2),
            z: real(3),
            t: real(6),
        };
        assert!(splitting_compute(null_point, Level1::ext_i(), Some(8)).is_none());
    }
}
