// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The honest signatures: the six primary vectors and the accept corpus.
//!
//! A vector file is one signature under one public key, as lines of
//! `name value`. [`Vector`] reads the lines a verifier needs. The other
//! lines of a file (the challenge scalar `s`, the degree `q`, the key's
//! basis `Vk*`) are there for tests of intermediate values, and
//! [`Vector::field`] returns any of them by name.
//!
//! Three sets have honest signatures. [`WithCorpus`] names the two
//! primary vectors of each, and [`Primary`] is one of those six.
//! Every negative in the records is derived from a primary vector.
//!
//! The accept corpus is 107 more signatures in [`accept_dir`]. A test
//! that walks all 113 reads [`files`].

use std::path::PathBuf;

use prism_field::{ParameterSet, P248, P376, P500};

use crate::{hex, Group};

/// One of the six primary vectors: two keys at each set with a corpus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Primary {
    /// `s5_lvl1.txt`, at `p248`.
    Lvl1,
    /// `s5_lvl1b.txt`, a second key at `p248`.
    Lvl1b,
    /// `s5_lvl3.txt`, at `p376`.
    Lvl3,
    /// `s5_lvl3b.txt`, a second key at `p376`.
    Lvl3b,
    /// `s5_lvl5.txt`, at `p500`.
    Lvl5,
    /// `s5_lvl5b.txt`, a second key at `p500`.
    Lvl5b,
}

impl Primary {
    /// All six, the first key of each set before its second.
    pub const ALL: [Self; 6] = [
        Self::Lvl1,
        Self::Lvl1b,
        Self::Lvl3,
        Self::Lvl3b,
        Self::Lvl5,
        Self::Lvl5b,
    ];

    /// The file's name, as the records cite it.
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Lvl1 => "s5_lvl1.txt",
            Self::Lvl1b => "s5_lvl1b.txt",
            Self::Lvl3 => "s5_lvl3.txt",
            Self::Lvl3b => "s5_lvl3b.txt",
            Self::Lvl5 => "s5_lvl5.txt",
            Self::Lvl5b => "s5_lvl5b.txt",
        }
    }

    /// The file, as text.
    pub const fn text(self) -> &'static str {
        match self {
            Self::Lvl1 => verify_data!("s5_lvl1.txt"),
            Self::Lvl1b => verify_data!("s5_lvl1b.txt"),
            Self::Lvl3 => verify_data!("s5_lvl3.txt"),
            Self::Lvl3b => verify_data!("s5_lvl3b.txt"),
            Self::Lvl5 => verify_data!("s5_lvl5.txt"),
            Self::Lvl5b => verify_data!("s5_lvl5b.txt"),
        }
    }

    /// The file, read.
    pub const fn vector(self) -> Vector<'static> {
        Vector::new(self.text())
    }

    /// Where the file is in a checkout of this repository.
    pub fn path(self) -> PathBuf {
        dir().join(self.file_name())
    }
}

/// A parameter set that has honest signatures.
pub trait WithCorpus: ParameterSet {
    /// The set's two primary vectors. The first is the one a timing run
    /// verifies.
    const PRIMARY: [Primary; 2];
}

impl WithCorpus for P248 {
    const PRIMARY: [Primary; 2] = [Primary::Lvl1, Primary::Lvl1b];
}

impl WithCorpus for P376 {
    const PRIMARY: [Primary; 2] = [Primary::Lvl3, Primary::Lvl3b];
}

impl WithCorpus for P500 {
    const PRIMARY: [Primary; 2] = [Primary::Lvl5, Primary::Lvl5b];
}

/// Why a vector file could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Malformed {
    /// The file has no line of this name.
    Missing(&'static str),
    /// The line of this name is not hex in whole bytes.
    Hex(&'static str),
}

impl core::fmt::Display for Malformed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "the vector has no `{name}` line"),
            Self::Hex(name) => write!(f, "the vector's `{name}` line is not hex"),
        }
    }
}

impl std::error::Error for Malformed {}

/// The lines of a signature in the order a verifier reads its bytes: the
/// response curve, the two response points, and then the salt.
const SIGNATURE_ELEMENTS: [&str; 5] = ["E", "Px", "Py", "Qx", "Qy"];

/// One vector file: an honest signature under a public key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vector<'a> {
    text: &'a str,
}

impl<'a> Vector<'a> {
    /// Read `text` as a vector file. Nothing is checked until a line is
    /// asked for.
    pub const fn new(text: &'a str) -> Self {
        Self { text }
    }

