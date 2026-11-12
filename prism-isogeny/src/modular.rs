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
//! `e` is the torsion exponent, at most 500, so eight `u64` limbs cover
//! every level. `Φ` matches the known-answer files. `M1` and `M2`
//! complete the same kernel blocks the model completes. The solution of
//! the underdetermined system is the one with free variables zero; the
//! files store PARI `matsolvemod`'s representative, so the test checks
//! the symplectic identities rather than those bytes.

const LIMBS: usize = 8;

/// An integer modulo `2^bits`, little-endian.
#[derive(Clone, Copy, Debug)]
pub struct Residue {
    limb: [u64; LIMBS],
    bits: u32,
}

impl PartialEq for Residue {
    fn eq(&self, other: &Self) -> bool {
        self.bits == other.bits && self.limb == other.limb
    }
}

impl Residue {
    pub fn zero(bits: u32) -> Self {
        let mut out = Self {
            limb: [0; LIMBS],
            bits,
        };
        out.mask();
        out
    }

    pub fn one(bits: u32) -> Self {
        Self::from_u64(bits, 1)
    }

    pub fn from_u64(bits: u32, value: u64) -> Self {
        let mut out = Self {
            limb: [0; LIMBS],
            bits,
        };
        out.limb[0] = value;
        out.mask();
        out
    }

    /// Decimal, reduced modulo `2^bits`.
    pub fn from_dec(bits: u32, text: &str) -> Self {
        let ten = Self::from_u64(bits, 10);
        let mut acc = Self::zero(bits);
        for byte in text.bytes() {
            let digit = Self::from_u64(bits, u64::from(byte - b'0'));
            acc = acc.mul(ten).add(digit);
        }
        acc
    }

    fn mask(&mut self) {
        let full = (self.bits / 64) as usize;
        let rem = self.bits % 64;
        if rem == 0 {
            for limb in &mut self.limb[full..] {
                *limb = 0;
            }
        } else {
            self.limb[full] &= (1u64 << rem) - 1;
            for limb in &mut self.limb[full + 1..] {
                *limb = 0;
            }
        }
    }

    fn is_zero(self) -> bool {
        self.limb.iter().all(|limb| *limb == 0)
    }

    fn is_odd(self) -> bool {
        self.limb[0] & 1 == 1
    }

    fn add(self, rhs: Self) -> Self {
        let mut out = self;
        let mut carry = 0u128;
        for i in 0..LIMBS {
            let sum = u128::from(out.limb[i]) + u128::from(rhs.limb[i]) + carry;
            out.limb[i] = sum as u64;
            carry = sum >> 64;
        }
        out.mask();
        out
    }

    fn sub(self, rhs: Self) -> Self {
        self.add(rhs.neg())
    }

    fn neg(self) -> Self {
        let mut out = self;
        let mut carry = 1u128;
        for limb in &mut out.limb {
            let sum = u128::from(!*limb) + carry;
            *limb = sum as u64;
            carry = sum >> 64;
        }
        out.mask();
        out
    }

    fn mul(self, rhs: Self) -> Self {
        let mut column = [0u128; LIMBS];
        for i in 0..LIMBS {
            if self.limb[i] == 0 {
                continue;
            }
            let mut carry = 0u128;
            for j in 0..(LIMBS - i) {
                let sum =
                    column[i + j] + u128::from(self.limb[i]) * u128::from(rhs.limb[j]) + carry;
                column[i + j] = sum & u128::from(u64::MAX);
                carry = sum >> 64;
            }
        }
        let mut out = Self {
            limb: [0; LIMBS],
            bits: self.bits,
        };
        for (limb, value) in out.limb.iter_mut().zip(column) {
            *limb = value as u64;
        }
        out.mask();
        out
    }

    /// Inverse of an odd residue. `None` when this is even.
    fn inv(self) -> Option<Self> {
        if !self.is_odd() {
            return None;
        }
        let two = Self::from_u64(self.bits, 2);
        let mut approx = Self::one(self.bits);
        // Each step doubles the number of correct bits, from 1 up to `bits`.
        let mut correct = 1u32;
        while correct < self.bits {
            let product = self.mul(approx);
            approx = approx.mul(two.sub(product));
            correct = correct.saturating_mul(2);
        }
        let product = self.mul(approx);
        Some(approx.mul(two.sub(product)))
    }
}

/// A 4×4 matrix over [`Residue`], row then column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    row: [[Residue; 4]; 4],
}

impl Mat4 {
    #[cfg(test)]
    fn block(self, rows: [usize; 2], cols: [usize; 2]) -> [[Residue; 2]; 2] {
        [
            [self.row[rows[0]][cols[0]], self.row[rows[0]][cols[1]]],
            [self.row[rows[1]][cols[0]], self.row[rows[1]][cols[1]]],
        ]
    }
}

#[cfg(test)]
fn transpose2(block: Block) -> Block {
    [[block[0][0], block[1][0]], [block[0][1], block[1][1]]]
}

