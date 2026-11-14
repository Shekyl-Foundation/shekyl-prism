// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The parameter sets, each a prime `p = f·2^e − 1` instantiated with `fp2`,
//! and the byte layout each of them encodes.
//!
//! A parameter set is a prime, and it is a type: [`P248`], [`P376`] and
//! [`P500`], named by `e`. The encoding lengths and the salt below are part
//! of that type's contract, so a `p248` encoding is refused by the `p500`
//! decoder and no caller re-derives a length from a second constant.
//!
//! The salt's bit length and the number of candidates a torsion search
//! tries are both [`width_bits`]: `e` rounded up to a multiple of 128.
//! Nothing here records a security level, and nothing here names a set by
//! the string committed vectors carry. The category a prime is assessed at
//! moves with research and is documentation: `docs/PARAMETER_SETS.md`,
//! which no code reads. The committed string lives with the corpus, in
//! `prism-verify`.
//!
//! `fp2` 0.6.0 is MIT OR Apache-2.0. Its default feature is `test-utils`, so
//! this crate turns default features off. Packed arithmetic stays off as well:
//! `65·2^376 − 1` is 383 bits and is not strictly below `2^382`, so the
//! feature's modulus rule does not hold for every set. One crate feature
//! cannot turn it on for some sets and leave the others scalar.

#![forbid(unsafe_code)]

use fp2::define_fp2_from_type;
use fp2::define_fp_core;

pub mod ops;

/// The bytes of one set's public salt.
///
/// Each set owns a distinct type, [`ParameterSet::Salt`], whose array is
/// exactly [`ParameterSet::SALT_LEN`] long. Every byte string of that length
/// is valid.
pub trait Salt: Copy + Eq + core::fmt::Debug {
    const LEN: usize;

    /// `None` unless `bytes` has length [`Self::LEN`].
    fn from_bytes(bytes: &[u8]) -> Option<Self>;

    /// The bytes [`Self::from_bytes`] accepts.
    fn as_bytes(&self) -> &[u8];

    /// The all-zero salt. Signing does not prefer it; probes do.
    fn zero() -> Self;

    /// Little-endian `counter` in the low `u64`, zero above that.
    /// `None` when the salt is shorter than a `u64`. Signing derives salts
    /// from the KDF; this is the probe layout Sage uses.
    fn from_counter(counter: u64) -> Option<Self>;
}

/// Field operations the isogeny step uses. `inv` of zero is `None`.
pub trait Field: Copy {
    fn zero() -> Self;
    fn one() -> Self;
    fn from_u64(v: u64) -> Self;
    fn add(self, rhs: Self) -> Self;
    fn sub(self, rhs: Self) -> Self;
    fn mul(self, rhs: Self) -> Self;
    /// `self · self`, cheaper than [`Self::mul`] in the extension field.
    fn square(self) -> Self;
    fn negate(self) -> Self;
    fn inv(self) -> Option<Self>;
    /// A square root, or `None` when this element is not a square.
    fn sqrt(self) -> Option<Self>;
    /// Quadratic residuosity, including zero. `F_p` uses its Legendre
    /// symbol. `F_{p^2}` uses the norm to `F_p` and that symbol.
    fn is_square(self) -> bool;
    fn is_zero(self) -> bool;
    fn same(self, rhs: Self) -> bool;
}

/// `e` rounded up to a multiple of 128.
///
/// This is the bit length of the public salt and the number of candidates
/// each torsion search tries. A parameter set carries no security
/// assessment; both widths are this rounding of its exponent. The two
/// primes ruled but not yet built, `e = 324` and `e = 664`, round to 384
/// and 768.
///
/// Panics if the rounding does not fit in `u32`.
pub const fn width_bits(e: u32) -> u32 {
    match e.div_ceil(128).checked_mul(128) {
        Some(bits) => bits,
        None => panic!("e rounded up to a multiple of 128 does not fit in u32"),
    }
}

