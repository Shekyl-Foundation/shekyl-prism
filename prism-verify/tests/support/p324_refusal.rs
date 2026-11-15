// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The inputs the `p324` refusal tests and the operation counts share.
//!
//! The salt counters pinned in the tests are for this curve and this
//! message. The counts are for the same pair, so the two cannot drift.

/// Column `n` of `p324_curves.tsv`. Off the square branch of the basis
/// search, and a permitted response model.
pub const CURVE: &str = "7";

/// Hashed into the challenge. Not a committed vector's message.
pub const MESSAGE: &[u8] = b"shekyl-prism-p324-refusals";
