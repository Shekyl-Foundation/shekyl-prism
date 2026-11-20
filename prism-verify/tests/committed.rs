// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The byte-level entry on the committed data, with nothing above this
//! crate built.
//!
//! `verify` accepts each of the 113 committed honest signatures, and
//! `verify_set` returns the pinned error on each row of the committed
//! identity refusals. Both need only `prism-vectors`. The negatives
//! derived from the honest vectors, the refusals built from public data
//! and the typed interface are tested above this crate, in
//! `shekyl-prism/tests/`, with the inputs of `prism-testkit`: the
//! builders use this crate's types, so a test that needs them cannot live
//! here without a dependency loop.

use prism_field::{ParameterSet, Salt, P248, P376, P500};
use prism_vectors::corpus::{files, Vector, HONEST_VECTORS};
use prism_vectors::refusals::rows;
use prism_verify::{
    challenge::ChallengeScalar,
    identity::KeyIdentity,
    set::{verify_set, CommittedSet, Set},
    signature::Response,
    verify, VerifyError,
};
use std::fs;

/// Public key, message and signature bytes of one vector file.
fn inputs(text: &str) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let vector = Vector::new(text);
    (
        vector.public_key().expect("the vector's public key"),
        vector.message().expect("the vector's message").to_vec(),
        vector.signature().expect("the vector's signature"),
    )
}

/// The set a vector file names.
fn set(text: &str) -> CommittedSet {
    let level = Vector::new(text).level().expect("the vector's level");
    CommittedSet::from_label(level).expect("the vector's level is a set")
}

#[test]
fn verify_accepts_every_committed_honest_signature() {
    let files = files();
    assert_eq!(files.len(), HONEST_VECTORS);
    assert_eq!(
        files.len(),
        113,
        "update the reviewed corpus count when adding fixtures"
    );
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        let (key, message, signature) = inputs(&text);
        let verdict = match set(&text) {
            CommittedSet::P248 => verify::<P248>(&key, &message, &signature),
            CommittedSet::P376 => verify::<P376>(&key, &message, &signature),
            CommittedSet::P500 => verify::<P500>(&key, &message, &signature),
        };
        assert_eq!(verdict, Ok(()), "{}", path.display());
    }
}

/// The error of each case of the committed identity refusals, by the name
/// the case has in the file. Stated here and not read from the file's
/// verdict column.
const PINNED: [(&str, VerifyError); 2] = [
    ("identity-response-q-negated", VerifyError::Kernel),
    ("identity-response-q-difference", VerifyError::Codomain),
];

#[test]
fn verify_returns_the_pinned_error_on_the_committed_identity_refusals() {
    let rows = rows().expect("the committed file parses");
    assert_eq!(rows.len(), Set::ALL.len() * PINNED.len());
    for row in rows {
        let set: Set = row.set.parse().expect("the row's set");
        let (_, error) = PINNED
            .into_iter()
            .find(|(case, _)| *case == row.case)
            .unwrap_or_else(|| panic!("no pinned error for {}", row.case));
        assert_eq!(
            verify_set(set, &row.public_key, &row.message, &row.signature),
            Err(error),
            "{} {}",
            row.set,
            row.case
        );
    }
}

/// Prime and composite degrees among the one-bit salt mutations of a vector.
fn salt_flips<L: ParameterSet>(text: &str) -> (usize, usize) {
    let (pk, msg, signature) = inputs(text);
    let r = Response::<L>::decode(&signature).unwrap();
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
/// The verdicts themselves are checked on the six primary vectors, in
/// `shekyl-prism/tests/acceptance.rs`.
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
