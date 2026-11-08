// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Naming a NIST level at run time.
//!
//! A level is a type everywhere else in this workspace. A binary that reads
//! a level from its arguments needs one value that stands for the type, and
//! this is it: the parse is the only place a string becomes a level, and an
//! unrecognized string is its own error rather than a stand-in refusal.

use core::fmt;
use core::str::FromStr;

use prism_field::{Level, Level1, Level3, Level5};

use crate::{verify, VerifyError};

/// One of Table 2's three columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nist {
    I,
    III,
    V,
}

impl Nist {
    /// Every level, in Table 2's order.
    pub const ALL: [Self; 3] = [Self::I, Self::III, Self::V];

    /// The level's name, as the pinned test vectors spell it.
    pub fn name(self) -> &'static str {
        match self {
            Self::I => Level1::NAME,
            Self::III => Level3::NAME,
            Self::V => Level5::NAME,
        }
    }
}

impl fmt::Display for Nist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A level name that is not one of Table 2's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownLevel;

impl fmt::Display for UnknownLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a NIST level of ePrint 2026/443 Table 2")
    }
}

impl std::error::Error for UnknownLevel {}

impl FromStr for Nist {
    type Err = UnknownLevel;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "1" | "I" | "NIST-I" => Ok(Self::I),
            "3" | "III" | "NIST-III" => Ok(Self::III),
            "5" | "V" | "NIST-V" => Ok(Self::V),
            _ => Err(UnknownLevel),
        }
    }
}

/// Run [`verify`] for a level named at run time.
pub fn verify_level(
    level: Nist,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    match level {
        Nist::I => verify::<Level1>(public_key, message, signature),
        Nist::III => verify::<Level3>(public_key, message, signature),
        Nist::V => verify::<Level5>(public_key, message, signature),
    }
}
