// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

mod support;

use prism_field::{ParameterSet, Salt, P248, P376, P500};
use prism_verify::{verify, ChallengeScalar, CommittedSet, KeyIdentity};
use std::fs;
use support::{files, negatives, primary, set, vector};

fn positive<L: ParameterSet>(text: &str) {
    let (pk, msg, response) = vector::<L>(text);
    assert_eq!(verify::<L>(&pk, &msg, &response.encode()), Ok(()));
}
#[test]
fn every_committed_honest_signature_is_accepted() {
    let files = files();
    assert_eq!(
        files.len(),
        113,
        "update the reviewed corpus count when adding fixtures"
    );
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        eprintln!("accept {}", path.display());
        match set(&text) {
            CommittedSet::P248 => positive::<P248>(&text),
            CommittedSet::P376 => positive::<P376>(&text),
            CommittedSet::P500 => positive::<P500>(&text),
        }
    }
}
fn negative<L: ParameterSet>(text: &str) {
    for case in negatives::<L>(text) {
        let verdict = verify::<L>(&case.key, &case.message, &case.signature);
        assert!(
            matches!(verdict, Err(e) if case.allowed.contains(&e)),
            "{}: {verdict:?}, expected one of {:?}",
            case.name,
            case.allowed
        );
    }
}
/// The committed refusal vectors of a set with a corpus are the two
/// identity-response negatives of the vector its timing sample is: the
/// cases the count records hold under the same names. So a row of
/// `testdata/identity_refusals.tsv` and the row of that case in
/// `docs/benchmarks/negative-verdicts.tsv` are one input.
fn committed_refusals_are_the_counted_negatives<L: ParameterSet>(set: CommittedSet, text: &str) {
    let cases = negatives::<L>(text);
    let rows: Vec<_> = prism_verify::reference::refusals()
        .expect("the committed file parses")
        .iter()
        .filter(|row| row.set == set.set())
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
        assert_eq!(case.allowed, [row.error], "{set} {}", row.case);
    }
}
#[test]
fn the_committed_refusal_vectors_are_the_counted_negatives() {
    let text = |name: &str| {
        fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata")
                .join(name),
        )
        .unwrap()
    };
    committed_refusals_are_the_counted_negatives::<P248>(CommittedSet::P248, &text("s5_lvl1.txt"));
    committed_refusals_are_the_counted_negatives::<P376>(CommittedSet::P376, &text("s5_lvl3.txt"));
    committed_refusals_are_the_counted_negatives::<P500>(CommittedSet::P500, &text("s5_lvl5.txt"));
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
    for path in primary() {
        let text = fs::read_to_string(path).unwrap();
        match set(&text) {
            CommittedSet::P248 => negative::<P248>(&text),
            CommittedSet::P376 => negative::<P376>(&text),
            CommittedSet::P500 => negative::<P500>(&text),
        }
    }
}
