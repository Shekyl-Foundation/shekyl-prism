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
//! Nothing here records a security level. The category a prime is assessed
//! at moves with research and is documentation: `docs/PARAMETER_SETS.md`,
//! which no code reads.
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

/// One parameter set: a prime `p = f·2^e − 1`, its two fields and the
/// lengths its encodings use.
pub trait ParameterSet: Copy + Clone + Eq + Send + Sync + 'static {
    type Base: Field;
    type Ext: Field;
    /// The set's name, `p` and then `e`: `p248`, `p376`, `p500`. For
    /// people to read. Nothing hashes it and nothing looks data up by it;
    /// that is [`Self::LEGACY_LABEL`].
    const NAME: &'static str;
    /// The string this set was known by before sets were named by prime.
    ///
    /// It is opaque and it never changes. Committed vectors and records
    /// carry it in their `level` field, and the research signer absorbs it
    /// into the streams its public test seeds expand to, so changing one
    /// byte of it would change every committed vector. It says nothing
    /// about the set, and a set added later has no need of one in this
    /// form: it would use its [`Self::NAME`].
    const LEGACY_LABEL: &'static str;
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
    /// Public salt length in bytes: `e` rounded up to a multiple of 128
    /// bits, so 32, 48 and 64 bytes at `p248`, `p376` and `p500`.
    const SALT_LEN: usize;
    /// How many candidates each search for a torsion basis point tries
    /// before it gives up: 256, 384 and 512.
    ///
    /// Three searches use it, and reaching it means something different
    /// in each.
    ///
    /// - The basis index of a public key
    ///   (`prism_verify::basis_hint`): the candidates are small
    ///   field elements, each tried as the abscissa of a point of full
    ///   even order. With no candidate inside the bound the key is refused
    ///   as `Basis`. The reference signer searches without a bound, so
    ///   this is a key the reference accepts and this verifier does not.
    /// - The second point of a basis of `E[p + 1]`
    ///   (`prism_isogeny::torsion::rational_basis`): the candidates are the
    ///   integers `1..=SEARCH_BOUND` as abscissas. With none inside the
    ///   bound there is no basis and the caller refuses.
    /// - The basis of the starting curve `y² = x³ + x`
    ///   (`prism_isogeny::torsion::starting_basis`): a fixed computation
    ///   for each prime, with no input. It either finds its basis inside
    ///   the bound or it does not, and a test holds that it does for every
    ///   set. No probability applies.
    ///
    /// Only the first has an estimate of how often it fails, and it is
    /// the argument the bound was first chosen by: heuristic, inherited,
    /// and not proved here. It treats the candidates as independent. When
    /// the curve coefficient is not a square, all `N` candidates are
    /// tried and each lies on the curve with probability about one half,
    /// so the search fails with probability about `2^-N`. When it is a
    /// square, about half the candidates are first discarded by a
    /// non-square filter on `1 + b²`, the remaining `N/2` or so each
    /// succeed with probability about one half, and the search fails with
    /// probability about `2^-(N/2)`. The one measurement is small: over
    /// the 113 committed keys the largest basis index is 16.
    ///
    /// The second search also requires full order and independence of the
    /// first point, and no failure estimate is made for it here.
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
        salt_len = $salt_len:literal,
        search_bound = $search_bound:literal,
        legacy_label = $legacy_label:literal,
        $fp:ident,
        $fp2:ident,
        [$($limb:expr),+ $(,)?]
    ) => {
        pub mod $modname {
            use super::ops::{tally, Kind, Op};
            use super::{define_fp2_from_type, define_fp_core, Field, ParameterSet, Salt};

            const SALT_BYTES: usize = $salt_len;

            // The salt is `e` rounded up to a multiple of 128 bits.
            const _: () = assert!(
                SALT_BYTES == ($e as usize).div_ceil(128) * 16,
                "the salt is e rounded up to a multiple of 128 bits"
            );

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
                const LEGACY_LABEL: &'static str = $legacy_label;
                const E: u32 = $e;
                const ODD: u64 = $f;
                const CHAIN_STEPS: u32 = $e - 2;
                const CHALLENGE_BITS: u32 = $e - 3;
                const FP_LEN: usize = <$fp>::ENCODED_LENGTH;
                const FP2_LEN: usize = <$fp2>::ENCODED_LENGTH;
                const PUBLIC_KEY_LEN: usize = <$fp2>::ENCODED_LENGTH;
                type Salt = SaltBytes;
                const S_LEN: usize = (Self::CHALLENGE_BITS as usize).div_ceil(8);
                const SALT_LEN: usize = SALT_BYTES;
                const SEARCH_BOUND: u16 = $search_bound;
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
    salt_len = 32,
    search_bound = 256,
    legacy_label = "NIST-I",
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
    salt_len = 48,
    search_bound = 384,
    legacy_label = "NIST-III",
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
    salt_len = 64,
    search_bound = 512,
    legacy_label = "NIST-V",
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
    use super::{Field, ParameterSet, Salt, P248, P376, P500};

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
        assert_eq!(L::SALT_LEN, (L::E as usize).div_ceil(128) * 16);
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

    /// The legacy labels are data: committed vectors and the research
    /// signer's seed streams carry these exact bytes.
    #[test]
    fn the_legacy_labels_are_the_committed_strings() {
        assert_eq!(P248::LEGACY_LABEL, "NIST-I");
        assert_eq!(P376::LEGACY_LABEL, "NIST-III");
        assert_eq!(P500::LEGACY_LABEL, "NIST-V");
        assert_eq!(
            (P248::SALT_LEN, P376::SALT_LEN, P500::SALT_LEN),
            (32, 48, 64)
        );
        assert_eq!(
            (P248::SEARCH_BOUND, P376::SEARCH_BOUND, P500::SEARCH_BOUND),
            (256, 384, 512)
        );
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
