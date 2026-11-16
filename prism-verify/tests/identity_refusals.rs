// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The committed refusal vectors of `prism_vectors::refusals`.
//!
//! Three things are held, each on its own:
//!
//! - the file is, byte for byte, what the builder of
//!   `support/refusal_vectors.rs` renders, and each row the vectors
//!   crate's reader returns is the builder's row;
//! - the verifier returns the pinned error on the committed bytes, read
//!   through that reader, which is what `prism-bench` times;
//! - the verdict of each case is the one stated here, so a file that was
//!   edited cannot relabel a row.

#[path = "support/identity.rs"]
mod identity;
#[path = "support/refusal_vectors.rs"]
mod refusal_vectors;
#[path = "support/verdict.rs"]
mod verdict;

use prism_vectors::refusals::{row, rows, Verdict, TEXT};
use prism_verify::{verify_set, Set, VerifyError};
use verdict::error_of;

/// The verdict of each case, by the name it has in the records, and the
/// error the verifier returns for it. Stated here and not read from the
/// file or from the helper.
const VERDICTS: [(&str, Verdict, VerifyError); 2] = [
    (
        "identity-response-q-negated",
        Verdict::Kernel,
        VerifyError::Kernel,
    ),
    (
        "identity-response-q-difference",
        Verdict::Codomain,
        VerifyError::Codomain,
    ),
];

/// The set a row names.
fn set_of(name: &str) -> Set {
    name.parse()
        .unwrap_or_else(|_| panic!("{name} is not a set"))
}

#[test]
fn the_file_is_what_the_builder_renders() {
    assert_eq!(TEXT, refusal_vectors::file());
}

#[test]
fn the_reader_returns_the_builders_rows() {
    let built = refusal_vectors::rows();
    let read = rows().expect("the committed file parses");
    assert_eq!(read.len(), built.len());
    for (read, built) in read.iter().zip(&built) {
        assert_eq!(set_of(read.set), built.set);
        assert_eq!(read.vector, built.vector);
        assert_eq!(read.case, built.form.name());
        assert_eq!(read.verdict, built.verdict());
        assert_eq!(error_of(read.verdict), built.error());
        assert_eq!(read.salt_counter, built.salt_counter);
        assert_eq!(read.message, built.message);
        assert_eq!(read.public_key, built.public_key);
        assert_eq!(read.signature, built.signature);
    }
}

#[test]
fn every_set_has_each_case_once_with_its_stated_verdict() {
    let read = rows().expect("the committed file parses");
    assert_eq!(read.len(), Set::ALL.len() * VERDICTS.len());
    for set in Set::ALL {
        for (case, verdict, error) in VERDICTS {
            let found: Vec<_> = read
                .iter()
                .filter(|row| set_of(row.set) == set && row.case == case)
                .collect();
            let [found] = found.as_slice() else {
                panic!("{set} {case}: {} rows", found.len());
            };
            assert_eq!(found.verdict, verdict, "{set} {case}");
            assert_eq!(error_of(found.verdict), error, "{set} {case}");
            assert_eq!(row(set.name(), verdict), Ok(*found), "{set} {case}");
        }
    }
}

#[test]
fn the_verifier_returns_the_pinned_error_on_the_committed_bytes() {
    for row in rows().expect("the committed file parses") {
        assert_eq!(
            verify_set(
                set_of(row.set),
                &row.public_key,
                &row.message,
                &row.signature
            ),
            Err(error_of(row.verdict)),
            "{} {}",
            row.set,
            row.case
        );
    }
}
