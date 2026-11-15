// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The fixtures of a parameter set that has no committed vectors.
//!
//! A set with no signer has no honest signature, so no corpus. What
//! stands in for one is written by `scripts/sage/no_corpus_fixture.sage`
//! and by `scripts/challenge_scalar.py`, the same way for every such set:
//!
//! - `<set>_field.tsv`: field arithmetic against Sage;
//! - `<set>_curves.tsv`: supersingular curves, points of known order, a
//!   `2^e` basis and its pairing;
//! - `<set>_challenge.tsv`: challenge scalars from an independent script.
//!
//! The files stay in `prism-verify/testdata`, where the scripts write
//! them, and are read from there.
//!
//! [`WithoutCorpus`] is such a set, and [`SageFixtures`] is what it
//! states: its files, and the curves of its file that tests name. A curve
//! is named by what it is used for. What makes it fit for that use is a
//! property of its coefficient, held by a test and not by the constant:
//! the tests of this module hold the branch of each, and the refusal
//! tests of `prism-verify` hold which coefficients the model check passes.
//! Those properties are accidents of each set's walk, so they are data of
//! the set and not a rule.

use crate::curve_table::{CurveNumber, CurveTable};
use prism_field::{ParameterSet, P324, P664};

/// One coefficient of a curve file that the verifier takes as a response
/// curve: the permitted model of its class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    pub curve: CurveNumber,
    /// Whether it is the negation of the file's coefficient that is
    /// permitted, and not the coefficient itself.
    pub negated: bool,
}

/// What a set without a corpus states about its fixtures.
#[derive(Clone, Copy, Debug)]
pub struct SageFixtures {
    /// `<set>_field.tsv`.
    pub field_table: &'static str,
    /// The seed of the generator the field file's random rows are drawn
    /// from: `PRISM` and the exponent in ASCII, as the script computes it.
    pub field_seed: u64,
    /// The curve file's name, as the count records cite it.
    pub curve_file: &'static str,
    /// `<set>_curves.tsv`. Read through [`curve_table`].
    curve_table: &'static str,
    /// How many curves the Sage script writes for this set.
    pub curves: usize,
    /// `<set>_challenge.tsv`.
    pub challenge_table: &'static str,
    /// A curve whose coefficient is a square, so the basis search takes
    /// its square branch.
    pub square_branch_curve: CurveNumber,
    /// A curve whose coefficient is not a square, so the basis search
    /// takes its other branch.
    pub other_branch_curve: CurveNumber,
    /// A curve whose own coefficient is a permitted response model. It is
    /// one of [`Self::models`].
    pub model_curve: CurveNumber,
    /// Every coefficient of the file, or its negation, that is a
    /// permitted response model, in file order.
    pub models: &'static [Model],
    /// The message the refusal tests and the operation counts hash into
    /// the challenge. Not a committed vector's message. The salt counters
    /// pinned in the tests are for this message under the curves named
    /// here, and the counts are for the same message and curve, so the
    /// two cannot drift.
    pub refusal_message: &'static [u8],
}

impl SageFixtures {
    /// One curve from each branch of the basis search, with whether its
    /// coefficient is a square.
    pub const fn branch_curves(&self) -> [(CurveNumber, bool); 2] {
        [
            (self.square_branch_curve, true),
            (self.other_branch_curve, false),
        ]
    }
}

/// A parameter set with no committed vectors, and so with Sage fixtures.
pub trait WithoutCorpus: ParameterSet {
    const FIXTURES: SageFixtures;
}

/// The committed curve file of `L`. It is the Sage script's output, so a
/// file this does not read, or one with another number of curves, is a
/// broken fixture.
pub fn curve_table<L: WithoutCorpus>() -> CurveTable {
    let fixtures = L::FIXTURES;
    let table = CurveTable::parse(fixtures.curve_table, L::COFACTOR)
        .unwrap_or_else(|reason| panic!("{}: {reason}", fixtures.curve_file));
    assert_eq!(
        table.rows().len(),
        fixtures.curves,
        "{}",
        fixtures.curve_file
    );
    table
}

