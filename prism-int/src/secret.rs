// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The secret integer and the secret bit.
//!
//! Nothing here calls a routine of the public type. The arithmetic is the
//! fixed-shape limb functions of `limbs.rs`, and every choice between two
//! values is a selection by a mask.

use core::fmt;

use subtle::{Choice, ConditionallySelectable, ConstantTimeEq};
use zeroize::Zeroize;

use crate::error::SecretError;
use crate::limbs;
use crate::public::Int;

/// 1 when the limb is zero, else 0, without a branch.
fn is_zero(limb: u64) -> u8 {
    limb.ct_eq(&0).unwrap_u8()
}

/// `if_set` when `bit` is 1 and `if_clear` when it is 0, limb by limb.
fn select_limbs<const N: usize>(if_clear: &[u64; N], if_set: &[u64; N], bit: u8) -> [u64; N] {
    let choice = Choice::from(bit);
    let mut out = [0u64; N];
    for i in 0..N {
        out[i] = u64::conditional_select(&if_clear[i], &if_set[i], choice);
    }
    out
}

/// `if_set` when `bit` is 1 and `if_clear` when it is 0.
fn select_flag(if_clear: u8, if_set: u8, bit: u8) -> u8 {
    u8::conditional_select(&if_clear, &if_set, Choice::from(bit))
}

/// The answer to a comparison of secrets: a bit that is not a `bool`.
///
/// A caller who could branch on comparisons of a secret with values of
/// their choosing would read the secret one bit at a time. So a
/// comparison of [`SecretInt`]s returns this type, which selects between
/// two secret integers ([`SecretInt::select`], [`SecretInt::swap`]) and
/// does nothing else. It becomes a `bool` only through
/// [`SecretBit::declassify`], which is one of the three counted exits.
///
/// The bit carries the overflow mark of the integers it was computed
/// from: see [`SecretInt`].
///
/// It does not convert to `bool`:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(1);
/// let b = SecretInt::<4>::from_u64(2);
/// let less = a.ct_lt(&b);
/// let branch: bool = less.into();
/// ```
///
/// and is not a condition:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(1);
/// let b = SecretInt::<4>::from_u64(2);
/// let less = a.ct_lt(&b);
/// if less {}
/// ```
///
/// The same lines compile when the bit leaves by its exit:
///
/// ```
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(1);
/// let b = SecretInt::<4>::from_u64(2);
/// let less = a.ct_lt(&b);
/// let branch: bool = less.declassify().unwrap();
/// if branch {}
/// ```
pub struct SecretBit {
    bit: u8,
    overflowed: u8,
}

impl fmt::Debug for SecretBit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretBit")
    }
}

impl Drop for SecretBit {
    fn drop(&mut self) {
        self.wipe();
    }
}

impl SecretBit {
    /// Overwrites the bit and its mark with zero.
    fn wipe(&mut self) {
        self.bit.zeroize();
        self.overflowed.zeroize();
        #[cfg(test)]
        crate::wipes::count();
    }

    /// The bit as a `bool`. **An exit**: one of the three ways a secret
    /// leaves this crate's types.
    ///
    /// Each call site outside this crate has a row in the secret-treatment
    /// table of `docs/SIGNER_SPEC.md` (2.1.3).
    ///
    /// # Errors
    ///
    /// [`SecretError::Overflowed`] if the bit was computed from an integer
    /// whose overflow mark was set. No bit is returned.
    pub fn declassify(&self) -> Result<bool, SecretError> {
        if self.overflowed != 0 {
            return Err(SecretError::Overflowed);
        }
        Ok(self.bit != 0)
    }
}

