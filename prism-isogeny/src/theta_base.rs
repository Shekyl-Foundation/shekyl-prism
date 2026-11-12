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

fn nonvanishing<F: Field>(c: [[i32; 2]; 2], d: [[i32; 2]; 2], zeta: F) -> (i32, i32) {
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
                return (ir0, ir1);
            }
        }
    }
    (0, 0)
}

/// `N` for `M` and `zeta`, in the row convention of the Sage function.
pub fn base_change_theta<F: Field>(matrix: [[u8; 4]; 4], zeta: F) -> [[F; 4]; 4] {
    let a = block(matrix, [0, 1], [0, 1]);
    let b = block(matrix, [2, 3], [0, 1]);
    let c = block(matrix, [0, 1], [2, 3]);
    let d = block(matrix, [2, 3], [2, 3]);
    let (ir0, ir1) = nonvanishing(c, d, zeta);
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
    result
}

#[cfg(test)]
mod tests {
    use super::base_change_theta;
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
            let got = base_change_theta::<L::Ext>(small_matrix(file, change_key), zeta);
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
}
