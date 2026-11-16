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

use crate::Group;

/// The directory of the archive, in a checkout of this repository.
pub fn dir() -> PathBuf {
    Group::Compare.dir().join("unsalted")
}

#[cfg(test)]
mod tests {
    use super::dir;
    use crate::corpus::{Primary, HONEST_VECTORS};

    #[test]
    fn the_archive_mirrors_the_corpus() {
        let count = |path: std::path::PathBuf| {
            std::fs::read_dir(path)
                .expect("an archive directory")
                .filter(|entry| {
                    entry
                        .as_ref()
                        .expect("an entry")
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "txt")
                })
                .count()
        };
        assert_eq!(count(dir()), Primary::ALL.len());
        assert_eq!(count(dir()) + count(dir().join("accept")), HONEST_VECTORS);
        for primary in Primary::ALL {
            assert!(dir().join(primary.file_name()).is_file(), "{primary:?}");
        }
    }
}