/// The candidate limit for a width that fits in a `u16`.
///
/// Panics if `bits` exceeds [`u16::MAX`]. Every ruled prime is far below
/// that, and a wider prime would be a new counter, not a silent truncation.
const fn candidate_bound(bits: u32) -> u16 {
    assert!(
        bits <= u16::MAX as u32,
        "the width fits a candidate counter"
    );
    // CLIPPY: the assert is the range check. `TryFrom` is not const-stable,
    // so the narrowing cast is the const form of that check.
    #[allow(clippy::cast_possible_truncation)]
    {
        bits as u16
    }
}

/// One parameter set: a prime `p = f·2^e − 1`, its two fields and the
/// lengths its encodings use.
pub trait ParameterSet: Copy + Clone + Eq + Send + Sync + 'static {
    type Base: Field;
    type Ext: Field;
    /// The set's name, `p` and then `e`: `p248`, `p376`, `p500`.
    ///
    /// Display only. Committed vectors and the research signer's seed
    /// streams do not use it.
    const NAME: &'static str;
    /// Valuation in `p = f·2^e − 1`.
    const E: u32;
    /// The odd cofactor `f` in `p + 1 = f·2^e`.
    const ODD: u64;
    /// Instantiated long-chain length `e′ = e − 2`. The short chain
    /// has length `CHAIN_STEPS − 1`; the rational torsion exponent is `E`.
    const CHAIN_STEPS: u32;
    /// Actual challenge width `e′ − 1 = E − 3`, including the forced odd bit.
    const CHALLENGE_BITS: u32;
    const FP_LEN: usize;
    const FP2_LEN: usize;
    const PUBLIC_KEY_LEN: usize;
    /// Byte length of `s`, which is [`Self::CHALLENGE_BITS`] rounded up. It
    /// is derived, not carried in the signature.
    const S_LEN: usize;
    /// This set's salt. Its length is [`Self::SALT_LEN`].
    type Salt: Salt;
    /// [`width_bits`] of [`Self::E`]. The salt's bit length, and the
    /// candidate limit of each torsion search.
    const WIDTH_BITS: u32;
    /// Public salt length in bytes: [`Self::WIDTH_BITS`] / 8.
    const SALT_LEN: usize;
    /// Candidates each torsion search tries: [`Self::WIDTH_BITS`].
    ///
    /// The public-key basis search, the second point of a rational basis
    /// and the starting curve's basis share this limit. Reaching it means
    /// something different in each search. The only failure estimate is
    /// the public-key search's, and it lives with that search.
    const SEARCH_BOUND: u16;
    /// Field elements a signature carries: `A_sig`, `x(P)`, `y(P)`, `x(Q)`,
    /// `y(Q)`.
    const SIGNATURE_ELEMENTS: usize;
    const SIGNATURE_LEN: usize;

    fn fp_zero() -> Self::Base;
    fn ext_i() -> Self::Ext;
    fn ext_from_base(re: Self::Base) -> Self::Ext;
    fn ext_parts(value: Self::Ext) -> (Self::Base, Self::Base);
    fn base_decode(bytes: &[u8]) -> Option<Self::Base>;
    fn base_encode(value: Self::Base) -> Vec<u8>;
    fn ext_decode(bytes: &[u8]) -> Option<Self::Ext>;
    fn ext_encode(value: Self::Ext) -> Vec<u8>;
}

