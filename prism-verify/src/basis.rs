// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The deterministic basis index, derived the way the C reference searches
//! for it.
//!
//! `find_nqr_factor` and `find_nA_x_coord` in
//! `external/PRISM_v2/src/ec/ref/lvlx/basis.c` are the search this module
//! mirrors. The index is not stored in the key, so the verifier re-derives
//! it; the points it names are Algorithm 3's business and are not built here.
//!
//! The C reference reaches the same index by two paths, and this module
//! takes the one path that covers both. Key generation stores the index when
//! it fits in seven bits and stores zero when it does not;
//! `ec_curve_to_basis_2f_from_hint` reads a stored index directly, and reads
//! the zero as "search again from 128". Both paths land on the *smallest*
//! qualifying index: the stored one is the smallest because generation
//! searched from 1, and the `start = 128` restart is the smallest at or
//! above 128 precisely because every index below 128 has already failed.
//! Searching from 1 without the seven-bit ceiling therefore answers both
//! cases, and [`C_HINT_MAX_INDEX`] is an encoding fact rather than a bound
//! on the search.

use prism_field::{Field, Level};

use crate::VerifyError;

/// Largest index the C reference can name in its hint byte.
///
/// `ec_curve_to_basis_2f_to_hint` packs the index into the seven bits above
/// the square flag. An index larger than this is not a defect: the C stores
/// zero and its verifier re-derives the index, which is what this module
/// does for every key.
pub const C_HINT_MAX_INDEX: u16 = 127;

/// Candidate indices the search tries before refusing: `2λ`.
///
/// The C loops without a bound. A verifier cannot, because whoever picks `A`
/// picks the search length, so the bound is derived from the level's own
/// security parameter rather than from the width of the C's hint byte:
///
/// | Branch | Trials in `2λ` candidates | Failure probability |
/// | --- | --- | --- |
/// | `A` not a square | all `2λ`, each on the curve with probability about 1/2 | about `2^-2λ` |
/// | `A` a square | about `λ`, the rest spent on the `1 + b^2` non-square filter | about `2^-λ` |
///
/// So a key this verifier refuses and the C accepts arises with probability
/// at most about `2^-λ` — the level's own security target, and the bar the
/// C's own comment aims at when it calls the fallback negligible. The
/// earlier seven-bit ceiling put the square branch at about `2^-63`
/// regardless of level, which is a weaker claim than the scheme's.
pub fn search_bound<L: Level>() -> u16 {
    // `L::LAMBDA` is 128, 192 or 256, so `2λ` is always a `u16`.
    u16::try_from(2 * L::LAMBDA).expect("2λ fits a u16 at every level")
}

/// The basis index, in the form the C reference compresses into one byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BasisHint {
    index: u16,
    coefficient_is_square: bool,
}

impl BasisHint {
    /// `b` when `A` is a square, `n` when it is not. At least 1, and at most
    /// [`search_bound`] for the level it was derived at.
    pub fn index(self) -> u16 {
        self.index
    }

    /// Whether `A` is a square, which selects the search.
    pub fn coefficient_is_square(self) -> bool {
        self.coefficient_is_square
    }

    /// The byte `ec_curve_to_basis_2f_to_hint` returns for this index:
    /// `(index << 1) | hint_A`.
    ///
    /// `None` when the index is above [`C_HINT_MAX_INDEX`]. The C stores
    /// `hint_P = 0` in that case and its verifier re-searches, so there is
    /// no byte that names the index — the absence is the signal, and it is
    /// not spelled as an in-range zero here.
    pub fn to_c_byte(self) -> Option<u8> {
        if self.index > C_HINT_MAX_INDEX {
            return None;
        }
        let index = u8::try_from(self.index).ok()?;
        Some((index << 1) | u8::from(self.coefficient_is_square))
    }

    /// Read a C hint byte. An index of zero is the C's "no index below 128"
    /// signal rather than a hint, so it does not name one.
    pub fn from_c_byte(byte: u8) -> Option<Self> {
        let index = byte >> 1;
        (index != 0).then_some(Self {
            index: u16::from(index),
            coefficient_is_square: byte & 1 == 1,
        })
    }
}

