// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Unbounded arithmetic on public fixture columns in raw O0 numerator units.

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use shekyl_prism::hazmat::int::{Int, SecretInt};

pub(crate) type Column = [BigInt; 4];

pub(crate) fn subtract_multiple(column: &mut Column, q: &BigInt, pivot: &Column) {
    for (x, y) in column.iter_mut().zip(pivot) {
        *x -= q * y;
    }
}

/// Full-rank column HNF. Each pivot clears its row in the remaining
/// generators. Later columns have canonical residues above their pivots.
pub(crate) fn column_hnf(mut generators: Vec<Column>) -> Vec<Column> {
    let mut basis: Vec<Column> = Vec::new();
    for row in (0..4).rev() {
        let mut pivot = generators.pop().unwrap();
        for col in &mut generators {
            while col[row] != BigInt::ZERO {
                let q = &pivot[row] / &col[row];
                subtract_multiple(&mut pivot, &q, col);
                std::mem::swap(&mut pivot, col);
            }
        }
        assert_ne!(pivot[row], BigInt::ZERO, "rank-deficient generators");
        if pivot[row].sign() == Sign::Minus {
            pivot = pivot.map(|x| -x);
        }
        for col in &mut basis {
            let q = col[row].div_floor(&pivot[row]);
            subtract_multiple(col, &q, &pivot);
        }
        basis.push(pivot);
    }
    basis.reverse();
    basis
}

pub(crate) fn gaussian_columns(u: Column, v: Column) -> [Column; 4] {
    let rotate = |[a, b, c, d]: &Column| [-b, a.clone(), -d, c.clone()];
    let iu = rotate(&u);
    let iv = rotate(&v);
    [u, iu, v, iv]
}

pub(crate) fn public_fixture<const W: usize>(value: &SecretInt<W>) -> BigInt {
    let x: Int<W> = value.declassify().expect("marked public fixture");
    let mut bytes = vec![0; W * 8];
    x.to_le_bytes(&mut bytes).unwrap();
    let sign = if x.is_negative() {
        Sign::Minus
    } else {
        Sign::Plus
    };
    BigInt::from_bytes_le(sign, &bytes)
}