#[cfg(test)]
fn mul2(left: Block, right: Block) -> Block {
    let bits = left[0][0].bits;
    let mut out = [[Residue::zero(bits); 2]; 2];
    for (out_row, left_row) in out.iter_mut().zip(left) {
        for (right_row, coeff) in right.iter().zip(left_row) {
            for (cell, factor) in out_row.iter_mut().zip(right_row) {
                *cell = cell.add(coeff.mul(*factor));
            }
        }
    }
    out
}

#[cfg(test)]
fn sub2(left: Block, right: Block) -> Block {
    [
        [left[0][0].sub(right[0][0]), left[0][1].sub(right[0][1])],
        [left[1][0].sub(right[1][0]), left[1][1].sub(right[1][1])],
    ]
}

/// `BᵀA = AᵀB`, `CᵀD = DᵀC`, and `AᵀD − BᵀC = I`, the dim-2 symplectic
/// identities in `complete_symplectic_matrix_dim2`.
#[cfg(test)]
fn is_symplectic(matrix: Mat4) -> bool {
    let a = matrix.block([0, 1], [0, 1]);
    let b = matrix.block([2, 3], [0, 1]);
    let c = matrix.block([0, 1], [2, 3]);
    let d = matrix.block([2, 3], [2, 3]);
    let bits = a[0][0].bits;
    let identity = [
        [Residue::one(bits), Residue::zero(bits)],
        [Residue::zero(bits), Residue::one(bits)],
    ];
    mul2(transpose2(b), a) == mul2(transpose2(a), b)
        && mul2(transpose2(c), d) == mul2(transpose2(d), c)
        && sub2(mul2(transpose2(a), d), mul2(transpose2(b), c)) == identity
}

type Block = [[Residue; 2]; 2];
type Columns = [[Residue; 2]; 4];

fn neg_block(block: Block) -> Block {
    let mut out = block;
    for row in &mut out {
        for entry in row {
            *entry = entry.neg();
        }
    }
    out
}

/// `Φ` in the basis `(b1, b2, b3, b4)`, entries modulo `2^e`.
fn phi(s: Residue, q: Residue) -> Mat4 {
    let bits = s.bits;
    let zero = Residue::zero(bits);
    let one = Residue::one(bits);
    Mat4 {
        row: [
            [s, q.neg(), zero, zero],
            [one, s, zero, zero],
            [zero, zero, s, one.neg()],
            [zero, zero, q, s],
        ],
    }
}

/// `Φ̂`, with `Φ̂Φ = 2^{2e′−1} I`. Modulo `2^e` that product is zero.
fn phi_hat(s: Residue, q: Residue) -> Mat4 {
    let bits = s.bits;
    let zero = Residue::zero(bits);
    let one = Residue::one(bits);
    Mat4 {
        row: [
            [s, q, zero, zero],
            [one.neg(), s, zero, zero],
            [zero, zero, s, one],
            [zero, zero, q.neg(), s],
        ],
    }
}

/// `(D stacked on −C)`, transposed: a 2×4 matrix.
fn kernel_frame(top: Block, bottom: Block) -> [[Residue; 4]; 2] {
    let neg = neg_block(bottom);
    [
        [top[0][0], top[1][0], neg[0][0], neg[1][0]],
        [top[0][1], top[1][1], neg[0][1], neg[1][1]],
    ]
}

fn solve(mut rows: Vec<[Residue; 4]>, mut rhs: Vec<Residue>) -> Option<[Residue; 4]> {
    let n = rows.len();
    let mut pivot_col = [None; 4];
    let mut used = [false; 4];
    for row in 0..n {
        let pivot = (0..4).find(|col| !used[*col] && rows[row][*col].is_odd())?;
        used[pivot] = true;
        pivot_col[row] = Some(pivot);
        let inv = rows[row][pivot].inv()?;
        for entry in &mut rows[row] {
            *entry = entry.mul(inv);
        }
        rhs[row] = rhs[row].mul(inv);
        let pivot_row = rows[row];
        let pivot_rhs = rhs[row];
        for (other, target) in rows.iter_mut().enumerate() {
            if other == row {
                continue;
            }
            let factor = target[pivot];
            if factor.is_zero() {
                continue;
            }
            for (entry, pivot_entry) in target.iter_mut().zip(pivot_row) {
                *entry = entry.sub(factor.mul(pivot_entry));
            }
            rhs[other] = rhs[other].sub(factor.mul(pivot_rhs));
        }
    }
    let bits = rhs[0].bits;
    let mut solution = [Residue::zero(bits); 4];
    for row in 0..n {
        let col = pivot_col[row]?;
        solution[col] = rhs[row];
    }
    Some(solution)
}

/// `complete_symplectic_matrix_dim2(C, D)`.
fn complete(c: Block, d: Block) -> Option<Mat4> {
    let bits = c[0][0].bits;
    let frame = kernel_frame(d, c);
    let xi = solve(
        frame.to_vec(),
        vec![Residue::one(bits), Residue::zero(bits)],
    )?;
    let v = [xi[2], xi[3], xi[0].neg(), xi[1].neg()];
    let mut three = frame.to_vec();
    three.push(v);
    let yi = solve(
        three,
        vec![Residue::zero(bits), Residue::one(bits), Residue::zero(bits)],
    )?;
    Some(Mat4 {
        row: [
            [xi[0], yi[0], c[0][0], c[0][1]],
            [xi[1], yi[1], c[1][0], c[1][1]],
            [xi[2], yi[2], d[0][0], d[0][1]],
            [xi[3], yi[3], d[1][0], d[1][1]],
        ],
    })
}

