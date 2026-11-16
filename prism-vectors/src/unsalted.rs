// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The archived unsalted vectors: the corpus as it was before the
//! challenge took a salt.
//!
//! These are not vectors of the current protocol, and no verifier of it
//! accepts them. They are kept because the comparison of the salted and
//! unsalted protocols (`prism-compare`, `docs/comparison/`) replays them.
//! The layout mirrors the current corpus: six primary files and an
//! `accept` directory, with the same file names.

use std::path::PathBuf;

use crate::corpus;
use crate::Group;

/// The directory of the archive, in a checkout of this repository.
pub fn dir() -> PathBuf {
    Group::Compare.dir().join("unsalted")
}

/// Every archived vector, the six primary files and the accept directory,
/// sorted by path. The layout is the corpus's.
///
/// # Panics
///
/// If the archive's accept directory cannot be read.
pub fn files() -> Vec<PathBuf> {
    corpus::layout(&dir())
}

#[cfg(test)]
mod tests {
    use super::{dir, files};
    use crate::corpus::{self, Primary, HONEST_VECTORS};

    #[test]
    fn the_archive_mirrors_the_corpus() {
        let files = files();
        assert_eq!(files.len(), HONEST_VECTORS);
        for primary in Primary::ALL {
            let path = dir().join(primary.file_name());
            assert!(files.contains(&path), "{primary:?}");
            let text = std::fs::read_to_string(&path).unwrap();
            let vector = corpus::Vector::new(&text);
            assert!(vector.curve_and_points().is_ok(), "{primary:?}");
            assert_eq!(vector.salt(), Err(corpus::Malformed::Missing("salt")));
            assert!(vector.field("j").is_some(), "{primary:?}");
        }
    }
}
