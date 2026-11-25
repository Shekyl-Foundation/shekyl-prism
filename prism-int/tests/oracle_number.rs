// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The public type's number theory against an arbitrary-precision
//! integer: greatest common divisors and cofactors, inverses, square
//! roots, the Jacobi symbol, and arithmetic modulo an odd number. See
//! `oracle.rs`, which Cargo runs first.

mod support;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};
use prism_int::Int;
use support::{big, same, Stream};

fn number<const N: usize>(stream: &mut Stream) {
    let one = BigInt::one();
    for _ in 0..20 {
        let (a, b) = (stream.int::<N>(N), stream.int::<N>(N));
        let (x, y) = (big(&a), big(&b));
        let (g, u, v) = a.xgcd(&b).unwrap();
        assert_eq!(big(&g), x.gcd(&y), "gcd at {N} limbs");
        assert!(a.gcd(&b).unwrap() == g);
        assert_eq!(
            big(&u) * &x + big(&v) * &y,
            big(&g),
            "the cofactors at {N} limbs"
        );
        if !x.is_zero() && !y.is_zero() {
            assert!(big(&u).abs() <= y.abs() / big(&g) && big(&v).abs() <= x.abs() / big(&g));
        }

        // A modulus above one: |b| + 2, when that fits.
        let m = if b.is_negative() { b.neg() } else { b.clone() }.add(&Int::from_u64(2));
        if let Ok(m) = m {
            let modulus = big(&m);
            match a.inv_mod(&m).unwrap() {
                Some(inverse) => {
                    let inverse = big(&inverse);
                    assert!(inverse.is_positive() && inverse < modulus);
                    assert_eq!(
                        (inverse * &x).mod_floor(&modulus),
                        one,
                        "inv_mod at {N} limbs"
                    );
                }
                None => assert!(
                    !x.gcd(&modulus).is_one(),
                    "inv_mod at {N} limbs: none for a unit"
                ),
            }
        }

        let square = if a.is_negative() { a.neg() } else { a.clone() };
        let root = square.sqrt_floor().unwrap();
        assert_eq!(big(&root), big(&square).sqrt(), "sqrt_floor at {N} limbs");
        assert_eq!(
            square.sqrt_exact().unwrap().is_some(),
            big(&root) * big(&root) == big(&square)
        );
        let half = stream.int::<N>(N / 2);
        let squared = half.mul(&half).unwrap();
        let magnitude = if half.is_negative() { half.neg() } else { half };
        assert!(
            squared.sqrt_exact().unwrap().unwrap() == magnitude,
            "sqrt_exact at {N} limbs"
        );
    }
    for _ in 0..8 {
        // An odd modulus of at most half the width.
        let mut bytes = vec![0u8; 4 * N];
        for byte in &mut bytes {
            *byte = stream.next() as u8;
        }
        bytes[0] |= 1;
        let m = Int::<N>::from_le_bytes(&bytes).unwrap();
        let modulus = big(&m);
        let (a, b, e) = (
            stream.int::<N>(N),
            stream.int::<N>(N),
            stream.int::<N>(N.min(3)),
        );
        let (x, y) = (big(&a), big(&b));
        same(a.mul_mod(&b, &m), &(&x * &y).mod_floor(&modulus), "mul_mod");
        let exponent = if e.is_negative() { e.neg() } else { e };
        let want = x.mod_floor(&modulus).modpow(&big(&exponent), &modulus);
        same(a.pow_mod(&exponent, &m), &want, "pow_mod");

        // The Jacobi symbol is multiplicative above, and is 0 exactly at
        // a common factor.
        let (ja, jb) = (a.jacobi(&m).unwrap(), b.jacobi(&m).unwrap());
        let product = Int::<N>::from_le_bytes(&{
            let (_, bytes) = (&x * &y).mod_floor(&modulus).to_bytes_le();
            bytes
        })
        .unwrap();
        assert_eq!(product.jacobi(&m).unwrap(), ja * jb, "jacobi at {N} limbs");
        assert_eq!(
            ja == 0,
            !x.gcd(&modulus).is_one(),
            "jacobi's zero at {N} limbs"
        );
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

    at_every_width!(number, 0x6e75_6d62_6572_2121);
}
