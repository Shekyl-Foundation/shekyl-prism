// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Pinned key-generation vectors from the C tree, the timing sample built
//! from them, and the pinned refusals a timing run uses at every set.
//!
//! The vectors are written by `scripts/export_c_basis_hints.py` against the
//! submodule commit `docs/C_REFERENCE.md` pins, from that script's frozen
//! seed, and they are pinned rather than generated here: the implementation
//! is tested against them. A `#` line in the file is provenance, ignored.

use std::sync::OnceLock;

use prism_field::{Field, ParameterSet, P248, P376, P500};

use crate::basis::BasisHint;
use crate::set::{CommittedSet, Set};
use crate::signature::{AffinePoint, Response};
use crate::{ChallengeScalar, KeyIdentity, Salt, VerifyError};

/// `A` from the C tree's key generation, one row per parameter set.
///
/// Lines are `<committed label> <hex of A> <hint byte>`. The hint is what
/// `ec_curve_to_basis_2f_to_hint` returned. It is not part of the public key.
const C_BASIS_HINTS: &str = include_str!("../testdata/c_basis_hints.txt");

/// The message the timing sample signs.
const SAMPLE_MESSAGE: &[u8] = b"shekyl-prism";

/// One pinned row: a C-generated key and the hint the C derived for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceKey {
    pub set: CommittedSet,
    /// The public key: `A` in this set's canonical encoding.
    pub public_key: Vec<u8>,
    /// What `ec_curve_to_basis_2f_to_hint` returned for that key.
    pub hint: BasisHint,
}

/// Why a pinned row could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorError {
    /// A row is not in the form its file's reader documents.
    Malformed,
    /// The file carries no row for this set.
    Missing,
}

impl core::fmt::Display for VectorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reason = match self {
            Self::Malformed => "a pinned row is not in its file's form",
            Self::Missing => "no pinned row for this set",
        };
        f.write_str(reason)
    }
}

impl std::error::Error for VectorError {}

/// The pinned row for a set.
pub fn reference_key(set: CommittedSet) -> Result<&'static ReferenceKey, VectorError> {
    static KEYS: OnceLock<Result<Vec<ReferenceKey>, VectorError>> = OnceLock::new();
    KEYS.get_or_init(|| parse_rows(C_BASIS_HINTS))
        .as_ref()
        .map_err(|err| *err)?
        .iter()
        .find(|row| row.set == set)
        .ok_or(VectorError::Missing)
}

fn parse_rows(text: &str) -> Result<Vec<ReferenceKey>, VectorError> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_row)
        .collect()
}

fn parse_row(line: &str) -> Result<ReferenceKey, VectorError> {
    let mut fields = line.split_whitespace();
    let mut next = || fields.next().ok_or(VectorError::Malformed);
    let set = CommittedSet::from_label(next()?).map_err(|_| VectorError::Malformed)?;
    let public_key = decode_hex(next()?)?;
    let hint = next()?
        .parse()
        .ok()
        .and_then(BasisHint::from_c_byte)
        .ok_or(VectorError::Malformed)?;
    Ok(ReferenceKey {
        set,
        public_key,
        hint,
    })
}

fn decode_hex(text: &str) -> Result<Vec<u8>, VectorError> {
    if !text.len().is_multiple_of(2) {
        return Err(VectorError::Malformed);
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            core::str::from_utf8(pair)
                .ok()
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or(VectorError::Malformed)
        })
        .collect()
}

/// A public key, message, and signature whose challenge degree is prime.
///
/// The salt search is public test preparation. A timing run builds this before the
/// clock starts, then times [`crate::verify_set`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prepared {
    pub set: CommittedSet,
    pub public_key: Vec<u8>,
    pub message: &'static [u8],
    pub signature: Vec<u8>,
}

/// Why a timing sample could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedError {
    /// The pinned key for this set is unreadable.
    Vector(VectorError),
    /// No test salt names a prime degree within the bounded search.
    Exhausted,
}

impl core::fmt::Display for PreparedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Vector(err) => write!(f, "the pinned key is unreadable: {err}"),
            Self::Exhausted => f.write_str("sample salt search exhausted"),
        }
    }
}

impl std::error::Error for PreparedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Vector(err) => Some(err),
            Self::Exhausted => None,
        }
    }
}

/// Build a [`Prepared`] for a set.
pub fn prepared(set: CommittedSet) -> Result<Prepared, PreparedError> {
    match set {
        CommittedSet::P248 => sample::<P248>(set),
        CommittedSet::P376 => sample::<P376>(set),
        CommittedSet::P500 => sample::<P500>(set),
    }
}

