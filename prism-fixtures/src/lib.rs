// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Committed test fixtures, read into typed values.
//!
//! Tests in more than one crate read the same committed files: the unit
//! tests of `prism-isogeny`, the tests and the count reporter of
//! `prism-verify`, and the fuzz targets. This crate is where a file is
//! read, so that each of them asks for a value by name and none of them
//! parses.
//!
//! It depends on `prism-field` and nothing else of this workspace. The
//! unit tests of `prism-isogeny` use it, and a crate they use cannot
//! depend on `prism-isogeny` itself. So a fixture here is given as field
//! elements, and what a test builds from them, a point or a response, it
//! builds itself.
//!
//! No shipped crate depends on this one. It is a dev-dependency of the
//! crates whose tests use it, and a dependency of the fuzz crate.

#![forbid(unsafe_code)]

pub mod curve_table;
pub mod hex;
pub mod no_corpus;
