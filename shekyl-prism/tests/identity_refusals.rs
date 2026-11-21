// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The committed refusal vectors of `prism_vectors::refusals`, through
//! both entries.
//!
//! The verifier returns the pinned error on the committed bytes, read
//! through the vectors crate's reader, which is what `prism-bench` times.
//! That the file is what the builder renders, and that no row can be
//! relabelled, is held where the builder is
//! (`prism-testkit/tests/refusal_vectors.rs`). `prism-verify` holds the
//! same rows through `verify_set` alone (`prism-verify/tests/committed.rs`).

#[path = "support/entries.rs"]
mod entries;

use entries::verify_set;
use prism_testkit::error_of;
use prism_vectors::refusals::rows;
use prism_verify::set::Set;

/// The set a row names.
fn set_of(name: &str) -> Set {
    name.parse()
        .unwrap_or_else(|_| panic!("{name} is not a set"))
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
