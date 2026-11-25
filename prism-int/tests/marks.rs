// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The overflow mark of the secret types: every operation passes it on,
//! and each of the three exits refuses it. One test for each operation.
//!
//! That an operation *sets* the mark when its own result does not fit is
//! in `vectors.rs`, at every width.

use prism_int::{Int, SecretBit, SecretError, SecretInt};

type Secret = SecretInt<2>;

/// A value whose mark is set: the largest magnitude, doubled.
fn marked() -> Secret {
    let top = Secret::from_public(&Int::from_le_limbs(&[u64::MAX, u64::MAX], false).unwrap());
    top.add(&top)
}

/// A marked value with small limbs, so that arithmetic on it does not
/// overflow again: what is seen is the mark passed on, and nothing new.
fn marked_small() -> Secret {
    marked().shr(120)
}

fn clean() -> Secret {
    Secret::from_i64(-5)
}

fn is_marked(value: &Secret) -> bool {
    match value.declassify() {
        Err(SecretError::Overflowed) => true,
        Ok(_) => false,
        Err(other) => panic!("an integer's exit gave {other:?}"),
    }
}

fn bit_is_marked(bit: &SecretBit) -> bool {
    match bit.declassify() {
        Err(SecretError::Overflowed) => true,
        Ok(_) => false,
        Err(other) => panic!("a bit's exit gave {other:?}"),
    }
}

#[test]
fn the_fixtures_are_what_they_say() {
    assert!(is_marked(&marked()));
    assert!(is_marked(&marked_small()));
    assert!(!is_marked(&clean()));
}

#[test]
fn every_exit_refuses_a_marked_value() {
    let value = marked();
    assert_eq!(value.declassify().unwrap_err(), SecretError::Overflowed);
    assert_eq!(
        value.ct_eq(&value).declassify(),
        Err(SecretError::Overflowed)
    );
    let mut bytes = [0xa5u8; 16];
    let mut negative = 0xa5u8;
    assert_eq!(
        value.export_le_bytes(&mut bytes, &mut negative),
        Err(SecretError::Overflowed)
    );
    assert_eq!((bytes, negative), ([0u8; 16], 0));
    // The mark is reported before a short buffer is.
    let mut short = [0xa5u8; 1];
    assert_eq!(
        value.export_le_bytes(&mut short, &mut negative),
        Err(SecretError::Overflowed)
    );
    assert_eq!(short, [0]);
}

#[test]
fn clone_passes_it_on() {
    assert!(is_marked(&marked().clone()));
    assert!(!is_marked(&clean().clone()));
}

#[test]
fn neg_passes_it_on() {
    assert!(is_marked(&marked().neg()));
    assert!(!is_marked(&clean().neg()));
}

#[test]
fn add_passes_it_on_from_either_operand() {
    assert!(is_marked(&marked().add(&clean())));
    assert!(is_marked(&clean().add(&marked())));
    // Both marked: with a sum that overflows again, and with one that fits.
    assert!(is_marked(&marked().add(&marked())));
    assert!(is_marked(&marked_small().add(&marked_small())));
    assert!(is_marked(&marked_small().add(&marked_small().neg())));
    assert!(!is_marked(&clean().add(&clean())));
}

#[test]
fn sub_passes_it_on_from_either_operand() {
    assert!(is_marked(&marked().sub(&clean())));
    assert!(is_marked(&clean().sub(&marked())));
    // A marked value minus itself has limbs of zero and is marked still.
    assert!(is_marked(&marked().sub(&marked())));
    assert!(is_marked(&marked().sub(&marked().neg())));
    assert!(is_marked(&marked_small().sub(&marked_small().neg())));
    assert!(!is_marked(&clean().sub(&clean())));
}

#[test]
fn mul_passes_it_on_from_either_operand() {
    assert!(is_marked(&marked().mul(&clean())));
    assert!(is_marked(&clean().mul(&marked())));
    // Also when the other operand is zero.
    assert!(is_marked(&marked().mul(&Secret::zero())));
    assert!(is_marked(&Secret::zero().mul(&marked())));
    // Both marked: with a product that overflows again, and with one that fits.
    assert!(is_marked(&marked().mul(&marked())));
    assert!(is_marked(&marked_small().mul(&marked_small())));
    assert!(!is_marked(&clean().mul(&clean())));
}