/// A signed integer of `N` limbs of 64 bits whose value must not be shown.
///
/// The range and the representation are those of [`Int`]: a magnitude of
/// `N` limbs and a sign, zero never negative. Here the sign is a secret
/// too, and no operation looks at it to choose its work.
///
/// # The operations
///
/// Only those whose work depends on `N` and on public counts: addition,
/// subtraction, negation, schoolbook multiplication, shifts, selection
/// and swap by a [`SecretBit`], and equality and comparison, which return
/// a [`SecretBit`]. Everything else (division, a greatest common divisor,
/// a square root, a bit length) is an operation of [`Int`] and is reached
/// through [`SecretInt::declassify`].
///
/// **No constant-time claim is made.** The operations are written with no
/// branch and no index on a secret limb. Whether the compiled code keeps
/// that shape is not measured here.
///
/// # Overflow
///
/// An operation cannot report an overflow without showing that it
/// happened. So a secret integer carries a mark, *overflowed*. An
/// operation whose true result does not fit `N` limbs sets it, and every
/// operation passes it on from its operands: arithmetic to its result,
/// a comparison to its [`SecretBit`], a selection from the value selected
/// and from the bit. The limbs of a marked value mean nothing. The mark is
/// read by the three exits and nowhere else, and each returns
/// [`SecretError::Overflowed`] and no value when it is set.
///
/// ```
/// use prism_int::{SecretError, SecretInt};
///
/// let big = SecretInt::<1>::from_u64(u64::MAX);
/// let wrapped = big.add(&big);
/// assert_eq!(wrapped.declassify().unwrap_err(), SecretError::Overflowed);
/// // A bit computed from it is refused as well.
/// let bit = wrapped.ct_eq(&big);
/// assert_eq!(bit.declassify().unwrap_err(), SecretError::Overflowed);
/// ```
///
/// # The three exits
///
/// [`SecretInt::declassify`], [`SecretBit::declassify`] and
/// [`SecretInt::export_le_bytes`]. There is no other way out, and the
/// examples below are the promises, each as a program that does not
/// compile beside the one that does.
///
/// No equality that returns a `bool`:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(1);
/// let b = SecretInt::<4>::from_u64(1);
/// let same = a == b;
/// ```
///
/// ```
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(1);
/// let b = SecretInt::<4>::from_u64(1);
/// let same = a.ct_eq(&b);
/// ```
///
/// No ordering that returns a `bool`:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(1);
/// let b = SecretInt::<4>::from_u64(2);
/// let less = a < b;
/// ```
///
/// ```
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(1);
/// let b = SecretInt::<4>::from_u64(2);
/// let less = a.ct_lt(&b);
/// ```
///
/// No hash:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let mut set = std::collections::HashSet::new();
/// set.insert(SecretInt::<4>::from_u64(1));
/// ```
///
/// ```
/// use prism_int::SecretInt;
///
/// let mut set = std::collections::HashSet::new();
/// set.insert(SecretInt::<4>::from_u64(1).declassify().unwrap());
/// ```
///
/// No `Display`, and a `Debug` that prints no value:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(5);
/// let text = format!("{a}");
/// ```
///
/// ```
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(5);
/// let text = format!("{a:?}");
/// assert_eq!(text, "SecretInt<4>");
/// ```
///
/// No conversion into the public type by `From`:
///
/// ```compile_fail
/// use prism_int::{Int, SecretInt};
///
/// let a = SecretInt::<4>::from_u64(5);
/// let public = Int::<4>::from(a);
/// ```
///
/// ```compile_fail
/// use prism_int::{Int, SecretInt};
///
/// let a = SecretInt::<4>::from_u64(5);
/// let public: Int<4> = a.into();
/// ```
///
/// nor by a reference:
///
/// ```compile_fail
/// use prism_int::{Int, SecretInt};
///
/// let a = SecretInt::<4>::from_u64(5);
/// let public: &Int<4> = &a;
/// ```
///
/// ```compile_fail
/// use prism_int::{Int, SecretInt};
///
/// let a = SecretInt::<4>::from_u64(5);
/// let public: &Int<4> = a.as_ref();
/// ```
///
/// ```
/// use prism_int::{Int, SecretInt};
///
/// let a = SecretInt::<4>::from_u64(5);
/// let public: Int<4> = a.declassify().unwrap();
/// let public: &Int<4> = &public;
/// ```
///
/// No copy by assignment:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(5);
/// let b = a;
/// let c = a;
/// ```
///
/// ```
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(5);
/// let b = a.clone();
/// let c = a;
/// ```
///
/// No method that formats the value, where the public type has one:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(5);
/// let text = a.public_hex();
/// ```
///
/// ```
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(5);
/// let text = a.declassify().unwrap().public_hex();
/// ```
///
/// And none of the public type's variable-time operations:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(5);
/// let bits = a.bit_length();
/// ```
///
/// ```
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(5);
/// let bits = a.declassify().unwrap().bit_length();
/// ```
///
/// # Wiped on drop
///
/// The limbs, the sign and the mark are overwritten with zeros when a
/// value is dropped. As for [`Int`], that clears the place it was dropped
/// from and not what a move left behind.
pub struct SecretInt<const N: usize> {
    limbs: [u64; N],
    negative: u8,
    overflowed: u8,
}

