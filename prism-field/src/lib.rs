// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The three primes in Table 2 of ePrint 2026/443, instantiated with `fp2`.
//!
//! `fp2` 0.6.0 is MIT OR Apache-2.0. Its default feature is `test-utils`, so
//! this crate turns default features off. Packed arithmetic stays off as well:
//! `65·2^376 − 1` is 383 bits and is not strictly below `2^382`, so the
//! feature's modulus rule does not hold for every level. One crate feature
//! cannot turn it on for some levels and leave the others scalar.

#![forbid(unsafe_code)]

use fp2::define_fp2_from_type;
use fp2::define_fp_core;

/// Field operations the isogeny step uses. `inv` of zero is `None`.
pub trait Field: Copy {
    fn zero() -> Self;
    fn one() -> Self;
    fn from_u64(v: u64) -> Self;
    fn add(self, rhs: Self) -> Self;
    fn sub(self, rhs: Self) -> Self;
    fn mul(self, rhs: Self) -> Self;
    fn negate(self) -> Self;
    fn inv(self) -> Option<Self>;
    /// A square root, or `None` when this element is not a square.
    fn sqrt(self) -> Option<Self>;
    fn is_zero(self) -> bool;
    fn same(self, rhs: Self) -> bool;
}

/// One NIST column of Table 2.
pub trait Level: Copy + Clone + Eq + Send + Sync + 'static {
    type Base: Field;
    type Ext: Field;
    const NAME: &'static str;
    const LAMBDA: u32;
    /// Valuation in `p = f·2^e − 1`.
    const E: u32;
    /// salt-PRISM `a`. Table 4 counts this many (2,2)-steps at verification.
    const CHAIN_STEPS: u32;
    const FP_LEN: usize;
    const FP2_LEN: usize;
    const PUBLIC_KEY_LEN: usize;
    const SIGNATURE_LEN: usize;
    const S_LEN: usize;

    fn fp_zero() -> Self::Base;
    fn ext_i() -> Self::Ext;
    fn ext_from_base(re: Self::Base) -> Self::Ext;
    fn ext_parts(value: Self::Ext) -> (Self::Base, Self::Base);
    fn base_decode(bytes: &[u8]) -> Option<Self::Base>;
    fn base_encode(value: Self::Base) -> Vec<u8>;
    fn ext_decode(bytes: &[u8]) -> Option<Self::Ext>;
    fn ext_encode(value: Self::Ext) -> Vec<u8>;
}

macro_rules! nist_level {
    (
        $modname:ident,
        $level:ident,
        $name:literal,
        $lambda:expr,
        $e:expr,
        $steps:expr,
        $fp:ident,
        $fp2:ident,
        [$($limb:expr),+ $(,)?]
    ) => {
        pub mod $modname {
            use super::{define_fp2_from_type, define_fp_core, Field, Level};

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
                    self * rhs
                }
                fn negate(self) -> Self {
                    -self
                }
                fn inv(self) -> Option<Self> {
                    if mask_ok(self.is_zero()) {
                        None
                    } else {
                        Some(self.invert())
                    }
                }
                fn sqrt(self) -> Option<Self> {
                    let mut value = self;
                    let mask = value.set_sqrt();
                    if mask_ok(mask) { Some(value) } else { None }
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
                    self * rhs
                }
                fn negate(self) -> Self {
                    -self
                }
                fn inv(self) -> Option<Self> {
                    if mask_ok(self.is_zero()) {
                        None
                    } else {
                        Some(self.invert())
                    }
                }
                fn sqrt(self) -> Option<Self> {
                    let mut value = self;
                    let mask = value.set_sqrt();
                    if mask_ok(mask) { Some(value) } else { None }
                }
                fn is_zero(self) -> bool {
                    mask_ok($fp2::is_zero(self))
                }
                fn same(self, rhs: Self) -> bool {
                    mask_ok(self.equals(&rhs))
                }
            }

            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub struct $level;

            impl Level for $level {
                type Base = $fp;
                type Ext = $fp2;
                const NAME: &'static str = $name;
                const LAMBDA: u32 = $lambda;
                const E: u32 = $e;
                const CHAIN_STEPS: u32 = $steps;
                const FP_LEN: usize = <$fp>::ENCODED_LENGTH;
                const FP2_LEN: usize = <$fp2>::ENCODED_LENGTH;
                const PUBLIC_KEY_LEN: usize = <$fp2>::ENCODED_LENGTH + 2;
                const S_LEN: usize = (($e - 1) as usize).div_ceil(8);
                const SIGNATURE_LEN: usize = 5 * <$fp2>::ENCODED_LENGTH + Self::S_LEN;

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
        pub use $modname::$level;
    };
}

