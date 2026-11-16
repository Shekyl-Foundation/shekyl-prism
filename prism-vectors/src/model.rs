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

/// The matrices file of a primary vector.
pub const fn matrices(vector: Primary) -> &'static str {
    match vector {
        Primary::Lvl1 => isogeny_data!("s5_model/s5_lvl1_matrices.json"),
        Primary::Lvl1b => isogeny_data!("s5_model/s5_lvl1b_matrices.json"),
        Primary::Lvl3 => isogeny_data!("s5_model/s5_lvl3_matrices.json"),
        Primary::Lvl3b => isogeny_data!("s5_model/s5_lvl3b_matrices.json"),
        Primary::Lvl5 => isogeny_data!("s5_model/s5_lvl5_matrices.json"),
        Primary::Lvl5b => isogeny_data!("s5_model/s5_lvl5b_matrices.json"),
    }
}

/// The null-point file of a primary vector.
pub const fn nulls(vector: Primary) -> &'static str {
    match vector {
        Primary::Lvl1 => isogeny_data!("s5_model/s5_lvl1_nulls.json"),
        Primary::Lvl1b => isogeny_data!("s5_model/s5_lvl1b_nulls.json"),
        Primary::Lvl3 => isogeny_data!("s5_model/s5_lvl3_nulls.json"),
        Primary::Lvl3b => isogeny_data!("s5_model/s5_lvl3b_nulls.json"),
        Primary::Lvl5 => isogeny_data!("s5_model/s5_lvl5_nulls.json"),
        Primary::Lvl5b => isogeny_data!("s5_model/s5_lvl5b_nulls.json"),
    }
}

/// The step-8 image file of a primary vector.
pub const fn step8(vector: Primary) -> &'static str {
    match vector {
        Primary::Lvl1 => isogeny_data!("s5_model/s5_lvl1_step8.json"),
        Primary::Lvl1b => isogeny_data!("s5_model/s5_lvl1b_step8.json"),
        Primary::Lvl3 => isogeny_data!("s5_model/s5_lvl3_step8.json"),
        Primary::Lvl3b => isogeny_data!("s5_model/s5_lvl3b_step8.json"),
        Primary::Lvl5 => isogeny_data!("s5_model/s5_lvl5_step8.json"),
        Primary::Lvl5b => isogeny_data!("s5_model/s5_lvl5b_step8.json"),
    }
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
