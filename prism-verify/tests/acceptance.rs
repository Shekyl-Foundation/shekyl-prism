// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

mod support;

use prism_field::{ParameterSet, Salt, P248, P376, P500};
use prism_verify::{verify, ChallengeScalar, KeyIdentity, Set};
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
            Set::P248 => positive::<P248>(&text),
            Set::P376 => positive::<P376>(&text),
            Set::P500 => positive::<P500>(&text),
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
            Set::P248 => (0, salt_flips::<P248>(&text)),
            Set::P376 => (1, salt_flips::<P376>(&text)),
            Set::P500 => (2, salt_flips::<P500>(&text)),
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
            Set::P248 => negative::<P248>(&text),
            Set::P376 => negative::<P376>(&text),
            Set::P500 => negative::<P500>(&text),
        }
    }
}
