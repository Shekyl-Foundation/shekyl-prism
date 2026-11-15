// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// The symplectic completion is the dim-2 reduction in
// `external/Theta_dim4` `pkg/basis_change/base_change_dim2.py`
// (`complete_symplectic_matrix_dim2`), Apache-2.0. See `NOTICE`.

//! Integers modulo `2^e` and the 4×4 matrices of the accept path.
//!
//! `e` is the torsion exponent of a parameter set, and a residue is typed
//! by its set: it has the limbs the set names (`ParameterSet::TorsionLimbs`)
//! and is reduced modulo `2^e` for that set's `e`. There is no width to
//! pass and none to get wrong. With `t = s⁻¹` modulo `2^e` and columns in
//! `(b1, b2, b3, b4)`,
//!
//! ```text
//! M1 = [(0, 0, −t, 0), (0, t², 0, 0) | (s, −1, 0, 0), (0, 0, s, s²)]
//! M2 = [(−1, 0, 0, 0), (0, 0, 0, −s) | (0, 0, −1, s), (1, t, 0, 0)]
//! ```
//!
//! The right-hand pair is the kernel half. It is `Φ` of `M1`'s complement
//! for `M2`, and `−Φ̂` of `M2`'s complement for `M1`. `q ≡ −s² (mod 2^e)`,
//! so the entries are determined by `s`.

use prism_field::{Limbs, ParameterSet};

/// An integer modulo `2^e` for the exponent `e` of `L`, little-endian.
#[derive(Clone, Copy, PartialEq)]
pub struct Residue<L: ParameterSet> {
    limb: L::TorsionLimbs,
}

impl<L: ParameterSet> core::fmt::Debug for Residue<L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Residue")
            .field("limb", &self.limb.as_ref())
            .field("bits", &L::E)
            .finish()
    }
}

impl<L: ParameterSet> Residue<L> {
    /// How many limbs the set's residues have.
    const LIMBS: usize = <L::TorsionLimbs as Limbs>::COUNT;

    /// The limbs hold `e` bits. A set whose limbs were too few would
    /// lose the top of every residue; it stops compiling here.
    const HOLDS_THE_EXPONENT: () = assert!(L::E as usize <= Self::LIMBS * u64::BITS as usize);

    /// Reduce a public little-endian scalar modulo `2^e`.
    pub fn from_le_bytes(bytes: &[u8]) -> Self {
        let mut out = Self::zero();
        for (i, byte) in bytes.iter().take(Self::LIMBS * 8).enumerate() {
            out.limb[i / 8] |= u64::from(*byte) << (8 * (i % 8));
        }
        out.mask();
        out
    }

    pub fn to_le_bytes(self) -> Vec<u8> {
        self.limb
            .as_ref()
            .iter()
            .flat_map(|limb| limb.to_le_bytes())
            .take(L::E.div_ceil(8) as usize)
            .collect()
    }

    pub fn zero() -> Self {
        let () = Self::HOLDS_THE_EXPONENT;
        Self {
            limb: <L::TorsionLimbs as Limbs>::ZERO,
        }
    }

    pub fn one() -> Self {
        Self::from_u64(1)
    }

    pub fn from_u64(value: u64) -> Self {
        let mut out = Self::zero();
        out.limb[0] = value;
        out.mask();
        out
    }

    /// Decimal, reduced modulo `2^e`.
    pub fn from_dec(text: &str) -> Self {
        let ten = Self::from_u64(10);
        let mut acc = Self::zero();
        for byte in text.bytes() {
            let digit = Self::from_u64(u64::from(byte - b'0'));
            acc = acc.multiply(ten).add(digit);
        }
        acc
    }

    fn mask(&mut self) {
        let full = (L::E / 64) as usize;
        let rem = L::E % 64;
        let limbs = self.limb.as_mut();
        if rem == 0 {
            for limb in &mut limbs[full..] {
                *limb = 0;
            }
        } else {
            limbs[full] &= (1u64 << rem) - 1;
            for limb in &mut limbs[full + 1..] {
                *limb = 0;
            }
        }
    }

    fn is_odd(self) -> bool {
        self.limb[0] & 1 == 1
    }

    fn add(self, rhs: Self) -> Self {
        let mut out = self;
        let mut carry = 0u128;
        for i in 0..Self::LIMBS {
            let sum = u128::from(out.limb[i]) + u128::from(rhs.limb[i]) + carry;
            out.limb[i] = sum as u64;
            carry = sum >> 64;
        }
        out.mask();
        out
    }

    fn sub(self, rhs: Self) -> Self {
        self.add(rhs.negate())
    }

    pub fn negate(self) -> Self {
        let mut out = self;
        let mut carry = 1u128;
        for limb in out.limb.as_mut() {
            let sum = u128::from(!*limb) + carry;
            *limb = sum as u64;
            carry = sum >> 64;
        }
        out.mask();
        out
    }