#[test]
fn the_shifts_pass_it_on() {
    assert!(is_marked(&marked().shl(0)));
    assert!(is_marked(&marked().shl(1)));
    assert!(is_marked(&marked().shr(0)));
    // Also when the shift leaves no bit of the value.
    assert!(is_marked(&marked().shr(128)));
    assert!(is_marked(&marked().shr(1000)));
    // A marked value shifted so that it overflows again.
    assert!(is_marked(&marked().shl(100)));
    assert!(is_marked(&marked().shl(1000)));
    assert!(!is_marked(&clean().shl(3)));
    assert!(!is_marked(&clean().shr(3)));
}

#[test]
fn the_comparisons_pass_it_on_from_either_operand() {
    assert!(bit_is_marked(&marked().ct_eq(&clean())));
    assert!(bit_is_marked(&clean().ct_eq(&marked())));
    assert!(bit_is_marked(&marked().ct_lt(&clean())));
    assert!(bit_is_marked(&clean().ct_lt(&marked())));
    assert!(bit_is_marked(&marked().ct_eq(&marked())));
    assert!(bit_is_marked(&marked().ct_lt(&marked())));
    assert!(!bit_is_marked(&clean().ct_eq(&clean())));
    assert!(!bit_is_marked(&clean().ct_lt(&clean())));
}

#[test]
fn select_passes_on_the_mark_of_the_value_selected_and_of_the_bit() {
    let (zero, one) = (Secret::zero(), Secret::one());
    let set = zero.ct_lt(&one);
    let clear = one.ct_lt(&zero);
    assert_eq!(
        (set.declassify(), clear.declassify()),
        (Ok(true), Ok(false))
    );
    // The value selected is the marked one.
    assert!(is_marked(&Secret::select(&clean(), &marked(), &set)));
    assert!(is_marked(&Secret::select(&marked(), &clean(), &clear)));
    // The value selected is clean, and the other is marked.
    assert!(!is_marked(&Secret::select(&marked(), &clean(), &set)));
    assert!(!is_marked(&Secret::select(&clean(), &marked(), &clear)));
    // The bit is marked, and both values are clean.
    let marked_bit = marked().ct_eq(&marked());
    assert!(is_marked(&Secret::select(&clean(), &clean(), &marked_bit)));
    // The bit and the value selected are both marked.
    assert!(is_marked(&Secret::select(
        &marked(),
        &marked(),
        &marked_bit
    )));
}

#[test]
fn swap_passes_on_the_marks_of_the_values_and_of_the_bit() {
    let (zero, one) = (Secret::zero(), Secret::one());
    let set = zero.ct_lt(&one);
    let clear = one.ct_lt(&zero);

    let (mut a, mut b) = (marked(), clean());
    Secret::swap(&mut a, &mut b, &set);
    assert!(!is_marked(&a) && is_marked(&b));
    Secret::swap(&mut a, &mut b, &clear);
    assert!(!is_marked(&a) && is_marked(&b));

    let marked_bit = marked().ct_eq(&marked());
    let (mut a, mut b) = (clean(), clean());
    Secret::swap(&mut a, &mut b, &marked_bit);
    assert!(is_marked(&a) && is_marked(&b));
}

#[test]
fn bytes_that_do_not_fit_set_it() {
    let mut bytes = [0u8; 17];
    assert!(!is_marked(&Secret::from_le_bytes(&bytes)));
    bytes[16] = 1;
    assert!(is_marked(&Secret::from_le_bytes(&bytes)));
    bytes[16] = 0x80;
    assert!(is_marked(&Secret::from_le_bytes(&bytes)));
    // A longer tail, with the one byte that is set at its end.
    let mut long = [0u8; 40];
    long[39] = 2;
    assert!(is_marked(&Secret::from_le_bytes(&long)));
}

