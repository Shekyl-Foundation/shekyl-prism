// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The committed honest vectors, as the verifier's types.
//!
//! `prism-vectors` reads a vector file and does not know the verifier.
//! These two functions are the step from its text to a decoded
//! [`Response`] and a [`CommittedSet`]. The paths come from
//! `prism-vectors` unchanged.

use prism_field::ParameterSet;
use prism_vectors::corpus::{Primary, Vector};
use prism_verify::{CommittedSet, Response};
use std::path::PathBuf;

pub use prism_vectors::corpus::files;

/// Public key, message and decoded response of one vector file.
///
/// # Panics
///
/// If `text` is not a vector file of the set `L`.
pub fn vector<L: ParameterSet>(text: &str) -> (Vec<u8>, Vec<u8>, Response<L>) {
    let parsed = Vector::new(text);
    let public_key = parsed.public_key().expect("the vector's public key");
    let message = parsed.message().expect("the vector's message").to_vec();
    let signature = parsed.signature().expect("the vector's signature");
    (
        public_key,
        message,
        Response::decode(&signature).expect("the vector's signature decodes"),
    )
}

/// The set a vector file names.
///
/// # Panics
///
/// If `text` names no level, or one that is not a set with a corpus.
pub fn set(text: &str) -> CommittedSet {
    let level = Vector::new(text).level().expect("the vector's level");
    CommittedSet::from_label(level).expect("the vector's level is a set")
}

/// The six vectors the negatives are derived from, two keys per set.
pub fn primary() -> Vec<PathBuf> {
    Primary::ALL.map(Primary::path).to_vec()
}
