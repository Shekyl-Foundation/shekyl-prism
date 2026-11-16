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
    use prism_field::{ParameterSet, P248, P324, P376, P500, P664};

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
        the_matrix_identities_hold::<L>(s);
    }

    /// What the three matrices of a challenge scalar satisfy, whatever the
    /// scalar: `M1` and `M2` are symplectic, and each one's kernel half is
    /// the image of the other's complement under `Φ` or `−Φ̂`. None of this
    /// needs a known answer, so it is also the check at a set that has no
    /// model file.
    fn the_matrix_identities_hold<L: ParameterSet>(s: Residue<L>) {
        let (phi, m1, m2) = challenge_matrices(s).expect("matrices");
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
        agrees::<P248>(prism_vectors::model::matrices(
            prism_vectors::corpus::Primary::Lvl1,
        ));
        agrees::<P248>(prism_vectors::model::matrices(
            prism_vectors::corpus::Primary::Lvl1b,
        ));
        agrees::<P376>(prism_vectors::model::matrices(
            prism_vectors::corpus::Primary::Lvl3,
        ));
        agrees::<P376>(prism_vectors::model::matrices(
            prism_vectors::corpus::Primary::Lvl3b,
        ));
        agrees::<P500>(prism_vectors::model::matrices(
            prism_vectors::corpus::Primary::Lvl5,
        ));
        agrees::<P500>(prism_vectors::model::matrices(
            prism_vectors::corpus::Primary::Lvl5b,
        ));
    }

    /// An integer modulo `2^e`, as little-endian bytes: a second
    /// implementation of the arithmetic, in base 256 with no limbs, to
    /// hold the limb code against. It shares nothing with [`Residue`] but
    /// the exponent.
    #[derive(Clone, Debug, PartialEq)]
    struct Bytes(Vec<u8>);

    impl Bytes {
        fn reduced<L: ParameterSet>(mut bytes: Vec<u8>) -> Self {
            let len = (L::E as usize).div_ceil(8);
            bytes.resize(len, 0);
            let spare = 8 * len as u32 - L::E;
            if spare > 0 {
                bytes[len - 1] &= u8::MAX >> spare;
            }
            Self(bytes)
        }

        fn add<L: ParameterSet>(&self, rhs: &Self) -> Self {
            let mut carry = 0u16;
            let sum = self
                .0
                .iter()
                .zip(&rhs.0)
                .map(|(a, b)| {
                    carry += u16::from(*a) + u16::from(*b);
                    let byte = carry as u8;
                    carry >>= 8;
                    byte
                })
                .collect();
            Self::reduced::<L>(sum)
        }

        fn multiply<L: ParameterSet>(&self, rhs: &Self) -> Self {
            let len = self.0.len();
            let mut wide = vec![0u32; 2 * len];
            for (i, a) in self.0.iter().enumerate() {
                for (j, b) in rhs.0.iter().enumerate() {
                    wide[i + j] += u32::from(*a) * u32::from(*b);
                }
                // Propagate after every row, so no cell overflows.
                for k in i..2 * len - 1 {
                    wide[k + 1] += wide[k] >> 8;
                    wide[k] &= 0xff;
                }
            }
            Self::reduced::<L>(wide[..len].iter().map(|byte| *byte as u8).collect())
        }
    }

    fn residue<L: ParameterSet>(bytes: &Bytes) -> Residue<L> {
        Residue::from_le_bytes(&bytes.0)
    }

    fn bytes<L: ParameterSet>(residue: Residue<L>) -> Bytes {
        Bytes(residue.to_le_bytes())
    }

    /// `2^bit` modulo `2^e`, which is zero from `bit = e` on.
    fn power_of_two<L: ParameterSet>(bit: u32) -> Bytes {
        let mut out = vec![0u8; (bit as usize) / 8 + 1];
        out[(bit / 8) as usize] = 1 << (bit % 8);
        Bytes::reduced::<L>(out)
    }

    fn minus_one<L: ParameterSet>() -> Bytes {
        Bytes::reduced::<L>(vec![0xff; (L::E as usize).div_ceil(8)])
    }

    /// SplitMix64, for operands that fill every limb.
    struct Stream(u64);

    impl Stream {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        fn value<L: ParameterSet>(&mut self) -> Bytes {
            let len = (L::E as usize).div_ceil(8);
            let mut out = Vec::with_capacity(len + 8);
            while out.len() < len {
                out.extend_from_slice(&self.next().to_le_bytes());
            }
            Bytes::reduced::<L>(out)
        }
    }

    /// The limb arithmetic against the byte arithmetic, at one set: values
    /// worked out by hand at the edges of the width, and random operands
    /// that fill every limb.
    fn the_limb_arithmetic_is_arithmetic_modulo_two_to_the_e<L: ParameterSet>(seed: u64) {
        let e = L::E;
        let one = power_of_two::<L>(0);
        let zero = Bytes::reduced::<L>(Vec::new());
        // `2^(e−1) · 2 = 2^e = 0`, and the top bit itself survives.
        let top = power_of_two::<L>(e - 1);
        assert_ne!(top, zero);
        assert_eq!(power_of_two::<L>(e), zero, "{}: 2^e is zero", L::NAME);
        let two = power_of_two::<L>(1);
        assert_eq!(bytes(residue::<L>(&top).multiply(residue(&two))), zero);
        // Powers of two that meet at the top bit, and one past it.
        for low in [1, 63, 64, 65, e / 2, e - 65, e - 64, e - 2] {
            let (left, right) = (power_of_two::<L>(low), power_of_two::<L>(e - 1 - low));
            assert_eq!(
                bytes(residue::<L>(&left).multiply(residue(&right))),
                top,
                "{}: 2^{low} · 2^{}",
                L::NAME,
                e - 1 - low
            );
            let past = power_of_two::<L>(e - low);
            assert_eq!(
                bytes(residue::<L>(&left).multiply(residue(&past))),
                zero,
                "{}: 2^{low} · 2^{} is 2^e",
                L::NAME,
                e - low
            );
        }
        // `(−1)² = 1`, `−(−1) = 1`, `−1 + 1 = 0`: carries through every limb.
        let m1 = minus_one::<L>();
        assert_eq!(bytes(residue::<L>(&m1).multiply(residue(&m1))), one);
        assert_eq!(bytes(residue::<L>(&m1).negate()), one);
        assert_eq!(bytes(residue::<L>(&m1).add(residue(&one))), zero);
        assert_eq!(bytes(Residue::<L>::one().negate()), m1);
        // The inverse of 3 modulo `2^e`, for an even `e`, is
        // `(2^(e+1) + 1)/3`: in binary `1010…1011`, so `0xab` and then
        // `0xaa` all the way up, cut off at bit `e`. Written here from that
        // closed form, not computed by either arithmetic. Every set's `e`
        // is even.
        assert_eq!(e % 2, 0, "{}: the closed form is for an even e", L::NAME);
        let three = Bytes::reduced::<L>(vec![3]);
        let third = {
            let mut out = vec![0xaa; (e as usize).div_ceil(8)];
            out[0] = 0xab;
            Bytes::reduced::<L>(out)
        };
        assert_eq!(
            three.multiply::<L>(&third),
            one,
            "{}: the closed form",
            L::NAME
        );
        assert_eq!(
            bytes(residue::<L>(&three).inv().expect("three is odd")),
            third,
            "{}: 1/3",
            L::NAME
        );
        assert!(residue::<L>(&two).inv().is_none(), "an even residue");
        // Random operands that fill every limb, and their inverses.
        let mut stream = Stream(seed);
        for round in 0..200 {
            let (a, b) = (stream.value::<L>(), stream.value::<L>());
            let (ra, rb) = (residue::<L>(&a), residue::<L>(&b));
            assert_eq!(bytes(ra), a, "{}: the bytes round trip", L::NAME);
            assert_eq!(
                bytes(ra.multiply(rb)),
                a.multiply::<L>(&b),
                "{} round {round}",
                L::NAME
            );
            assert_eq!(
                bytes(ra.add(rb)),
                a.add::<L>(&b),
                "{} round {round}",
                L::NAME
            );
            assert_eq!(
                bytes(ra.sub(rb)),
                a.add::<L>(&bytes(rb.negate())),
                "{} round {round}",
                L::NAME
            );
            assert_eq!(bytes(ra.add(ra.negate())), zero);
            let mut odd = a.clone();
            odd.0[0] |= 1;
            let inverse = residue::<L>(&odd).inv().expect("odd");
            assert_eq!(
                odd.multiply::<L>(&bytes(inverse)),
                one,
                "{} round {round}: the inverse, checked by the byte product",
                L::NAME
            );
        }
    }

    #[test]
    fn the_limb_arithmetic_is_arithmetic_modulo_two_to_the_e_at_every_set() {
        the_limb_arithmetic_is_arithmetic_modulo_two_to_the_e::<P248>(0x5052_4953_4d02_4800);
        the_limb_arithmetic_is_arithmetic_modulo_two_to_the_e::<P324>(0x5052_4953_4d03_2400);
        the_limb_arithmetic_is_arithmetic_modulo_two_to_the_e::<P376>(0x5052_4953_4d03_7600);
        the_limb_arithmetic_is_arithmetic_modulo_two_to_the_e::<P500>(0x5052_4953_4d05_0000);
        the_limb_arithmetic_is_arithmetic_modulo_two_to_the_e::<P664>(0x5052_4953_4d06_6400);
    }

    /// The two sets with no model file have no known-answer matrices. The
    /// identities the matrices satisfy need none: they are checked here
    /// for scalars at the edges of the width and for random odd ones.
    fn the_matrices_of_any_odd_scalar_satisfy_their_identities<L: ParameterSet>(seed: u64) {
        let e = L::E;
        let mut scalars = vec![
            power_of_two::<L>(0),
            Bytes::reduced::<L>(vec![3]),
            minus_one::<L>(),
            power_of_two::<L>(e - 1).add::<L>(&power_of_two::<L>(0)),
            power_of_two::<L>(e / 2).add::<L>(&minus_one::<L>()),
        ];
        let mut stream = Stream(seed);
        for _ in 0..20 {
            let mut odd = stream.value::<L>();
            odd.0[0] |= 1;
            scalars.push(odd);
        }
        for scalar in scalars {
            the_matrix_identities_hold::<L>(residue(&scalar));
        }
        // An even scalar has no inverse, and so no matrices.
        assert!(challenge_matrices(residue::<L>(&power_of_two::<L>(1))).is_none());
    }

    #[test]
    fn the_matrix_identities_hold_at_every_set() {
        the_matrices_of_any_odd_scalar_satisfy_their_identities::<P248>(0x5052_4953_4d4d_0248);
        the_matrices_of_any_odd_scalar_satisfy_their_identities::<P324>(0x5052_4953_4d4d_0324);
        the_matrices_of_any_odd_scalar_satisfy_their_identities::<P376>(0x5052_4953_4d4d_0376);
        the_matrices_of_any_odd_scalar_satisfy_their_identities::<P500>(0x5052_4953_4d4d_0500);
        the_matrices_of_any_odd_scalar_satisfy_their_identities::<P664>(0x5052_4953_4d4d_0664);
    }

    /// Known answers from outside this crate: products, inverses and the
    /// powers the matrices are built from, computed by
    /// `scripts/residue_known_answers.py` with Python's integers. CI holds
    /// the file to the script.
    const KNOWN_ANSWERS: &str = prism_vectors::kat::RESIDUE_KNOWN_ANSWERS;

    fn hex(text: &str) -> Vec<u8> {
        prism_vectors::hex::decode(text).expect("hex")
    }

    /// Every row of the file for one set. Returns how many products and
    /// how many inverses were held, so the caller can require both.
    fn the_known_answers_hold<L: ParameterSet>() -> (usize, usize) {
        let value = |text: &str| {
            let bytes = hex(text);
            assert_eq!(bytes.len(), (L::E as usize).div_ceil(8), "{}", L::NAME);
            Residue::<L>::from_le_bytes(&bytes)
        };
        let (mut products, mut inverses) = (0, 0);
        for line in KNOWN_ANSWERS.lines().filter(|line| !line.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields[1] != L::NAME {
                continue;
            }
            match (fields[0], fields.len()) {
                ("MUL", 5) => {
                    let (a, b, product) = (value(fields[2]), value(fields[3]), value(fields[4]));
                    assert_eq!(a.multiply(b), product, "{} product {products}", L::NAME);
                    assert_eq!(b.multiply(a), product, "{} product {products}", L::NAME);
                    // The file's bytes are the residue's bytes: nothing
                    // was cut off on the way in.
                    assert_eq!(a.to_le_bytes(), hex(fields[2]));
                    products += 1;
                }
                ("INV", 6) => {
                    let (s, t) = (value(fields[2]), value(fields[3]));
                    let (s2, t2) = (value(fields[4]), value(fields[5]));
                    assert_eq!(s.inv(), Some(t), "{} inverse {inverses}", L::NAME);
                    assert_eq!(t.inv(), Some(s), "{} inverse {inverses}", L::NAME);
                    assert_eq!(s.multiply(t), Residue::one());
                    assert_eq!(s.multiply(s), s2);
                    assert_eq!(t.multiply(t), t2);
                    // The matrices of this scalar have those values where
                    // their definition puts them, and satisfy their
                    // identities.
                    let (_, m1, m2) = challenge_matrices(s).expect("an odd scalar");
                    assert_eq!(m1.row[1][1], t2, "M1 holds t²");
                    assert_eq!(m1.row[3][3], s2, "M1 holds s²");
                    assert_eq!(m1.row[2][0], t.negate(), "M1 holds −t");
                    assert_eq!(m1.row[0][2], s, "M1 holds s");
                    assert_eq!(m2.row[1][3], t, "M2 holds t");
                    assert_eq!(m2.row[3][1], s.negate(), "M2 holds −s");
                    the_matrix_identities_hold::<L>(s);
                    inverses += 1;
                }
                other => panic!("a row of kind {other:?}"),
            }
        }
        (products, inverses)
    }

    #[test]
    fn the_known_answers_from_python_hold_at_every_set() {
        // Two boundaries at the four sets below 512 bits, three at `p664`:
        // six products and three scalars for each, on top of three and
        // four, and eight random rows of each kind.
        assert_eq!(the_known_answers_hold::<P248>(), (23, 18));
        assert_eq!(the_known_answers_hold::<P324>(), (23, 18));
        assert_eq!(the_known_answers_hold::<P376>(), (23, 18));
        assert_eq!(the_known_answers_hold::<P500>(), (23, 18));
        assert_eq!(the_known_answers_hold::<P664>(), (29, 21));
    }
}
