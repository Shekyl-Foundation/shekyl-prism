// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Both types against an arbitrary-precision integer, on operands drawn
//! from a fixed stream, at one, two and three limbs and at the five widths
//! of the parameter sets: the ring operations and division.
//!
//! The oracle is `num-bigint`. It decides whether a result fits, so every
//! overflow here is one the oracle saw and the crate reported. The
//! committed vectors (`vectors.rs`) hold the edges by name; this holds the
//! bulk.
//!
//! What is built on division (greatest common divisors, roots, modular
//! arithmetic) is in `oracle_number.rs`, which Cargo runs after this file.
//! Those routines loop until a remainder is zero, so a wrong division can
//! keep them from ending; here a wrong division fails first.

mod support;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};
use prism_int::{IntError, SecretError, SecretInt};
use support::{big, fits, same, Stream};

/// The secret type's result is the oracle's: the value when it fits, and
/// the mark when it does not.
fn same_secret<const N: usize>(got: &SecretInt<N>, want: &BigInt, what: &str) {
    match got.declassify() {
        Ok(value) => assert_eq!(&big(&value), want, "secret {what} at {N} limbs"),
        Err(error) => {
            assert_eq!(error, SecretError::Overflowed, "secret {what} at {N} limbs");
            assert!(
                !fits::<N>(want),
                "secret {what} at {N} limbs: marked a result that fits"
            );
        }
    }
}

fn ring<const N: usize>(stream: &mut Stream) {
    for round in 0..150 {
        // Half of the pairs are short enough that their product fits.
        let limbs = if round % 2 == 0 { N } else { N.div_ceil(2) };
        let (a, b) = (stream.int::<N>(limbs), stream.int::<N>(limbs));
        let (x, y) = (big(&a), big(&b));
        same(a.add(&b), &(&x + &y), "add");
        same(a.sub(&b), &(&x - &y), "sub");
        same(a.mul(&b), &(&x * &y), "mul");
        assert_eq!(a.cmp(&b), x.cmp(&y), "cmp at {N} limbs");
        assert_eq!(
            a.partial_cmp(&b),
            Some(x.cmp(&y)),
            "partial_cmp at {N} limbs"
        );
        assert_eq!(a == b, x == y, "eq at {N} limbs");
        assert_eq!(big(&a.neg()), -&x);

        let (sa, sb) = (SecretInt::from_public(&a), SecretInt::from_public(&b));
        same_secret(&sa.add(&sb), &(&x + &y), "add");
        same_secret(&sa.sub(&sb), &(&x - &y), "sub");
        same_secret(&sa.mul(&sb), &(&x * &y), "mul");
        same_secret(&sa.neg(), &-&x, "neg");
        assert_eq!(sa.ct_eq(&sb).declassify(), Ok(x == y));
        assert_eq!(sa.ct_lt(&sb).declassify(), Ok(x < y));
        let picked = SecretInt::select(&sa, &sb, &sa.ct_lt(&sb));
        assert_eq!(big(&picked.declassify().unwrap()), x.clone().max(y.clone()));

        let count = match stream.below(4) {
            0 => stream.below(8) as u32,
            1 => 64 * stream.below(N as u64 + 2) as u32,
            _ => stream.below(64 * N as u64 + 70) as u32,
        };
        let magnitude = x.abs();
        let down = if x.is_negative() {
            -(&magnitude >> count)
        } else {
            &magnitude >> count
        };
        same(a.shl(count), &(&x << count), "shl");
        same(Ok(a.shr(count)), &down, "shr");
        same_secret(&sa.shl(count), &(&x << count), "shl");
        same_secret(&sa.shr(count), &down, "shr");

        assert_eq!(u64::from(a.bit_length()), x.bits());
        assert_eq!(a.trailing_zeros().map(u64::from), x.trailing_zeros());
        let bits = stream.below(65) as u32;
        let modulus = BigInt::one() << bits;
        assert_eq!(
            BigInt::from(a.low_bits(bits).unwrap()),
            &magnitude % &modulus
        );
        assert_eq!(
            BigInt::from(a.residue_pow2(bits).unwrap()),
            x.mod_floor(&modulus)
        );
    }
}

fn division<const N: usize>(stream: &mut Stream) {
    for round in 0..150 {
        let a = stream.int::<N>(N);
        // Divisors of every length, the short ones more often.
        let divisor_limbs = if round % 3 == 0 {
            N
        } else {
            1 + stream.below(N as u64) as usize
        };
        let b = stream.int::<N>(divisor_limbs);
        let (x, y) = (big(&a), big(&b));
        if y.is_zero() {
            assert_eq!(a.div_rem_trunc(&b).unwrap_err(), IntError::DivisionByZero);
            continue;
        }
        let (quotient, remainder) = a.div_rem_trunc(&b).unwrap();
        assert_eq!(
            (big(&quotient), big(&remainder)),
            x.div_rem(&y),
            "trunc at {N} limbs"
        );
        let (quotient, remainder) = a.div_rem_floor(&b).unwrap();
        assert_eq!(
            (big(&quotient), big(&remainder)),
            x.div_mod_floor(&y),
            "floor at {N} limbs"
        );
        let positive = y.abs();
        let residue = x.mod_floor(&positive);
        same(a.rem_euclid(&b), &residue, "rem_euclid");

        // The rounded quotient is the nearest integer, and the nearer to
        // zero of two: |2·(a − q·b)| ≤ |b|, and at equality |q| is the
        // smaller.
        let rounded = big(&a.div_round(&b).unwrap());
        let twice: BigInt = (&x - &rounded * &y) * 2;
        assert!(twice.abs() <= positive, "div_round at {N} limbs");
        if twice.abs() == positive {
            assert_eq!(rounded, &x / &y, "div_round's tie at {N} limbs");
        }

        match a.rem_centred(&b) {
            Ok(centred) => {
                let centred = big(&centred);
                assert!(y.is_positive());
                assert!((&x - &centred).mod_floor(&y).is_zero());
                assert!(
                    &centred * 2 <= y && &centred * 2 > -&y,
                    "rem_centred at {N} limbs"
                );
            }
            Err(error) => {
                assert_eq!(error, IntError::Modulus);
                assert!(y.is_negative());
            }
        }
        match a.div_exact(&b) {
            Ok(quotient) => assert_eq!(big(&quotient) * &y, x),
            Err(error) => {
                assert_eq!(error, IntError::Inexact);
                assert!(!(&x % &y).is_zero());
            }
        }
        // A product divides exactly.
        if let Ok(product) = a.mul(&b) {
            assert!(
                product.div_exact(&b).unwrap() == a,
                "div_exact of a product at {N} limbs"
            );
        }
    }
}

macro_rules! at_every_width {
    ($name:ident, $seed:expr) => {
        #[test]
        fn $name() {
            let mut stream = Stream($seed);
            super::$name::<1>(&mut stream);
            super::$name::<2>(&mut stream);
            super::$name::<3>(&mut stream);
            super::$name::<26>(&mut stream);
            super::$name::<33>(&mut stream);
            super::$name::<38>(&mut stream);
            super::$name::<49>(&mut stream);
            super::$name::<65>(&mut stream);
        }
    };
}

mod against_the_oracle {
    use super::Stream;

    at_every_width!(ring, 0x7072_6973_6d2d_696e);
    at_every_width!(division, 0x6469_7669_7369_6f6e);
}
