// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The committed refusal vectors of `testdata/identity_refusals.tsv`.
//!
//! Three things are held, each on its own:
//!
//! - the file is, byte for byte, what the builder of
//!   `support/refusal_vectors.rs` renders, and each row the library's
//!   reader returns is the builder's row;
//! - the verifier returns the pinned error on the committed bytes, read
//!   through that reader, which is what `prism-bench` times;
//! - the verdict of each case is the one stated here, so a file that was
//!   edited cannot relabel a row.

#[path = "support/identity.rs"]
mod identity;
#[path = "support/refusal_vectors.rs"]
mod refusal_vectors;

use prism_verify::reference::{refusal, refusals};
use prism_verify::{verify_set, Set, VerifyError};

/// The committed file, as text.
const COMMITTED: &str = include_str!("../testdata/identity_refusals.tsv");

/// The verdict of each case, by the name it has in the records.
const VERDICTS: [(&str, VerifyError); 2] = [
    ("identity-response-q-negated", VerifyError::Kernel),
    ("identity-response-q-difference", VerifyError::Codomain),
];

#[test]
fn the_file_is_what_the_builder_renders() {
    assert_eq!(COMMITTED, refusal_vectors::file());
}

#[test]
fn the_reader_returns_the_builders_rows() {
    let built = refusal_vectors::rows();
    let read = refusals().expect("the committed file parses");
    assert_eq!(read.len(), built.len());
    for (read, built) in read.iter().zip(&built) {
        assert_eq!(read.set, built.set);
        assert_eq!(read.vector, built.vector);
        assert_eq!(read.case, built.form.name());
        assert_eq!(read.error, built.error());
        assert_eq!(read.salt_counter, built.salt_counter);
        assert_eq!(read.message, built.message);
        assert_eq!(read.public_key, built.public_key);
        assert_eq!(read.signature, built.signature);
    }
}

#[test]
fn every_set_has_each_case_once_with_its_stated_verdict() {
    let read = refusals().expect("the committed file parses");
    assert_eq!(read.len(), Set::ALL.len() * VERDICTS.len());
    for set in Set::ALL {
        for (case, error) in VERDICTS {
            let rows: Vec<_> = read
                .iter()
                .filter(|row| row.set == set && row.case == case)
                .collect();
            let [row] = rows.as_slice() else {
                panic!("{set} {case}: {} rows", rows.len());
            };
            assert_eq!(row.error, error, "{set} {case}");
            assert_eq!(refusal(set, error), Ok(*row), "{set} {case}");
        }
    }
}

#[test]
fn the_verifier_returns_the_pinned_error_on_the_committed_bytes() {
    for row in refusals().expect("the committed file parses") {
        assert_eq!(
            verify_set(row.set, &row.public_key, &row.message, &row.signature),
            Err(row.error),
            "{} {}",
            row.set,
            row.case
        );
    }
}
