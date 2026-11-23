// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The public integer: construction, the ring operations, comparison.
//!
//! Division is in `division.rs` and what is built on it in `number.rs`.

use core::cmp::Ordering;
use core::fmt;

use zeroize::Zeroize;

use crate::error::IntError;
use crate::limbs::{self, LIMB_BITS};

/// A signed integer of `N` limbs of 64 bits whose value may be shown.
///
/// The range is `−(2^(64N) − 1)` to `2^(64N) − 1`: the magnitude is held in
/// the limbs and the sign beside it, so a negation never overflows. Zero
/// has one representation, and it is not negative.
///
/// The time of an operation may depend on the values. A value that must
/// not be shown belongs in [`SecretInt`](crate::SecretInt), and reaches
/// this type only through
/// [`SecretInt::declassify`](crate::SecretInt::declassify).
///
/// An operation whose result does not fit returns
/// [`IntError::Overflow`] and no value. Nothing wraps and nothing is
/// cut short.
///
/// # What it does not do
///
/// It is not `Copy`: a value is hundreds of bytes. It has no `Display`,
/// and its `Debug` prints the type's name and no value, because in the
/// signer a public integer is often a declassified secret, and a failed
/// `assert_eq!` or an `unwrap` would print it. A value the caller knows
/// to be public is formatted by [`Int::public_hex`].
///
/// ```
/// use prism_int::Int;
///
/// let a = Int::<4>::from_i64(-7);
/// assert_eq!(format!("{a:?}"), "Int<4>");
/// assert_eq!(a.public_hex(), "-0x7");
/// ```
///
/// The limbs are overwritten with zeros when a value is dropped. That
/// clears the place it was dropped from. It does not clear what a move
/// left behind on the stack, and nothing in safe Rust can.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Int<const N: usize> {
    limbs: [u64; N],
    negative: bool,
}

impl<const N: usize> fmt::Debug for Int<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Int<{N}>")
    }
}

impl<const N: usize> Drop for Int<N> {
    fn drop(&mut self) {
        self.wipe();
    }
}

impl<const N: usize> PartialOrd for Int<N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const N: usize> Ord for Int<N> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_magnitude(&self.limbs, &other.limbs),
            (true, true) => cmp_magnitude(&other.limbs, &self.limbs),
        }
    }
}

/// Compares two magnitudes, from the top limb down.
pub(crate) fn cmp_magnitude<const N: usize>(a: &[u64; N], b: &[u64; N]) -> Ordering {
    for i in (0..N).rev() {
        match a[i].cmp(&b[i]) {
            Ordering::Equal => {}
            unequal => return unequal,
        }
    }
    Ordering::Equal
}

impl<const N: usize> Int<N> {
    /// Refuses `N = 0` when the type is used, at compile time.
    const AT_LEAST_ONE_LIMB: () = assert!(N > 0, "an integer has at least one limb");

    /// The bits of the magnitude: `64·N`.
    pub const BITS: u32 = {
        let () = Self::AT_LEAST_ONE_LIMB;
        assert!(N <= (u32::MAX / LIMB_BITS) as usize, "too many limbs");
        N as u32 * LIMB_BITS
    };

    /// A value from its magnitude and sign. A zero is never negative.
    pub(crate) fn from_parts(limbs: [u64; N], negative: bool) -> Self {
        let () = Self::AT_LEAST_ONE_LIMB;
        let negative = negative && limbs::fold(&limbs) != 0;
        Self { limbs, negative }
    }

    /// The magnitude.
    pub(crate) fn magnitude(&self) -> &[u64; N] {
        &self.limbs
    }

    /// Overwrites the value with zero.
    fn wipe(&mut self) {
        self.limbs.zeroize();
        self.negative = false;
        #[cfg(test)]
        crate::wipes::count();
    }

    /// Zero.
    pub fn zero() -> Self {
        Self::from_parts([0u64; N], false)
    }

    /// One.
    pub fn one() -> Self {
        Self::from_u64(1)
    }

    /// The value of a `u64`.
    pub fn from_u64(value: u64) -> Self {
        let mut limbs = [0u64; N];
        let () = Self::AT_LEAST_ONE_LIMB;
        limbs[0] = value;
        Self::from_parts(limbs, false)
    }

    /// The value of an `i64`.
    pub fn from_i64(value: i64) -> Self {
        let mut limbs = [0u64; N];
        let () = Self::AT_LEAST_ONE_LIMB;
        limbs[0] = value.unsigned_abs();
        Self::from_parts(limbs, value.is_negative())
    }