macro_rules! parameter_set {
    (
        $modname:ident,
        $set:ident,
        f = $f:literal,
        e = $e:literal,
        $fp:ident,
        $fp2:ident,
        [$($limb:expr),+ $(,)?]
    ) => {
        pub mod $modname {
            use super::ops::{tally, Kind, Op};
            use super::{
                candidate_bound, define_fp2_from_type, define_fp_core, width_bits, Field,
                ParameterSet, Salt,
            };

            /// [`width_bits`] of this set's exponent.
            const WIDTH: u32 = width_bits($e);
            const SALT_BYTES: usize = WIDTH as usize / 8;

            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub struct SaltBytes([u8; SALT_BYTES]);

            impl Salt for SaltBytes {
                const LEN: usize = SALT_BYTES;

                fn from_bytes(bytes: &[u8]) -> Option<Self> {
                    Some(Self(bytes.try_into().ok()?))
                }

                fn as_bytes(&self) -> &[u8] {
                    &self.0
                }

                fn zero() -> Self {
                    Self([0; SALT_BYTES])
                }

                fn from_counter(counter: u64) -> Option<Self> {
                    let width = core::mem::size_of::<u64>();
                    if SALT_BYTES < width {
                        return None;
                    }
                    let mut bytes = [0u8; SALT_BYTES];
                    bytes[..width].copy_from_slice(&counter.to_le_bytes());
                    Some(Self(bytes))
                }
            }

            // Two facts the rest of the workspace reads off a set without
            // re-testing: the challenge scalar occupies at least one byte, so
            // its derivation may touch both ends of the buffer; and
            // `p = f·2^e − 1` exceeds `2^64`, so every `u64` this crate lifts
            // into the field is already canonical.
            const _: () = assert!(
                $e > 64,
                "a set needs e > 64: one challenge byte, and a prime above 2^64"
            );

            define_fp_core!(
                typename = $fp,
                modulus = [$($limb),+],
            );
            define_fp2_from_type!(
                typename = $fp2,
                base_field = $fp,
                use_sum_of_products = false,
            );

            fn mask_ok(mask: u32) -> bool {
                mask == u32::MAX
            }

            impl Field for $fp {
                fn zero() -> Self {
                    Self::ZERO
                }
                fn one() -> Self {
                    Self::ONE
                }
                fn from_u64(v: u64) -> Self {
                    let mut buf = vec![0u8; Self::ENCODED_LENGTH];
                    let bytes = v.to_le_bytes();
                    buf[..bytes.len()].copy_from_slice(&bytes);
                    let (value, mask) = Self::decode(&buf);
                    assert!(mask_ok(mask), "u64 is canonical in this field");
                    value
                }
                fn add(self, rhs: Self) -> Self {
                    self + rhs
                }
                fn sub(self, rhs: Self) -> Self {
                    self - rhs
                }
                fn mul(self, rhs: Self) -> Self {
                    tally(Kind::Base, Op::Mul);
                    self * rhs
                }
                fn square(self) -> Self {
                    tally(Kind::Base, Op::Square);
                    <$fp>::square(self)
                }
                fn negate(self) -> Self {
                    -self
                }
                fn inv(self) -> Option<Self> {
                    tally(Kind::Base, Op::Inv);
                    if mask_ok(self.is_zero()) {
                        None
                    } else {
                        Some(self.invert())
                    }
                }
                fn sqrt(self) -> Option<Self> {
                    tally(Kind::Base, Op::Sqrt);
                    let mut value = self;
                    let mask = value.set_sqrt();
                    if mask_ok(mask) { Some(value) } else { None }
                }
                fn is_square(self) -> bool {
                    tally(Kind::Base, Op::IsSquare);
                    self.legendre() >= 0
                }
                fn is_zero(self) -> bool {
                    mask_ok($fp::is_zero(self))
                }
                fn same(self, rhs: Self) -> bool {
                    mask_ok(self.equals(&rhs))
                }
            }

            impl Field for $fp2 {
                fn zero() -> Self {
                    Self::ZERO
                }
                fn one() -> Self {
                    Self::ONE
                }
                fn from_u64(v: u64) -> Self {
                    Self::new(&<$fp as Field>::from_u64(v), &<$fp>::ZERO)
                }
                fn add(self, rhs: Self) -> Self {
                    self + rhs
                }
                fn sub(self, rhs: Self) -> Self {
                    self - rhs
                }
                fn mul(self, rhs: Self) -> Self {
                    tally(Kind::Ext, Op::Mul);
                    self * rhs
                }
                fn square(self) -> Self {
                    tally(Kind::Ext, Op::Square);
                    <Self as fp2::traits::Fq>::square(self)
                }
                fn negate(self) -> Self {
                    -self
                }
                fn inv(self) -> Option<Self> {
                    tally(Kind::Ext, Op::Inv);
                    if mask_ok(self.is_zero()) {
                        None
                    } else {
                        Some(self.invert())
                    }
                }
                fn sqrt(self) -> Option<Self> {
                    tally(Kind::Ext, Op::Sqrt);
                    let mut value = self;
                    let mask = value.set_sqrt();
                    if mask_ok(mask) { Some(value) } else { None }
                }
                fn is_square(self) -> bool {
                    tally(Kind::Ext, Op::IsSquare);
                    self.legendre() >= 0
                }
                fn is_zero(self) -> bool {
                    mask_ok($fp2::is_zero(self))
                }
                fn same(self, rhs: Self) -> bool {
                    mask_ok(self.equals(&rhs))
                }
            }

            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub struct $set;

            impl ParameterSet for $set {
                type Base = $fp;
                type Ext = $fp2;
                const NAME: &'static str = concat!("p", stringify!($e));
                const E: u32 = $e;
                const ODD: u64 = $f;
                const CHAIN_STEPS: u32 = $e - 2;
                const CHALLENGE_BITS: u32 = $e - 3;
                const FP_LEN: usize = <$fp>::ENCODED_LENGTH;
                const FP2_LEN: usize = <$fp2>::ENCODED_LENGTH;
                const PUBLIC_KEY_LEN: usize = <$fp2>::ENCODED_LENGTH;
                type Salt = SaltBytes;
                const S_LEN: usize = (Self::CHALLENGE_BITS as usize).div_ceil(8);
                const WIDTH_BITS: u32 = WIDTH;
                const SALT_LEN: usize = SALT_BYTES;
                const SEARCH_BOUND: u16 = candidate_bound(WIDTH);
                const SIGNATURE_ELEMENTS: usize = 5;
                const SIGNATURE_LEN: usize =
                    Self::SIGNATURE_ELEMENTS * <$fp2>::ENCODED_LENGTH + Self::SALT_LEN;

                fn fp_zero() -> Self::Base {
                    <$fp>::ZERO
                }
                fn ext_i() -> Self::Ext {
                    <$fp2>::new(&<$fp>::ZERO, &<$fp>::ONE)
                }
                fn ext_from_base(re: Self::Base) -> Self::Ext {
                    <$fp2>::new(&re, &<$fp>::ZERO)
                }
                fn ext_parts(value: Self::Ext) -> (Self::Base, Self::Base) {
                    (value.x0(), value.x1())
                }
                fn base_decode(bytes: &[u8]) -> Option<Self::Base> {
                    let (value, mask) = <$fp>::decode(bytes);
                    if mask_ok(mask) { Some(value) } else { None }
                }
                fn base_encode(value: Self::Base) -> Vec<u8> {
                    value.encode().to_vec()
                }
                fn ext_decode(bytes: &[u8]) -> Option<Self::Ext> {
                    let (value, mask) = <$fp2>::decode(bytes);
                    if mask_ok(mask) { Some(value) } else { None }
                }
                fn ext_encode(value: Self::Ext) -> Vec<u8> {
                    value.encode().to_vec()
                }
            }
        }
        pub use $modname::$set;
    };
}

