// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The widths of the fixed-width integers a parameter set sizes.
//!
//! The verifier tests the challenge degree `q = 2^(2e′−1) − s²` for
//! primality, with `e′ = e − 2`. The integers of that test are fixed-width,
//! and the width is a property of the parameter set: [`ParameterSet::Degree`]
//! names it as a type, so the test for a set is compiled at that set's
//! width and a set whose degree does not fit a width that exists does not
//! compile.
//!
//! The entries of the verifier's matrices are integers modulo `2^e`, and
//! they are sized the same way: [`ParameterSet::TorsionLimbs`] is the limb
//! array of one at that set.
//!
//! [`ParameterSet::TorsionLimbs`]: crate::ParameterSet::TorsionLimbs
//! [`ParameterSet::Degree`]: crate::ParameterSet::Degree

use core::fmt::Debug;
use core::ops::{Index, IndexMut};

/// Bits in one limb.
pub const LIMB_BITS: usize = u64::BITS as usize;

/// Little-endian 64-bit limbs of one fixed length: `[u64; N]`.
pub trait Limbs:
    Copy + Eq + AsRef<[u64]> + AsMut<[u64]> + Index<usize, Output = u64> + IndexMut<usize>
{
    /// Every limb zero.
    const ZERO: Self;
    /// How many limbs.
    const COUNT: usize;
}

impl<const N: usize> Limbs for [u64; N] {
    const ZERO: Self = [0; N];
    const COUNT: usize = N;
}

/// One width of fixed-width integer, with the scratch arrays its
/// arithmetic needs.
///
/// Stable Rust cannot write `[u64; N + 2]` for a generic `N`, so a width
/// names each array it uses. [`Self::CONSISTENT`] holds the three lengths
/// to each other when the width is used.
pub trait DegreeWidth: Copy + Eq + Debug + 'static {
    /// An integer: `COUNT` limbs.
    type Limbs: Limbs;
    /// A full product of two integers: `2 · COUNT` limbs.
    type Product: Limbs;
    /// The running row of a Montgomery multiplication: `COUNT + 2` limbs.
    type Reduction: Limbs;

    /// The width in bits.
    const BITS: usize = <Self::Limbs as Limbs>::COUNT * LIMB_BITS;

    /// Evaluated by the arithmetic that uses the scratch arrays. A width
    /// whose arrays are not the lengths above does not compile there.
    const CONSISTENT: () = assert!(
        <Self::Product as Limbs>::COUNT == 2 * <Self::Limbs as Limbs>::COUNT
            && <Self::Reduction as Limbs>::COUNT == <Self::Limbs as Limbs>::COUNT + 2,
        "a width's scratch arrays are 2·COUNT and COUNT + 2 limbs"
    );
}

/// The width of `LIMBS` limbs.
///
/// [`DegreeWidth`] is implemented for the limb counts named below and for
/// no other count, so naming another does not compile. [`SetDegree`] is
/// the subset a parameter set may name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DegreeLimbs<const LIMBS: usize>;

/// Limb count of the 64-bit width.
///
/// The carry out of the top limb is checked here, against a `u64`. A
/// parameter set does not name this width.
pub const DEGREE_LIMBS_64: usize = 1;

/// Limb count of the 128-bit width.
///
/// The widest width a `u128` can check exactly. A parameter set does not
/// name this width.
pub const DEGREE_LIMBS_128: usize = 2;

/// Limb count of the 1,024-bit width.
///
/// Every set whose degree fits in 1,024 bits names this width. The four
/// sets below `p664` were built on it before the width was a property of
/// the set, and they keep it.
pub const DEGREE_LIMBS_1024: usize = 16;

/// Limb count of the 1,344-bit width. The width `p664` names: its degree
/// needs 1,324 bits, and twenty limbs are 1,280.
pub const DEGREE_LIMBS_1344: usize = 21;