/// Derive the basis index for a non-zero `coefficient`.
///
/// `A = 0` never reaches here: [`crate::KeyIdentity`] refuses the starting
/// curve, which is also the one curve the C's entangled basis generation
/// cannot handle.
///
/// The search is bounded by [`search_bound`], and absence is
/// [`VerifyError::Basis`]. That bound is the only divergence from the C
/// reference's unbounded loop, and `docs/VERIFIER_SPEC.md` records what it
/// costs.
pub fn basis_hint<L: Level>(coefficient: L::Ext) -> Result<BasisHint, VerifyError> {
    let coefficient_is_square = coefficient.is_square();
    let index = if coefficient_is_square {
        nqr_factor_index::<L>(coefficient)
    } else {
        multiple_index::<L>(coefficient)
    };
    index
        .map(|index| BasisHint {
            index,
            coefficient_is_square,
        })
        .ok_or(VerifyError::Basis)
}

/// Square `A`: the first `b` for which `1 + b^2` is a non-square in `F_p`
/// and `x(P) = −A / (1 + i·b)` is on the curve.
fn nqr_factor_index<L: Level>(coefficient: L::Ext) -> Option<u16> {
    let square = coefficient.mul(coefficient);
    (1..=search_bound::<L>()).find(|candidate| {
        let b = u64::from(*candidate);
        if L::Base::from_u64(b * b + 1).is_square() {
            return false;
        }
        // `x(P) = −A/z` for `z = 1 + i·b` is on the curve exactly when
        // `A^2(z − 1) − z^2` is a non-square: with `z` of non-square norm
        // `1 + b^2` and `A` a square, the cofactor `A/z^3` of the curve
        // equation is a non-square, so the two characters are opposite. That
        // is the C reference's inversion-free test, and it is also why the
        // C's debug check `!is_square(x(P))` is implied rather than tested:
        // `−A/z` is a square times a non-square.
        let shift = L::ext_i().mul(L::Ext::from_u64(b));
        let z = L::Ext::one().add(shift);
        !shift.mul(square).sub(z.mul(z)).is_square()
    })
}

/// Non-square `A`: the first `n` for which `x(P) = n·A` is on the curve.
///
/// The C holds `n` in a `uint8_t`, so its counter wraps once the fallback
/// search passes 255 while `x` keeps accumulating `A`. The abscissa it ends
/// on is still the one this index names, because `x` is what the loop tests
/// and the counter is only reported back as a hint.
fn multiple_index<L: Level>(coefficient: L::Ext) -> Option<u16> {
    let mut x = coefficient;
    for candidate in 1..=search_bound::<L>() {
        if montgomery_rhs::<L>(coefficient, x).is_square() {
            return Some(candidate);
        }
        x = x.add(coefficient);
    }
    None
}

/// `x^3 + A x^2 + x`. A square value means `x` is the abscissa of a point.
fn montgomery_rhs<L: Level>(coefficient: L::Ext, x: L::Ext) -> L::Ext {
    let square = x.mul(x);
    square.mul(x).add(coefficient.mul(square)).add(x)
}

#[cfg(test)]
mod tests {
    use super::{basis_hint, montgomery_rhs, search_bound, BasisHint, C_HINT_MAX_INDEX};
    use prism_field::{Field, Level, Level1, Level3, Level5};

    /// `x(P)` for an index, by the definition in `docs/VERIFIER_SPEC.md`:
    /// `−A/(1 + i·b)` for a square `A`, and `n·A` otherwise. The search
    /// itself never builds this point, so this is the definition rather than
    /// a second copy of the predicate.
    fn named_point<L: Level>(coefficient: L::Ext, hint: BasisHint) -> Option<L::Ext> {
        let index = u64::from(hint.index());
        if hint.coefficient_is_square() {
            let z = L::Ext::one().add(L::ext_i().mul(L::Ext::from_u64(index)));
            Some(coefficient.mul(z.inv()?).negate())
        } else {
            Some(coefficient.mul(L::Ext::from_u64(index)))
        }
    }

    /// `true` when the index names a point of the curve and clears the
    /// square-`A` branch's `1 + b^2` filter.
    fn qualifies<L: Level>(coefficient: L::Ext, index: u16, is_square: bool) -> bool {
        let hint = BasisHint {
            index,
            coefficient_is_square: is_square,
        };
        let b = u64::from(index);
        if is_square && L::Base::from_u64(b * b + 1).is_square() {
            return false;
        }
        named_point::<L>(coefficient, hint)
            .is_some_and(|x| montgomery_rhs::<L>(coefficient, x).is_square())
    }

