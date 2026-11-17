// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Print the committed identity refusals, `identity_refusals.tsv` of
//! `prism-vectors`.
//!
//! `cargo run --release --locked -p prism-testkit --example identity_refusals`
//! prints the file. `tests/refusal_vectors.rs` holds the committed file
//! to this output, so the file is never edited by hand.

fn main() {
    print!("{}", prism_testkit::refusal_vectors::file());
}
