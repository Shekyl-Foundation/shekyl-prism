// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// `base_change_theta_dim2` from `external/Theta_dim4`
// `pkg/basis_change/base_change_dim2.py` (Apache-2.0, Pierrick Dartois).
// See `NOTICE`.

//! Level-2 theta coordinates after a symplectic base change of `A[4]`.
//!
//! The matrix is in `Sp_4(Z/4Z)`, entries `0..3`, row then column. `zeta`
//! is `e_4` of the product basis. The result is the matrix `N` such that
//! the new theta coordinates are `N` times the product coordinates.

use prism_field::Field;

fn mod4(value: i32) -> i32 {
    value.rem_euclid(4)
}

fn block(matrix: [[u8; 4]; 4], rows: [usize; 2], cols: [usize; 2]) -> [[i32; 2]; 2] {
    [
        [
            i32::from(matrix[rows[0]][cols[0]]),
            i32::from(matrix[rows[0]][cols[1]]),
        ],
        [
            i32::from(matrix[rows[1]][cols[0]]),
            i32::from(matrix[rows[1]][cols[1]]),
        ],
    ]
}

fn power<F: Field>(zeta: F, exponent: i32) -> F {
    let mut value = F::one();
    for _ in 0..mod4(exponent) {
        value = value.mul(zeta);
    }
    value
}

/// The first of the four half-period shifts whose sums are not all zero,
/// or `None` when every one of them vanishes.
///
/// Each sum is a polynomial in `zeta` with small non-negative
/// coefficients, so whether they all vanish depends on `zeta` and on the
/// two blocks alone. Over all 65,536 pairs of blocks the four shifts all
/// vanish in exactly one case: `zeta² = −3` and one of 1,152 pairs, where
/// every sum is `3 + zeta²`. The tests below walk every pair.
///
/// Nothing that reaches this through `prism_verify::verify` is in that
/// case, for two independent reasons. `zeta` there is a Weil pairing
/// value on the 4-torsion, so `zeta⁴ = 1`, and `zeta² = −3` would make
/// `9 = 1`. And the matrix there is a product of symplectic matrices,
/// while none of the 1,152 pairs is the right half of a symplectic
/// matrix. The refusal stays: a default here would be the first shift,
/// which is indistinguishable from a search that succeeded.
fn nonvanishing<F: Field>(c: [[i32; 2]; 2], d: [[i32; 2]; 2], zeta: F) -> Option<(i32, i32)> {
    for ir0 in 0..2 {
        for ir1 in 0..2 {
            let mut sum = [F::zero(), F::zero(), F::zero(), F::zero()];
            for j0 in 0..2 {
                for j1 in 0..2 {
                    let k0 = mod4(c[0][0] * j0 + c[0][1] * j1);
                    let k1 = mod4(c[1][0] * j0 + c[1][1] * j1);
                    let l0 = mod4(d[0][0] * j0 + d[0][1] * j1);
                    let l1 = mod4(d[1][0] * j0 + d[1][1] * j1);
                    let exponent = -((k0 + 2 * ir0) * l0) - ((k1 + 2 * ir1) * l1);
                    let index = usize::try_from(mod4(k0 + ir0) % 2 + 2 * (mod4(k1 + ir1) % 2))
                        .expect("theta index");
                    sum[index] = sum[index].add(power(zeta, exponent));
                }
            }
            if sum.iter().any(|entry| !entry.is_zero()) {
                return Some((ir0, ir1));
            }
        }
    }
    None
}