impl<const N: usize> fmt::Debug for SecretInt<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretInt<{N}>")
    }
}

impl<const N: usize> Clone for SecretInt<N> {
    fn clone(&self) -> Self {
        Self {
            limbs: self.limbs,
            negative: self.negative,
            overflowed: self.overflowed,
        }
    }
}

impl<const N: usize> Drop for SecretInt<N> {
    fn drop(&mut self) {
        self.wipe();
    }
}

impl<const N: usize> SecretInt<N> {
    /// A value from its parts. The sign of a zero magnitude is cleared by
    /// a mask, so that zero has one representation and equality can
    /// compare the parts.
    fn from_parts(limbs: [u64; N], negative: u8, overflowed: u8) -> Self {
        let negative = negative & (is_zero(limbs::fold(&limbs)) ^ 1);
        Self {
            limbs,
            negative,
            overflowed,
        }
    }

    /// Overwrites the value with zero.
    fn wipe(&mut self) {
        self.limbs.zeroize();
        self.negative.zeroize();
        self.overflowed.zeroize();
        #[cfg(test)]
        crate::wipes::count();
    }

    /// Zero.
    pub fn zero() -> Self {
        Self::from_public(&Int::zero())
    }

    /// One.
    pub fn one() -> Self {
        Self::from_public(&Int::one())
    }

    /// The value of a `u64`.
    pub fn from_u64(value: u64) -> Self {
        Self::from_public(&Int::from_u64(value))
    }

    /// The value of an `i64`.
    pub fn from_i64(value: i64) -> Self {
        Self::from_public(&Int::from_i64(value))
    }

    /// The secret integer with the value of a public one. This direction
    /// is not an exit.
    pub fn from_public(value: &Int<N>) -> Self {
        Self::from_parts(*value.magnitude(), u8::from(value.is_negative()), 0)
    }

    /// A non-negative value from the bytes of its magnitude, least
    /// significant first.
    ///
    /// The slice may have any length, which is public. A byte at or past
    /// `8·N` that is not zero does not fit: the overflow mark is set, as
    /// by any other operation, and nothing is returned early.
    pub fn from_le_bytes(bytes: &[u8]) -> Self {
        let mut limbs = [0u64; N];
        let mut lost = 0u8;
        for (i, &byte) in bytes.iter().enumerate() {
            if i / 8 < N {
                limbs[i / 8] |= u64::from(byte) << (8 * (i % 8));
            } else {
                lost |= byte;
            }
        }
        Self::from_parts(limbs, 0, is_zero(u64::from(lost)) ^ 1)
    }

    /// `−self`. It always fits.
    pub fn neg(&self) -> Self {
        Self::from_parts(self.limbs, self.negative ^ 1, self.overflowed)
    }

    /// `self + rhs`. The mark is set if the sum does not fit.
    pub fn add(&self, rhs: &Self) -> Self {
        self.add_signed(rhs, rhs.negative)
    }

    /// `self − rhs`. The mark is set if the difference does not fit.
    pub fn sub(&self, rhs: &Self) -> Self {
        self.add_signed(rhs, rhs.negative ^ 1)
    }