impl WithoutCorpus for P324 {
    const FIXTURES: SageFixtures = SageFixtures {
        field_table: include_str!("../../prism-verify/testdata/p324_field.tsv"),
        field_seed: u64::from_be_bytes(*b"PRISM324"),
        curve_file: "p324_curves.tsv",
        curve_table: include_str!("../../prism-verify/testdata/p324_curves.tsv"),
        curves: 8,
        challenge_table: include_str!("../../prism-verify/testdata/p324_challenge.tsv"),
        square_branch_curve: CurveNumber::new(1),
        other_branch_curve: CurveNumber::new(7),
        model_curve: CurveNumber::new(7),
        models: &[
            Model {
                curve: CurveNumber::new(5),
                negated: true,
            },
            Model {
                curve: CurveNumber::new(7),
                negated: false,
            },
        ],
        refusal_message: b"shekyl-prism-p324-refusals",
    };
}

impl WithoutCorpus for P664 {
    const FIXTURES: SageFixtures = SageFixtures {
        field_table: include_str!("../../prism-verify/testdata/p664_field.tsv"),
        field_seed: u64::from_be_bytes(*b"PRISM664"),
        curve_file: "p664_curves.tsv",
        curve_table: include_str!("../../prism-verify/testdata/p664_curves.tsv"),
        curves: 8,
        challenge_table: include_str!("../../prism-verify/testdata/p664_challenge.tsv"),
        square_branch_curve: CurveNumber::new(1),
        other_branch_curve: CurveNumber::new(3),
        model_curve: CurveNumber::new(3),
        models: &[
            Model {
                curve: CurveNumber::new(3),
                negated: false,
            },
            Model {
                curve: CurveNumber::new(4),
                negated: true,
            },
        ],
        refusal_message: b"shekyl-prism-p664-refusals",
    };
}

#[cfg(test)]
mod tests {
    use super::{curve_table, Model, WithoutCorpus};
    use prism_field::{P324, P664};

    /// What every set's fixtures must satisfy, whatever its walk gave.
    fn the_fixtures_are_consistent<L: WithoutCorpus>() {
        let fixtures = L::FIXTURES;
        let table = curve_table::<L>();
        let numbers: Vec<String> = table
            .rows()
            .iter()
            .map(|row| row.number.to_string())
            .collect();
        let expected: Vec<String> = (1..=fixtures.curves)
            .map(|number| number.to_string())
            .collect();
        assert_eq!(numbers, expected, "{}", fixtures.curve_file);
        for (number, square) in fixtures.branch_curves() {
            assert_eq!(
                table.curve(number).coefficient_is_square,
                square,
                "{} curve {number}",
                L::NAME
            );
        }
        // The model curve is a model as it stands, and every model is a
        // curve of the file, each once.
        assert!(fixtures.models.contains(&Model {
            curve: fixtures.model_curve,
            negated: false,
        }));
        for (position, model) in fixtures.models.iter().enumerate() {
            assert!(table.rows().iter().any(|row| row.number == model.curve));
            assert!(
                !fixtures.models[..position]
                    .iter()
                    .any(|earlier| earlier.curve == model.curve),
                "{} curve {} is listed twice",
                L::NAME,
                model.curve
            );
        }
        // The file names and the hashed message carry the set's name.
        assert_eq!(fixtures.curve_file, format!("{}_curves.tsv", L::NAME));
        let message = format!("shekyl-prism-{}-refusals", L::NAME);
        assert_eq!(fixtures.refusal_message, message.as_bytes());
        // The seed is `PRISM` and the exponent, as the Sage script has it.
        let seed = format!("PRISM{}", L::E);
        assert_eq!(fixtures.field_seed.to_be_bytes(), seed.as_bytes());
        assert!(fixtures.field_table.contains(&format!("for {}:", L::NAME)));
        assert!(fixtures.challenge_table.contains(L::NAME));
    }

    #[test]
    fn the_fixtures_of_every_set_without_a_corpus_are_consistent() {
        the_fixtures_are_consistent::<P324>();
        the_fixtures_are_consistent::<P664>();
    }

    #[test]
    fn the_seed_of_p324_is_the_one_its_files_were_drawn_from() {
        assert_eq!(
            <P324 as WithoutCorpus>::FIXTURES.field_seed,
            0x5052_4953_4D33_3234
        );
    }
}
