// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The tests of the primality arithmetic.
//!
//! They live beside the algorithm, so the algorithm file stays the
//! algorithm. A child of `prime` sees its private integers, which an
//! integration test cannot.

use super::{
    challenge_degree, degree_is_prime, is_baillie_psw, miller_rabin_base2, strong_lucas_selfridge,
    Mod, Nat, TRIAL_PRIMES,
};

mod probe {
    //! Byte entry points for the two halves, so a test can show that a
    //! pseudoprime for one half is refused by the other.

    use super::Nat;
    use prism_field::DegreeWidth;

    pub(super) fn miller_rabin_base2<W: DegreeWidth>(bytes: &[u8]) -> bool {
        Nat::<W>::from_le_bytes(bytes).is_some_and(super::miller_rabin_base2)
    }

    pub(super) fn strong_lucas_selfridge<W: DegreeWidth>(bytes: &[u8]) -> bool {
        Nat::<W>::from_le_bytes(bytes).is_some_and(super::strong_lucas_selfridge)
    }
}

/// The tests of this module at one width. Every width a parameter set
/// names runs all of them: the verdict on an integer must not depend on
/// the width it is held in.
macro_rules! self_test_at {
    ($name:ident, $width:ty) => {
        mod $name {
            use super::probe;
            use super::{Mod, Nat, TRIAL_PRIMES};
            use prism_field::DegreeWidth;

            type Width = $width;
            const WIDTH_BITS: usize = <Width as DegreeWidth>::BITS;

            fn is_baillie_psw(bytes: &[u8]) -> bool {
                super::is_baillie_psw::<Width>(bytes)
            }
            fn challenge_degree(exponent: u32, scalar: &[u8]) -> Option<Vec<u8>> {
                super::challenge_degree::<Width>(exponent, scalar)
            }
            fn degree_is_prime(exponent: u32, scalar: &[u8]) -> bool {
                super::degree_is_prime::<Width>(exponent, scalar)
            }

            /// The primes below 300, for a dense sweep of the small cases.
            const PRIMES_BELOW_300: &[u32] = &[
                2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79,
                83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167,
                173, 179, 181, 191, 193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257,
                263, 269, 271, 277, 281, 283, 293,
            ];

            fn le(value: u128) -> Vec<u8> {
                let bytes = value.to_le_bytes();
                let len = bytes.iter().rposition(|b| *b != 0).map_or(1, |at| at + 1);
                bytes[..len].to_vec()
            }

            #[test]
            fn every_integer_below_three_hundred_gets_the_right_verdict() {
                for value in 0u32..300 {
                    let expected = PRIMES_BELOW_300.contains(&value);
                    assert_eq!(is_baillie_psw(&le(u128::from(value))), expected, "{value}");
                }
            }

            #[test]
            fn a_wide_integer_is_not_its_low_limb() {
                // The low limb spells a trial prime; the integer does not. Each of
                // these is even or a multiple of three, so the sieve refuses it.
                for prime in [2u128, 3, 5, 97] {
                    for multiple in 1u128..=5 {
                        let wide = multiple * (1u128 << 64) + prime;
                        assert!(!is_baillie_psw(&le(wide)), "2^64·{multiple} + {prime}");
                    }
                }
                // 2^64 + 13 is prime and 2^64 + 15 is not, so the sieve is not simply
                // refusing everything above one limb.
                assert!(is_baillie_psw(&le((1u128 << 64) + 13)));
                assert!(!is_baillie_psw(&le((1u128 << 64) + 15)));
            }

            #[test]
            fn the_degree_is_the_power_of_two_minus_the_square() {
                // `s = 1` at exponent 4: `q = 2^7 − 1 = 127`.
                assert_eq!(challenge_degree(4, &[1]).as_deref(), Some(&[127][..]));
                assert!(degree_is_prime(4, &[1]));
                // An even scalar names no degree. `12^2` is already past `2^7`.
                assert!(challenge_degree(4, &[2]).is_none());
                assert!(challenge_degree(4, &[12]).is_none());
            }

            #[test]
            fn an_integer_wider_than_the_module_is_refused() {
                assert!(!is_baillie_psw(&[0xff; WIDTH_BITS / 8 + 1]));
                // All ones at the full width fits, and it is composite: the width
                // is a multiple of 64, and `2^n − 1` is composite for composite `n`.
                assert!(!is_baillie_psw(&[0xff; WIDTH_BITS / 8]));
            }

            #[test]
            fn the_sieve_table_is_the_primes_below_one_hundred() {
                for &prime in TRIAL_PRIMES {
                    assert!(PRIMES_BELOW_300.contains(&prime), "{prime}");
                    assert!(prime < 100, "{prime}");
                }
                let expected = PRIMES_BELOW_300.iter().filter(|p| **p < 100).count();
                assert_eq!(TRIAL_PRIMES.len(), expected);
            }

            #[test]
            fn base2_pseudoprimes_fail_the_lucas_half() {
                for n in [2047u128, 3277, 4033, 4681, 8321] {
                    assert!(probe::miller_rabin_base2::<Width>(&le(n)), "{n}");
                    assert!(!is_baillie_psw(&le(n)), "{n}");
                }
            }

            #[test]
            fn strong_lucas_pseudoprimes_fail_the_miller_half() {
                for n in [5459u128, 5777, 10877, 16109, 18971] {
                    assert!(probe::strong_lucas_selfridge::<Width>(&le(n)), "{n}");
                    assert!(!is_baillie_psw(&le(n)), "{n}");
                }
                // The standard Lucas test accepts 323. The strong test does not.
                assert!(!probe::strong_lucas_selfridge::<Width>(&le(323)));
            }

            #[test]
            fn carmichael_numbers_are_refused() {
                for n in [561u128, 1105, 1729, 2465, 2821, 6601, 8911] {
                    assert!(!is_baillie_psw(&le(n)), "{n}");
                }
            }

            #[test]
            fn mersenne_primes_pass_at_every_width() {
                for exponent in [61u32, 89, 107, 127, 521] {
                    let mut bytes = vec![0xffu8; (exponent as usize).div_ceil(8)];
                    if let Some(top) = bytes.last_mut() {
                        *top = u8::MAX >> (7 - (exponent - 1) % 8);
                    }
                    assert!(is_baillie_psw(&bytes), "2^{exponent} − 1");
                }
            }

            #[test]
            fn montgomery_multiplication_agrees_with_small_products() {
                let modulus = Nat::<Width>::from_u64(1_000_003);
                let ctx = Mod::new(modulus);
                for left in [0u64, 1, 2, 999, 1_000_002] {
                    for right in [0u64, 1, 7, 65_537, 1_000_002] {
                        let expected = Nat::<Width>::from_u64((left * right) % 1_000_003);
                        assert_eq!(
                            ctx.mul(Nat::<Width>::from_u64(left), Nat::<Width>::from_u64(right)),
                            expected,
                            "{left}·{right}"
                        );
                    }
                }
                // `2^(p−1) = 1 (mod p)` for the prime above.
                assert_eq!(
                    ctx.pow(Nat::<Width>::from_u64(2), Nat::<Width>::from_u64(1_000_002)),
                    Nat::<Width>::one()
                );
            }

            #[test]
            fn a_perfect_square_is_refused_before_either_half() {
                for root in [3u128, 5, 101, 1_000_003, 4_294_967_291] {
                    assert!(!is_baillie_psw(&le(root * root)), "{root}^2");
                }
            }

            #[test]
            fn a_known_prime_degree_passes() {
                // q = 2^{17} − 3^2 = 131063, which is prime. e = 9 gives the exponent
                // 2e−1 = 17, and s = 3 is the odd integer below 2^{8}.
                assert!(degree_is_prime(9, &[3]));
                // 2^{17} − 2^2 is even.
                assert!(!degree_is_prime(9, &[2]));
            }

            #[test]
            fn a_scalar_at_or_above_the_power_names_no_degree() {
                // s^2 = 2^{2e−1} leaves q = 0, and anything larger is negative.
                assert!(!degree_is_prime(1, &[1]));
                assert!(!degree_is_prime(9, &le(1 << 9)));
                // An exponent whose power exceeds the module's width is refused.
                assert!(!degree_is_prime(u32::MAX, &[3]));
                assert!(!degree_is_prime(0, &[3]));
                assert!(!degree_is_prime((WIDTH_BITS as u32) / 2 + 1, &le(3)));
            }
        }
    };
}