/// `N` for `M` and `zeta`, in the row convention of the Sage function.
///
/// `None` when no half-period shift gives a nonzero sum, so there is no
/// `N` to return; see [`nonvanishing`] for when that is. It is not a test
/// of the inputs: a matrix that is not symplectic, or a `zeta` that is
/// not a primitive fourth root of unity, still gets a matrix back, and it
/// means nothing.
pub fn base_change_theta<F: Field>(matrix: [[u8; 4]; 4], zeta: F) -> Option<[[F; 4]; 4]> {
    let a = block(matrix, [0, 1], [0, 1]);
    let b = block(matrix, [2, 3], [0, 1]);
    let c = block(matrix, [0, 1], [2, 3]);
    let d = block(matrix, [2, 3], [2, 3]);
    let (ir0, ir1) = nonvanishing(c, d, zeta)?;
    let mut result = [[F::zero(); 4]; 4];
    for i0 in 0..2 {
        for i1 in 0..2 {
            for j0 in 0..2 {
                for j1 in 0..2 {
                    let k0 = mod4(a[0][0] * i0 + a[0][1] * i1 + c[0][0] * j0 + c[0][1] * j1);
                    let k1 = mod4(a[1][0] * i0 + a[1][1] * i1 + c[1][0] * j0 + c[1][1] * j1);
                    let l0 = mod4(b[0][0] * i0 + b[0][1] * i1 + d[0][0] * j0 + d[0][1] * j1);
                    let l1 = mod4(b[1][0] * i0 + b[1][1] * i1 + d[1][0] * j0 + d[1][1] * j1);
                    let exponent = i0 * j0 + i1 * j1 - (k0 + 2 * ir0) * l0 - (k1 + 2 * ir1) * l1;
                    let row = usize::try_from(i0 + 2 * i1).expect("row");
                    let col = usize::try_from(mod4(k0 + ir0) % 2 + 2 * (mod4(k1 + ir1) % 2))
                        .expect("column");
                    result[row][col] = result[row][col].add(power(zeta, exponent));
                }
            }
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::{base_change_theta, nonvanishing};
    use prism_field::{Field, Level, Level1, Level3, Level5};

    fn digits<L: Level>(text: &str, negative: bool) -> L::Base {
        let mut bytes = vec![0u8; L::FP_LEN];
        for byte in text.bytes() {
            let mut carry = u16::from(byte - b'0');
            for limb in &mut bytes {
                carry += u16::from(*limb) * 10;
                *limb = carry as u8;
                carry >>= 8;
            }
        }
        let value = L::base_decode(&bytes).expect("base");
        if negative {
            value.negate()
        } else {
            value
        }
    }

    fn sage_fp2<L: Level>(text: &str) -> L::Ext {
        let text = text.replace(' ', "");
        if text == "0" {
            return L::Ext::zero();
        }
        if text == "i" {
            return L::ext_i();
        }
        if text == "-i" {
            return L::ext_i().negate();
        }
        let (im_text, re_text, re_negative) = if let Some((im, re)) = text.split_once("*i+") {
            (im, re, false)
        } else if let Some((im, re)) = text.split_once("*i-") {
            (im, re, true)
        } else if let Some(im) = text.strip_suffix("*i") {
            (im, "0", false)
        } else {
            ("0", text.as_str(), false)
        };
        let im_negative = im_text.starts_with('-');
        let im_digits = im_text.trim_start_matches(['+', '-']);
        let im = if im_digits.is_empty() {
            L::Base::zero()
        } else {
            digits::<L>(im_digits, im_negative)
        };
        let re_digits = re_text.trim_start_matches(['+', '-']);
        let re = if re_digits.is_empty() {
            L::Base::zero()
        } else {
            digits::<L>(re_digits, re_negative || re_text.starts_with('-'))
        };
        L::ext_from_base(re).add(L::ext_from_base(im).mul(L::ext_i()))
    }

    fn small_matrix(text: &str, key: &str) -> [[u8; 4]; 4] {
        let marker = format!("\"{key}\"");
        let at = text.find(&marker).expect(key);
        let mut rest = &text[at + marker.len()..];
        let mut numbers = Vec::new();
        while numbers.len() < 16 {
            let byte = rest.as_bytes()[0];
            if byte.is_ascii_digit() {
                let end = rest
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(rest.len());
                numbers.push(rest[..end].parse::<u8>().unwrap());
                rest = &rest[end..];
            } else {
                rest = &rest[1..];
            }
        }
        let mut row = [[0u8; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                row[i][j] = numbers[i * 4 + j];
            }
        }
        row
    }

    fn quoted(text: &str, key: &str) -> Vec<Vec<String>> {
        let marker = format!("\"{key}\"");
        let at = text.find(&marker).expect(key);
        let mut rest = &text[at + marker.len()..];
        let mut rows = Vec::new();
        let mut current = Vec::new();
        while rows.len() < 4 || current.len() < 4 {
            let open = rest.find('"').unwrap();
            rest = &rest[open + 1..];
            let close = rest.find('"').unwrap();
            let token = &rest[..close];
            rest = &rest[close + 1..];
            if token.chars().any(|c| c.is_ascii_digit()) || token == "i" || token == "-i" {
                current.push(token.to_string());
                if current.len() == 4 {
                    rows.push(current);
                    current = Vec::new();
                    if rows.len() == 4 {
                        break;
                    }
                }
            }
        }
        rows
    }

    fn agrees<L: Level>(file: &str) {
        let marker = "\"e4\": \"";
        let at = file.find(marker).expect("e4");
        let rest = &file[at + marker.len()..];
        let end = rest.find('"').unwrap();
        let zeta = sage_fp2::<L>(&rest[..end]);
        for (change_key, image_key) in [("Mbc1", "N1"), ("Mbc2", "N2")] {
            let got = base_change_theta::<L::Ext>(small_matrix(file, change_key), zeta)
                .expect("a symplectic matrix and a primitive fourth root");
            let expect = quoted(file, image_key);
            for row in 0..4 {
                for col in 0..4 {
                    assert!(
                        got[row][col].same(sage_fp2::<L>(&expect[row][col])),
                        "{} {change_key} ({row},{col})",
                        L::NAME
                    );
                }
            }
        }
    }

    #[test]
    fn the_theta_base_change_matches_the_closed_form_files() {
        agrees::<Level1>(include_str!("../testdata/s5_model/s5_lvl1_matrices.json"));
        agrees::<Level1>(include_str!("../testdata/s5_model/s5_lvl1b_matrices.json"));
        agrees::<Level3>(include_str!("../testdata/s5_model/s5_lvl3_matrices.json"));
        agrees::<Level3>(include_str!("../testdata/s5_model/s5_lvl3b_matrices.json"));
        agrees::<Level5>(include_str!("../testdata/s5_model/s5_lvl5_matrices.json"));
        agrees::<Level5>(include_str!("../testdata/s5_model/s5_lvl5b_matrices.json"));
    }

    /// The two blocks the search reads, from the eight entries of the
    /// matrix's right half: rows 0 and 1 (`c`), then rows 2 and 3 (`d`).
    fn blocks(index: u32) -> ([[i32; 2]; 2], [[i32; 2]; 2]) {
        let entry = |k: u32| ((index >> (2 * k)) & 3) as i32;
        (
            [[entry(0), entry(1)], [entry(2), entry(3)]],
            [[entry(4), entry(5)], [entry(6), entry(7)]],
        )
    }

    /// Every pair of blocks, `4^8` of them.
    const PAIRS: u32 = 1 << 16;

    /// Whether the two blocks can be the right half of a matrix of
    /// `Sp_4(Z/4Z)`: its two columns pair to zero under the symplectic
    /// form, and they are independent modulo 2.
    fn symplectic_half(c: [[i32; 2]; 2], d: [[i32; 2]; 2]) -> bool {
        let form = c[0][0] * d[0][1] + c[1][0] * d[1][1] - c[0][1] * d[0][0] - c[1][1] * d[1][0];
        let column = |j: usize| [c[0][j] % 2, c[1][j] % 2, d[0][j] % 2, d[1][j] % 2];
        let (first, second) = (column(0), column(1));
        form.rem_euclid(4) == 0 && first != [0; 4] && second != [0; 4] && first != second
    }

    /// The pairs of blocks, of all `4^8`, for which the search fails.
    fn failures<F: Field>(zeta: F) -> Vec<u32> {
        (0..PAIRS)
            .filter(|&index| {
                let (c, d) = blocks(index);
                nonvanishing(c, d, zeta).is_none()
            })
            .collect()
    }

    /// A matrix with the given right half and zeros on the left.
    fn with_right_half(index: u32) -> [[u8; 4]; 4] {
        let (c, d) = blocks(index);
        [
            [0, 0, c[0][0] as u8, c[0][1] as u8],
            [0, 0, c[1][0] as u8, c[1][1] as u8],
            [0, 0, d[0][0] as u8, d[0][1] as u8],
            [0, 0, d[1][0] as u8, d[1][1] as u8],
        ]
    }

    /// The search does not fail for a `zeta` that is not a primitive fourth
    /// root of unity, whatever the matrix: not for `1`, `−1` or `0`, and not
    /// for two arbitrary elements. Every one of the `4^8` right halves is
    /// tried, so this covers every matrix that is not symplectic and the
    /// all-zero matrix. The function returns a matrix for all of them; it
    /// does not check its inputs.
    #[test]
    fn the_search_succeeds_for_every_matrix_when_zeta_squared_is_not_minus_three() {
        type F = <Level1 as Level>::Ext;
        let arbitrary =
            F::from_u64(0x9e37_79b9_7f4a_7c15).add(Level1::ext_i().mul(F::from_u64(41)));
        let cases = [
            ("1", F::one()),
            ("-1", F::one().negate()),
            ("0", F::zero()),
            ("i", Level1::ext_i()),
            ("-i", Level1::ext_i().negate()),
            ("arbitrary", arbitrary),
            ("arbitrary squared", arbitrary.mul(arbitrary)),
        ];
        for (name, zeta) in cases {
            assert!(!zeta.mul(zeta).same(F::from_u64(3).negate()), "{name}");
            assert_eq!(failures(zeta), Vec::<u32>::new(), "zeta = {name}");
        }
    }

    /// Named cases of the test above, through the public function: the
    /// all-zero matrix, and a matrix that is not symplectic modulo 4 (its
    /// right columns are equal), each at a primitive fourth root and at `1`.
    #[test]
    fn a_matrix_comes_back_for_the_zero_matrix_and_for_one_that_is_not_symplectic() {
        type F = <Level1 as Level>::Ext;
        let not_symplectic = [[1u8; 4]; 4];
        let (c, d) = ([[1, 1], [1, 1]], [[1, 1], [1, 1]]);
        assert!(!symplectic_half(c, d));
        for zeta in [Level1::ext_i(), F::one()] {
            assert!(base_change_theta([[0u8; 4]; 4], zeta).is_some());
            assert!(base_change_theta(not_symplectic, zeta).is_some());
        }
    }

    /// The one way the search fails: `zeta² = −3`, and one of 1,152 right
    /// halves. The function returns `None` for each of them, and none of
    /// them is the right half of a symplectic matrix. At every level.
    fn fails_only_at_a_root_of_minus_three<L: Level>() {
        let root = L::Ext::from_u64(3)
            .negate()
            .sqrt()
            .expect("the extension field has a square root of every element of the base field");
        for zeta in [root, root.negate()] {
            let failing = failures(zeta);
            assert_eq!(failing.len(), 1152, "{}", L::NAME);
            for &index in &failing {
                let (c, d) = blocks(index);
                assert!(!symplectic_half(c, d), "{} {index}", L::NAME);
                assert!(
                    base_change_theta(with_right_half(index), zeta).is_none(),
                    "{} {index}",
                    L::NAME
                );
            }
            // The left half plays no part in the search.
            let example = [[3, 1, 0, 0], [2, 3, 0, 2], [1, 1, 0, 0], [0, 2, 1, 0]];
            assert!(base_change_theta(example, zeta).is_none(), "{}", L::NAME);
            assert!(
                base_change_theta(example, L::ext_i()).is_some(),
                "{}",
                L::NAME
            );
        }
        let halves = (0..PAIRS)
            .filter(|&index| {
                let (c, d) = blocks(index);
                symplectic_half(c, d)
            })
            .count();
        assert_eq!(halves, 11_520, "{}", L::NAME);
    }

    #[test]
    fn the_search_fails_only_at_a_root_of_minus_three_and_then_returns_none() {
        fails_only_at_a_root_of_minus_three::<Level1>();
        fails_only_at_a_root_of_minus_three::<Level3>();
        fails_only_at_a_root_of_minus_three::<Level5>();
    }
}