    /// A value from the limbs of its magnitude, least significant first,
    /// and its sign.
    ///
    /// The slice may be shorter than `N`, and it may be longer if the
    /// limbs past `N` are zero.
    ///
    /// # Errors
    ///
    /// [`IntError::Overflow`] if a limb at or past `N` is not zero.
    pub fn from_le_limbs(magnitude: &[u64], negative: bool) -> Result<Self, IntError> {
        let mut limbs = [0u64; N];
        for (i, &limb) in magnitude.iter().enumerate() {
            if i < N {
                limbs[i] = limb;
            } else if limb != 0 {
                limbs.zeroize();
                return Err(IntError::Overflow);
            }
        }
        Ok(Self::from_parts(limbs, negative))
    }

    /// A non-negative value from the bytes of its magnitude, least
    /// significant first.
    ///
    /// The slice may have any length. How a sign is encoded is the
    /// caller's format: a negative value is made with [`Int::neg`].
    ///
    /// # Errors
    ///
    /// [`IntError::Overflow`] if a byte at or past `8·N` is not zero.
    pub fn from_le_bytes(bytes: &[u8]) -> Result<Self, IntError> {
        let mut limbs = [0u64; N];
        for (i, &byte) in bytes.iter().enumerate() {
            if i / 8 < N {
                limbs[i / 8] |= u64::from(byte) << (8 * (i % 8));
            } else if byte != 0 {
                limbs.zeroize();
                return Err(IntError::Overflow);
            }
        }
        Ok(Self::from_parts(limbs, false))
    }

    /// Writes the magnitude into `out`, least significant byte first, and
    /// fills the rest of `out` with zeros. The sign is not written: see
    /// [`Int::is_negative`].
    ///
    /// # Errors
    ///
    /// [`IntError::Overflow`] if the magnitude needs more bytes than `out`
    /// has. `out` is then all zeros.
    pub fn to_le_bytes(&self, out: &mut [u8]) -> Result<(), IntError> {
        let mut lost = 0u8;
        for i in 0..8 * N {
            let byte = (self.limbs[i / 8] >> (8 * (i % 8))) as u8;
            match out.get_mut(i) {
                Some(place) => *place = byte,
                None => lost |= byte,
            }
        }
        for place in out.iter_mut().skip(8 * N) {
            *place = 0;
        }
        if lost != 0 {
            out.zeroize();
            return Err(IntError::Overflow);
        }
        Ok(())
    }

    /// The value as `0x…` in lowercase hexadecimal with no leading zero,
    /// after a `-` if it is negative. Zero is `0x0`.
    ///
    /// This prints the value. It is for a value the caller knows to be
    /// public: a constant, a test's operand, a degree. The name is there
    /// to be searched for.
    pub fn public_hex(&self) -> String {
        let mut digits = String::new();
        for i in (0..N).rev() {
            if digits.is_empty() {
                if self.limbs[i] != 0 {
                    digits = format!("{:x}", self.limbs[i]);
                }
            } else {
                digits.push_str(&format!("{:016x}", self.limbs[i]));
            }
        }
        if digits.is_empty() {
            digits.push('0');
        }
        let sign = if self.negative { "-" } else { "" };
        format!("{sign}0x{digits}")
    }

    /// Whether the value is below zero.
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// Whether the value is zero.
    pub fn is_zero(&self) -> bool {
        limbs::fold(&self.limbs) == 0
    }

    /// Whether the value is odd.
    pub fn is_odd(&self) -> bool {
        self.limbs[0] & 1 == 1
    }

    /// The low `count` bits of the magnitude, for `count` up to 64.
    ///
    /// For a negative value this is not the residue modulo `2^count`:
    /// that is [`Int::residue_pow2`].
    ///
    /// # Errors
    ///
    /// [`IntError::Count`] if `count` is above 64.
    pub fn low_bits(&self, count: u32) -> Result<u64, IntError> {
        Ok(self.limbs[0] & low_mask(count)?)
    }

    /// The value modulo `2^count`, in `[0, 2^count)`, for `count` up to 64.
    ///
    /// The reference reads a residue modulo 2, 4 or 8 of a possibly
    /// negative integer as `ibz_get(x) & 1`, `& 3` or `& 7`; this is that
    /// residue.
    ///
    /// # Errors
    ///
    /// [`IntError::Count`] if `count` is above 64.
    pub fn residue_pow2(&self, count: u32) -> Result<u64, IntError> {
        let mask = low_mask(count)?;
        let low = self.limbs[0];
        Ok(if self.negative {
            low.wrapping_neg() & mask
        } else {
            low & mask
        })
    }

    /// The bit length of the magnitude: 0 for zero, and otherwise one more
    /// than the index of the highest set bit.
    pub fn bit_length(&self) -> u32 {
        bit_length(&self.limbs)
    }

    /// Bit `index` of the magnitude, for an index below [`Int::BITS`].
    pub(crate) fn bit(&self, index: u32) -> bool {
        (self.limbs[(index / LIMB_BITS) as usize] >> (index % LIMB_BITS)) & 1 == 1
    }