self_test_at!(
    at_1024_bits,
    prism_field::DegreeLimbs<{ prism_field::width::DEGREE_LIMBS_1024 }>
);
self_test_at!(
    at_1344_bits,
    prism_field::DegreeLimbs<{ prism_field::width::DEGREE_LIMBS_1344 }>
);

/// The arithmetic at one and two limbs, against `u128`.
///
/// A challenge degree leaves the top limb of its width far from full, so
/// at the widths the sets name no test reaches the branches that handle a
/// carry out of the top limb: the wrapped sum in `add_mod`, the dropped
/// bit in `half_mod`, the overflow limb in `mont_mul`. A modulus with its
/// top bit set reaches all three. Two limbs is the widest a `u128` can
/// check exactly, and one limb puts every carry on the only limb there is.
mod narrow {
    use super::{is_baillie_psw, Mod, Nat};
    use prism_field::width::{
        DEGREE_LIMBS_1024, DEGREE_LIMBS_128, DEGREE_LIMBS_1344, DEGREE_LIMBS_64,
    };
    use prism_field::{DegreeLimbs, DegreeWidth};

    /// The 64-bit width: one limb, where every carry lands on the only limb.
    type Width64 = DegreeLimbs<DEGREE_LIMBS_64>;
    /// The 128-bit width: the widest a `u128` checks exactly.
    type Width128 = DegreeLimbs<DEGREE_LIMBS_128>;
    /// The 1,024-bit width the sets up to `p500` name.
    type Width1024 = DegreeLimbs<DEGREE_LIMBS_1024>;
    /// The 1,344-bit width `p664` names.
    type Width1344 = DegreeLimbs<DEGREE_LIMBS_1344>;

