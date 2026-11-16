// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The stages of one verification, as the operation counter names them.
//!
//! A stage is a purpose, not a position in the source: work moved earlier
//! or later keeps its stage, so counts taken before and after a change line
//! up.
//!
//! This module exists only with the `op-count` feature, which also turns
//! the counter on ([`prism_field::ops`]): a default build has neither. The
//! verifier marks its stages unconditionally all the same, through a
//! function private to the crate that is empty without the feature.

use prism_field::ops;

pub use crate::mark::Stage;

impl Stage {
    /// Every stage, in slot order.
    pub const ALL: [Self; 11] = [
        Self::Outside,
        Self::Public,
        Self::Challenge,
        Self::Torsion,
        Self::Scale,
        Self::Pairing,
        Self::Kernel,
        Self::Chain1,
        Self::Chain2,
        Self::Codomain,
        Self::Image,
    ];

    /// The counter slot of this stage.
    #[must_use]
    pub const fn slot(self) -> usize {
        self as usize
    }

    /// The name the count file prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Outside => "outside",
            Self::Public => "public",
            Self::Challenge => "challenge",
            Self::Torsion => "torsion",
            Self::Scale => "scale",
            Self::Pairing => "pairing",
            Self::Kernel => "kernel",
            Self::Chain1 => "chain1",
            Self::Chain2 => "chain2",
            Self::Codomain => "codomain",
            Self::Image => "image",
        }
    }
}

const _: () = assert!(Stage::ALL.len() <= ops::SLOTS);
