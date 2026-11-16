// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Research-only archived unsalted-v1 hashing. Never a production dependency.
#![forbid(unsafe_code)]
use prism_field::ParameterSet;
use prism_verify::KeyIdentity;
use sha3::{
    digest::{core_api::CoreWrapper, ExtendableOutput, Update, XofReader},
    CShake256, CShake256Core,
};

pub fn unsalted<L: ParameterSet>(
    key: &[u8],
    message: &[u8],
    index: u16,
    bits: u32,
) -> Option<Vec<u8>> {
    KeyIdentity::<L>::decode(key).ok()?;
    if bits == 0 || bits > L::CHALLENGE_BITS {
        return None;
    }
    let mut hash: CShake256 =
        CoreWrapper::from_core(CShake256Core::new(b"shekyl-prism-challenge-v1"));
    hash.update(key);
    hash.update(message);
    hash.update(&index.to_le_bytes());
    let mut bytes = vec![0; (bits as usize).div_ceil(8)];
    hash.finalize_xof().read(&mut bytes);
    bytes[0] |= 1;
    *bytes.last_mut()? &= u8::MAX >> (7 - (bits - 1) % 8);
    Some(bytes)
}

pub fn first_prime<L: ParameterSet>(key: &[u8], message: &[u8]) -> Option<(u16, Vec<u8>)> {
    for index in 0..=u16::MAX {
        let scalar = unsalted::<L>(key, message, index, L::CHALLENGE_BITS)?;
        if prism_verify::degree_is_prime::<L::Degree>(L::E - 2, &scalar) {
            return Some((index, scalar));
        }
    }
    None
}

/// Research-only adapter of the archived index wire to the unchanged real
/// algebraic verifier. The salt padding is ignored by verify_scalar;
/// archived hashing, including the actual carried index, happens here.
#[cfg(feature = "experiments")]
pub fn verify_unsalted<L: ParameterSet>(
    key: &[u8],
    message: &[u8],
    signature: &[u8],
    bits: u32,
) -> Result<(), prism_verify::VerifyError> {
    let prefix = L::SIGNATURE_ELEMENTS * L::FP2_LEN;
    if signature.len() != prefix + 2 {
        return Err(prism_verify::VerifyError::Length);
    }
    let index = u16::from_le_bytes(
        signature[prefix..]
            .try_into()
            .map_err(|_| prism_verify::VerifyError::Length)?,
    );
    let scalar =
        unsalted::<L>(key, message, index, bits).ok_or(prism_verify::VerifyError::Length)?;
    let mut response = signature[..prefix].to_vec();
    response.resize(L::SIGNATURE_LEN, 0);
    prism_verify::research::verify_scalar::<L>(key, &response, &scalar, bits)
}

#[cfg(feature = "experiments")]
pub mod native;

/// Public-data hex encoding for reproducible experiment records.
pub fn encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn decode(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).ok()?, 16).ok())
        .collect()
}

/// One archived unsalted vector: its `level` field (the committed label),
/// message, key, the five
/// response elements in wire order, and the carried index.
pub struct Archived {
    pub file: String,
    pub level: String,
    pub message: Vec<u8>,
    pub key: Vec<u8>,
    pub response: Vec<u8>,
    pub index: u16,
}

/// Every archived unsalted vector kept for the comparison, in name order.
pub fn archived_vectors() -> Vec<Archived> {
    let root = prism_vectors::unsalted::dir();
    prism_vectors::unsalted::files()
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("archived vector");
            let vector = prism_vectors::corpus::Vector::new(&text);
            Archived {
                file: path
                    .strip_prefix(&root)
                    .expect("under the archive root")
                    .to_string_lossy()
                    .into_owned(),
                level: vector.level().expect("archived level").to_string(),
                message: vector.message().expect("archived message").to_vec(),
                key: vector.public_key().expect("archived key"),
                response: vector.curve_and_points().expect("archived response"),
                index: vector
                    .field("j")
                    .expect("archived index")
                    .parse()
                    .expect("archived index"),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{P248, P376, P500};
    use prism_vectors::corpus::Vector;
    fn check<L: ParameterSet>(vector: Vector<'_>) {
        let key = vector.public_key().unwrap();
        let message = vector.message().unwrap();
        let j: u16 = vector.field("j").unwrap().parse().unwrap();
        let scalar = unsalted::<L>(&key, message, j, L::CHALLENGE_BITS).unwrap();
        assert_eq!(prism_signer::decimal(&scalar), vector.field("s").unwrap());
        assert!(prism_verify::degree_is_prime::<L::Degree>(
            L::E - 2,
            &scalar
        ));
        let mut sig = vector.curve_and_points().unwrap();
        sig.extend(j.to_le_bytes());
        assert_eq!(
            prism_verify::verify::<L>(&key, message, &sig),
            Err(prism_verify::VerifyError::Length)
        );
        #[cfg(feature = "experiments")]
        assert_eq!(
            verify_unsalted::<L>(&key, message, &sig, L::CHALLENGE_BITS),
            Ok(())
        );
    }
    #[test]
    fn archived_hashes_match_every_original_and_default_verifier_rejects_old_wire() {
        let files = prism_vectors::unsalted::files();
        assert_eq!(files.len(), prism_vectors::corpus::HONEST_VECTORS);
        for path in files {
            let text = std::fs::read_to_string(&path).unwrap();
            let vector = Vector::new(&text);
            match prism_verify::CommittedSet::from_label(vector.level().unwrap()).unwrap() {
                prism_verify::CommittedSet::P248 => check::<P248>(vector),
                prism_verify::CommittedSet::P376 => check::<P376>(vector),
                prism_verify::CommittedSet::P500 => check::<P500>(vector),
            }
        }
    }
}