    fn nat<W: DegreeWidth>(value: u128) -> Nat<W> {
        Nat::from_le_bytes(&value.to_le_bytes()[..W::BITS / 8]).expect("the value fits the width")
    }

    fn value<W: DegreeWidth>(nat: Nat<W>) -> u128 {
        let mut bytes = [0u8; 16];
        let le = nat.to_le_bytes();
        bytes[..le.len()].copy_from_slice(&le);
        u128::from_le_bytes(bytes)
    }

    /// SplitMix64, twice for a `u128`.
    struct Stream(u64);

    impl Stream {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        fn wide(&mut self) -> u128 {
            u128::from(self.next()) << 64 | u128::from(self.next())
        }
    }

    /// `left · right mod modulus` by doubling, with every sum taken in a
    /// way that cannot overflow.
    fn mul_mod(left: u128, right: u128, modulus: u128) -> u128 {
        let add = |a: u128, b: u128| {
            let gap = modulus - b;
            if a >= gap {
                a - gap
            } else {
                a + b
            }
        };
        let (mut sum, mut term) = (0, left % modulus);
        for bit in 0..u128::BITS {
            if (right >> bit) & 1 == 1 {
                sum = add(sum, term);
            }
            term = add(term, term);
        }
        sum
    }

    /// Every modular routine against the reference, for moduli of the full
    /// width and operands spread over the residues. Returns how many of
    /// the sums wrapped past the width, so the caller can require that the
    /// carry branches ran.
    fn arithmetic_agrees<W: DegreeWidth>(seed: u64) -> u32 {
        let mask = if W::BITS == u128::BITS as usize {
            u128::MAX
        } else {
            (1u128 << W::BITS) - 1
        };
        let top = 1u128 << (W::BITS - 1);
        let mut stream = Stream(seed);
        let mut wrapped = 0;
        for round in 0..2_000 {
            // Odd, with the top bit of the width set. Every fourth one is
            // within a few units of the top of the width.
            let modulus = if round % 4 == 0 {
                mask - 2 * u128::from(stream.next() % 64)
            } else {
                (stream.wide() & mask) | top | 1
            };
            let ctx = Mod::<W>::new(nat(modulus));
            let mut residue = || match stream.next() % 4 {
                0 => modulus - 1 - u128::from(stream.next() % 4),
                1 => u128::from(stream.next() % 4),
                _ => (stream.wide() & mask) % modulus,
            };
            let (a, b) = (residue(), residue());
            let (na, nb, n) = (nat::<W>(a), nat::<W>(b), nat::<W>(modulus));
            let sum = mul_mod(1, a, modulus);
            assert_eq!(sum, a);
            let expected_add = {
                let gap = modulus - b;
                if a >= gap {
                    a - gap
                } else {
                    a + b
                }
            };
            wrapped += u32::from((a & mask).checked_add(b).is_none_or(|sum| sum > mask));
            assert_eq!(
                value(na.add_mod(nb, n)),
                expected_add,
                "{a} + {b} mod {modulus}"
            );
            let expected_sub = if a >= b { a - b } else { a + (modulus - b) };
            assert_eq!(
                value(na.sub_mod(nb, n)),
                expected_sub,
                "{a} − {b} mod {modulus}"
            );
            let half = na.half_mod(n);
            assert_eq!(
                value(half.add_mod(half, n)),
                a,
                "twice half of {a} mod {modulus}"
            );
            assert_eq!(value(na.rem(n)), a);
            assert_eq!(
                value(ctx.mul(na, nb)),
                mul_mod(a, b, modulus),
                "{a} · {b} mod {modulus}"
            );
            assert_eq!(value(ctx.lower(ctx.lift(na))), a, "lift then lower");
            // A short power, by repeated reference multiplication.
            let exponent = stream.next() % 50;
            let mut expected = 1 % modulus;
            for _ in 0..exponent {
                expected = mul_mod(expected, a, modulus);
            }
            assert_eq!(
                value(ctx.pow(na, Nat::from_u64(exponent))),
                expected,
                "{a}^{exponent} mod {modulus}"
            );
        }
        wrapped
    }

