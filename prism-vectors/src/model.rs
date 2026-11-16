// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The Sage model's files: intermediate values of the accept path on the
//! six primary vectors, and the model's verdicts.
//!
//! `scripts/sage/s5_model.sage` runs the accept path of the paper's
//! section 5 with the `Theta_dim4` library at its pinned commit and
//! writes, for each primary vector, three JSON files:
//!
//! - `<vector>_matrices.json`: the change-of-basis matrices and the theta
//!   base change;
//! - `<vector>_nulls.json`: the theta null point after every step of both
//!   chains;
//! - `<vector>_step8.json`: the images of the evaluation points.
//!
//! Each is given here as text. The tests that compare the verifier with
//! them read the JSON themselves, since what they compare is their own
//! intermediate values.

use crate::corpus::Primary;

/// One model file of a primary vector. `include_str!` needs a literal, so
/// the six stems are written once, here, and each kind is a suffix.
macro_rules! model_file {
    ($vector:expr, $kind:literal) => {
        match $vector {
            Primary::Lvl1 => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/data/isogeny/s5_model/s5_lvl1_",
                    $kind
                ))
            }
            Primary::Lvl1b => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/data/isogeny/s5_model/s5_lvl1b_",
                    $kind
                ))
            }
            Primary::Lvl3 => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/data/isogeny/s5_model/s5_lvl3_",
                    $kind
                ))
            }
            Primary::Lvl3b => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/data/isogeny/s5_model/s5_lvl3b_",
                    $kind
                ))
            }
            Primary::Lvl5 => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/data/isogeny/s5_model/s5_lvl5_",
                    $kind
                ))
            }
            Primary::Lvl5b => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/data/isogeny/s5_model/s5_lvl5b_",
                    $kind
                ))
            }
        }
    };
}

/// The matrices file of a primary vector.
pub const fn matrices(vector: Primary) -> &'static str {
    model_file!(vector, "matrices.json")
}

/// The null-point file of a primary vector.
pub const fn nulls(vector: Primary) -> &'static str {
    model_file!(vector, "nulls.json")
}

/// The step-8 image file of a primary vector.
pub const fn step8(vector: Primary) -> &'static str {
    model_file!(vector, "step8.json")
}

/// `negatives.json`: the model's verdict on each negative it derives from
/// a primary vector, keyed by the vector's file name.
pub const NEGATIVES: &str = isogeny_data!("s5_model/negatives.json");

/// `accept_verdicts.tsv`: the model's verdict on every file of the accept
/// corpus, as `file`, `level`, `verdict` in tab-separated columns.
pub const ACCEPT_VERDICTS: &str = isogeny_data!("s5_model/accept_verdicts.tsv");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_primary_vector_has_its_three_files() {
        for vector in Primary::ALL {
            for text in [matrices(vector), nulls(vector), step8(vector)] {
                assert!(text.trim_start().starts_with('{'), "{vector:?}");
            }
            assert!(NEGATIVES.contains(vector.file_name()), "{vector:?}");
        }
        assert_eq!(ACCEPT_VERDICTS.lines().next(), Some("file\tlevel\tverdict"));
    }
}
