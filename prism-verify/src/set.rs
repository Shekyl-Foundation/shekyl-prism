// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Naming a parameter set at run time.
//!
//! A parameter set is a type everywhere else in this workspace. A binary
//! that reads a set from its arguments, or from a record, needs one value
//! that stands for the type, and this is it. There are two ways in, and
//! they are kept apart:
//!
//! - [`str::parse`] reads the set's name, `p248`, `p376` or `p500`. That is
//!   what a person types.
//! - [`Set::from_committed_label`] reads the opaque string committed vectors
//!   and records carry in their `level` field. That is what data says.
//!
//! Each has its own error, [`UnknownSet`] and [`UnknownCommittedLabel`], so a
//! message never suggests the other way's strings, and neither is a
//! stand-in refusal.
//!
//! The committed string is not a fact about the prime. [`Committed`] is
//! implemented for the three sets that already have vectors and seed
//! streams. A prime with no corpus does not implement it, and the field
//! crate never sees the string.

use core::fmt;
use core::str::FromStr;

use prism_field::{ParameterSet, P248, P376, P500};

use crate::{verify, VerifyError};

/// A parameter set that committed vectors and the research signer's seed
/// streams already name.
///
/// The string is opaque and it never changes. It is the `level` field of
/// those vectors, and the research signer absorbs the same bytes into its
/// seed streams. A set added later implements this when its corpus exists,
/// and not before.
pub trait Committed: ParameterSet {
    const LABEL: &'static str;
}

impl Committed for P248 {
    const LABEL: &'static str = "NIST-I";
}

impl Committed for P376 {
    const LABEL: &'static str = "NIST-III";
}

impl Committed for P500 {
    const LABEL: &'static str = "NIST-V";
}

/// One of the parameter sets, as a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Set {
    P248,
    P376,
    P500,
}

impl Set {
    /// Every set, by increasing prime.
    pub const ALL: [Self; 3] = [Self::P248, Self::P376, Self::P500];

    /// The set's name: [`ParameterSet::NAME`].
    pub fn name(self) -> &'static str {
        match self {
            Self::P248 => P248::NAME,
            Self::P376 => P376::NAME,
            Self::P500 => P500::NAME,
        }
    }

    /// [`Committed::LABEL`] for this set.
    pub fn committed_label(self) -> &'static str {
        match self {
            Self::P248 => P248::LABEL,
            Self::P376 => P376::LABEL,
            Self::P500 => P500::LABEL,
        }
    }

    /// The set a committed vector or record names in its `level` field.
    pub fn from_committed_label(label: &str) -> Result<Self, UnknownCommittedLabel> {
        Self::ALL
            .into_iter()
            .find(|set| set.committed_label() == label)
            .ok_or(UnknownCommittedLabel)
    }

    /// Public salt length for this set, in bytes.
    pub fn salt_len(self) -> usize {
        match self {
            Self::P248 => P248::SALT_LEN,
            Self::P376 => P376::SALT_LEN,
            Self::P500 => P500::SALT_LEN,
        }
    }
}

impl fmt::Display for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A string that is not the name of a parameter set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownSet;

/// A string that is not the `level` field of any committed vector or
/// record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownCommittedLabel;

impl fmt::Display for UnknownCommittedLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "not a level field of the committed data: expected {}, {} or {}",
            P248::LABEL,
            P376::LABEL,
            P500::LABEL
        )
    }
}

impl std::error::Error for UnknownCommittedLabel {}

impl fmt::Display for UnknownSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "not the name of a parameter set: expected {}, {} or {}",
            P248::NAME,
            P376::NAME,
            P500::NAME
        )
    }
}

impl std::error::Error for UnknownSet {}

impl FromStr for Set {
    type Err = UnknownSet;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|set| set.name() == text)
            .ok_or(UnknownSet)
    }
}

/// Run [`verify`] for a set named at run time.
pub fn verify_set(
    set: Set,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    match set {
        Set::P248 => verify::<P248>(public_key, message, signature),
        Set::P376 => verify::<P376>(public_key, message, signature),
        Set::P500 => verify::<P500>(public_key, message, signature),
    }
}