    /// The product modulo `2^(64·LIMBS)`, then reduced: the limbs of a
    /// row that would land past the last limb are never formed.
    pub fn multiply(self, rhs: Self) -> Self {
        let mut out = Self::zero();
        for i in 0..Self::LIMBS {
            if self.limb[i] == 0 {
                continue;
            }
            let mut carry = 0u128;
            for j in 0..(Self::LIMBS - i) {
                let sum = u128::from(out.limb[i + j])
                    + u128::from(self.limb[i]) * u128::from(rhs.limb[j])
                    + carry;
                out.limb[i + j] = sum as u64;
                carry = sum >> 64;
            }
        }
        out.mask();
        out
    }

    /// Inverse of an odd residue. `None` when this is even.
    pub fn inv(self) -> Option<Self> {
        if !self.is_odd() {
            return None;
        }
        let two = Self::from_u64(2);
        let mut approx = Self::one();
        // Each step doubles the number of correct bits, from 1 up to `e`.
        let mut correct = 1u32;
        while correct < L::E {
            let product = self.multiply(approx);
            approx = approx.multiply(two.sub(product));
            correct = correct.saturating_mul(2);
        }
        let product = self.multiply(approx);
        Some(approx.multiply(two.sub(product)))
    }
}

/// A 4×4 matrix over [`Residue`], row then column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4<L: ParameterSet> {
    row: [[Residue<L>; 4]; 4],
}

impl<L: ParameterSet> Mat4<L> {
    pub fn rows(self) -> [[Residue<L>; 4]; 4] {
        self.row
    }
    #[cfg(test)]
    fn block(self, rows: [usize; 2], cols: [usize; 2]) -> Block<L> {
        [
            [self.row[rows[0]][cols[0]], self.row[rows[0]][cols[1]]],
            [self.row[rows[1]][cols[0]], self.row[rows[1]][cols[1]]],
        ]
    }
}

#[cfg(test)]
fn transpose2<L: ParameterSet>(block: Block<L>) -> Block<L> {
    [[block[0][0], block[1][0]], [block[0][1], block[1][1]]]
}

#[cfg(test)]
fn mul2<L: ParameterSet>(left: Block<L>, right: Block<L>) -> Block<L> {
    let mut out = [[Residue::zero(); 2]; 2];
    for (out_row, left_row) in out.iter_mut().zip(left) {
        for (right_row, coeff) in right.iter().zip(left_row) {
            for (cell, factor) in out_row.iter_mut().zip(right_row) {
                *cell = cell.add(coeff.multiply(*factor));
            }
        }
    }
    out
}

#[cfg(test)]
fn sub2<L: ParameterSet>(left: Block<L>, right: Block<L>) -> Block<L> {
    [
        [left[0][0].sub(right[0][0]), left[0][1].sub(right[0][1])],
        [left[1][0].sub(right[1][0]), left[1][1].sub(right[1][1])],
    ]
}

/// `BᵀA = AᵀB`, `CᵀD = DᵀC`, and `AᵀD − BᵀC = I`, the dim-2 symplectic
/// identities in `complete_symplectic_matrix_dim2`.
#[cfg(test)]
fn is_symplectic<L: ParameterSet>(matrix: Mat4<L>) -> bool {
    let a = matrix.block([0, 1], [0, 1]);
    let b = matrix.block([2, 3], [0, 1]);
    let c = matrix.block([0, 1], [2, 3]);
    let d = matrix.block([2, 3], [2, 3]);
    let identity = [
        [Residue::one(), Residue::zero()],
        [Residue::zero(), Residue::one()],
    ];
    mul2(transpose2(b), a) == mul2(transpose2(a), b)
        && mul2(transpose2(c), d) == mul2(transpose2(d), c)
        && sub2(mul2(transpose2(a), d), mul2(transpose2(b), c)) == identity
}

#[cfg(test)]
type Block<L> = [[Residue<L>; 2]; 2];

/// `Φ` in the basis `(b1, b2, b3, b4)`, entries modulo `2^e`.
fn phi<L: ParameterSet>(s: Residue<L>, q: Residue<L>) -> Mat4<L> {
    let zero = Residue::zero();
    let one = Residue::one();
    Mat4 {
        row: [
            [s, q.negate(), zero, zero],
            [one, s, zero, zero],
            [zero, zero, s, one.negate()],
            [zero, zero, q, s],
        ],
    }
}

/// `Φ̂`, with `Φ̂Φ = 2^{2e′−1} I`. Modulo `2^e` that product is zero.
#[cfg_attr(not(test), allow(dead_code))]
fn phi_hat<L: ParameterSet>(s: Residue<L>, q: Residue<L>) -> Mat4<L> {
    let zero = Residue::zero();
    let one = Residue::one();
    Mat4 {
        row: [
            [s, q, zero, zero],
            [one.negate(), s, zero, zero],
            [zero, zero, s, one],
            [zero, zero, q.negate(), s],
        ],
    }
}

