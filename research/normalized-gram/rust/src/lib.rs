// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//! Fixed-limb primitives from the rust-lagrange-ct research prototype.
//! The exp64 reducer carries its Gram matrix and uses masked power-of-two scaling.
//! research/normalized-gram adds `ngram` (normalized class Gram) and the
//! 1db834d originals (`orig_*`) as the baseline.
pub mod ct;
pub mod exact;
pub mod sets;
pub mod zi;
pub mod zi_exp;
pub mod ngram;
pub mod orig_ct;
pub mod orig_zi;
pub mod orig_exp;