fn sample<L: ParameterSet>(set: CommittedSet) -> Result<Prepared, PreparedError> {
    let key = reference_key(set).map_err(PreparedError::Vector)?;
    let id = KeyIdentity::<L>::decode(&key.public_key)
        .map_err(|_| PreparedError::Vector(VectorError::Malformed))?;
    let salt = (0u64..65_536)
        .find_map(|counter| {
            let salt = L::Salt::from_counter(counter)?;
            ChallengeScalar::derive(&id, SAMPLE_MESSAGE, salt)
                .degree_is_prime::<L>()
                .then_some(salt)
        })
        .ok_or(PreparedError::Exhausted)?;
    // The public checks read `y(P)` and the salt. The sample carries
    // an admissible curve but invalid point coordinates, so verification
    // stops at `Kernel`. The honest benchmark uses `honest` instead.
    let response = Response::<L> {
        curve: Response::<L>::decode(&honest(set).map_err(PreparedError::Vector)?.signature)
            .map_err(|_| PreparedError::Vector(VectorError::Malformed))?
            .curve,
        p: AffinePoint {
            x: L::Ext::zero(),
            y: L::Ext::one(),
        },
        q: AffinePoint {
            x: L::Ext::zero(),
            y: L::Ext::zero(),
        },
        salt,
    };
    Ok(Prepared {
        set,
        public_key: key.public_key.clone(),
        message: SAMPLE_MESSAGE,
        signature: response.encode(),
    })
}

/// An honest, committed §5 response for benchmarking the complete verifier.
/// It is test material; the keys and signing seeds are public.
pub fn honest(set: CommittedSet) -> Result<Prepared, VectorError> {
    let text = match set {
        CommittedSet::P248 => include_str!("../testdata/s5_lvl1.txt"),
        CommittedSet::P376 => include_str!("../testdata/s5_lvl3.txt"),
        CommittedSet::P500 => include_str!("../testdata/s5_lvl5.txt"),
    };
    let field = |name: &str| {
        text.lines()
            .find_map(|row| row.strip_prefix(name))
            .ok_or(VectorError::Malformed)
    };
    let public_key = decode_hex(field("A ")?)?;
    let mut signature = Vec::new();
    for name in ["E ", "Px ", "Py ", "Qx ", "Qy "] {
        signature.extend(decode_hex(field(name)?)?);
    }
    let salt = decode_hex(field("salt ")?)?;
    if salt.len() != set.salt_len() {
        return Err(VectorError::Malformed);
    }
    signature.extend(salt);
    Ok(Prepared {
        set,
        public_key,
        message: field("message ")?.as_bytes(),
        signature,
    })
}

/// Responses built from public data alone, at every set, that pass the
/// pairing check and are refused after it.
///
/// The file's own header says how each row is built and on which curve.
/// Rows are tab-separated, in the columns of [`REFUSAL_COLUMNS`], the last
/// three in hex.
const IDENTITY_REFUSALS: &str = include_str!("../testdata/identity_refusals.tsv");

/// The line of [`IDENTITY_REFUSALS`] that names its columns.
const REFUSAL_COLUMNS: &str =
    "set\tvector\tcase\tverdict\tsalt_counter\tmessage\tpublic_key\tsignature";

/// The verdicts a row of [`IDENTITY_REFUSALS`] may carry, by the text the
/// file uses for each. They are the two refusals a response with no
/// isogeny behind it can reach once it has passed the pairing check. The
/// text is the file's format and is stated here, so renaming an error
/// does not change what the file says.
pub const REFUSAL_VERDICTS: [(&str, VerifyError); 2] = [
    ("Kernel", VerifyError::Kernel),
    ("Codomain", VerifyError::Codomain),
];

/// One pinned refusal: an input and the one error the verifier must return
/// for it. Nothing here is accepted, at any set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub set: Set,
    /// The committed file, and the curve in it, that the key is.
    pub vector: &'static str,
    /// The name the case has in `docs/benchmarks/negative-verdicts.tsv`.
    pub case: &'static str,
    /// The pinned verdict.
    pub error: VerifyError,
    /// The counter the salt encodes: the first whose challenge degree is
    /// prime under this key and message.
    pub salt_counter: u64,
    pub message: Vec<u8>,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

/// Every pinned refusal, in the file's order.
pub fn refusals() -> Result<&'static [Refusal], VectorError> {
    static ROWS: OnceLock<Result<Vec<Refusal>, VectorError>> = OnceLock::new();
    ROWS.get_or_init(|| {
        let mut lines = IDENTITY_REFUSALS
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'));
        // The first line names the columns. A file with other columns is
        // not one this reader can be trusted to read.
        if lines.next() != Some(REFUSAL_COLUMNS) {
            return Err(VectorError::Malformed);
        }
        lines.map(parse_refusal).collect()
    })
    .as_deref()
    .map_err(|err| *err)
}

/// The pinned refusal of a set that ends in `error`.
pub fn refusal(set: Set, error: VerifyError) -> Result<&'static Refusal, VectorError> {
    refusals()?
        .iter()
        .find(|row| row.set == set && row.error == error)
        .ok_or(VectorError::Missing)
}

fn parse_refusal(line: &'static str) -> Result<Refusal, VectorError> {
    let fields: Vec<&'static str> = line.split('\t').collect();
    let &[set, vector, case, verdict, salt_counter, message, public_key, signature] =
        fields.as_slice()
    else {
        return Err(VectorError::Malformed);
    };
    let (_, error) = REFUSAL_VERDICTS
        .into_iter()
        .find(|(text, _)| *text == verdict)
        .ok_or(VectorError::Malformed)?;
    Ok(Refusal {
        set: set.parse().map_err(|_| VectorError::Malformed)?,
        vector,
        case,
        error,
        salt_counter: salt_counter.parse().map_err(|_| VectorError::Malformed)?,
        message: decode_hex(message)?,
        public_key: decode_hex(public_key)?,
        signature: decode_hex(signature)?,
    })
}