/// `(Φ, M1, M2)` modulo `2^e`, from the challenge scalar.
/// `None` when `s` is even, so it has no inverse modulo `2^e`.
pub fn challenge_matrices<L: ParameterSet>(s: Residue<L>) -> Option<(Mat4<L>, Mat4<L>, Mat4<L>)> {
    let t = s.inv()?;
    let zero = Residue::zero();
    let one = Residue::one();
    let s2 = s.multiply(s);
    let t2 = t.multiply(t);
    let m1 = Mat4 {
        row: [
            [zero, zero, s, zero],
            [zero, t2, one.negate(), zero],
            [t.negate(), zero, zero, s],
            [zero, zero, zero, s2],
        ],
    };
    let m2 = Mat4 {
        row: [
            [one.negate(), zero, zero, one],
            [zero, zero, zero, t],
            [zero, zero, one.negate(), zero],
            [zero, s.negate(), s, zero],
        ],
    };
    Some((phi(s, s2.negate()), m1, m2))
}

#[cfg(test)]
mod tests {
    use super::{challenge_matrices, is_symplectic, Residue};
    use prism_field::{ParameterSet, P248, P376, P500};

    fn matrix<L: ParameterSet>(text: &str, key: &str) -> [[Residue<L>; 4]; 4] {
        let at = text.find(&format!("\"{key}\"")).expect(key);
        let mut rest = &text[at..];
        let mut numbers = Vec::new();
        while numbers.len() < 16 {
            let open = rest.find('"').unwrap();
            rest = &rest[open + 1..];
            let close = rest.find('"').unwrap();
            let token = &rest[..close];
            rest = &rest[close + 1..];
            if !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit()) {
                numbers.push(Residue::from_dec(token));
            }
        }
        let mut row = [[numbers[0]; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                row[i][j] = numbers[i * 4 + j];
            }
        }
        row
    }

    fn scalar<L: ParameterSet>(text: &str, key: &str) -> Residue<L> {
        let marker = format!("\"{key}\": \"");
        let at = text.find(&marker).expect(key);
        let rest = &text[at + marker.len()..];
        let end = rest.find('"').unwrap();
        Residue::from_dec(&rest[..end])
    }

    fn agrees<L: ParameterSet>(file: &str) {
        let text = file;
        let s = scalar::<L>(text, "s");
        let (phi, m1, m2) = challenge_matrices(s).expect("matrices");
        assert_eq!(phi.row, matrix(text, "Mphi"));
        assert_eq!(m1.row, matrix(text, "M1"));
        assert_eq!(m2.row, matrix(text, "M2"));
        assert!(is_symplectic(m1), "M1");
        assert!(is_symplectic(m2), "M2");
        let kernel = |matrix: super::Mat4<L>, cols: [usize; 2]| {
            let mut row = [[matrix.row[0][0]; 2]; 4];
            for (out, source) in row.iter_mut().zip(matrix.row) {
                out[0] = source[cols[0]];
                out[1] = source[cols[1]];
            }
            row
        };
        let image = |factor: super::Mat4<L>, columns: [[Residue<L>; 2]; 4]| {
            let mut out = [[Residue::zero(); 2]; 4];
            for (out_row, factor_row) in out.iter_mut().zip(factor.row) {
                for (column, coeff) in columns.iter().zip(factor_row) {
                    for (cell, entry) in out_row.iter_mut().zip(column) {
                        *cell = cell.add(coeff.multiply(*entry));
                    }
                }
            }
            out
        };
        let dual = super::phi_hat(s, s.multiply(s).negate());
        assert_eq!(
            kernel(m2, [2, 3]),
            image(phi, kernel(m1, [0, 1])),
            "M2 kernel half"
        );
        let mut expected = image(dual, kernel(m2, [0, 1]));
        for row in &mut expected {
            for entry in row {
                *entry = entry.negate();
            }
        }
        assert_eq!(kernel(m1, [2, 3]), expected, "M1 kernel half");
    }

    #[test]
    fn the_known_answer_matrices_match_at_every_set() {
        agrees::<P248>(include_str!("../testdata/s5_model/s5_lvl1_matrices.json"));
        agrees::<P248>(include_str!("../testdata/s5_model/s5_lvl1b_matrices.json"));
        agrees::<P376>(include_str!("../testdata/s5_model/s5_lvl3_matrices.json"));
        agrees::<P376>(include_str!("../testdata/s5_model/s5_lvl3b_matrices.json"));
        agrees::<P500>(include_str!("../testdata/s5_model/s5_lvl5_matrices.json"));
        agrees::<P500>(include_str!("../testdata/s5_model/s5_lvl5b_matrices.json"));
    }
}
