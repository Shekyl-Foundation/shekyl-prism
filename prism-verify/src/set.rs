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
//! - [`str::parse`] reads the set's name, such as `p248` or `p324`, into a
//!   [`Set`]. That is what a person types.
//! - [`CommittedSet::from_label`] reads the opaque string committed vectors
//!   and records carry in their `level` field. That is what data says.
//!
//! Each has its own error, [`UnknownSet`] and [`UnknownCommittedLabel`], so a
//! message never suggests the other way's strings, and neither is a
//! stand-in refusal.
//!
//! The committed string is not a fact about the prime. [`Committed`] is
//! implemented for the three sets that already have vectors and seed
//! streams. A prime with no corpus does not implement it, and the field
//! crate never sees the string. The same split holds for values:
//! [`CommittedSet`] can only be a set with a corpus, and [`Set::committed`]
//! says [`NoCorpus`] for one without. No label is invented for such a set.

use core::fmt;
use core::str::FromStr;

use prism_field::{ParameterSet, P248, P324, P376, P500};

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
    P324,
    P376,
    P500,
}

impl Set {
    /// Every set, by increasing prime.
    pub const ALL: [Self; 4] = [Self::P248, Self::P324, Self::P376, Self::P500];

    /// The set's name: [`ParameterSet::NAME`].
    pub fn name(self) -> &'static str {
        match self {
            Self::P248 => P248::NAME,
            Self::P324 => P324::NAME,
            Self::P376 => P376::NAME,
            Self::P500 => P500::NAME,
        }
    }

    /// This set as one that has committed data, or [`NoCorpus`] when no
    /// vector or record exists for it.
    pub fn committed(self) -> Result<CommittedSet, NoCorpus> {
        match self {
            Self::P248 => Ok(CommittedSet::P248),
            Self::P324 => Err(NoCorpus(self)),
            Self::P376 => Ok(CommittedSet::P376),
            Self::P500 => Ok(CommittedSet::P500),
        }
    }

    /// Public salt length for this set, in bytes.
    pub fn salt_len(self) -> usize {
        match self {
            Self::P248 => P248::SALT_LEN,
            Self::P324 => P324::SALT_LEN,
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

/// A parameter set that has committed data: vectors, records, and seed
/// streams of the research signer. The value form of [`Committed`].
///
/// A function that needs a set's committed label, its pinned key or an
/// honest signature takes this type, so a set without a corpus cannot be
/// passed to it and none has to answer "what is the label of a set that
/// has none".
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommittedSet {
    P248,
    P376,
    P500,
}

impl CommittedSet {
    /// Every set with committed data, by increasing prime.
    pub const ALL: [Self; 3] = [Self::P248, Self::P376, Self::P500];

    /// The same set among all sets.
    pub fn set(self) -> Set {
        match self {
            Self::P248 => Set::P248,
            Self::P376 => Set::P376,
            Self::P500 => Set::P500,
        }
    }

    /// [`Committed::LABEL`] for this set.
    pub fn label(self) -> &'static str {
        match self {
            Self::P248 => P248::LABEL,
            Self::P376 => P376::LABEL,
            Self::P500 => P500::LABEL,
        }
    }

    /// The set a committed vector or record names in its `level` field.
    pub fn from_label(label: &str) -> Result<Self, UnknownCommittedLabel> {
        Self::ALL
            .into_iter()
            .find(|set| set.label() == label)
            .ok_or(UnknownCommittedLabel)
    }

    /// Public salt length for this set, in bytes.
    pub fn salt_len(self) -> usize {
        self.set().salt_len()
    }
}

impl fmt::Display for CommittedSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.set().name())
    }
}

/// The set has no committed data: no vector, no record, no label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoCorpus(pub Set);

impl fmt::Display for NoCorpus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} has no committed vectors or records, and no label",
            self.0
        )
    }
}

impl std::error::Error for NoCorpus {}

/// A string that is not the name of a parameter set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownSet;

/// A string that is not the `level` field of any committed vector or
/// record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownCommittedLabel;

impl fmt::Display for UnknownCommittedLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a level field of the committed data: expected ")?;
        list(f, CommittedSet::ALL.map(CommittedSet::label))
    }
}

impl std::error::Error for UnknownCommittedLabel {}

impl fmt::Display for UnknownSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not the name of a parameter set: expected ")?;
        list(f, Set::ALL.map(Set::name))
    }
}

impl std::error::Error for UnknownSet {}

/// `a, b or c`, for an error that names what would have been accepted.
fn list<const N: usize>(f: &mut fmt::Formatter<'_>, names: [&str; N]) -> fmt::Result {
    for (index, name) in names.iter().enumerate() {
        let separator = match N - index {
            1 => "",
            2 => " or ",
            _ => ", ",
        };
        write!(f, "{name}{separator}")?;
    }
    Ok(())
}

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
        Set::P324 => verify::<P324>(public_key, message, signature),
        Set::P376 => verify::<P376>(public_key, message, signature),
        Set::P500 => verify::<P500>(public_key, message, signature),
    }
}
