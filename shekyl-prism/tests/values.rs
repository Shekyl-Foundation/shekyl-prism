// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! What a `PublicKey` and a `Signature` are as values: their encoding
//! round-trips, and equality and hashing are those of the encoding.
//!
//! That the typed path accepts and refuses what the byte-level entry
//! does is held in `prism-verify/tests/support/entries.rs`, beside the
//! generator of the negatives, on every input that crate's tests verify.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};

use prism_vectors::corpus::{files, Primary, Vector, HONEST_VECTORS};
use shekyl_prism::{ParameterSet, PublicKey, Signature, VerifyError, P248, P376, P500};

fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn round_trip<S: ParameterSet>(vector: &Vector<'_>) {
    let key_bytes = vector.public_key().unwrap();
    let signature_bytes = vector.signature().unwrap();
    let key = PublicKey::<S>::from_bytes(&key_bytes).unwrap();
    let signature = Signature::<S>::from_bytes(&signature_bytes).unwrap();
    assert_eq!(key.as_bytes(), key_bytes);
    assert_eq!(key.to_bytes(), key_bytes);
    assert_eq!(signature.as_bytes(), signature_bytes);
    assert_eq!(signature.to_bytes(), signature_bytes);
    assert_eq!(key_bytes.len(), PublicKey::<S>::LEN);
    assert_eq!(signature_bytes.len(), Signature::<S>::LEN);
    assert_eq!(PublicKey::<S>::from_bytes(&key.to_bytes()).unwrap(), key);
    assert_eq!(
        Signature::<S>::from_bytes(&signature.to_bytes()).unwrap(),
        signature
    );
    assert_eq!(key.verify(vector.message().unwrap(), &signature), Ok(()));
    // A clone is the same value and verifies the same.
    assert_eq!(key.clone(), key);
    assert_eq!(
        key.clone()
            .verify(vector.message().unwrap(), &signature.clone()),
        Ok(())
    );
}

#[test]
fn every_honest_key_and_signature_round_trips() {
    let files = files();
    assert_eq!(files.len(), HONEST_VECTORS);
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        let vector = Vector::new(&text);
        match vector.level().unwrap() {
            "NIST-I" => round_trip::<P248>(&vector),
            "NIST-III" => round_trip::<P376>(&vector),
            "NIST-V" => round_trip::<P500>(&vector),
            other => panic!("{}: level {other}", path.display()),
        }
    }
}

#[test]
fn equality_and_hash_are_those_of_the_encoding() {
    let first = Primary::Lvl1.vector();
    let second = Primary::Lvl1b.vector();
    let key = |v: &Vector<'_>| PublicKey::<P248>::from_bytes(&v.public_key().unwrap()).unwrap();
    let signature =
        |v: &Vector<'_>| Signature::<P248>::from_bytes(&v.signature().unwrap()).unwrap();

    // Two values made from the same bytes, separately.
    assert_eq!(key(&first), key(&first));
    assert_eq!(hash_of(&key(&first)), hash_of(&key(&first)));
    assert_eq!(signature(&first), signature(&first));
    assert_eq!(hash_of(&signature(&first)), hash_of(&signature(&first)));
    // A value hashes as its bytes do.
    assert_eq!(
        hash_of(&key(&first)),
        hash_of(&first.public_key().unwrap().as_slice())
    );
    assert_eq!(
        hash_of(&signature(&first)),
        hash_of(&first.signature().unwrap().as_slice())
    );
    // Different bytes, different values.
    assert_ne!(first.public_key().unwrap(), second.public_key().unwrap());
    assert_ne!(key(&first), key(&second));
    assert_ne!(signature(&first), signature(&second));
}

#[test]
fn debug_names_the_set_and_gives_the_encoding() {
    let vector = Primary::Lvl1.vector();
    let key = PublicKey::<P248>::from_bytes(&vector.public_key().unwrap()).unwrap();
    let text = format!("{key:?}");
    let hex: String = key.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(text, format!("PublicKey<p248>({hex})"));
    let signature = Signature::<P248>::from_bytes(&vector.signature().unwrap()).unwrap();
    assert!(format!("{signature:?}").starts_with("Signature<p248>("));
}

/// A key of one set's length is not a key of another set: the type
/// parameter chooses the checks, and the first is the length.
#[test]
fn bytes_of_another_set_are_refused_by_length() {
    let vector = Primary::Lvl1.vector();
    assert_eq!(
        PublicKey::<P376>::from_bytes(&vector.public_key().unwrap()).unwrap_err(),
        VerifyError::Length
    );
    assert_eq!(
        Signature::<P500>::from_bytes(&vector.signature().unwrap()).unwrap_err(),
        VerifyError::Length
    );
}
