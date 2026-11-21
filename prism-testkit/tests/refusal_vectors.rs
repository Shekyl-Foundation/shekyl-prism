// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The committed refusal vectors of `prism_vectors::refusals`, against
//! the builder that renders them.
//!
//! No test here calls a verifier. Each holds built data to committed
//! data:
//!
//! - the file is, byte for byte, what `prism_testkit::refusal_vectors`
//!   renders, and each row the vectors crate's reader returns is the
//!   builder's row;
//! - the verdict of each case is the one stated here, so a file that was
//!   edited cannot relabel a row;
//! - the rows of a set with a corpus are the two identity-response
//!   negatives of that set's first primary vector, the cases the count
//!   records hold under the same names.
//!
//! That a verifier returns the pinned error on these bytes is held in
//! `prism-verify/tests/committed.rs` and, through both entries, in
//! `shekyl-prism/tests/identity_refusals.rs`.

use prism_field::{P248, P376, P500};
use prism_testkit::{error_of, negatives, refusal_vectors};
use prism_vectors::corpus::WithCorpus;
use prism_vectors::refusals::{row, rows, Verdict, TEXT};
use prism_verify::{
    set::{CommittedSet, Set},
    VerifyError,
};

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
