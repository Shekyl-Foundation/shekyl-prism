// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//! Fixed-limb primitives from the rust-lagrange-ct research prototype.
//! The exp64 reducer carries its Gram matrix and uses masked power-of-two scaling.
pub mod ct;
pub mod exact;
pub mod sets;
pub mod zi;
pub mod zi_exp;
pub mod native;