    #[test]
    fn two_limb_arithmetic_is_u128_arithmetic_with_the_top_bit_set() {
        let wrapped = arithmetic_agrees::<Width128>(0x5052_4953_4D32);
        assert!(wrapped > 500, "{wrapped} sums wrapped past the width");
    }

    #[test]
    fn one_limb_arithmetic_is_u128_arithmetic_with_the_top_bit_set() {
        let wrapped = arithmetic_agrees::<Width64>(0x5052_4953_4D31);
        assert!(wrapped > 500, "{wrapped} sums wrapped past the width");
    }

    /// Trial division, for an integer of at most 64 bits.
    fn is_prime_u64(n: u64) -> bool {
        n >= 2 && (2..=n.isqrt()).all(|d| !n.is_multiple_of(d))
    }

    /// The verdict at one limb, where the modulus can fill the width.
    #[test]
    fn one_limb_verdicts_are_trial_division() {
        for n in 0u64..5_000 {
            assert_eq!(
                is_baillie_psw::<Width64>(&n.to_le_bytes()),
                is_prime_u64(n),
                "{n}"
            );
        }
        // The top of a 32-bit range, where trial division is still quick,
        // and the largest primes below `2^64`, which fill the limb.
        for n in (u64::from(u32::MAX) - 2_000..=u64::from(u32::MAX)).step_by(1) {
            assert_eq!(
                is_baillie_psw::<Width64>(&n.to_le_bytes()),
                is_prime_u64(n),
                "{n}"
            );
        }
        for (offset, prime) in [
            (59u64, true),
            (83, true),
            (95, true),
            (57, false),
            (1, false),
        ] {
            let n = u64::MAX - offset + 1;
            assert_eq!(
                is_baillie_psw::<Width64>(&n.to_le_bytes()),
                prime,
                "2^64 − {offset}"
            );
        }
        // One limb holds nothing wider.
        assert!(!is_baillie_psw::<Width64>(&[0xff; 9]));
    }

    /// A prime that fits the 21-limb width and not the 16-limb one:
    /// `2^1279 − 1`, a Mersenne prime. The challenge degree of `p664` has
    /// 1,323 bits, so this is the size the wide width exists for, and the
    /// one place in this crate where a "prime" verdict above 1,024 bits is
    /// held against a number known to be prime from outside it. The
    /// 16-limb width refuses the integer as too wide; it does not judge
    /// its low limbs.
    #[test]
    fn a_mersenne_prime_above_the_narrower_width_passes_at_the_wider() {
        const EXPONENT: usize = 1279;
        let mut mersenne = vec![0xffu8; EXPONENT.div_ceil(8)];
        *mersenne.last_mut().expect("nonempty") = u8::MAX >> (8 - EXPONENT % 8);
        const {
            assert!(EXPONENT > <Width1024 as DegreeWidth>::BITS);
            assert!(EXPONENT < <Width1344 as DegreeWidth>::BITS);
        }
        assert!(is_baillie_psw::<Width1344>(&mersenne));
        assert!(!is_baillie_psw::<Width1024>(&mersenne));
        // Its odd neighbours below are composite: 2^1279 − 3 and −5.
        for below in [2u8, 4] {
            let mut neighbour = mersenne.clone();
            neighbour[0] -= below;
            assert!(
                !is_baillie_psw::<Width1344>(&neighbour),
                "2^1279 − {}",
                below + 1
            );
        }
        // 2^1277 − 1 has a prime exponent and is composite.
        let mut composite = vec![0xffu8; 1277usize.div_ceil(8)];
        *composite.last_mut().expect("nonempty") = u8::MAX >> (8 - 1277 % 8);
        assert!(!is_baillie_psw::<Width1344>(&composite));
    }

    /// The same integer has one verdict at every width that holds it.
    #[test]
    fn a_verdict_does_not_depend_on_the_width() {
        let mut stream = Stream(0x5052_4953_4D57);
        let mut primes = 0;
        for _ in 0..3_000 {
            let n = stream.wide() | 1;
            let bytes = n.to_le_bytes();
            let verdict = is_baillie_psw::<Width128>(&bytes);
            assert_eq!(is_baillie_psw::<Width1024>(&bytes), verdict, "{n}");
            assert_eq!(is_baillie_psw::<Width1344>(&bytes), verdict, "{n}");
            primes += u32::from(verdict);
        }
        // About one odd 128-bit integer in 44 is prime.
        assert!((30..150).contains(&primes), "{primes} primes of 3,000");
    }
}
