// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Pinned key-generation vectors from the C tree, and the timing sample
//! built from them.
//!
//! The vectors are written by `scripts/export_c_basis_hints.py` against the
//! submodule commit `docs/C_REFERENCE.md` pins, from that script's frozen
//! seed, and they are pinned rather than generated here: the implementation
//! is tested against them. A `#` line in the file is provenance, ignored.

use std::sync::OnceLock;

use prism_field::{Field, Level, Level1, Level3, Level5};

use crate::basis::BasisHint;
use crate::level::Nist;
use crate::signature::{AffinePoint, Response};
use crate::{ChallengeScalar, KeyIdentity, Salt};

/// `A` from the C tree's key generation, one row per NIST level.
///
/// Lines are `<level-name> <hex of A> <hint byte>`. The hint is what
/// `ec_curve_to_basis_2f_to_hint` returned. It is not part of the public key.
const C_BASIS_HINTS: &str = include_str!("../testdata/c_basis_hints.txt");

/// The message the timing sample signs.
const SAMPLE_MESSAGE: &[u8] = b"shekyl-prism";

/// One pinned row: a C-generated key and the hint the C derived for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceKey {
    pub level: Nist,
    /// The public key: `A` in this level's canonical encoding.
    pub public_key: Vec<u8>,
    /// What `ec_curve_to_basis_2f_to_hint` returned for that key.
    pub hint: BasisHint,
}

/// Why a pinned row could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorError {
    /// A row is not `<level-name> <hex of A> <hint byte>`.
    Malformed,
    /// The file carries no row for this level.
    Missing,
}

impl core::fmt::Display for VectorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reason = match self {
            Self::Malformed => "a pinned row is not <level> <hex A> <hint>",
            Self::Missing => "no pinned row for this level",
        };
        f.write_str(reason)
    }
}

impl std::error::Error for VectorError {}

/// The pinned row for a level.
pub fn reference_key(level: Nist) -> Result<&'static ReferenceKey, VectorError> {
    static KEYS: OnceLock<Result<Vec<ReferenceKey>, VectorError>> = OnceLock::new();
    KEYS.get_or_init(|| parse_rows(C_BASIS_HINTS))
        .as_ref()
        .map_err(|err| *err)?
        .iter()
        .find(|row| row.level == level)
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
    let level = next()?.parse().map_err(|_| VectorError::Malformed)?;
    let public_key = decode_hex(next()?)?;
    let hint = next()?
        .parse()
        .ok()
        .and_then(BasisHint::from_c_byte)
        .ok_or(VectorError::Malformed)?;
    Ok(ReferenceKey {
        level,
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
/// clock starts, then times [`crate::verify_level`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prepared {
    pub level: Nist,
    pub public_key: Vec<u8>,
    pub message: &'static [u8],
    pub signature: Vec<u8>,
}

/// Why a timing sample could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedError {
    /// The pinned key for this level is unreadable.
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

/// Build a [`Prepared`] for a level.
pub fn prepared(level: Nist) -> Result<Prepared, PreparedError> {
    match level {
        Nist::I => sample::<Level1>(level),
        Nist::III => sample::<Level3>(level),
        Nist::V => sample::<Level5>(level),
    }
}

fn sample<L: Level>(level: Nist) -> Result<Prepared, PreparedError> {
    let key = reference_key(level).map_err(PreparedError::Vector)?;
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
        curve: Response::<L>::decode(&honest(level).map_err(PreparedError::Vector)?.signature)
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
        level,
        public_key: key.public_key.clone(),
        message: SAMPLE_MESSAGE,
        signature: response.encode(),
    })
}

/// An honest, committed §5 response for benchmarking the complete verifier.
/// It is test material; the keys and signing seeds are public.
pub fn honest(level: Nist) -> Result<Prepared, VectorError> {
    let text = match level {
        Nist::I => include_str!("../testdata/s5_lvl1.txt"),
        Nist::III => include_str!("../testdata/s5_lvl3.txt"),
        Nist::V => include_str!("../testdata/s5_lvl5.txt"),
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
    if salt.len() != level.salt_len() {
        return Err(VectorError::Malformed);
    }
    signature.extend(salt);
    Ok(Prepared {
        level,
        public_key,
        message: field("message ")?.as_bytes(),
        signature,
    })
}
