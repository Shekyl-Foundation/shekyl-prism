// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The secret integer and the secret bit.
//!
//! Nothing here calls a routine of the public type. The arithmetic is the
//! fixed-shape limb functions of `limbs.rs`, and every choice between two
//! values is a selection by a mask.
//!
//! Every value an operation here gives a name to on its way to the result
//! is held in a [`Scratch`], which overwrites it with zeros when the
//! operation returns. [`SecretInt`] says, under *Wiped on drop*, what
//! that reaches and what it does not.

use core::fmt;
use core::ops::{Deref, DerefMut};

use prism_ct_barrier::mask_from_bit;
use zeroize::Zeroize;

use crate::error::SecretError;
use crate::limbs;
use crate::public::Int;

#[path = "secret_steps.rs"]
mod steps;

/// 1 when the limb is zero, else 0, without a branch.
fn is_zero(limb: u64) -> u8 {
    (((limb | limb.wrapping_neg()) >> 63) as u8) ^ 1
}

/// Writes `if_set` to `out` when `bit` is 1 and `if_clear` when it is 0,
/// limb by limb. The caller owns `out`, so nothing selected is left in a
/// place this function made.
fn select_limbs<const N: usize>(
    if_clear: &[u64; N],
    if_set: &[u64; N],
    bit: u8,
    out: &mut [u64; N],
) {
    let mask = Scratch::new(mask_from_bit(u64::from(bit)));
    for i in 0..N {
        out[i] = if_clear[i] ^ ((if_clear[i] ^ if_set[i]) & *mask);
    }
}

/// A value a secret operation holds on the way to its result: limbs, a
/// carry, a flag. It is overwritten with zeros when it goes out of scope,
/// which is when the operation returns, by whichever path.
///
/// Once an operation has bound it to a name it is read and written through
/// a reference and not moved again, so the zeros are written where the
/// value was. The move that put it there is another matter: see *Wiped on
/// drop* of [`SecretInt`].
struct Scratch<T: Zeroize>(T);

impl<T: Zeroize> Scratch<T> {
    /// Holds `value`.
    fn new(value: T) -> Self {
        Self(value)
    }

    /// Overwrites the value with zero.
    fn wipe(&mut self) {
        self.0.zeroize();
        #[cfg(test)]
        crate::wipes::count();
    }
}

impl<A: Zeroize, B: Zeroize> Scratch<(A, B)> {
    /// Holds the two results of a limb function, each on its own.
    fn pair((first, second): (A, B)) -> (Scratch<A>, Scratch<B>) {
        (Scratch(first), Scratch(second))
    }
}

impl<T: Zeroize> Drop for Scratch<T> {
    fn drop(&mut self) {
        self.wipe();
    }
}

impl<T: Zeroize> Deref for Scratch<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Zeroize> DerefMut for Scratch<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// `if_set` when `bit` is 1 and `if_clear` when it is 0.
fn select_flag(if_clear: u8, if_set: u8, bit: u8) -> u8 {
    if_clear ^ ((if_clear ^ if_set) & mask_from_bit(u64::from(bit)) as u8)
}