    /// The number of trailing zero bits, which is the two-adic valuation,
    /// or `None` for zero, which has none.
    pub fn trailing_zeros(&self) -> Option<u32> {
        let mut below = 0u32;
        for &limb in &self.limbs {
            if limb != 0 {
                return Some(below + limb.trailing_zeros());
            }
            below += LIMB_BITS;
        }
        None
    }

    /// `−self`. It always fits.
    pub fn neg(&self) -> Self {
        Self::from_parts(self.limbs, !self.negative)
    }

    /// `|self|`.
    pub(crate) fn abs(&self) -> Self {
        Self::from_parts(self.limbs, false)
    }

    /// `self + rhs`.
    ///
    /// # Errors
    ///
    /// [`IntError::Overflow`] if the sum does not fit.
    pub fn add(&self, rhs: &Self) -> Result<Self, IntError> {
        self.add_signed(&rhs.limbs, rhs.negative)
    }

    /// `self − rhs`.
    ///
    /// # Errors
    ///
    /// [`IntError::Overflow`] if the difference does not fit.
    pub fn sub(&self, rhs: &Self) -> Result<Self, IntError> {
        self.add_signed(&rhs.limbs, !rhs.negative)
    }

    /// `self` plus the value of that magnitude and sign.
    fn add_signed(&self, magnitude: &[u64; N], negative: bool) -> Result<Self, IntError> {
        if self.negative == negative {
            let (mut sum, carry) = limbs::add(&self.limbs, magnitude);
            if carry != 0 {
                sum.zeroize();
                return Err(IntError::Overflow);
            }
            return Ok(Self::from_parts(sum, negative));
        }
        Ok(match cmp_magnitude(&self.limbs, magnitude) {
            Ordering::Less => Self::from_parts(limbs::sub(magnitude, &self.limbs).0, negative),
            _ => Self::from_parts(limbs::sub(&self.limbs, magnitude).0, self.negative),
        })
    }

    /// `self · rhs`.
    ///
    /// # Errors
    ///
    /// [`IntError::Overflow`] if the product does not fit.
    pub fn mul(&self, rhs: &Self) -> Result<Self, IntError> {
        let (mut product, lost) = limbs::mul(&self.limbs, &rhs.limbs);
        if lost != 0 {
            product.zeroize();
            return Err(IntError::Overflow);
        }
        Ok(Self::from_parts(product, self.negative != rhs.negative))
    }

    /// `self · 2^count`.
    ///
    /// # Errors
    ///
    /// [`IntError::Overflow`] if a set bit would leave at the top.
    pub fn shl(&self, count: u32) -> Result<Self, IntError> {
        let (mut shifted, lost) = limbs::shl(&self.limbs, count);
        if lost != 0 {
            shifted.zeroize();
            return Err(IntError::Overflow);
        }
        Ok(Self::from_parts(shifted, self.negative))
    }

    /// The magnitude divided by `2^count` and rounded down, with the sign
    /// kept: `self / 2^count` rounded toward zero, as the reference's
    /// `ibz_div_2exp`.
    pub fn shr(&self, count: u32) -> Self {
        Self::from_parts(limbs::shr(&self.limbs, count), self.negative)
    }
}

/// A mask of the low `count` bits of a limb.
fn low_mask(count: u32) -> Result<u64, IntError> {
    match count {
        0 => Ok(0),
        1..=LIMB_BITS => Ok(u64::MAX >> (LIMB_BITS - count)),
        _ => Err(IntError::Count),
    }
}

/// The number of limbs up to and including the highest that is not zero.
pub(crate) fn significant<const N: usize>(a: &[u64; N]) -> usize {
    for i in (0..N).rev() {
        if a[i] != 0 {
            return i + 1;
        }
    }
    0
}

/// The bit length of a magnitude.
pub(crate) fn bit_length<const N: usize>(a: &[u64; N]) -> u32 {
    match significant(a) {
        0 => 0,
        used => used as u32 * LIMB_BITS - a[used - 1].leading_zeros(),
    }
}

#[cfg(test)]
mod tests {
    //! What a drop does, as far as safe code can show it: see the tests of
    //! `secret.rs`.

    use super::Int;
    use crate::wipes;

    #[test]
    fn the_wipe_leaves_zeros() {
        let mut value = Int::<3>::from_i64(-9).shl(100).unwrap();
        assert_ne!(value.limbs, [0, 0, 0]);
        assert!(value.negative);
        value.wipe();
        assert_eq!(value.limbs, [0, 0, 0]);
        assert!(!value.negative);
    }

    #[test]
    fn a_drop_wipes() {
        let value = Int::<3>::from_u64(7);
        let before = wipes::seen();
        drop(value);
        assert_eq!(wipes::seen(), before + 1);
    }

    #[test]
    fn a_zero_is_not_negative() {
        assert!(!Int::<3>::from_parts([0, 0, 0], true).negative);
        assert!(Int::<3>::from_parts([0, 0, 1], true).negative);
    }
}
