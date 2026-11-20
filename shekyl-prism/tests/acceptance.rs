// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The honest corpus and the negatives derived from it, through both
//! entries: every input here goes through `support/entries.rs`, which
//! runs the byte-level `verify` and the typed interface and holds the
//! rule that relates them. The two walks pin how many inputs there are and
//! that the entries never differ on them.
//!
//! The inputs come from `prism-testkit`. `prism-verify` has a walk of its
//! own over the 113 honest vectors through `verify` alone
//! (`prism-verify/tests/committed.rs`), so that crate's test run stands
//! without this one.

#[path = "support/entries.rs"]
mod entries;

use entries::{both, Outcome, Tally};
use prism_field::{ParameterSet, P248, P376, P500};
use prism_testkit::{files, negatives, primary, set, vector};
use prism_verify::set::CommittedSet;
use std::fs;

fn positive<L: ParameterSet>(text: &str) -> Outcome {
    let (pk, msg, response) = vector::<L>(text);
    let outcome = both::<L>(&pk, &msg, &response.encode());
    assert_eq!(outcome.byte, Ok(()));
    assert_eq!(outcome.typed, Ok(()));
    outcome
}
#[test]
fn every_committed_honest_signature_is_accepted() {
    let files = files();
    assert_eq!(
        files.len(),
        113,
        "update the reviewed corpus count when adding fixtures"
    );
    let mut tally = Tally::default();
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        eprintln!("accept {}", path.display());
        tally.add(match set(&text) {
            CommittedSet::P248 => positive::<P248>(&text),
            CommittedSet::P376 => positive::<P376>(&text),
            CommittedSet::P500 => positive::<P500>(&text),
        });
    }
    // Both entries accept all 113.
    assert_eq!(
        tally,
        Tally {
            inputs: 113,
            accepted: 113,
            key_refused: 0,
            different: 0
        }
    );
}
fn negative<L: ParameterSet>(text: &str, tally: &mut Tally) {
    for case in negatives::<L>(text) {
        let outcome = both::<L>(&case.key, &case.message, &case.signature);
        let verdict = outcome.byte;
        assert!(
            matches!(verdict, Err(e) if case.allowed.contains(&e)),
            "{}: {verdict:?}, expected one of {:?}",
            case.name,
            case.allowed
        );
        tally.add(outcome);
    }
}
/// The negatives derived from the six primary vectors: the rows of
/// `docs/benchmarks/negative-verdicts.tsv` at the three sets with a
/// corpus.
const NEGATIVES: usize = 2_742;
#[test]
fn malleability_and_message_mutations_are_refused() {
    let mut tally = Tally::default();
    for path in primary() {
        let text = fs::read_to_string(path).unwrap();
        match set(&text) {
            CommittedSet::P248 => negative::<P248>(&text, &mut tally),
            CommittedSet::P376 => negative::<P376>(&text, &mut tally),
            CommittedSet::P500 => negative::<P500>(&text, &mut tally),
        }
    }
    // One negative of each primary vector has a faulty key: `key-zero`,
    // the starting curve, under the honest signature. The typed path
    // refuses those six at the key, and both entries say `Origin`. Every
    // other negative has a valid key and a faulty signature or relation.
    // No negative has both faults, so the entries never differ here;
    // `entries_agree.rs` builds the inputs on which they do.
    assert_eq!(
        tally,
        Tally {
            inputs: NEGATIVES,
            accepted: 0,
            key_refused: 6,
            different: 0
        }
    );
}