/// A zero the secret type computes is the zero: its sign is cleared, so it
/// equals zero and is not below it. The exits would hide a negative zero,
/// because the public type clears the sign too, so this is asked of the
/// secret comparisons.
#[test]
fn a_computed_zero_is_not_negative() {
    let zero = Secret::zero();
    let minus = Secret::from_i64(-7);
    let zeros = [
        minus.sub(&minus),
        minus.add(&minus.neg()),
        zero.neg(),
        minus.mul(&zero),
        zero.mul(&minus),
        minus.shr(3),
        minus.shr(128),
        Secret::from_i64(0),
        Secret::select(&minus, &zero.neg(), &zero.ct_eq(&zero)),
    ];
    for (index, value) in zeros.iter().enumerate() {
        assert_eq!(value.ct_eq(&zero).declassify(), Ok(true), "zero {index}");
        assert_eq!(value.ct_lt(&zero).declassify(), Ok(false), "zero {index}");
        assert_eq!(zero.ct_lt(value).declassify(), Ok(false), "zero {index}");
        let mut bytes = [0xa5u8; 16];
        let mut negative = 0xa5u8;
        assert_eq!(value.export_le_bytes(&mut bytes, &mut negative), Ok(()));
        assert_eq!((bytes, negative), ([0u8; 16], 0), "zero {index}");
    }
}

/// Neither type prints a value, and no error has one to print.
#[test]
fn nothing_prints_a_value() {
    let public = Int::<2>::from_i64(-0x1234_5678);
    let secret = Secret::from_i64(-0x1234_5678);
    let bit = secret.ct_eq(&secret);
    assert_eq!(format!("{public:?}"), "Int<2>");
    assert_eq!(format!("{secret:?}"), "SecretInt<2>");
    assert_eq!(format!("{bit:?}"), "SecretBit");
    assert_eq!(format!("{:?}", Some(&public)), "Some(Int<2>)");
    let refused = marked().declassify();
    assert_eq!(format!("{refused:?}"), "Err(Overflowed)");
    assert_eq!(public.public_hex(), "-0x12345678");
}

/// Every refusal says what it is, in words of its own.
#[test]
fn every_error_has_its_own_message() {
    use prism_int::IntError;
    use std::collections::BTreeSet;

    let public = [
        IntError::Overflow,
        IntError::DivisionByZero,
        IntError::Inexact,
        IntError::Negative,
        IntError::Modulus,
        IntError::Count,
        IntError::StepBound,
    ];
    let mut messages: BTreeSet<String> = public.iter().map(ToString::to_string).collect();
    messages.insert(SecretError::Overflowed.to_string());
    messages.insert(SecretError::DoesNotFit.to_string());
    assert_eq!(messages.len(), public.len() + 2);
    assert!(messages.iter().all(|message| message.len() > 10));
    assert_eq!(IntError::DivisionByZero.to_string(), "division by zero");
    assert_eq!(
        SecretError::Overflowed.to_string(),
        "a secret computation overflowed"
    );
}

/// Limbs past the width are accepted when they are zero and refused when
/// one is not, wherever it is.
#[test]
fn limbs_past_the_width_must_be_zero() {
    use prism_int::IntError;

    let fits = Int::<2>::from_le_limbs(&[7, 9, 0, 0], true).unwrap();
    assert_eq!(fits.public_hex(), "-0x90000000000000007");
    assert!(Int::<2>::from_le_limbs(&[7], false).unwrap() == Int::from_u64(7));
    assert!(Int::<2>::from_le_limbs(&[], true).unwrap() == Int::zero());
    assert_eq!(
        Int::<2>::from_le_limbs(&[7, 9, 1], false).unwrap_err(),
        IntError::Overflow
    );
    assert_eq!(
        Int::<2>::from_le_limbs(&[0, 0, 0, 1 << 63], false).unwrap_err(),
        IntError::Overflow
    );
    assert_eq!(
        Int::<1>::from_le_limbs(&[0, 1], false).unwrap_err(),
        IntError::Overflow
    );
    assert!(Int::<2>::from_i64(0) == Int::zero() && !Int::<2>::from_i64(0).is_negative());
    assert!(Int::<2>::from_i64(i64::MIN).is_negative());
    assert_eq!(
        Int::<2>::from_i64(i64::MIN).public_hex(),
        "-0x8000000000000000"
    );
}
