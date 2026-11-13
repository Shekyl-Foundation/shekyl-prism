// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The stages of one verification, as the operation counter names them.
//!
//! A stage is a purpose, not a position in the source: work moved earlier
//! or later keeps its stage, so counts taken before and after a change line
//! up. The marks cost nothing in a default build; see [`prism_field::ops`].

use prism_field::ops;

/// What the verifier is doing when a field operation is counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Not inside a verification.
    Outside,
    /// Lengths, decoding, sign, model and the basis search.
    Public,
    /// Challenge derivation and the primality test. The hash and
    /// Baillie–PSW do not use the field, so this reads near zero.
    Challenge,
    /// The key's basis, the on-curve test, and the order and span checks.
    Torsion,
    /// `[s]` of the two signature points.
    Scale,
    /// The two Weil pairings and the power they are compared through.
    Pairing,
    /// Lifts, the canonical four-torsion, the matrices, both chain kernels
    /// and both theta base changes.
    Kernel,
    /// The long chain.
    Chain1,
    /// The short chain.
    Chain2,
    /// The comparison of the two codomains.
    Codomain,
    /// The three image comparisons.
    Image,
}

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

    /// Count what follows on this thread under this stage.
    #[inline(always)]
    pub(crate) fn enter(self) {
        ops::enter(self.slot());
    }
}

const _: () = assert!(Stage::ALL.len() <= ops::SLOTS);