parameter_set!(
    p248,
    P248,
    f = 5,
    e = 248,
    FpP248,
    Fp2P248,
    [
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0x04FFFFFFFFFFFFFFu64,
    ]
);

parameter_set!(
    p376,
    P376,
    f = 65,
    e = 376,
    FpP376,
    Fp2P376,
    [
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0x40FFFFFFFFFFFFFFu64,
    ]
);

parameter_set!(
    p500,
    P500,
    f = 27,
    e = 500,
    FpP500,
    Fp2P500,
    [
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0x01AFFFFFFFFFFFFFu64,
    ]
);

/// Packed arithmetic is not enabled. See the crate docs.
pub const FIELD_BACKEND: &str = "scalar";

#[cfg(test)]
mod tests {
    use super::{width_bits, Field, ParameterSet, Salt, P248, P376, P500};

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn check_base<L: ParameterSet>(add: &str, mul: &str, inv: &str, sqrt_sq: &str) {
        let two = L::Base::from_u64(2);
        let three = L::Base::from_u64(3);
        assert_eq!(hex(&L::base_encode(two.add(three))), add);
        assert_eq!(hex(&L::base_encode(two.mul(three))), mul);
        let inverse = two.inv().expect("2 is invertible");
        assert_eq!(hex(&L::base_encode(inverse)), inv);
        assert!(inverse.mul(two).same(L::Base::one()));
        let four = two.square();
        let root = L::base_decode(&hex_bytes(sqrt_sq)).expect("root decodes");
        assert!(root.square().same(four));
        assert_eq!(L::FP_LEN * 2, L::FP2_LEN);
        assert_eq!(L::PUBLIC_KEY_LEN, L::FP2_LEN);
        assert_eq!(L::WIDTH_BITS, L::E.div_ceil(128) * 128);
        assert_eq!(L::SALT_LEN * 8, L::WIDTH_BITS as usize);
        assert_eq!(u32::from(L::SEARCH_BOUND), L::WIDTH_BITS);
        assert_eq!(L::SALT_LEN, <L::Salt as Salt>::LEN);
        assert_eq!(L::CHAIN_STEPS, L::E - 2);
        assert_eq!(L::CHALLENGE_BITS, L::E - 3);
        assert_eq!(L::S_LEN, (L::CHALLENGE_BITS as usize).div_ceil(8));
        assert_eq!(
            L::SIGNATURE_LEN,
            L::SIGNATURE_ELEMENTS * L::FP2_LEN + L::SALT_LEN
        );
    }

