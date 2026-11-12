// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Salted challenge derivation. Public verification never searches.
use crate::{identity::KeyIdentity, prime};
use prism_field::{Level, Salt};
use sha3::digest::core_api::CoreWrapper;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::{CShake256, CShake256Core};

pub const CHALLENGE_DOMAIN: &[u8] = b"shekyl-prism-challenge-v2";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChallengeScalar {
    bytes: Vec<u8>,
}
impl ChallengeScalar {
    /// cSHAKE256(pk || salt || message), projected to odd E-3 bits.
    pub fn derive<L: Level>(identity: &KeyIdentity<'_, L>, message: &[u8], salt: Salt<L>) -> Self {
        Self::with_width::<L>(identity, message, salt, L::CHALLENGE_BITS)
    }
    fn with_width<L: Level>(
        identity: &KeyIdentity<'_, L>,
        message: &[u8],
        salt: Salt<L>,
        bits: u32,
    ) -> Self {
        let mut hash: CShake256 = CoreWrapper::from_core(CShake256Core::new(CHALLENGE_DOMAIN));
        hash.update(identity.as_bytes());
        hash.update(salt.as_bytes());
        hash.update(message);
        let mut bytes = vec![0; (bits as usize).div_ceil(8)];
        hash.finalize_xof().read(&mut bytes);
        bytes[0] |= 1;
        *bytes.last_mut().expect("nonempty width") &= top_byte_mask(bits);
        Self { bytes }
    }
    /// Research-only projection; it cannot increase the production width.
    #[cfg(feature = "research-reduced-width")]
    pub fn derive_width<L: Level>(
        identity: &KeyIdentity<'_, L>,
        message: &[u8],
        salt: Salt<L>,
        bits: u32,
    ) -> Option<Self> {
        (bits > 0 && bits <= L::CHALLENGE_BITS)
            .then(|| Self::with_width::<L>(identity, message, salt, bits))
    }
    #[cfg(feature = "research-reduced-width")]
    pub(crate) fn research_bytes<L: Level>(bytes: &[u8], bits: u32) -> Option<Self> {
        if bits == 0
            || bits > L::CHALLENGE_BITS
            || bytes.len() != (bits as usize).div_ceil(8)
            || bytes.first()? & 1 == 0
            || bytes.last()? & !top_byte_mask(bits) != 0
        {
            return None;
        }
        Some(Self {
            bytes: bytes.to_vec(),
        })
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn degree_is_prime<L: Level>(&self) -> bool {
        const {
            assert!((L::E as usize) * 2 - 1 < prime::WIDTH_BITS);
        }
        prime::degree_is_prime(L::E - 2, &self.bytes)
    }
}
fn top_byte_mask(bits: u32) -> u8 {
    u8::MAX >> (7 - (bits.wrapping_sub(1) % 8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{Field, Level1, Level3, Level5};
    #[test]
    fn salted_transcript_binds_message_and_all_salt_bytes_at_every_row() {
        fn check<L: Level>() {
            let key = L::ext_encode(L::Ext::from_u64(3));
            let id = KeyIdentity::<L>::decode(&key).unwrap();
            let mut bytes = vec![0; L::SALT_LEN];
            let salt = Salt::<L>::from_bytes(&bytes).unwrap();
            let original = ChallengeScalar::derive(&id, b"binary\0message", salt);
            assert_eq!(original.as_bytes().len(), L::S_LEN);
            assert_eq!(original.as_bytes()[0] & 1, 1);
            assert_eq!(
                original.as_bytes().last().unwrap() & !top_byte_mask(L::CHALLENGE_BITS),
                0
            );
            assert_ne!(original, ChallengeScalar::derive(&id, b"binary", salt));
            for i in 0..bytes.len() {
                bytes[i] ^= 1;
                assert_ne!(
                    original,
                    ChallengeScalar::derive(
                        &id,
                        b"binary\0message",
                        Salt::from_bytes(&bytes).unwrap()
                    )
                );
                bytes[i] ^= 1;
            }
        }
        check::<Level1>();
        check::<Level3>();
        check::<Level5>();
    }
    #[test]
    fn top_mask_keeps_the_requested_bits() {
        for bits in 1..=64 {
            assert_eq!(top_byte_mask(bits).count_ones(), (bits - 1) % 8 + 1);
        }
    }
}