macro_rules! degree_width {
    ($($limbs:ident),+ $(,)?) => {
        $(
            impl DegreeWidth for DegreeLimbs<$limbs> {
                type Limbs = [u64; $limbs];
                type Product = [u64; 2 * $limbs];
                type Reduction = [u64; $limbs + 2];
            }
        )+
    };
}

degree_width!(
    DEGREE_LIMBS_64,
    DEGREE_LIMBS_128,
    DEGREE_LIMBS_1024,
    DEGREE_LIMBS_1344,
);

/// A [`DegreeWidth`] a parameter set may name.
///
/// The 64-bit and 128-bit widths exist so the carry out of the top limb
/// can be checked. A set does not name them. Adding a wider set means
/// naming its limb count, implementing [`DegreeWidth`] for it, and
/// implementing this trait for it.
pub trait SetDegree: DegreeWidth {}

impl SetDegree for DegreeLimbs<DEGREE_LIMBS_1024> {}
impl SetDegree for DegreeLimbs<DEGREE_LIMBS_1344> {}

/// The limb count every set with `e` up to 514 uses: [`DEGREE_LIMBS_1024`].
///
/// Narrowing each set to what [`degree_bits`] asks for is a later change.
/// Until then a set whose degree fits in 1,024 bits names that width, and
/// [`degree_limbs`] does not return a count below this one.
pub const FLOOR_DEGREE_LIMBS: usize = DEGREE_LIMBS_1024;

/// The bits an integer needs to hold every value the primality test of a
/// set with exponent `e` forms: `2e − 4`.
///
/// The degree is `q = 2^(2e′−1) − s²` with `e′ = e − 2`, so `q < 2^(2e−5)`.
/// Two values the test forms are not below that power: the power itself,
/// from which `s²` is subtracted, and `q + 1` in the Lucas half, which can
/// equal it. Each has its top bit at index `2e − 5` and so needs `2e − 4`
/// bits. Every other intermediate is a residue below `q`, or a sum of two
/// residues whose carry out of the width the arithmetic handles.
pub const fn degree_bits(e: u32) -> usize {
    assert!(e >= 3, "the chain exponent e − 2 is at least one");
    2 * e as usize - 4
}

/// The limbs of the width a set with exponent `e` uses: enough for
/// [`degree_bits`], and not below [`FLOOR_DEGREE_LIMBS`].
pub const fn degree_limbs(e: u32) -> usize {
    let needed = degree_bits(e).div_ceil(LIMB_BITS);
    if needed < FLOOR_DEGREE_LIMBS {
        FLOOR_DEGREE_LIMBS
    } else {
        needed
    }
}

/// The limb count of a residue modulo `2^e` for every set with `e` up to
/// 512: 512 bits.
///
/// The verifier's matrices have entries modulo `2^e`. They were built on
/// eight limbs when the widest exponent was 500, and the four sets below
/// `p664` keep eight. As with [`FLOOR_DEGREE_LIMBS`], narrowing them is a
/// later change.
pub const FLOOR_TORSION_LIMBS: usize = 8;

/// The limbs of a residue modulo `2^e`: enough for `e` bits, and not below
/// [`FLOOR_TORSION_LIMBS`].
pub const fn torsion_limbs(e: u32) -> usize {
    let needed = (e as usize).div_ceil(LIMB_BITS);
    if needed < FLOOR_TORSION_LIMBS {
        FLOOR_TORSION_LIMBS
    } else {
        needed
    }
}

#[cfg(test)]
mod tests {
    use super::{
        degree_bits, degree_limbs, torsion_limbs, DegreeLimbs, DegreeWidth, Limbs,
        DEGREE_LIMBS_1024, DEGREE_LIMBS_128, DEGREE_LIMBS_1344, DEGREE_LIMBS_64,
        FLOOR_DEGREE_LIMBS, FLOOR_TORSION_LIMBS, LIMB_BITS,
    };

