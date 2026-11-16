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

// The counter is measuring equipment and not part of the library: its
// module exists only when counting is asked for. `tally` is what the
// field implementations call, with or without it.
#[cfg(feature = "op-count")]
pub mod ops;
mod tally;
pub mod width;

pub use width::{DegreeLimbs, DegreeWidth, Limbs, SetDegree};

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
/// assessment; both widths are this rounding of its exponent. `e = 664`
/// rounds to 768.
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

/// The odd cofactor `f` of a prime `p = f·2^e − 1`, with its prime factors.
///
/// A parameter set states the primes of `f` beside `f`. The two are one
/// value because the torsion code needs both and must not be handed a
/// list that belongs to another cofactor: with a prime left out, a point
/// of too small an order passes for full order.
///
/// The only way to make one is [`Cofactor::new`], which refuses anything
/// but an odd `f > 1` with exactly its distinct prime factors, increasing,
/// and no more of them than [`Cofactor::MAX_ODD_PRIMES`]. A set is built
/// with it when the set is compiled, so a wrong list does not compile, and
/// code that takes a `Cofactor` checks nothing again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cofactor {
    value: u64,
    odd_primes: &'static [u64],
}

/// Why a cofactor and a list of primes are not a [`Cofactor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CofactorError {
    /// `f` is zero or even. `p + 1 = f·2^e` has all its twos in `2^e`.
    NotOdd,
    /// No prime is listed. That is right only for `f = 1`, which no set
    /// has, and an empty list would check no odd order at all.
    NoPrimes,
    /// More primes than [`Cofactor::MAX_ODD_PRIMES`]. The basis search
    /// holds one projected pair of points for each, and the ones past its
    /// buffer would go unchecked.
    TooManyPrimes,
    /// A listed number is not an odd prime.
    NotAnOddPrime,
    /// The list does not increase, so a prime is repeated or out of order.
    NotIncreasing,
    /// A listed prime does not divide `f`.
    NotADivisor,
    /// Dividing `f` by every listed prime as often as it goes does not
    /// leave 1: a prime factor of `f` is missing.
    PrimeMissing,
}

impl Cofactor {
    /// The most distinct odd primes a cofactor may have. The basis search
    /// holds one projected pair of points for each on its stack.
    pub const MAX_ODD_PRIMES: usize = 4;

    /// The cofactor `value` with `odd_primes` as its distinct prime
    /// factors, or the first reason they are not that.
    pub const fn new(value: u64, odd_primes: &'static [u64]) -> Result<Self, CofactorError> {
        // Zero is even, so this refuses it too.
        if value.is_multiple_of(2) {
            return Err(CofactorError::NotOdd);
        }
        if odd_primes.is_empty() {
            return Err(CofactorError::NoPrimes);
        }
        if odd_primes.len() > Self::MAX_ODD_PRIMES {
            return Err(CofactorError::TooManyPrimes);
        }
        let mut rest = value;
        let mut previous = 0;
        let mut i = 0;
        while i < odd_primes.len() {
            let prime = odd_primes[i];
            if !is_odd_prime(prime) {
                return Err(CofactorError::NotAnOddPrime);
            }
            if prime <= previous {
                return Err(CofactorError::NotIncreasing);
            }
            if !rest.is_multiple_of(prime) {
                return Err(CofactorError::NotADivisor);
            }
            while rest.is_multiple_of(prime) {
                rest /= prime;
            }
            previous = prime;
            i += 1;
        }
        if rest != 1 {
            return Err(CofactorError::PrimeMissing);
        }
        Ok(Self { value, odd_primes })
    }

    /// `f`.
    pub const fn value(self) -> u64 {
        self.value
    }

    /// The distinct primes dividing `f`, increasing. Never empty, and
    /// never longer than [`Self::MAX_ODD_PRIMES`].
    pub const fn odd_primes(self) -> &'static [u64] {
        self.odd_primes
    }

    /// The bits of `f`, so `p` has `e` plus this many.
    pub const fn bits(self) -> u32 {
        u64::BITS - self.value.leading_zeros()
    }
}

