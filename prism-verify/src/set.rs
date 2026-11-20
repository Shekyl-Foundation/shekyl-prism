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

use prism_field::{ParameterSet, P248, P324, P376, P500, P664};

use crate::{verify, VerifyError};

/// A parameter set that committed vectors and the research signer's seed
/// streams already name.
///
/// The string is opaque and it never changes. It is the `level` field of
/// those vectors, and the research signer absorbs the same bytes into its
/// seed streams. A set added later implements this when its corpus exists,
/// and not before.
pub trait Committed: ParameterSet {
    /// The `level` field of the set's committed vectors and records.
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
///
/// All five are verified by the same code. Three have honest signatures
/// to test acceptance with ([`CommittedSet`]); `p324` and `p664` are
/// held by refusals alone, and the crate documentation says what that
/// leaves unshown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Set {
    /// `p248`, [`prism_field::P248`].
    P248,
    /// `p324`, [`prism_field::P324`]. No corpus.
    P324,
    /// `p376`, [`prism_field::P376`].
    P376,
    /// `p500`, [`prism_field::P500`].
    P500,
    /// `p664`, [`prism_field::P664`]. No corpus.
    P664,
}

impl Set {
    /// Every set, by increasing prime.
    pub const ALL: [Self; 5] = [Self::P248, Self::P324, Self::P376, Self::P500, Self::P664];

    /// The set's name: [`ParameterSet::NAME`].
    pub fn name(self) -> &'static str {
        match self {
            Self::P248 => P248::NAME,
            Self::P324 => P324::NAME,
            Self::P376 => P376::NAME,
            Self::P500 => P500::NAME,
            Self::P664 => P664::NAME,
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
            Self::P664 => Err(NoCorpus(self)),
        }
    }

    /// Public salt length for this set, in bytes.
    pub(crate) fn salt_len(self) -> usize {
        match self {
            Self::P248 => P248::SALT_LEN,
            Self::P324 => P324::SALT_LEN,
            Self::P376 => P376::SALT_LEN,
            Self::P500 => P500::SALT_LEN,
            Self::P664 => P664::SALT_LEN,
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
    /// `p248`, whose committed label is `NIST-I`.
    P248,
    /// `p376`, whose committed label is `NIST-III`.
    P376,
    /// `p500`, whose committed label is `NIST-V`.
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
pub struct NoCorpus(
    /// The set that was asked for.
    pub Set,
);

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
///
/// A complete entry, as [`verify`] is: it assumes nothing of its
/// arguments, and its refusals are that function's, in that order.
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
        Set::P664 => verify::<P664>(public_key, message, signature),
    }
}

#[cfg(test)]
mod tests {
    use super::{CommittedSet, Set};

    /// The salt of a set is its exponent `e` rounded up to a multiple of
    /// 128 bits (`docs/PARAMETER_SETS.md`), and a committed set's is its
    /// set's. A tool reads these to split a signature without naming the
    /// set as a type.
    #[test]
    fn the_salt_length_is_the_exponent_rounded_up_to_128_bits() {
        let rounded = |e: usize| e.div_ceil(128) * 128 / 8;
        for (set, e, bytes) in [
            (Set::P248, 248, 32),
            (Set::P324, 324, 48),
            (Set::P376, 376, 48),
            (Set::P500, 500, 64),
            (Set::P664, 664, 96),
        ] {
            assert_eq!(set.salt_len(), bytes, "{set:?}");
            assert_eq!(rounded(e), bytes, "{set:?}");
        }
        assert_eq!(Set::ALL.len(), 5);
        for committed in CommittedSet::ALL {
            assert_eq!(committed.salt_len(), committed.set().salt_len());
            assert!(committed.salt_len() >= 32, "{committed:?}");
        }
    }
}
