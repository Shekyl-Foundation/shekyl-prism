// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The challenge scalar, and the signer's search for an index that names a
//! prime degree.

use prism_field::{ChallengeIndex, Level};
use sha3::digest::core_api::CoreWrapper;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::{CShake256, CShake256Core};

use crate::identity::KeyIdentity;
use crate::{prime, VerifyError};

/// Customization string for the challenge hash. One context, versioned.
/// Nothing has consumed `-v1`.
pub const CHALLENGE_DOMAIN: &[u8] = b"shekyl-prism-challenge-v1";

/// The challenge scalar `s_j`: odd, and strictly below `2^{e−3}`.
///
/// Transform-shaped — [`Self::derive`] is its definition — so nothing stores
/// it and the signature does not carry it. The constructor is the only way
/// to build one, which is what makes the domain an invariant rather than a
/// claim at each use site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChallengeScalar {
    /// Little-endian, `L::S_LEN` bytes.
    bytes: Vec<u8>,
}

impl ChallengeScalar {
    /// `s_j = cSHAKE256(S = CHALLENGE_DOMAIN; identity ‖ message ‖ LE16(j))`,
    /// projected onto the odd integers below `2^{e′−1}`, `e′ = e − 2`.
    pub fn derive<L: Level>(
        identity: &KeyIdentity<'_, L>,
        message: &[u8],
        index: ChallengeIndex,
    ) -> Self {
        Self::derive_width::<L>(identity, message, index, L::CHALLENGE_BITS)
    }

    /// The same hash, projected onto the odd integers below `2^bits`.
    ///
    /// The instantiated verifier passes [`Level::CHALLENGE_BITS`], `e′ − 1`.
    pub fn derive_width<L: Level>(
        identity: &KeyIdentity<'_, L>,
        message: &[u8],
        index: ChallengeIndex,
        bits: u32,
    ) -> Self {
        let mut hasher: CShake256 = CoreWrapper::from_core(CShake256Core::new(CHALLENGE_DOMAIN));
        hasher.update(identity.as_bytes());
        hasher.update(message);
        hasher.update(&index.to_canonical_bytes());
        let mut bytes = vec![0u8; (bits as usize).div_ceil(8)];
        hasher.finalize_xof().read(&mut bytes);
        project_onto_domain(&mut bytes, bits);
        Self { bytes }
    }

    /// Little-endian bytes of `s`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// `true` when `q = 2^{2e′−1} − s^2` is a Baillie–PSW prime, `e′ = e − 2`.
    pub fn degree_is_prime<L: Level>(&self) -> bool {
        const {
            assert!(
                (L::E as usize) * 2 - 1 < prime::WIDTH_BITS,
                "this level's challenge degree is wider than the primality module"
            );
        }
        prime::degree_is_prime(L::E - 2, &self.bytes)
    }
}

/// Force a hash output onto the challenge's domain: odd, and strictly below
/// `2^bits`.
///
/// `bytes` is `bits.div_ceil(8)` long and `bits` is at least one for every
/// level, so both ends of the slice exist and may be the same byte.
fn project_onto_domain(bytes: &mut [u8], bits: u32) {
    let mask = top_byte_mask(bits);
    if let Some(low) = bytes.first_mut() {
        *low |= 1;
    }
    if let Some(high) = bytes.last_mut() {
        *high &= mask;
    }
}

/// Mask for the top byte of a `bits`-wide little-endian integer.
///
/// The top byte of a `bits.div_ceil(8)`-byte buffer carries
/// `((bits − 1) mod 8) + 1` significant bits, which is `1..=8`: a width that
/// is a multiple of eight keeps the whole byte, and no width keeps none of
/// it. The mask always leaves bit zero set, so the odd-forcing above it
/// survives a one-byte scalar.
fn top_byte_mask(bits: u32) -> u8 {
    u8::MAX >> (7 - (bits.wrapping_sub(1) % 8))
}

/// Why the signer could not name a challenge index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexError {
    /// The key is not one this level would accept, so no index would help.
    Identity(VerifyError),
    /// No index in the search range names a prime degree. There is no
    /// fallback degree, so this is a signing error.
    Exhausted,
}

impl core::fmt::Display for IndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Identity(err) => write!(f, "the key is refused: {err}"),
            Self::Exhausted => f.write_str("no index names a prime degree"),
        }
    }
}

impl std::error::Error for IndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Identity(err) => Some(err),
            Self::Exhausted => None,
        }
    }
}

/// The smallest index whose challenge degree is prime: the signer's search.
///
/// Signing is deterministic for a given key and message because this takes
/// the smallest such index. The verifier does not call this, and it does not
/// check that a carried index is the smallest. Distinct indices can hash
/// to the same scalar and degree; the security argument must account for
/// such collisions and for any new isogeny at a different degree. See
/// `docs/SECURITY_ARGUMENT.md`, sections 5 and 6. The native research signer
/// may continue past this helper's result when final verification fails.
///
/// The key goes through the same [`KeyIdentity::decode`] the verifier uses,
/// so the signer cannot search against a key the verifier would refuse, and
/// the two cannot disagree on the byte string they hash. The search is at
/// exponent `e′ = e − 2`.
pub fn smallest_challenge_index<L: Level>(
    public_key: &[u8],
    message: &[u8],
) -> Result<ChallengeIndex, IndexError> {
    let (index, _) = smallest_at_exponent::<L>(public_key, message, L::E - 2)?;
    Ok(index)
}