/// Trial division, which is enough for a factor of a 64-bit cofactor that
/// is checked once, when a set is compiled.
const fn is_odd_prime(candidate: u64) -> bool {
    if candidate < 3 || candidate.is_multiple_of(2) {
        return false;
    }
    let mut divisor = 3;
    while divisor <= candidate / divisor {
        if candidate.is_multiple_of(divisor) {
            return false;
        }
        divisor += 2;
    }
    true
}

/// One parameter set: a prime `p = f·2^e − 1`, its two fields and the
/// lengths its encodings use.
pub trait ParameterSet: Copy + Clone + Eq + core::fmt::Debug + Send + Sync + 'static {
    type Base: Field;
    type Ext: Field;
    /// The set's name, `p` and then `e`: `p248`, `p376`, `p500`.
    ///
    /// Display only. Committed vectors and the research signer's seed
    /// streams do not use it.
    const NAME: &'static str;
    /// Valuation in `p = f·2^e − 1`.
    const E: u32;
    /// The odd cofactor `f` in `p + 1 = f·2^e`, with its prime factors.
    const COFACTOR: Cofactor;
    /// The width of the integers the challenge degree's primality test
    /// runs on at this set. A [`SetDegree`]: the 1,024-bit or 1,344-bit
    /// width, not a width that exists only so a carry can be checked.
    /// See [`width`].
    type Degree: SetDegree;
    /// The limbs of an integer modulo `2^e` at this set, which is what the
    /// entries of the verifier's matrices are.
    type TorsionLimbs: Limbs;
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
        odd_primes = [$($odd_prime:literal),+ $(,)?],
        e = $e:literal,
        $fp:ident,
        $fp2:ident,
        [$($limb:expr),+ $(,)?]
    ) => {
        pub mod $modname {
            use super::tally::{tally, Kind, Op};
            use super::{
                candidate_bound, define_fp2_from_type, define_fp_core,
                width::{degree_bits, degree_limbs, torsion_limbs, DegreeLimbs, DegreeWidth},
                width_bits, Cofactor, Field, ParameterSet, Salt,
            };

            /// Checked here, when the set is compiled.
            const COFACTOR: Cofactor = match Cofactor::new($f, &[$($odd_prime),+]) {
                Ok(cofactor) => cofactor,
                Err(_) => panic!(
                    "f must be odd, and odd_primes exactly its distinct prime factors, increasing"
                ),
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
            // The width the set names holds every value its primality test
            // forms. A count that is not a `SetDegree` fails on the
            // associated type below, before this comparison.
            const _: () = assert!(
                degree_bits($e) <= <DegreeLimbs<{ degree_limbs($e) }> as DegreeWidth>::BITS,
                "the set's width holds every value its primality test forms"
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
                const COFACTOR: Cofactor = COFACTOR;
                // A limb count that is not a width fails here.
                type Degree = DegreeLimbs<{ degree_limbs($e) }>;
                type TorsionLimbs = [u64; torsion_limbs($e)];
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
    odd_primes = [5],
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
    p324,
    P324,
    f = 3,
    odd_primes = [3],
    e = 324,
    FpP324,
    Fp2P324,
    [
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0x000000000000002Fu64,
    ]
);

parameter_set!(
    p376,
    P376,
    f = 65,
    odd_primes = [5, 13],
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
    odd_primes = [3],
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
parameter_set!(
    p664,
    P664,
    f = 17,
    odd_primes = [17],
    e = 664,
    FpP664,
    Fp2P664,
    [
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0x0000000010FFFFFFu64,
    ]
);

/// Packed arithmetic is not enabled. See the crate docs.
pub const FIELD_BACKEND: &str = "scalar";

#[cfg(test)]
mod tests {
    use super::{
        width_bits, Cofactor, CofactorError, Field, ParameterSet, Salt, P248, P324, P376, P500,
        P664,
    };

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

    /// Each listed prime is prime and odd, each divides `f`, and dividing
    /// `f` by all of them repeatedly leaves 1. Written out here without the
    /// constructor the macro checks with, so the two cannot share a mistake.
    fn the_odd_primes_factor_the_cofactor<L: ParameterSet>() {
        let (f, primes) = (L::COFACTOR.value(), L::COFACTOR.odd_primes());
        let mut rest = f;
        assert!(!primes.is_empty(), "{}", L::NAME);
        assert!(primes.len() <= Cofactor::MAX_ODD_PRIMES, "{}", L::NAME);
        for &prime in primes {
            assert!(prime > 2 && prime % 2 == 1, "{} {prime} is odd", L::NAME);
            assert!(
                (2..prime).all(|d| prime % d != 0),
                "{} {prime} is prime",
                L::NAME
            );
            assert_eq!(f % prime, 0, "{} {prime} divides f", L::NAME);
            while rest % prime == 0 {
                rest /= prime;
            }
        }
        assert_eq!(rest, 1, "{}: the primes account for all of f", L::NAME);
        assert_eq!(Cofactor::new(f, primes), Ok(L::COFACTOR));
    }

    #[test]
    fn every_set_states_the_primes_of_its_cofactor() {
        the_odd_primes_factor_the_cofactor::<P248>();
        the_odd_primes_factor_the_cofactor::<P324>();
        the_odd_primes_factor_the_cofactor::<P376>();
        the_odd_primes_factor_the_cofactor::<P500>();
        the_odd_primes_factor_the_cofactor::<P664>();
        let stated = |cofactor: Cofactor| (cofactor.value(), cofactor.odd_primes());
        assert_eq!(stated(P248::COFACTOR), (5, &[5][..]));
        assert_eq!(stated(P324::COFACTOR), (3, &[3][..]));
        assert_eq!(stated(P376::COFACTOR), (65, &[5, 13][..]));
        assert_eq!(stated(P500::COFACTOR), (27, &[3][..]));
    }

    /// A cofactor exists only with exactly its prime factors. Each refusal
    /// has its own reason, and nothing here drops a prime and continues:
    /// the torsion code takes a `Cofactor` and checks nothing again.
    #[test]
    fn a_cofactor_with_a_wrong_list_of_primes_cannot_be_built() {
        use CofactorError::{
            NoPrimes, NotADivisor, NotAnOddPrime, NotIncreasing, NotOdd, PrimeMissing,
            TooManyPrimes,
        };
        let built = |value, primes: &'static [u64]| Cofactor::new(value, primes).map(|_| ());
        for (value, primes) in [
            (5, &[5][..]),
            (3, &[3]),
            (65, &[5, 13]),
            (27, &[3]),
            (3 * 5 * 7 * 11, &[3, 5, 7, 11]),
        ] {
            assert_eq!(built(value, primes), Ok(()), "{value} with {primes:?}");
        }
        for (value, primes, reason, why) in [
            (0, &[3][..], NotOdd, "a zero cofactor"),
            (30, &[3, 5], NotOdd, "an even cofactor"),
            (6, &[2], NotOdd, "an even cofactor with its two"),
            (3, &[], NoPrimes, "an empty list checks nothing"),
            (65, &[], NoPrimes, "none listed"),
            (1, &[], NoPrimes, "a cofactor of one"),
            (
                3 * 5 * 7 * 11 * 13,
                &[3, 5, 7, 11, 13],
                TooManyPrimes,
                "a fifth prime is not dropped",
            ),
            (3, &[0], NotAnOddPrime, "zero is not a prime"),
            (3, &[1], NotAnOddPrime, "one is not a prime"),
            (15, &[2, 3, 5], NotAnOddPrime, "two is not an odd prime"),
            (27, &[9], NotAnOddPrime, "nine is not a prime"),
            (65, &[13, 5], NotIncreasing, "not increasing"),
            (65, &[5, 5, 13], NotIncreasing, "repeated"),
            (3, &[5], NotADivisor, "a prime that does not divide"),
            (
                27,
                &[3, 5],
                NotADivisor,
                "a second prime that does not divide",
            ),
            (65, &[5], PrimeMissing, "a prime left out"),
        ] {
            assert_eq!(built(value, primes), Err(reason), "{why}");
        }
        assert_eq!(Cofactor::MAX_ODD_PRIMES, 4);
    }

    /// The width of `f` in bits, which with `e` is the width of `p`.
    #[test]
    fn the_bits_of_a_cofactor_are_its_width() {
        let bits = |value, primes| Cofactor::new(value, primes).map(Cofactor::bits);
        assert_eq!(bits(3, &[3]), Ok(2));
        assert_eq!(bits(5, &[5]), Ok(3));
        assert_eq!(bits(27, &[3]), Ok(5));
        assert_eq!(bits(65, &[5, 13]), Ok(7));
    }

    /// Compare two little-endian byte strings of one length as integers.
    fn below(left: &[u8], right: &[u8]) -> bool {
        assert_eq!(left.len(), right.len());
        left.iter().rev().lt(right.iter().rev())
    }

    /// The decoder takes exactly the integers below `p`, in exactly
    /// `FP_LEN` bytes. Every expected verdict is the integer comparison
    /// with `p`, where `p` is computed from `f` and `e`.
    fn decoding_is_the_integer_order<L: ParameterSet>() {
        let len = L::FP_LEN;
        let mut modulus = modulus_minus_one(L::COFACTOR.value(), L::E, len);
        let below_p = modulus.clone();
        // p − 1, then p, then p + 1.
        assert!(L::base_decode(&below_p).is_some(), "{}: p − 1", L::NAME);
        modulus[0] += 1;
        assert_eq!(modulus[0], 0xff, "p ends in all ones");
        assert!(L::base_decode(&modulus).is_none(), "{}: p", L::NAME);
        let above = {
            // p + 1 = f·2^e: zero below bit e.
            let mut bytes = vec![0u8; len];
            let (byte, bit) = ((L::E / 8) as usize, L::E % 8);
            for (offset, part) in (u128::from(L::COFACTOR.value()) << bit)
                .to_le_bytes()
                .iter()
                .enumerate()
            {
                if *part != 0 {
                    bytes[byte + offset] = *part;
                }
            }
            bytes
        };
        assert!(L::base_decode(&above).is_none(), "{}: p + 1", L::NAME);

        // The spare bits of the top byte: the bits of p stop inside it.
        let bits = L::E + L::COFACTOR.bits();
        let top_bits = bits - 8 * (len as u32 - 1);
        assert!(
            (1..=8).contains(&top_bits),
            "{}: p fills FP_LEN bytes",
            L::NAME
        );
        // All ones in the bits of p, 2^bits − 1, is above p.
        let mut ones = vec![0xffu8; len];
        ones[len - 1] = u8::MAX >> (8 - top_bits);
        assert!(L::base_decode(&ones).is_none(), "{}: 2^bits − 1", L::NAME);

        // Each bit of the top byte alone, and with every lower byte set.
        // Some of these are below p and must decode; the rest must not.
        let mut decoded = 0;
        let mut refused = 0;
        for bit in 0..8u8 {
            for lower in [0x00u8, 0xff] {
                let mut bytes = vec![lower; len];
                bytes[len - 1] = 1 << bit;
                let expect = below(&bytes, &modulus);
                assert_eq!(
                    L::base_decode(&bytes).is_some(),
                    expect,
                    "{}: top byte {:#04x}, lower bytes {lower:#04x}",
                    L::NAME,
                    1u8 << bit
                );
                decoded += usize::from(expect);
                refused += usize::from(!expect);
            }
        }
        assert!(
            refused > 0,
            "{}: no top-byte case is at or above p",
            L::NAME
        );
        assert!(decoded > 0, "{}: no top-byte case is below p", L::NAME);
        // Every value of the top byte with the lower bytes at their largest.
        for top in 0..=u8::MAX {
            let mut bytes = vec![0xffu8; len];
            bytes[len - 1] = top;
            assert_eq!(
                L::base_decode(&bytes).is_some(),
                below(&bytes, &modulus),
                "{}: top byte {top:#04x} over all ones",
                L::NAME
            );
        }

        // A decoded value encodes to the bytes it came from.
        let value = L::base_decode(&below_p).expect("p − 1");
        assert_eq!(L::base_encode(value), below_p, "{}: round trip", L::NAME);
        assert_eq!(L::base_encode(L::Base::one().negate()), below_p);

        // Any other length is refused, whatever the bytes.
        for wrong in [0, len - 1, len + 1, len + 7, 2 * len] {
            assert!(
                L::base_decode(&vec![0u8; wrong]).is_none(),
                "{}: {wrong} bytes",
                L::NAME
            );
        }

        // The extension field decodes two such integers: one out of range
        // in either half refuses the whole.
        let mut pair = [below_p.clone(), below_p.clone()].concat();
        let both = L::ext_decode(&pair).expect("both halves below p");
        assert_eq!(
            L::ext_encode(both),
            pair,
            "{}: extension round trip",
            L::NAME
        );
        pair[..len].copy_from_slice(&modulus);
        assert!(L::ext_decode(&pair).is_none(), "{}: re = p", L::NAME);
        let pair = [below_p.clone(), modulus.clone()].concat();
        assert!(L::ext_decode(&pair).is_none(), "{}: im = p", L::NAME);
        for wrong in [2 * len - 1, 2 * len + 1, len] {
            assert!(L::ext_decode(&vec![0u8; wrong]).is_none());
        }
    }

    #[test]
    fn the_decoder_accepts_exactly_the_integers_below_p() {
        decoding_is_the_integer_order::<P248>();
        decoding_is_the_integer_order::<P324>();
        decoding_is_the_integer_order::<P376>();
        decoding_is_the_integer_order::<P500>();
        decoding_is_the_integer_order::<P664>();
    }

    /// The sizes of `p324`, each computed here from `f = 3` and `e = 324`
    /// and then compared with what the set says. Its field element is the
    /// first that is not a whole number of 8-byte limbs.
    #[test]
    fn p324_has_the_sizes_its_prime_gives() {
        let (f, e): (u64, u32) = (3, 324);
        let bits = e + (64 - f.leading_zeros());
        assert_eq!(bits, 326);
        let field = bits.div_ceil(8) as usize;
        assert_eq!(field, 41);
        assert_ne!(field % 8, 0, "not a whole number of limbs");
        assert_eq!(bits.div_ceil(64), 6, "six 64-bit limbs");
        let salt = (e.div_ceil(128) * 128 / 8) as usize;
        assert_eq!((P324::COFACTOR.value(), P324::E), (f, e));
        assert_eq!(P324::NAME, "p324");
        assert_eq!(P324::FP_LEN, field);
        assert_eq!(P324::FP2_LEN, 2 * field);
        assert_eq!(P324::PUBLIC_KEY_LEN, 82);
        assert_eq!(P324::WIDTH_BITS, 384);
        assert_eq!(P324::SALT_LEN, salt);
        assert_eq!(P324::SALT_LEN, 48);
        assert_eq!(P324::SEARCH_BOUND, 384);
        assert_eq!(P324::SIGNATURE_ELEMENTS, 5);
        assert_eq!(P324::SIGNATURE_LEN, 5 * 82 + 48);
        assert_eq!(P324::SIGNATURE_LEN, 458);
        assert_eq!(P324::CHAIN_STEPS, 322);
        assert_eq!(P324::CHALLENGE_BITS, 321);
        assert_eq!(P324::S_LEN, 41);
        // The top byte of p − 1 is 0x2f: the top limb is 0x2f, with two
        // spare bits in that byte.
        let top = modulus_minus_one(f, e, field)[field - 1];
        assert_eq!(top, 0x2f);
        // p ≡ 7 (mod 8): p − 1 ends in ...110.
        assert_eq!(modulus_minus_one(f, e, field)[0] & 7, 6);
        // The same residue at every set, which the square roots rely on.
        for (f, e, len) in [(5u64, 248u32, 32usize), (65, 376, 48), (27, 500, 64)] {
            assert_eq!(modulus_minus_one(f, e, len)[0] & 7, 6);
        }
    }

    /// The sizes of `p664`, each computed here from `f = 17` and `e = 664`
    /// and then compared with what the set says. It is the first field
    /// above eight limbs, and the first whose challenge degree does not
    /// fit 1,024 bits.
    #[test]
    fn p664_has_the_sizes_its_prime_gives() {
        let (f, e): (u64, u32) = (17, 664);
        let bits = e + (64 - f.leading_zeros());
        assert_eq!(bits, 669);
        let field = bits.div_ceil(8) as usize;
        assert_eq!(field, 84);
        assert_eq!(bits.div_ceil(64), 11, "eleven 64-bit limbs");
        let salt = (e.div_ceil(128) * 128 / 8) as usize;
        assert_eq!((P664::COFACTOR.value(), P664::E), (f, e));
        assert_eq!(P664::COFACTOR.odd_primes(), &[17]);
        assert_eq!(P664::NAME, "p664");
        assert_eq!(P664::FP_LEN, field);
        assert_eq!(P664::FP2_LEN, 2 * field);
        assert_eq!(P664::PUBLIC_KEY_LEN, 168);
        assert_eq!(P664::WIDTH_BITS, 768);
        assert_eq!(P664::SALT_LEN, salt);
        assert_eq!(P664::SALT_LEN, 96);
        assert_eq!(P664::SEARCH_BOUND, 768);
        assert_eq!(P664::SIGNATURE_ELEMENTS, 5);
        assert_eq!(P664::SIGNATURE_LEN, 5 * 168 + 96);
        assert_eq!(P664::SIGNATURE_LEN, 936);
        assert_eq!(P664::CHAIN_STEPS, 662);
        assert_eq!(P664::CHALLENGE_BITS, 661);
        assert_eq!(P664::S_LEN, 83);
        // The top limb is 17·2^24 − 1 = 0x10ffffff, in the low four bytes
        // of the eleventh limb: bytes 80 to 83 of p − 1, of which the top
        // one is 0x10, with three spare bits.
        let minus_one = modulus_minus_one(f, e, field);
        assert_eq!(&minus_one[80..], &[0xff, 0xff, 0xff, 0x10]);
        assert_eq!(u32::from_le_bytes([0xff, 0xff, 0xff, 0x10]), 0x10ff_ffff);
        assert_eq!(8 * field as u32 - bits, 3, "three spare bits");
        // p ≡ 7 (mod 8): p − 1 ends in ...110.
        assert_eq!(minus_one[0] & 7, 6);
        // The widths of the two integers the verifier sizes by the set:
        // the challenge degree has 2e − 5 bits and needs 2e − 4, which is
        // 21 limbs, and a residue modulo 2^e needs 11.
        assert_eq!(crate::width::degree_bits(e), 1324);
        assert_eq!(
            <<P664 as ParameterSet>::Degree as crate::DegreeWidth>::BITS,
            1344
        );
        assert_eq!(
            <<P664 as ParameterSet>::TorsionLimbs as crate::Limbs>::COUNT,
            11
        );
    }

    /// The name, the cofactor, the exponent and the limbs of a set agree:
    /// the field's `−1` is `f·2^e − 2`, computed from `f` and `e` without
    /// reading the limb literals.
    fn the_prime_is<L: ParameterSet>(name: &str, f: u64, e: u32) {
        assert_eq!(L::NAME, name);
        assert_eq!((L::COFACTOR.value(), L::E), (f, e));
        assert_eq!(L::COFACTOR.value() % 2, 1, "{name}: f is odd");
        assert_eq!(
            L::base_encode(L::Base::one().negate()),
            modulus_minus_one(L::COFACTOR.value(), L::E, L::FP_LEN),
            "{name}: p + 1 = f·2^e"
        );
    }

    #[test]
    fn every_set_is_the_prime_its_name_and_cofactor_say() {
        the_prime_is::<P248>("p248", 5, 248);
        the_prime_is::<P324>("p324", 3, 324);
        the_prime_is::<P376>("p376", 65, 376);
        the_prime_is::<P500>("p500", 27, 500);
        the_prime_is::<P664>("p664", 17, 664);
    }

    /// The sizes of the sets in the code. The sizes are the pin.
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
        check::<P324>(48, 384);
        check::<P376>(48, 384);
        check::<P500>(64, 512);
        check::<P664>(96, 768);
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
        check::<P324>();
        check::<P376>();
        check::<P500>();
        check::<P664>();
    }

    #[test]
    fn a_noncanonical_integer_does_not_decode() {
        let mut bytes = vec![0u8; P248::FP_LEN];
        bytes[P248::FP_LEN - 1] = 0xff;
        assert!(P248::base_decode(&bytes).is_none());
    }
}