    /// `p − 1 = f·2^e − 2` as little-endian bytes, from `f` and `e` alone.
    fn modulus_minus_one(f: u64, e: u32, len: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; len];
        // `f·2^e`: the bytes of `f` shifted up by `e` bits.
        let (byte, bit) = ((e / 8) as usize, e % 8);
        let shifted = u128::from(f) << bit;
        for (offset, part) in shifted.to_le_bytes().iter().enumerate() {
            if *part != 0 {
                bytes[byte + offset] = *part;
            }
        }
        // Minus two. The low byte is zero, since `e` is far above 8.
        let mut borrow = 2u16;
        for value in &mut bytes {
            let wide = 0x100 + u16::from(*value) - borrow;
            *value = wide as u8;
            borrow = u16::from(wide < 0x100);
        }
        assert_eq!(borrow, 0);
        bytes
    }

    /// The name, the cofactor, the exponent and the limbs of a set agree:
    /// the field's `−1` is `f·2^e − 2`, computed from `f` and `e` without
    /// reading the limb literals.
    fn the_prime_is<L: ParameterSet>(name: &str, f: u64, e: u32) {
        assert_eq!(L::NAME, name);
        assert_eq!((L::ODD, L::E), (f, e));
        assert_eq!(L::ODD % 2, 1, "{name}: f is odd");
        assert_eq!(
            L::base_encode(L::Base::one().negate()),
            modulus_minus_one(L::ODD, L::E, L::FP_LEN),
            "{name}: p + 1 = f·2^e"
        );
    }

    #[test]
    fn every_set_is_the_prime_its_name_and_cofactor_say() {
        the_prime_is::<P248>("p248", 5, 248);
        the_prime_is::<P376>("p376", 65, 376);
        the_prime_is::<P500>("p500", 27, 500);
    }

    /// The committed sizes of the three built sets, and the widths of the
    /// two ruled exponents that have no type yet. The sizes are the pin.
    #[test]
    fn the_width_is_e_rounded_up_to_128_bits() {
        fn check<L: ParameterSet>(salt_bytes: usize, candidates: u16) {
            let rounded = L::E.div_ceil(128) * 128;
            assert_eq!(L::WIDTH_BITS, rounded, "{}", L::NAME);
            assert_eq!(L::SALT_LEN, salt_bytes, "{}", L::NAME);
            assert_eq!(L::SEARCH_BOUND, candidates, "{}", L::NAME);
            assert_eq!(usize::from(L::SEARCH_BOUND), L::SALT_LEN * 8);
            assert_eq!(width_bits(L::E), rounded);
        }
        check::<P248>(32, 256);
        check::<P376>(48, 384);
        check::<P500>(64, 512);
        // Ruled, not built. Salt bytes are the width over 8: 48 and 96.
        assert_eq!(width_bits(324), 384);
        assert_eq!(width_bits(664), 768);
    }

    fn hex_bytes(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
            .collect()
    }

    #[test]
    fn p248_matches_the_committed_integers() {
        check_base::<P248>(
            "0500000000000000000000000000000000000000000000000000000000000000",
            "0600000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000008002",
            "0200000000000000000000000000000000000000000000000000000000000000",
        );
        assert_eq!(P248::PUBLIC_KEY_LEN, 64);
        assert_eq!(P248::SIGNATURE_LEN, 352);
        assert_eq!(P248::CHAIN_STEPS, 246);
    }

    #[test]
    fn p376_matches_the_committed_integers() {
        check_base::<P376>(
            "050000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "060000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008020",
            "020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        );
        assert_eq!(P376::PUBLIC_KEY_LEN, 96);
        assert_eq!(P376::SIGNATURE_LEN, 528);
        assert_eq!(P376::CHAIN_STEPS, 374);
    }

    #[test]
    fn p500_matches_the_committed_integers() {
        check_base::<P500>(
            "05000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "06000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000d800",
            "02000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        );
        assert_eq!(P500::PUBLIC_KEY_LEN, 128);
        assert_eq!(P500::SIGNATURE_LEN, 704);
        assert_eq!(P500::CHAIN_STEPS, 498);
    }

    #[test]
    fn extension_matches_schoolbook_multiplication() {
        // (1 + 2i)(3 + 4i) = (3 - 8) + (4 + 6)i = -5 + 10i, since i^2 = -1.
        let base = <P248 as ParameterSet>::Base::from_u64;
        let left =
            P248::ext_from_base(base(1)).add(P248::ext_i().mul(P248::ext_from_base(base(2))));
        let right =
            P248::ext_from_base(base(3)).add(P248::ext_i().mul(P248::ext_from_base(base(4))));
        let product = left.mul(right);
        let (re, im) = P248::ext_parts(product);
        let minus_five = <P248 as ParameterSet>::Base::zero().sub(base(5));
        assert!(re.same(minus_five));
        assert!(im.same(base(10)));
    }

    #[test]
    fn every_salt_byte_pattern_has_one_length_delimited_encoding() {
        fn check<L: ParameterSet>() {
            for byte in [0, 1, 127, 255] {
                let bytes = vec![byte; L::SALT_LEN];
                assert_eq!(L::Salt::from_bytes(&bytes).unwrap().as_bytes(), bytes);
            }
            assert!(L::Salt::from_bytes(&vec![0; L::SALT_LEN - 1]).is_none());
            assert!(L::Salt::from_bytes(&vec![0; L::SALT_LEN + 1]).is_none());
            let counter = L::Salt::from_counter(0x0102).unwrap();
            assert_eq!(&counter.as_bytes()[..2], &[0x02, 0x01]);
            assert!(counter.as_bytes()[8..].iter().all(|byte| *byte == 0));
        }
        check::<P248>();
        check::<P376>();
        check::<P500>();
    }

    #[test]
    fn a_noncanonical_integer_does_not_decode() {
        let mut bytes = vec![0u8; P248::FP_LEN];
        bytes[P248::FP_LEN - 1] = 0xff;
        assert!(P248::base_decode(&bytes).is_none());
    }
}