    /// `self` plus the magnitude of `rhs` with the sign given.
    ///
    /// The sum of the magnitudes and both orders of their difference are
    /// all computed, and one is selected: the signs choose nothing.
    fn add_signed(&self, rhs: &Self, rhs_negative: u8) -> Self {
        let (sum, carry) = limbs::add(&self.limbs, &rhs.limbs);
        let (forward, borrow) = limbs::sub(&self.limbs, &rhs.limbs);
        let (backward, _) = limbs::sub(&rhs.limbs, &self.limbs);
        let same_sign = self.negative ^ rhs_negative ^ 1;
        // 1 when |self| < |rhs|: the difference is then `backward`, with
        // the sign of `rhs`.
        let smaller = borrow as u8;
        let difference = select_limbs(&forward, &backward, smaller);
        let difference_sign = select_flag(self.negative, rhs_negative, smaller);
        let magnitude = select_limbs(&difference, &sum, same_sign);
        let negative = select_flag(difference_sign, self.negative, same_sign);
        let overflowed = self.overflowed | rhs.overflowed | (same_sign & carry as u8);
        Self::from_parts(magnitude, negative, overflowed)
    }

    /// `self · rhs`, by all `N²` limb products. The mark is set if the
    /// product does not fit.
    pub fn mul(&self, rhs: &Self) -> Self {
        let (product, lost) = limbs::mul(&self.limbs, &rhs.limbs);
        let overflowed = self.overflowed | rhs.overflowed | (is_zero(lost) ^ 1);
        Self::from_parts(product, self.negative ^ rhs.negative, overflowed)
    }

    /// `self · 2^count`, for a public `count`. The mark is set if a set
    /// bit leaves at the top.
    pub fn shl(&self, count: u32) -> Self {
        let (shifted, lost) = limbs::shl(&self.limbs, count);
        Self::from_parts(
            shifted,
            self.negative,
            self.overflowed | (is_zero(lost) ^ 1),
        )
    }

    /// The magnitude divided by `2^count` and rounded down, with the sign
    /// kept, for a public `count`: `self / 2^count` rounded toward zero.
    pub fn shr(&self, count: u32) -> Self {
        Self::from_parts(
            limbs::shr(&self.limbs, count),
            self.negative,
            self.overflowed,
        )
    }

    /// `if_set` when the bit is set and `if_clear` when it is not.
    ///
    /// The result has the mark of the value selected, and the bit's.
    pub fn select(if_clear: &Self, if_set: &Self, bit: &SecretBit) -> Self {
        Self::from_parts(
            select_limbs(&if_clear.limbs, &if_set.limbs, bit.bit),
            select_flag(if_clear.negative, if_set.negative, bit.bit),
            select_flag(if_clear.overflowed, if_set.overflowed, bit.bit) | bit.overflowed,
        )
    }

    /// Exchanges `a` and `b` when the bit is set, and leaves them when it
    /// is not. Each keeps the mark of the value it ends with, and both
    /// take the bit's.
    pub fn swap(a: &mut Self, b: &mut Self, bit: &SecretBit) {
        let first = Self::select(a, b, bit);
        let second = Self::select(b, a, bit);
        *a = first;
        *b = second;
    }

    /// Whether `self` and `rhs` are equal, as a secret bit.
    pub fn ct_eq(&self, rhs: &Self) -> SecretBit {
        // Zero is never negative, so equal values have equal parts.
        let same_magnitude = is_zero(limbs::difference(&self.limbs, &rhs.limbs));
        let same_sign = self.negative ^ rhs.negative ^ 1;
        SecretBit {
            bit: same_magnitude & same_sign,
            overflowed: self.overflowed | rhs.overflowed,
        }
    }

    /// Whether `self` is below `rhs`, as a secret bit.
    pub fn ct_lt(&self, rhs: &Self) -> SecretBit {
        let below = limbs::sub(&self.limbs, &rhs.limbs).1 as u8;
        let above = limbs::sub(&rhs.limbs, &self.limbs).1 as u8;
        // Of the same sign: two non-negative values compare as their
        // magnitudes, two negative ones the other way round. Of different
        // signs: the negative one is below.
        let same_sign = select_flag(below, above, self.negative);
        let bit = select_flag(same_sign, self.negative, self.negative ^ rhs.negative);
        SecretBit {
            bit,
            overflowed: self.overflowed | rhs.overflowed,
        }
    }

