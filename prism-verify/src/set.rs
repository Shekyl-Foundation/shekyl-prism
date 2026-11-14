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
//! - [`Set::from_legacy_label`] reads the opaque string committed vectors
//!   and records carry in their `level` field. That is what data says.
//!
//! An unrecognized string is its own error either way, not a stand-in
//! refusal.

use core::fmt;
use core::str::FromStr;

use prism_field::{ParameterSet, P248, P376, P500};

use crate::{verify, VerifyError};

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

    /// The string committed data knows this set by:
    /// [`ParameterSet::LEGACY_LABEL`].
    pub fn legacy_label(self) -> &'static str {
        match self {
            Self::P248 => P248::LEGACY_LABEL,
            Self::P376 => P376::LEGACY_LABEL,
            Self::P500 => P500::LEGACY_LABEL,
        }
    }

    /// The set a committed vector or record names in its `level` field.
    pub fn from_legacy_label(label: &str) -> Result<Self, UnknownSet> {
        Self::ALL
            .into_iter()
            .find(|set| set.legacy_label() == label)
            .ok_or(UnknownSet)
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

/// A string that names no parameter set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownSet;

impl fmt::Display for UnknownSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a parameter set: expected p248, p376 or p500")
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