nist_level!(
    level1,
    Level1,
    "NIST-I",
    128,
    248,
    192,
    FpLevel1,
    Fp2Level1,
    [
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0x04FFFFFFFFFFFFFFu64,
    ]
);

nist_level!(
    level3,
    Level3,
    "NIST-III",
    192,
    376,
    256,
    FpLevel3,
    Fp2Level3,
    [
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0xFFFFFFFFFFFFFFFFu64,
        0x40FFFFFFFFFFFFFFu64,
    ]
);

nist_level!(
    level5,
    Level5,
    "NIST-V",
    256,
    500,
    320,
    FpLevel5,
    Fp2Level5,
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
    use super::{Field, Level, Level1, Level3, Level5};

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn check_base<L: Level>(add: &str, mul: &str, inv: &str, sqrt_sq: &str) {
        let two = L::Base::from_u64(2);
        let three = L::Base::from_u64(3);
        assert_eq!(hex(&L::base_encode(two.add(three))), add);
        assert_eq!(hex(&L::base_encode(two.mul(three))), mul);
        let inverse = two.inv().expect("2 is invertible");
        assert_eq!(hex(&L::base_encode(inverse)), inv);
        assert!(inverse.mul(two).same(L::Base::one()));
        let four = two.mul(two);
        let root = L::base_decode(&hex_bytes(sqrt_sq)).expect("root decodes");
        assert!(root.mul(root).same(four));
        assert_eq!(L::FP_LEN * 2, L::FP2_LEN);
        assert_eq!(L::PUBLIC_KEY_LEN, L::FP2_LEN + 2);
        assert_eq!(L::SIGNATURE_LEN, 5 * L::FP2_LEN + L::S_LEN);
    }

    fn hex_bytes(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
            .collect()
    }

    #[test]
    fn level_i_matches_the_committed_integers() {
        check_base::<Level1>(
            "0500000000000000000000000000000000000000000000000000000000000000",
            "0600000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000008002",
            "0200000000000000000000000000000000000000000000000000000000000000",
        );
        assert_eq!(Level1::PUBLIC_KEY_LEN, 66);
        assert_eq!(Level1::SIGNATURE_LEN, 351);
    }

    #[test]
    fn level_iii_matches_the_committed_integers() {
        check_base::<Level3>(
            "050000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "060000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008020",
            "020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        );
        assert_eq!(Level3::PUBLIC_KEY_LEN, 98);
        assert_eq!(Level3::SIGNATURE_LEN, 527);
    }

    #[test]
    fn level_v_matches_the_committed_integers() {
        check_base::<Level5>(
            "05000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "06000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000d800",
            "02000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        );
        assert_eq!(Level5::PUBLIC_KEY_LEN, 130);
        assert_eq!(Level5::SIGNATURE_LEN, 703);
    }

    #[test]
    fn extension_matches_schoolbook_multiplication() {
        // (1 + 2i)(3 + 4i) = (3 - 8) + (4 + 6)i = -5 + 10i, since i^2 = -1.
        let base = <Level1 as Level>::Base::from_u64;
        let left =
            Level1::ext_from_base(base(1)).add(Level1::ext_i().mul(Level1::ext_from_base(base(2))));
        let right =
            Level1::ext_from_base(base(3)).add(Level1::ext_i().mul(Level1::ext_from_base(base(4))));
        let product = left.mul(right);
        let (re, im) = Level1::ext_parts(product);
        let minus_five = <Level1 as Level>::Base::zero().sub(base(5));
        assert!(re.same(minus_five));
        assert!(im.same(base(10)));
    }

    #[test]
    fn a_noncanonical_integer_does_not_decode() {
        let mut bytes = vec![0u8; Level1::FP_LEN];
        bytes[Level1::FP_LEN - 1] = 0xff;
        assert!(Level1::base_decode(&bytes).is_none());
    }
}
