// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The mark the verifier leaves where a stage begins.
//!
//! Private to the crate, and always compiled, so that the verifier marks
//! its stages unconditionally and no call site carries a `cfg`. Without
//! the `op-count` feature [`Stage::enter`] is empty. With it, the mark
//! makes the stage's slot current in the counter, and the public module
//! `stage`, which exists only then, names [`Stage`] as its own and gives
//! it its slot and its printed name.

/// What the verifier is doing when a field operation is counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Not inside a verification. No mark names it, so a build that does
    /// not count has no use for it.
    #[cfg(feature = "op-count")]
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
    /// Count what follows on this thread under this stage, if this build
    /// counts.
    #[inline(always)]
    pub(crate) fn enter(self) {
        #[cfg(feature = "op-count")]
        prism_field::ops::enter(self.slot());
        #[cfg(not(feature = "op-count"))]
        let _ = self;
    }
}
