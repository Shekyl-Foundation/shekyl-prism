// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The fixtures of `p324`: its Sage curve file, the curves of it that
//! tests name, and the message the refusal tests share.
//!
//! `p324` has no signer, so no committed vector. What stands in for one is
//! `prism-verify/testdata/p324_curves.tsv`, written by
//! `scripts/sage/p324_fixture.sage`. The file stays where the script
//! writes it and is read from there.
//!
//! A curve is named here by what it is used for. What makes it fit for
//! that use is a property of its coefficient, held by a test and not by
//! the constant: the tests of this module hold the branch of each, and
//! `p324_refusals` in `prism-verify` holds which coefficients the model
//! check passes.

use crate::curve_table::{CurveNumber, CurveTable};

/// The curve file's name, as the count records cite it.
pub const CURVE_FILE: &str = "p324_curves.tsv";

const CURVE_TABLE: &str = include_str!("../../prism-verify/testdata/p324_curves.tsv");

/// How many curves the Sage script walks to.
pub const CURVES: usize = 8;

/// A curve whose coefficient is a square, so the basis search takes its
/// square branch.
pub const SQUARE_BRANCH_CURVE: CurveNumber = CurveNumber::new(1);

/// A curve off the square branch whose coefficient is the permitted model
/// of its class, so the verifier takes it as a response curve.
pub const MODEL_CURVE: CurveNumber = CurveNumber::new(7);

/// A curve whose negated coefficient is the permitted model of its class.
/// With [`MODEL_CURVE`] it is all the file has of those.
pub const NEGATED_MODEL_CURVE: CurveNumber = CurveNumber::new(5);

/// One curve from each branch of the basis search, with whether its
/// coefficient is a square.
pub const BRANCH_CURVES: [(CurveNumber, bool); 2] =
    [(SQUARE_BRANCH_CURVE, true), (MODEL_CURVE, false)];

/// The message the refusal tests and the operation counts hash into the
/// challenge. Not a committed vector's message. The salt counters pinned
/// in the tests are for this message under the curves named above, and
/// the counts are for the same message and curve, so the two cannot
/// drift.
pub const REFUSAL_MESSAGE: &[u8] = b"shekyl-prism-p324-refusals";

/// The committed curve file. It is the Sage script's output, so a file
/// this does not read, or one with another number of curves, is a broken
/// fixture.
pub fn curve_table() -> CurveTable<'static> {
    let table =
        CurveTable::parse(CURVE_TABLE).unwrap_or_else(|reason| panic!("{CURVE_FILE}: {reason}"));
    assert_eq!(table.rows().len(), CURVES, "{CURVE_FILE}");
    table
}

#[cfg(test)]
mod tests {
    use super::{curve_table, BRANCH_CURVES, CURVES, NEGATED_MODEL_CURVE};

    #[test]
    fn the_committed_file_is_read_and_has_the_named_curves() {
        let table = curve_table();
        let numbers: Vec<String> = table
            .rows()
            .iter()
            .map(|row| row.number.to_string())
            .collect();
        let expected: Vec<String> = (1..=CURVES).map(|number| number.to_string()).collect();
        assert_eq!(numbers, expected);
        for (number, square) in BRANCH_CURVES {
            assert_eq!(
                table.curve(number).coefficient_is_square,
                square,
                "curve {number}"
            );
        }
        assert_eq!(table.curve(NEGATED_MODEL_CURVE).number, NEGATED_MODEL_CURVE);
    }
}