fn left_columns(matrix: Mat4) -> Columns {
    let mut out = [[matrix.row[0][0]; 2]; 4];
    for (out_row, matrix_row) in out.iter_mut().zip(matrix.row) {
        out_row[0] = matrix_row[0];
        out_row[1] = matrix_row[1];
    }
    out
}

fn mul_columns(matrix: Mat4, columns: Columns) -> Columns {
    let bits = matrix.row[0][0].bits;
    let mut out = [[Residue::zero(bits); 2]; 4];
    for (out_row, matrix_row) in out.iter_mut().zip(matrix.row) {
        for (source, coeff) in columns.iter().zip(matrix_row) {
            for (cell, factor) in out_row.iter_mut().zip(source) {
                *cell = cell.add(coeff.mul(*factor));
            }
        }
    }
    out
}

fn split_columns(columns: Columns) -> (Block, Block) {
    (
        [
            [columns[0][0], columns[0][1]],
            [columns[1][0], columns[1][1]],
        ],
        [
            [columns[2][0], columns[2][1]],
            [columns[3][0], columns[3][1]],
        ],
    )
}

fn augment(left: Columns, right: Columns) -> Mat4 {
    let mut row = [[left[0][0]; 4]; 4];
    for i in 0..4 {
        row[i][0] = left[i][0];
        row[i][1] = left[i][1];
        row[i][2] = right[i][0];
        row[i][3] = right[i][1];
    }
    Mat4 { row }
}

fn neg_columns(columns: Columns) -> Columns {
    let mut out = columns;
    for row in &mut out {
        for entry in row {
            *entry = entry.neg();
        }
    }
    out
}

/// `(Φ, M1, M2)` modulo `2^e`, from the challenge scalar. `q` is reduced
/// modulo `2^e` here: at every level `q ≡ −s² (mod 2^e)`.
pub fn challenge_matrices(s: Residue) -> Option<(Mat4, Mat4, Mat4)> {
    let q = s.mul(s).neg();
    let endomorphism = phi(s, q);
    let dual = phi_hat(s, q);
    let bits = s.bits;
    let zero = Residue::zero(bits);
    let c1 = [[s, zero], [Residue::one(bits).neg(), zero]];
    let d1 = [[zero, s], [zero, q.neg()]];
    let m1_0 = complete(c1, d1)?;
    let complement = left_columns(m1_0);
    let image = mul_columns(endomorphism, complement);
    let (c2, d2) = split_columns(image);
    let m2 = complete(c2, d2)?;
    let dual_image = mul_columns(dual, left_columns(m2));
    let m1 = augment(complement, neg_columns(dual_image));
    Some((endomorphism, m1, m2))
}

#[cfg(test)]
mod tests {
    use super::{challenge_matrices, is_symplectic, Residue};

    fn matrix(text: &str, key: &str, bits: u32) -> [[Residue; 4]; 4] {
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
                numbers.push(Residue::from_dec(bits, token));
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

    fn scalar(text: &str, key: &str, bits: u32) -> Residue {
        let marker = format!("\"{key}\": \"");
        let at = text.find(&marker).expect(key);
        let rest = &text[at + marker.len()..];
        let end = rest.find('"').unwrap();
        Residue::from_dec(bits, &rest[..end])
    }

    fn agrees(file: &str, bits: u32) {
        let text = file;
        let s = scalar(text, "s", bits);
        let (phi, m1, m2) = challenge_matrices(s).expect("matrices");
        // `Φ` is determined by `s`. `M1` and `M2` are a symplectic
        // completion of the same kernel blocks the model completes.
        // The files store PARI `matsolvemod`'s representative of that
        // completion; the representative here is the free-variable-zero
        // solution of the same system, so the comparison is the
        // symplectic identities rather than those bytes.
        assert_eq!(phi.row, matrix(text, "Mphi", bits));
        assert!(is_symplectic(m1), "M1");
        assert!(is_symplectic(m2), "M2");
    }

    #[test]
    fn the_known_answer_matrices_match_at_every_level() {
        agrees(
            include_str!("../testdata/s5_model/s5_lvl1_matrices.json"),
            248,
        );
        agrees(
            include_str!("../testdata/s5_model/s5_lvl1b_matrices.json"),
            248,
        );
        agrees(
            include_str!("../testdata/s5_model/s5_lvl3_matrices.json"),
            376,
        );
        agrees(
            include_str!("../testdata/s5_model/s5_lvl3b_matrices.json"),
            376,
        );
        agrees(
            include_str!("../testdata/s5_model/s5_lvl5_matrices.json"),
            500,
        );
        agrees(
            include_str!("../testdata/s5_model/s5_lvl5b_matrices.json"),
            500,
        );
    }
}
