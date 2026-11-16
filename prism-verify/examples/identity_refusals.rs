// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Print the committed identity refusals, `identity_refusals.tsv` of
//! `prism-vectors`.
//!
//! `cargo run --release --locked -p prism-verify --example identity_refusals`
//! prints the file. `tests/identity_refusals.rs` holds the committed file
//! to this output, so the file is never edited by hand.

#[path = "../tests/support/identity.rs"]
mod identity;
#[path = "../tests/support/refusal_vectors.rs"]
mod refusal_vectors;
#[path = "../tests/support/verdict.rs"]
mod verdict;

fn main() {
    print!("{}", refusal_vectors::file());
}