/// The answer to a comparison of secrets: a bit that is not a `bool`.
///
/// A caller who could branch on comparisons of a secret with values of
/// their choosing would read the secret one bit at a time. So a
/// comparison of [`SecretInt`]s returns this type, which combines with other secret predicates,
/// selects or swaps secret integers, and can mark an invalid result. It becomes a `bool` only through
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
/// Arithmetic, comparisons and selections run fixed public loops. Secret
/// shift counts and bit lengths stay in the secret domain. The reducer
/// has fixed-width multiplication and 128-by-64 division. General
/// division, square roots and greatest common divisors are public operations.
///
/// The release shift consumer is inspected by `scripts/ci/check_ct_shift.py`.
/// Instruction latency and the integrated signer require separate evidence.
///
/// # Overflow
///
/// An operation cannot report an overflow without showing that it
/// happened. So a secret integer carries a mark, *overflowed*. An
/// operation whose true result does not fit `N` limbs sets it. An admission
/// failure sets the same mark through [`SecretInt::mark_if`]. Every
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
/// A secret bit length stays inside the secret domain:
///
/// ```compile_fail
/// use prism_int::SecretInt;
///
/// let a = SecretInt::<4>::from_u64(5);
/// let bits: u32 = a.bit_length();
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
///
/// An operation wipes what it held as well. Every intermediate it names
/// (the sum and the two differences an addition selects from, a product,
/// a shifted magnitude, a carry, a borrow, a flag) is overwritten with
/// zeros before the operation returns, and [`SecretInt::swap`] copies into
/// its two operands in place and wipes the two values it selected.
///
/// **What that does not cover.** No safe code can clear these, and this
/// crate forbids the other kind:
///
/// - what a limb function holds while it works, and its result before the
///   operation has taken hold of it: the limb functions are shared with
///   the public type, return their limbs by value, and widen each carry
///   to 128 bits on the way;
/// - the result of an operation, in the place it was built before it was
///   moved to the caller, and any other copy a move leaves: a compiler
///   usually builds a result where the caller wants it, and promises
///   nothing;
/// - a sign or a mark passed between two functions of this crate as an
///   argument, and anything the compiler keeps in a register or spills to
///   the stack on its own account;
/// - the public integer [`SecretInt::declassify`] returns. The magnitude
///   is handed to it by value, which is one more copy the compiler may
///   leave. It is wiped when dropped, as every [`Int`] is, and the
///   intermediates of the public operations run on it are not, but for
///   the dividend of a division.
///
/// What the tests show is that each wipe runs and writes zeros. They
/// cannot read a stack after the fact, and nobody has measured what the
/// compiled code leaves.
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
    ///
    /// The limbs are copied from where the caller holds them, which the
    /// caller then wipes.
    fn from_parts(limbs: &[u64; N], negative: u8, overflowed: u8) -> Self {
        Self {
            limbs: *limbs,
            negative: negative & (is_zero(limbs::fold(limbs)) ^ 1),
            overflowed,
        }
    }

    /// Makes `self` a copy of `other`, in place.
    fn assign(&mut self, other: &Self) {
        self.limbs = other.limbs;
        self.negative = other.negative;
        self.overflowed = other.overflowed;
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
        Self::from_parts(value.magnitude(), u8::from(value.is_negative()), 0)
    }

    /// A non-negative value from the bytes of its magnitude, least
    /// significant first.
    ///
    /// The slice may have any length, which is public. A byte at or past
    /// `8·N` that is not zero does not fit: the overflow mark is set, as
    /// by any other operation, and nothing is returned early.
    pub fn from_le_bytes(bytes: &[u8]) -> Self {
        let mut magnitude = Scratch::new([0u64; N]);
        let mut lost = Scratch::new(0u8);
        for (i, byte) in bytes.iter().enumerate() {
            if i / 8 < N {
                magnitude[i / 8] |= u64::from(*byte) << (8 * (i % 8));
            } else {
                *lost |= *byte;
            }
        }
        Self::from_parts(&magnitude, 0, is_zero(u64::from(*lost)) ^ 1)
    }

    /// `−self`. It always fits.
    pub fn neg(&self) -> Self {
        Self::from_parts(&self.limbs, self.negative ^ 1, self.overflowed)
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
        let (sum, carry) = Scratch::pair(limbs::add(&self.limbs, &rhs.limbs));
        // The borrow is 1 when |self| < |rhs|: the difference is then
        // `backward`, with the sign of `rhs`.
        let (forward, smaller) = Scratch::pair(limbs::sub(&self.limbs, &rhs.limbs));
        let (backward, _) = Scratch::pair(limbs::sub(&rhs.limbs, &self.limbs));
        let same_sign = Scratch::new(self.negative ^ rhs_negative ^ 1);
        let mut difference = Scratch::new([0u64; N]);
        select_limbs(&forward, &backward, *smaller as u8, &mut difference);
        let difference_sign =
            Scratch::new(select_flag(self.negative, rhs_negative, *smaller as u8));
        let mut magnitude = Scratch::new([0u64; N]);
        select_limbs(&difference, &sum, *same_sign, &mut magnitude);
        Self::from_parts(
            &magnitude,
            select_flag(*difference_sign, self.negative, *same_sign),
            self.overflowed | rhs.overflowed | (*same_sign & *carry as u8),
        )
    }

    /// `self · rhs`, by all `N²` limb products. The mark is set if the
    /// product does not fit.
    pub fn mul(&self, rhs: &Self) -> Self {
        let (product, lost) = Scratch::pair(limbs::mul(&self.limbs, &rhs.limbs));
        Self::from_parts(
            &product,
            self.negative ^ rhs.negative,
            self.overflowed | rhs.overflowed | (is_zero(*lost) ^ 1),
        )
    }

    /// `self · 2^count`, for a public `count`. The mark is set if a set
    /// bit leaves at the top.
    pub fn shl(&self, count: u32) -> Self {
        let (shifted, lost) = Scratch::pair(limbs::shl(&self.limbs, count));
        Self::from_parts(
            &shifted,
            self.negative,
            self.overflowed | (is_zero(*lost) ^ 1),
        )
    }

    /// The magnitude divided by `2^count` and rounded down, with the sign
    /// kept, for a public `count`: `self / 2^count` rounded toward zero.
    pub fn shr(&self, count: u32) -> Self {
        let shifted = Scratch::new(limbs::shr(&self.limbs, count));
        Self::from_parts(&shifted, self.negative, self.overflowed)
    }

    /// `if_set` when the bit is set and `if_clear` when it is not.
    ///
    /// The result has the mark of the value selected, and the bit's.
    pub fn select(if_clear: &Self, if_set: &Self, bit: &SecretBit) -> Self {
        let mut chosen = Scratch::new([0u64; N]);
        select_limbs(&if_clear.limbs, &if_set.limbs, bit.bit, &mut chosen);
        Self::from_parts(
            &chosen,
            select_flag(if_clear.negative, if_set.negative, bit.bit),
            select_flag(if_clear.overflowed, if_set.overflowed, bit.bit) | bit.overflowed,
        )
    }

    /// Exchanges `a` and `b` when the bit is set, and leaves them when it
    /// is not. Each keeps the mark of the value it ends with, and both
    /// take the bit's.
    pub fn swap(a: &mut Self, b: &mut Self, bit: &SecretBit) {
        // Copied into place, and not moved: `first` and `second` are then
        // dropped where they are, which wipes them.
        let first = Self::select(a, b, bit);
        let second = Self::select(b, a, bit);
        a.assign(&first);
        b.assign(&second);
    }

    /// Whether `self` and `rhs` are equal, as a secret bit.
    pub fn ct_eq(&self, rhs: &Self) -> SecretBit {
        // Zero is never negative, so equal values have equal parts.
        SecretBit {
            bit: is_zero(limbs::difference(&self.limbs, &rhs.limbs))
                & (self.negative ^ rhs.negative ^ 1),
            overflowed: self.overflowed | rhs.overflowed,
        }
    }

    /// Whether `self` is below `rhs`, as a secret bit.
    pub fn ct_lt(&self, rhs: &Self) -> SecretBit {
        // The two differences are computed for their borrows alone, and
        // wiped at the end of the line that made them.
        let (_, below) = Scratch::pair(limbs::sub(&self.limbs, &rhs.limbs));
        let (_, above) = Scratch::pair(limbs::sub(&rhs.limbs, &self.limbs));
        // Of the same sign: two non-negative values compare as their
        // magnitudes, two negative ones the other way round. Of different
        // signs: the negative one is below.
        let same_sign = Scratch::new(select_flag(*below as u8, *above as u8, self.negative));
        SecretBit {
            bit: select_flag(*same_sign, self.negative, self.negative ^ rhs.negative),
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

    /// Byte `index` of the magnitude, counted from the least significant.
    fn byte(&self, index: usize) -> u8 {
        (self.limbs[index / 8] >> (8 * (index % 8))) as u8
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
        let mut lost = Scratch::new(0u8);
        for i in 0..8 * N {
            match magnitude.get_mut(i) {
                Some(place) => *place = self.byte(i),
                None => *lost |= self.byte(i),
            }
        }
        for place in magnitude.iter_mut().skip(8 * N) {
            *place = 0;
        }
        *negative = self.negative;
        let refusal = if self.overflowed != 0 {
            Some(SecretError::Overflowed)
        } else if *lost != 0 {
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

    use super::{Scratch, SecretBit, SecretInt};
    use crate::wipes;

    /// The wipes `run` does, the drop of its result not among them.
    fn wipes_of<T>(run: impl FnOnce() -> T) -> usize {
        let before = wipes::seen();
        let result = run();
        let after = wipes::seen();
        drop(result);
        after - before
    }

    #[test]
    fn the_wipe_of_a_scratch_value_leaves_zeros() {
        let mut limbs = Scratch::new([1u64, 2, 3]);
        limbs.wipe();
        assert_eq!(*limbs, [0, 0, 0]);
        let mut flag = Scratch::new(1u8);
        flag.wipe();
        assert_eq!(*flag, 0);
        let (mut limbs, mut carry) = Scratch::pair(([4u64, 5], 1u64));
        assert_eq!((*limbs, *carry), ([4, 5], 1));
        limbs.wipe();
        carry.wipe();
        assert_eq!((*limbs, *carry), ([0, 0], 0));
    }

    #[test]
    fn a_scratch_value_is_wiped_when_it_goes_out_of_scope() {
        assert_eq!(wipes_of(|| drop(Scratch::new([7u64; 3]))), 1);
        assert_eq!(wipes_of(|| drop(Scratch::pair(([7u64; 3], 1u64)))), 2);
    }

    /// Each operation wipes every intermediate it names. The counts are
    /// those of the code above: an addition holds the sum and its carry,
    /// two differences and their borrows, the difference and the
    /// magnitude selected, and two flags.
    #[test]
    fn every_operation_wipes_what_it_held() {
        let a = SecretInt::<3>::from_i64(-9).shl(70);
        let b = SecretInt::<3>::from_u64(5);
        assert_eq!(wipes_of(|| a.add(&b)), 12);
        assert_eq!(wipes_of(|| a.sub(&b)), 12);
        assert_eq!(wipes_of(|| a.mul(&b)), 2);
        assert_eq!(wipes_of(|| a.shl(3)), 2);
        assert_eq!(wipes_of(|| a.shr(3)), 1);
        assert_eq!(wipes_of(|| a.neg()), 0);
        assert_eq!(wipes_of(|| a.ct_eq(&b)), 0);
        assert_eq!(wipes_of(|| a.ct_lt(&b)), 5);
        assert_eq!(wipes_of(|| SecretInt::<3>::from_le_bytes(&[1; 30])), 2);
        let bit = a.ct_lt(&b);
        assert_eq!(wipes_of(|| SecretInt::select(&a, &b, &bit)), 2);
        // A swap selects twice, and wipes the two values it selected.
        let (mut x, mut y) = (a.clone(), b.clone());
        assert_eq!(wipes_of(|| SecretInt::swap(&mut x, &mut y, &bit)), 6);
        assert!(x.ct_eq(&b).declassify().unwrap() && y.ct_eq(&a).declassify().unwrap());
        let (mut bytes, mut sign) = ([0u8; 24], 0u8);
        assert_eq!(wipes_of(|| a.export_le_bytes(&mut bytes, &mut sign)), 1);
        assert_eq!(
            wipes_of(|| a.export_le_bytes(&mut bytes[..1], &mut sign)),
            1
        );
    }

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