/// The smallest index whose degree `2^{2·exponent−1} − s^2` is prime.
///
/// `exponent` is the chain exponent the scalar is derived for. The
/// instantiated search passes `E − 2`.
pub fn smallest_at_exponent<L: Level>(
    public_key: &[u8],
    message: &[u8],
    exponent: u32,
) -> Result<(ChallengeIndex, ChallengeScalar), IndexError> {
    let identity = KeyIdentity::<L>::decode(public_key).map_err(IndexError::Identity)?;
    // `0 < s < 2^{exponent−1}` has a positive odd `s` only once the upper
    // bound is at least 2. Exponent 1 leaves the empty integer, whose
    // degree is the prime 2, and that integer is not a challenge scalar.
    if exponent < 2 {
        return Err(IndexError::Exhausted);
    }
    let bits = exponent - 1;
    ChallengeIndex::search_order()
        .find_map(|index| {
            let scalar = ChallengeScalar::derive_width::<L>(&identity, message, index, bits);
            if prime::degree_is_prime(exponent, scalar.as_bytes()) {
                Some((index, scalar))
            } else {
                None
            }
        })
        .ok_or(IndexError::Exhausted)
}

#[cfg(test)]
mod tests {
    use super::{top_byte_mask, ChallengeScalar, IndexError};
    use crate::identity::KeyIdentity;
    use prism_field::{ChallengeIndex, Field, Level, Level1, Level3, Level5};

    #[test]
    fn the_top_byte_mask_keeps_one_to_eight_bits() {
        // A width that is a multiple of eight keeps the whole byte: this is
        // the case an `(e − 1) % 8 == 0` level would reach.
        for bits in [8u32, 16, 248, 1024] {
            assert_eq!(top_byte_mask(bits), 0xff, "{bits}");
        }
        for (bits, mask) in [
            (1u32, 0x01u8),
            (2, 0x03),
            (7, 0x7f),
            (9, 0x01),
            (247, 0x7f),
            (375, 0x7f),
            (499, 0x07),
        ] {
            assert_eq!(top_byte_mask(bits), mask, "{bits}");
        }
        // Every mask leaves bit zero set, so a one-byte scalar stays odd.
        for bits in 1u32..=64 {
            assert_eq!(top_byte_mask(bits) & 1, 1, "{bits}");
        }
    }

    fn check_domain<L: Level>() {
        let key = L::ext_encode(L::Ext::from_u64(3));
        let identity = KeyIdentity::<L>::decode(&key).expect("A = 3 is a usable key");
        for index in [0u16, 1, 2, 7, 4660, u16::MAX] {
            let index = ChallengeIndex::from_canonical_bytes(index.to_le_bytes());
            let scalar = ChallengeScalar::derive::<L>(&identity, b"shekyl-prism", index);
            let bytes = scalar.as_bytes();
            assert_eq!(bytes.len(), L::S_LEN, "{} {index}", L::NAME);
            let low = bytes.first().copied().expect("one byte at least");
            assert_eq!(low & 1, 1, "{} {index} is even", L::NAME);
            // `0 < s < 2^{e′−1}`: the bit length does not reach `e − 3`.
            let high = bytes.last().copied().expect("one byte at least");
            let bit_len = (bytes.len() - 1) * 8 + (8 - high.leading_zeros() as usize);
            assert!(
                bit_len <= (L::E - 3) as usize,
                "{} {index} reaches 2^{{{}}}",
                L::NAME,
                L::E - 3
            );
        }
    }

    #[test]
    fn the_scalar_is_odd_and_below_the_bound_at_every_level() {
        check_domain::<Level1>();
        check_domain::<Level3>();
        check_domain::<Level5>();
    }

    #[test]
    fn an_exponent_below_two_names_no_scalar() {
        let key = <Level1 as Level>::ext_encode(<Level1 as Level>::Ext::from_u64(3));
        let err = super::smallest_at_exponent::<Level1>(&key, b"m", 1);
        assert!(matches!(err, Err(IndexError::Exhausted)));
        assert!(!crate::prime::degree_is_prime(1, &[]));
        assert!(!crate::prime::degree_is_prime(1, &[0]));
    }

    #[test]
    fn the_signer_refuses_a_key_the_verifier_would_refuse() {
        let origin = vec![0u8; Level1::PUBLIC_KEY_LEN];
        assert!(matches!(
            super::smallest_challenge_index::<Level1>(&origin, b"m"),
            Err(IndexError::Identity(crate::VerifyError::Origin))
        ));
        let short = vec![0u8; Level1::PUBLIC_KEY_LEN - 1];
        assert!(matches!(
            super::smallest_challenge_index::<Level1>(&short, b"m"),
            Err(IndexError::Identity(crate::VerifyError::Length))
        ));
    }
}