    /// The whole file.
    pub const fn text(&self) -> &'a str {
        self.text
    }

    /// The value of the line named `name`, without the name.
    pub fn field(&self, name: &str) -> Option<&'a str> {
        self.text
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix(' '))
    }

    fn required(&self, name: &'static str) -> Result<&'a str, Malformed> {
        self.field(name).ok_or(Malformed::Missing(name))
    }

    fn bytes(&self, name: &'static str) -> Result<Vec<u8>, Malformed> {
        hex::decode(self.required(name)?).ok_or(Malformed::Hex(name))
    }

    /// The opaque string the file's `level` line carries. It names the
    /// set to the research signer's seed streams; a verifier maps it to a
    /// set and does not otherwise read it.
    pub fn level(&self) -> Result<&'a str, Malformed> {
        self.required("level")
    }

    /// The message that was signed: the bytes of the `message` line.
    pub fn message(&self) -> Result<&'a [u8], Malformed> {
        self.required("message").map(str::as_bytes)
    }

    /// The public key: the `A` line, in the set's canonical encoding.
    pub fn public_key(&self) -> Result<Vec<u8>, Malformed> {
        self.bytes("A")
    }

    /// The salt alone.
    pub fn salt(&self) -> Result<Vec<u8>, Malformed> {
        self.bytes("salt")
    }

    /// The signature as a verifier takes it: the response curve, `x(P)`,
    /// `y(P)`, `x(Q)`, `y(Q)` and the salt, concatenated.
    pub fn signature(&self) -> Result<Vec<u8>, Malformed> {
        let mut signature = Vec::new();
        for name in SIGNATURE_ELEMENTS {
            signature.extend(self.bytes(name)?);
        }
        signature.extend(self.salt()?);
        Ok(signature)
    }
}

/// The directory of the six primary vectors, in a checkout of this
/// repository.
pub fn dir() -> PathBuf {
    Group::Verify.dir()
}

/// The directory of the accept corpus: 107 more honest signatures.
pub fn accept_dir() -> PathBuf {
    dir().join("accept")
}

/// How many honest signatures are committed: the accept corpus and the
/// six primary vectors.
pub const HONEST_VECTORS: usize = 113;

/// Every committed honest vector, the accept corpus and the six primary
/// ones, sorted by path.
///
/// # Panics
///
/// If the accept directory cannot be read: the crate is then not in a
/// checkout that has its data.
pub fn files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(accept_dir())
        .expect("the accept corpus directory")
        .map(|entry| entry.expect("an entry of the accept corpus").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "txt"))
        .collect();
    files.extend(Primary::ALL.map(Primary::path));
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_primary_vector_reads_and_names_its_own_file() {
        for primary in Primary::ALL {
            let vector = primary.vector();
            assert!(vector.level().is_ok(), "{primary:?}");
            assert!(!vector.message().unwrap().is_empty(), "{primary:?}");
            let key = vector.public_key().unwrap();
            let signature = vector.signature().unwrap();
            // A key is one extension element and a signature five and a
            // salt, whatever the set.
            assert_eq!(
                signature.len(),
                5 * key.len() + vector.salt().unwrap().len(),
                "{primary:?}"
            );
            assert_eq!(
                std::fs::read_to_string(primary.path()).unwrap(),
                primary.text(),
                "{primary:?}"
            );
        }
    }

    #[test]
    fn the_sets_name_their_own_vectors() {
        fn sizes<L: WithCorpus>() {
            for primary in L::PRIMARY {
                let vector = primary.vector();
                assert_eq!(vector.public_key().unwrap().len(), L::PUBLIC_KEY_LEN);
                assert_eq!(vector.signature().unwrap().len(), L::SIGNATURE_LEN);
            }
        }
        sizes::<P248>();
        sizes::<P376>();
        sizes::<P500>();
    }

    #[test]
    fn the_corpus_has_the_stated_number_of_vectors() {
        let files = files();
        assert_eq!(files.len(), HONEST_VECTORS);
        for path in files {
            let text = std::fs::read_to_string(&path).unwrap();
            let vector = Vector::new(&text);
            assert!(vector.signature().is_ok(), "{}", path.display());
            assert!(vector.public_key().is_ok(), "{}", path.display());
        }
    }

    #[test]
    fn a_line_is_found_by_its_whole_name() {
        let vector = Vector::new("Px 01\nP 02\nsalt zz\n");
        assert_eq!(vector.field("P"), Some("02"));
        assert_eq!(vector.field("Px"), Some("01"));
        assert_eq!(vector.field("Q"), None);
        assert_eq!(vector.salt(), Err(Malformed::Hex("salt")));
        assert_eq!(vector.level(), Err(Malformed::Missing("level")));
    }
}
