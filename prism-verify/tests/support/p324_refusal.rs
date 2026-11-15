// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The message the `p324` refusal tests and the operation counts share.
//!
//! The salt counters pinned in the tests are for this message under the
//! curves `curve_table::p324` names. The counts are for the same message
//! and curve, so the two cannot drift.

/// Hashed into the challenge. Not a committed vector's message.
pub const MESSAGE: &[u8] = b"shekyl-prism-p324-refusals";