    /// The value as a public integer. **An exit**: one of the three ways a
    /// secret leaves this crate's types.
    ///
    /// Each call site outside this crate has a row in the secret-treatment
    /// table of `docs/SIGNER_SPEC.md` (2.1.3), which says what leaves, why
    /// the operation needs it, and how the last stage treats it.
    ///
    /// # Errors
    ///
    /// [`SecretError::Overflowed`] if the overflow mark is set. No value
    /// is returned.
    pub fn declassify(&self) -> Result<Int<N>, SecretError> {
        if self.overflowed != 0 {
            return Err(SecretError::Overflowed);
        }
        Ok(Int::from_parts(self.limbs, self.negative != 0))
    }

    /// Writes the magnitude into `magnitude`, least significant byte
    /// first, with zeros after it, and the sign into `negative`: 1 for a
    /// value below zero and 0 otherwise. **An exit**: one of the three
    /// ways a secret leaves this crate's types.
    ///
    /// The bytes are a secret still. This is for handing one to a layer
    /// that holds it as such: the encoding of a secret key, a scalar for
    /// the curve arithmetic. Each call site outside this crate has a row
    /// in the secret-treatment table of `docs/SIGNER_SPEC.md` (2.1.3).
    ///
    /// # Errors
    ///
    /// [`SecretError::Overflowed`] if the overflow mark is set, and
    /// [`SecretError::DoesNotFit`] if the magnitude needs more bytes than
    /// `magnitude` has. Both outputs are then all zeros. Either error
    /// shows something of the secret, and the table's row says why it
    /// cannot happen at that call site.
    pub fn export_le_bytes(
        &self,
        magnitude: &mut [u8],
        negative: &mut u8,
    ) -> Result<(), SecretError> {
        let mut lost = 0u8;
        for i in 0..8 * N {
            let byte = (self.limbs[i / 8] >> (8 * (i % 8))) as u8;
            match magnitude.get_mut(i) {
                Some(place) => *place = byte,
                None => lost |= byte,
            }
        }
        for place in magnitude.iter_mut().skip(8 * N) {
            *place = 0;
        }
        *negative = self.negative;
        let refusal = if self.overflowed != 0 {
            Some(SecretError::Overflowed)
        } else if lost != 0 {
            Some(SecretError::DoesNotFit)
        } else {
            None
        };
        match refusal {
            Some(error) => {
                magnitude.zeroize();
                negative.zeroize();
                Err(error)
            }
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    //! What a drop does, as far as safe code can show it: that the wipe
    //! leaves zeros, and that a drop runs the wipe. It cannot read the
    //! memory of a value after its drop, nor what a move left behind.

    use super::{SecretBit, SecretInt};
    use crate::wipes;

    #[test]
    fn the_wipe_of_a_secret_integer_leaves_zeros() {
        // Negative, marked, and with a limb that is not zero.
        let part = SecretInt::<3>::from_i64(-3).shl(190);
        let mut value = part.add(&part);
        assert_eq!(
            (value.limbs, value.negative, value.overflowed),
            ([0, 0, 1 << 63], 1, 1)
        );
        value.wipe();
        assert_eq!(
            (value.limbs, value.negative, value.overflowed),
            ([0, 0, 0], 0, 0)
        );
    }

    #[test]
    fn the_wipe_of_a_secret_bit_leaves_zeros() {
        let marked = SecretInt::<1>::from_u64(u64::MAX).shl(1);
        let mut bit = marked.ct_eq(&marked);
        assert_eq!((bit.bit, bit.overflowed), (1, 1));
        bit.wipe();
        assert_eq!((bit.bit, bit.overflowed), (0, 0));
    }

    #[test]
    fn a_drop_wipes() {
        let value = SecretInt::<3>::from_u64(7);
        let before = wipes::seen();
        drop(value);
        assert_eq!(wipes::seen(), before + 1);

        let value = SecretInt::<3>::from_u64(7);
        let bit: SecretBit = value.ct_eq(&value);
        let before = wipes::seen();
        drop(bit);
        assert_eq!(wipes::seen(), before + 1);
    }
}
