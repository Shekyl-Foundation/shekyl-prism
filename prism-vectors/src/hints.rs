// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Key-generation hints from the C reference: one public key per set
//! with a corpus, and the basis hint the C tree derived for it.
//!
//! A line is `<level> <hex of A> <hint byte>`. The level is the opaque
//! string the C tree and the committed vectors use for a set. The hint is
//! what `ec_curve_to_basis_2f_to_hint` returned; it is not part of a
//! public key, and a verifier that derives the basis itself is tested
//! against it. Lines that start with `#` are provenance: the pinned
//! commit of the C tree and the seeds of the run.

use std::sync::OnceLock;

use crate::hex;

/// The committed file, as text.
pub const TEXT: &str = verify_data!("c_basis_hints.txt");

/// One row: a C-generated key and the hint the C derived for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyHint {
    /// The level string of the row, such as `NIST-I`.
    pub level: &'static str,
    /// The public key: `A` in the set's canonical encoding.
    pub public_key: Vec<u8>,
    /// The byte `ec_curve_to_basis_2f_to_hint` returned.
    pub hint: u8,
}

/// Why a hint could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A row is not `<level> <hex of A> <hint byte>`.
    Malformed,
    /// The file has no row for this level.
    Missing,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Malformed => "a hint row is not <level> <hex A> <hint byte>",
            Self::Missing => "the hint file has no row for this level",
        })
    }
}

impl std::error::Error for Error {}

/// Every row of the file, in file order.
pub fn rows() -> Result<&'static [KeyHint], Error> {
    static ROWS: OnceLock<Result<Vec<KeyHint>, Error>> = OnceLock::new();
    ROWS.get_or_init(|| {
        TEXT.lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(parse)
            .collect()
    })
    .as_deref()
    .map_err(|err| *err)
}

/// The row whose level string is `level`.
pub fn row(level: &str) -> Result<&'static KeyHint, Error> {
    rows()?
        .iter()
        .find(|row| row.level == level)
        .ok_or(Error::Missing)
}

fn parse(line: &'static str) -> Result<KeyHint, Error> {
    let mut fields = line.split_whitespace();
    let mut next = || fields.next().ok_or(Error::Malformed);
    let level = next()?;
    let public_key = hex::decode(next()?).ok_or(Error::Malformed)?;
    let hint = next()?.parse().map_err(|_| Error::Malformed)?;
    Ok(KeyHint {
        level,
        public_key,
        hint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::Primary;

    #[test]
    fn there_is_one_row_for_each_level_the_vectors_name() {
        let rows = rows().expect("the committed file parses");
        let mut levels: Vec<&str> = Primary::ALL
            .iter()
            .map(|primary| primary.vector().level().unwrap())
            .collect();
        levels.dedup();
        assert_eq!(rows.iter().map(|row| row.level).collect::<Vec<_>>(), levels);
        for level in levels {
            assert!(row(level).is_ok(), "{level}");
        }
        assert_eq!(row("no-such-level"), Err(Error::Missing));
    }

    #[test]
    fn a_short_row_or_a_wide_hint_is_refused() {
        assert_eq!(parse("NIST-I 00"), Err(Error::Malformed));
        assert_eq!(parse("NIST-I 0 1"), Err(Error::Malformed));
        assert_eq!(parse("NIST-I 00 256"), Err(Error::Malformed));
        assert!(parse("NIST-I 00 255").is_ok());
    }
}