    #[test]
    fn a_width_is_its_limbs_and_its_scratch_arrays() {
        fn check<W: DegreeWidth>(limbs: usize) {
            let () = W::CONSISTENT;
            assert_eq!(<W::Limbs as Limbs>::COUNT, limbs);
            assert_eq!(<W::Product as Limbs>::COUNT, 2 * limbs);
            assert_eq!(<W::Reduction as Limbs>::COUNT, limbs + 2);
            assert_eq!(W::BITS, LIMB_BITS * limbs);
            assert!(<W::Limbs as Limbs>::ZERO
                .as_ref()
                .iter()
                .all(|limb| *limb == 0));
        }
        check::<DegreeLimbs<DEGREE_LIMBS_64>>(DEGREE_LIMBS_64);
        check::<DegreeLimbs<DEGREE_LIMBS_128>>(DEGREE_LIMBS_128);
        check::<DegreeLimbs<DEGREE_LIMBS_1024>>(DEGREE_LIMBS_1024);
        check::<DegreeLimbs<DEGREE_LIMBS_1344>>(DEGREE_LIMBS_1344);
    }

    /// Each constant is the bit width its name claims, and the floor is
    /// the 1,024-bit width.
    #[test]
    fn a_width_constant_is_the_bit_width_it_names() {
        assert_eq!(DEGREE_LIMBS_64 * LIMB_BITS, 64);
        assert_eq!(DEGREE_LIMBS_128 * LIMB_BITS, 128);
        assert_eq!(DEGREE_LIMBS_1024 * LIMB_BITS, 1024);
        assert_eq!(DEGREE_LIMBS_1344 * LIMB_BITS, 1344);
        assert_eq!(FLOOR_DEGREE_LIMBS, DEGREE_LIMBS_1024);
    }

    /// The four sets built before `p664` stay on 1,024 bits, and each fits
    /// with room. `p664` needs 1,324 bits, which is 21 limbs: 20 limbs are
    /// 1,280 bits, 44 short.
    #[test]
    fn the_limbs_of_a_set_hold_its_degree() {
        for (e, bits, limbs) in [
            (248, 492, DEGREE_LIMBS_1024),
            (324, 644, DEGREE_LIMBS_1024),
            (376, 748, DEGREE_LIMBS_1024),
            (500, 996, DEGREE_LIMBS_1024),
            (664, 1324, DEGREE_LIMBS_1344),
        ] {
            assert_eq!(degree_bits(e), bits, "e = {e}");
            assert_eq!(degree_limbs(e), limbs, "e = {e}");
            assert!(bits <= LIMB_BITS * limbs);
        }
        assert!(
            degree_bits(664) > LIMB_BITS * (DEGREE_LIMBS_1344 - 1),
            "one limb under p664's width does not hold its degree"
        );
        // The floor ends where the need passes it.
        assert_eq!(degree_limbs(514), FLOOR_DEGREE_LIMBS);
        assert_eq!(degree_limbs(515), FLOOR_DEGREE_LIMBS + 1);
    }

    /// Eight limbs hold `e` up to 512, which is every set below `p664`.
    /// `p664` needs eleven: ten limbs are 640 bits, 24 short.
    #[test]
    fn the_limbs_of_a_residue_hold_the_exponent() {
        for (e, limbs) in [(248, 8), (324, 8), (376, 8), (500, 8), (664, 11)] {
            assert_eq!(torsion_limbs(e), limbs, "e = {e}");
            assert!(e as usize <= 64 * limbs);
        }
        // Ten limbs hold 640 bits and no more.
        assert_eq!(torsion_limbs(640), 10);
        assert_eq!(torsion_limbs(641), 11);
        assert_eq!(torsion_limbs(512), FLOOR_TORSION_LIMBS);
        assert_eq!(torsion_limbs(513), FLOOR_TORSION_LIMBS + 1);
    }
}
