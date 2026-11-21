// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Salted challenge derivation. Public verification never searches.
//!
//! The challenge is a hash of the key's bytes, the signature's salt and
//! the message, cut to an odd scalar `s` of [`ParameterSet::CHALLENGE_BITS`]
//! bits. The degree the signature must have is `q = 2^{2e′−1} − s²`, and
//! the verifier refuses unless `q` is prime. A signer searches salts for
//! one whose degree is prime; the verifier hashes once, with the salt it
//! was given.
use crate::{identity::KeyIdentity, prime};
use prism_field::width::degree_bits;
use prism_field::{DegreeWidth, ParameterSet, Salt};
use sha3::digest::core_api::CoreWrapper;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::{CShake256, CShake256Core};

/// The cSHAKE256 customization string of the challenge hash. Part of the
/// protocol: a signer and a verifier that differ in one byte of it derive
/// different challenges.
pub const CHALLENGE_DOMAIN: &[u8] = b"shekyl-prism-challenge-v2";

/// The challenge scalar `s`, little-endian.
///
/// The field is private and the one constructor in a default build is
/// [`Self::derive`], so a value is odd, is below
/// `2^CHALLENGE_BITS` for the set it was derived at, and is the hash of
/// a key, a salt and a message. The set is not in the value: a scalar
/// derived at one set and asked about at another is answered for the
/// other set's exponent, and nothing refuses that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChallengeScalar {
    bytes: Vec<u8>,
}
impl ChallengeScalar {
    /// cSHAKE256 under [`CHALLENGE_DOMAIN`] of
    /// `public key ‖ salt ‖ message`, read to
    /// [`ParameterSet::CHALLENGE_BITS`] bits (`e − 3`) with the lowest bit
    /// set.
    ///
    /// It assumes nothing: the key is a [`KeyIdentity`], which only its
    /// decoder makes, and every salt and every message is hashed. It does
    /// not decide whether the degree is prime; that is
    /// [`Self::degree_is_prime`], and a caller that skips it has a scalar
    /// the verifier would refuse as `Composite` most of the time.
    pub fn derive<L: ParameterSet>(
        identity: &KeyIdentity<'_, L>,
        message: &[u8],
        salt: L::Salt,
    ) -> Self {
        Self::with_width::<L>(identity, message, salt, L::CHALLENGE_BITS)
    }
    fn with_width<L: ParameterSet>(
        identity: &KeyIdentity<'_, L>,
        message: &[u8],
        salt: L::Salt,
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
    /// Not PRISM: the challenge cut to `bits` bits, for the reduced-width
    /// experiment. `None` when `bits` is zero or above the set's width, so
    /// it cannot widen the challenge.
    #[cfg(feature = "research-reduced-width")]
    pub fn derive_width<L: ParameterSet>(
        identity: &KeyIdentity<'_, L>,
        message: &[u8],
        salt: L::Salt,
        bits: u32,
    ) -> Option<Self> {
        (bits > 0 && bits <= L::CHALLENGE_BITS)
            .then(|| Self::with_width::<L>(identity, message, salt, bits))
    }
    #[cfg(feature = "research-reduced-width")]
    pub(crate) fn research_bytes<L: ParameterSet>(bytes: &[u8], bits: u32) -> Option<Self> {
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
    /// `s`, little-endian, in the whole bytes its width takes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Whether the degree `2^{2e′−1} − s²` of this challenge is a
    /// Baillie–PSW prime at the set `L`, with `e′ = e − 2`.
    ///
    /// `L` must be the set the scalar was derived at; the value does not
    /// record it and this does not test it. Both entries refuse with
    /// `Composite` when this is `false`.
    pub fn degree_is_prime<L: ParameterSet>(&self) -> bool {
        // The set's width holds every value its primality test forms. A
        // set whose degree outgrew its width stops compiling here.
        const {
            assert!(degree_bits(L::E) <= <L::Degree as DegreeWidth>::BITS);
        }
        prime::degree_is_prime::<L::Degree>(L::CHAIN_STEPS, &self.bytes)
    }
}
fn top_byte_mask(bits: u32) -> u8 {
    u8::MAX >> (7 - (bits.wrapping_sub(1) % 8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{Field, P248, P324, P376, P500, P664};
    #[test]
    fn salted_transcript_binds_message_and_all_salt_bytes_at_every_row() {
        fn check<L: ParameterSet>() {
            let key = L::ext_encode(L::Ext::from_u64(3));
            let id = KeyIdentity::<L>::decode(&key).unwrap();
            let salt = L::Salt::zero();
            let mut bytes = salt.as_bytes().to_vec();
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
                        L::Salt::from_bytes(&bytes).unwrap()
                    )
                );
                bytes[i] ^= 1;
            }
        }
        check::<P248>();
        check::<P324>();
        check::<P376>();
        check::<P500>();
        check::<P664>();
    }
    #[test]
    fn top_mask_keeps_the_requested_bits() {
        for bits in 1..=64 {
            assert_eq!(top_byte_mask(bits).count_ones(), (bits - 1) % 8 + 1);
        }
    }
}
