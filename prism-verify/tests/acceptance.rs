// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The honest corpus and the negatives derived from it, through both
//! entries: every input here goes through `support/entries.rs`, which
//! runs the byte-level `verify` and the typed interface and holds the
//! rule that relates them. The two walks pin how many inputs there are and
//! that the entries never differ on them.

#[path = "support/entries.rs"]
mod entries;
mod support;
#[path = "support/verdict.rs"]
mod verdict;

use entries::{both, Outcome, Tally};
use prism_field::{ParameterSet, Salt, P248, P376, P500};
use prism_vectors::corpus::WithCorpus;
use prism_verify::{ChallengeScalar, CommittedSet, KeyIdentity};
use std::fs;
use support::{files, negatives, primary, set, vector};
use verdict::error_of;

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
const NEGATIVES: usize = 2_724;
/// The committed refusal vectors of a set with a corpus are the two
/// identity-response negatives of the vector its timing sample is: the
/// cases the count records hold under the same names. So a row of
/// the committed identity refusals and the row of that case in
/// `docs/benchmarks/negative-verdicts.tsv` are one input.
fn committed_refusals_are_the_counted_negatives<L: WithCorpus>(set: CommittedSet) {
    let [primary, _] = L::PRIMARY;
    let cases = negatives::<L>(primary.text());
    let rows: Vec<_> = prism_vectors::refusals::rows()
        .expect("the committed file parses")
        .iter()
        .filter(|row| row.set == set.set().name())
        .collect();
    assert_eq!(rows.len(), 2, "{set}");
    for row in rows {
        let case = cases
            .iter()
            .find(|case| case.name == row.case)
            .unwrap_or_else(|| panic!("{set}: no negative named {}", row.case));
        assert_eq!(case.key, row.public_key, "{set} {}", row.case);
        assert_eq!(case.message, row.message, "{set} {}", row.case);
        assert_eq!(case.signature, row.signature, "{set} {}", row.case);
        assert_eq!(case.allowed, [error_of(row.verdict)], "{set} {}", row.case);
    }
}
#[test]
fn the_committed_refusal_vectors_are_the_counted_negatives() {
    committed_refusals_are_the_counted_negatives::<P248>(CommittedSet::P248);
    committed_refusals_are_the_counted_negatives::<P376>(CommittedSet::P376);
    committed_refusals_are_the_counted_negatives::<P500>(CommittedSet::P500);
}
/// Prime and composite degrees among the one-bit salt mutations of a vector.
fn salt_flips<L: ParameterSet>(text: &str) -> (usize, usize) {
    let (pk, msg, r) = vector::<L>(text);
    let id = KeyIdentity::<L>::decode(&pk).unwrap();
    let mut prime = 0;
    for bit in 0..L::SALT_LEN * 8 {
        let mut bytes = r.salt.as_bytes().to_vec();
        bytes[bit / 8] ^= 1 << (bit % 8);
        let salt = L::Salt::from_bytes(&bytes).unwrap();
        prime += usize::from(ChallengeScalar::derive(&id, &msg, salt).degree_is_prime::<L>());
    }
    (prime, L::SALT_LEN * 8 - prime)
}
/// The split the salt-mutation negative records: over every committed
/// signature, how many one-bit salt changes land on a prime degree (refused
/// as `Image`) and how many on a composite one (refused as `Composite`).
/// `negative` checks the verdicts themselves on the six primary vectors.
#[test]
fn one_bit_salt_mutations_split_between_composite_and_prime() {
    let mut split = [(0usize, 0usize); 3];
    for path in files() {
        let text = fs::read_to_string(&path).unwrap();
        let (row, (prime, composite)) = match set(&text) {
            CommittedSet::P248 => (0, salt_flips::<P248>(&text)),
            CommittedSet::P376 => (1, salt_flips::<P376>(&text)),
            CommittedSet::P500 => (2, salt_flips::<P500>(&text)),
        };
        split[row].0 += prime;
        split[row].1 += composite;
    }
    for (name, (prime, composite)) in ["I", "III", "V"].iter().zip(split) {
        eprintln!("salt-bit-flips set={name} prime={prime} composite={composite}");
        // A salt bit that did not reach the hash would leave every degree
        // prime; a hash that ignored the salt entirely is the same failure.
        assert!(composite > prime, "set {name}");
    }
}
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