    /// `a + b·i` for small `a` and `b`. Every element of `F_p` is a square
    /// in `F_{p^2}` — `a^{(p^2-1)/2} = (a^{p-1})^{(p+1)/2} = 1` — so a family
    /// of small integers would exercise the square branch only. The
    /// imaginary part is what reaches the other one.
    fn coefficients<L: Level>() -> impl Iterator<Item = (u64, u64, L::Ext)> {
        (1u64..=6).flat_map(|re| {
            (0u64..=6).map(move |im| {
                let value = L::ext_from_base(L::Base::from_u64(re))
                    .add(L::ext_i().mul(L::Ext::from_u64(im)));
                (re, im, value)
            })
        })
    }

    fn check_level<L: Level>() {
        let mut squares = 0;
        let mut nonsquares = 0;
        for (re, im, coefficient) in coefficients::<L>() {
            if coefficient.mul(coefficient).same(L::Ext::from_u64(4)) {
                continue;
            }
            let is_square = coefficient.is_square();
            if is_square {
                squares += 1;
            } else {
                nonsquares += 1;
            }
            let hint = basis_hint::<L>(coefficient).expect("an index within the bound");
            assert_eq!(
                hint.coefficient_is_square(),
                is_square,
                "{} A={re}+{im}i",
                L::NAME
            );
            assert!(
                hint.index() >= 1 && hint.index() <= search_bound::<L>(),
                "{} A={re}+{im}i index {}",
                L::NAME,
                hint.index()
            );
            // The derived index names a point of the curve, and it is the
            // smallest index that does: the search order is the C's.
            assert!(
                qualifies::<L>(coefficient, hint.index(), is_square),
                "{} A={re}+{im}i index {} is not on the curve",
                L::NAME,
                hint.index()
            );
            for earlier in 1..hint.index() {
                assert!(
                    !qualifies::<L>(coefficient, earlier, is_square),
                    "{} A={re}+{im}i skipped a usable index {earlier}",
                    L::NAME
                );
            }
        }
        assert!(
            squares > 0 && nonsquares > 0,
            "{} covered both branches",
            L::NAME
        );
    }

    #[test]
    fn the_derived_index_is_the_smallest_on_curve_index() {
        check_level::<Level1>();
        check_level::<Level3>();
        check_level::<Level5>();
    }

    #[test]
    fn the_search_bound_is_twice_the_security_parameter() {
        // The bound is the level's own, not the C hint byte's width. Every
        // level searches past the seven-bit ceiling the C can encode.
        assert_eq!(search_bound::<Level1>(), 256);
        assert_eq!(search_bound::<Level3>(), 384);
        assert_eq!(search_bound::<Level5>(), 512);
        for bound in [
            search_bound::<Level1>(),
            search_bound::<Level3>(),
            search_bound::<Level5>(),
        ] {
            assert!(bound > C_HINT_MAX_INDEX);
        }
    }

    #[test]
    fn the_c_hint_byte_round_trips() {
        for index in 1..=C_HINT_MAX_INDEX {
            for is_square in [false, true] {
                let hint = BasisHint {
                    index,
                    coefficient_is_square: is_square,
                };
                let byte = hint.to_c_byte().expect("an index the byte can name");
                assert_eq!(BasisHint::from_c_byte(byte), Some(hint), "{byte}");
                assert_eq!(u16::from(byte >> 1), index);
                assert_eq!(byte & 1 == 1, is_square);
            }
        }
        // Index zero is the C's "no index below 128" signal, not a hint.
        assert_eq!(BasisHint::from_c_byte(0), None);
        assert_eq!(BasisHint::from_c_byte(1), None);
        // An index past the ceiling has no byte: the C stores the signal and
        // its verifier re-searches, which is what this module always does.
        for index in [C_HINT_MAX_INDEX + 1, search_bound::<Level5>()] {
            let hint = BasisHint {
                index,
                coefficient_is_square: true,
            };
            assert_eq!(hint.to_c_byte(), None, "{index}");
        }
    }
}
