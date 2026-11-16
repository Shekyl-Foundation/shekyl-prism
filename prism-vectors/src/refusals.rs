// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The identity refusals: responses built from public data alone that a
//! verifier must refuse, two at each of the five sets.
//!
//! Nothing in the file is accepted. Each row is a public key, a message
//! and a signature, with the one refusal a verifier must return. The file's
//! own header says how a row is made and which curve it is on.
//!
//! [`Verdict`] is this crate's name for the two refusals. The crate does
//! not depend on a verifier, so a consumer maps it to its own error type.

use std::sync::OnceLock;

use crate::hex;

/// The committed file, as text, with its header.
pub const TEXT: &str = verify_data!("identity_refusals.tsv");

/// The line of the file that names its columns.
pub const COLUMNS: &str =
    "set\tvector\tcase\tverdict\tsalt_counter\tmessage\tpublic_key\tsignature";

/// The refusal a row pins. These are the two a response with no isogeny
/// behind it can reach once it has passed the pairing check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// Refused at the start of the chain work.
    Kernel,
    /// Refused where the two chains' codomains are compared, after both
    /// chains have run.
    Codomain,
}

impl Verdict {
    /// Both verdicts.
    pub const ALL: [Self; 2] = [Self::Kernel, Self::Codomain];

    /// The text the file uses for this verdict. It is the file's format,
    /// stated here, and not the name of any verifier's error.
    pub const fn text(self) -> &'static str {
        match self {
            Self::Kernel => "Kernel",
            Self::Codomain => "Codomain",
        }
    }

    fn from_text(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|verdict| verdict.text() == text)
    }
}

/// One pinned refusal: an input and the one refusal a verifier must
/// return for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// The name of the parameter set, such as `p248`.
    pub set: &'static str,
    /// The committed file, and the curve in it, that the key is.
    pub vector: &'static str,
    /// The name the case has in the operation-count records.
    pub case: &'static str,
    /// The pinned verdict.
    pub verdict: Verdict,
    /// The counter the salt encodes: the first whose challenge degree is
    /// prime under this key and message.
    pub salt_counter: u64,
    /// The message.
    pub message: Vec<u8>,
    /// The public key, in the set's canonical encoding.
    pub public_key: Vec<u8>,
    /// The signature: response curve, `x(P)`, `y(P)`, `x(Q)`, `y(Q)`, salt.
    pub signature: Vec<u8>,
}

/// Why a refusal could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The file is not in the form this reader documents.
    Malformed,
    /// The file has no row for this set and verdict.
    Missing,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Malformed => "the identity-refusal file is not in its documented form",
            Self::Missing => "the identity-refusal file has no row for this set and verdict",
        })
    }
}

impl std::error::Error for Error {}

/// Every pinned refusal, in the file's order.
pub fn rows() -> Result<&'static [Refusal], Error> {
    static ROWS: OnceLock<Result<Vec<Refusal>, Error>> = OnceLock::new();
    ROWS.get_or_init(|| {
        let mut lines = TEXT
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'));
        // The first line names the columns. A file with other columns is
        // not one this reader can be trusted to read.
        if lines.next() != Some(COLUMNS) {
            return Err(Error::Malformed);
        }
        lines.map(parse).collect()
    })
    .as_deref()
    .map_err(|err| *err)
}

/// The pinned refusal of the set named `set` that ends in `verdict`.
pub fn row(set: &str, verdict: Verdict) -> Result<&'static Refusal, Error> {
    rows()?
        .iter()
        .find(|row| row.set == set && row.verdict == verdict)
        .ok_or(Error::Missing)
}

fn parse(line: &'static str) -> Result<Refusal, Error> {
    let fields: Vec<&'static str> = line.split('\t').collect();
    let &[set, vector, case, verdict, salt_counter, message, public_key, signature] =
        fields.as_slice()
    else {
        return Err(Error::Malformed);
    };
    Ok(Refusal {
        set,
        vector,
        case,
        verdict: Verdict::from_text(verdict).ok_or(Error::Malformed)?,
        salt_counter: salt_counter.parse().map_err(|_| Error::Malformed)?,
        message: hex::decode(message).ok_or(Error::Malformed)?,
        public_key: hex::decode(public_key).ok_or(Error::Malformed)?,
        signature: hex::decode(signature).ok_or(Error::Malformed)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{ParameterSet, P248, P324, P376, P500, P664};

    #[test]
    fn every_set_has_both_verdicts_once_at_its_own_sizes() {
        let sizes = [
            (P248::NAME, P248::PUBLIC_KEY_LEN, P248::SIGNATURE_LEN),
            (P324::NAME, P324::PUBLIC_KEY_LEN, P324::SIGNATURE_LEN),
            (P376::NAME, P376::PUBLIC_KEY_LEN, P376::SIGNATURE_LEN),
            (P500::NAME, P500::PUBLIC_KEY_LEN, P500::SIGNATURE_LEN),
            (P664::NAME, P664::PUBLIC_KEY_LEN, P664::SIGNATURE_LEN),
        ];
        let rows = rows().expect("the committed file parses");
        assert_eq!(rows.len(), sizes.len() * Verdict::ALL.len());
        for (set, key, signature) in sizes {
            for verdict in Verdict::ALL {
                let found = row(set, verdict).expect("a row");
                assert_eq!(found.public_key.len(), key, "{set}");
                assert_eq!(found.signature.len(), signature, "{set}");
                assert_eq!(
                    rows.iter()
                        .filter(|row| row.set == set && row.verdict == verdict)
                        .count(),
                    1,
                    "{set} {verdict:?}"
                );
            }
        }
    }

    #[test]
    fn a_row_with_another_verdict_or_another_width_is_refused() {
        assert_eq!(row("no-such-set", Verdict::Kernel), Err(Error::Missing));
        assert_eq!(
            parse("p248\tv\tc\tImage\t0\t00\t00\t00"),
            Err(Error::Malformed)
        );
        assert_eq!(
            parse("p248\tv\tc\tKernel\t0\t00\t00"),
            Err(Error::Malformed)
        );
        assert_eq!(
            parse("p248\tv\tc\tKernel\tx\t00\t00\t00"),
            Err(Error::Malformed)
        );
        assert!(parse("p248\tv\tc\tKernel\t7\t00\t01\t02").is_ok());
    }
}
