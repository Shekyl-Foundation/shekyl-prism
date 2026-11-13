// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Research-only archived unsalted-v1 hashing. Never a production dependency.
#![forbid(unsafe_code)]
use prism_field::Level;
use prism_verify::KeyIdentity;
use sha3::{
    digest::{core_api::CoreWrapper, ExtendableOutput, Update, XofReader},
    CShake256, CShake256Core,
};

pub fn unsalted<L: Level>(key: &[u8], message: &[u8], index: u16, bits: u32) -> Option<Vec<u8>> {
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

pub fn first_prime<L: Level>(key: &[u8], message: &[u8]) -> Option<(u16, Vec<u8>)> {
    for index in 0..=u16::MAX {
        let scalar = unsalted::<L>(key, message, index, L::CHALLENGE_BITS)?;
        if prism_verify::degree_is_prime(L::E - 2, &scalar) {
            return Some((index, scalar));
        }
    }
    None
}

/// Research-only adapter of the archived index wire to the unchanged real
/// algebraic verifier. The salt padding is ignored by verify_scalar;
/// archived hashing, including the actual carried index, happens here.
#[cfg(feature = "experiments")]
pub fn verify_unsalted<L: Level>(
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

/// One archived unsalted vector: level name, message, key, the five
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
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/unsalted");
    let mut paths = Vec::new();
    for dir in [root.clone(), root.join("accept")] {
        for entry in std::fs::read_dir(dir).expect("archived vector directory") {
            let path = entry.expect("archived vector entry").path();
            if path.extension().is_some_and(|s| s == "txt") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path).expect("archived vector");
            let rows: std::collections::HashMap<_, _> =
                text.lines().filter_map(|s| s.split_once(' ')).collect();
            let mut response = Vec::new();
            for name in ["E", "Px", "Py", "Qx", "Qy"] {
                response.extend(decode(rows[name]).expect("archived hex"));
            }
            Archived {
                file: path
                    .strip_prefix(&root)
                    .expect("under the archive root")
                    .to_string_lossy()
                    .into_owned(),
                level: rows["level"].to_string(),
                message: rows["message"].as_bytes().to_vec(),
                key: decode(rows["A"]).expect("archived key hex"),
                response,
                index: rows["j"].parse().expect("archived index"),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::{Level1, Level3, Level5};
    use std::path::{Path, PathBuf};
    fn hex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .chunks_exact(2)
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect()
    }
    fn files() -> Vec<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/unsalted");
        let mut out: Vec<_> = std::fs::read_dir(root.join("accept"))
            .unwrap()
            .map(|p| p.unwrap().path())
            .collect();
        out.extend(
            std::fs::read_dir(root)
                .unwrap()
                .map(|p| p.unwrap().path())
                .filter(|p| p.extension().is_some_and(|s| s == "txt")),
        );
        assert_eq!(out.len(), 113);
        out
    }
    fn check<L: Level>(rows: &std::collections::HashMap<&str, &str>) {
        let key = hex(rows["A"]);
        let j: u16 = rows["j"].parse().unwrap();
        let scalar = unsalted::<L>(&key, rows["message"].as_bytes(), j, L::CHALLENGE_BITS).unwrap();
        assert_eq!(prism_signer::decimal(&scalar), rows["s"]);
        assert!(prism_verify::degree_is_prime(L::E - 2, &scalar));
        let mut sig = Vec::new();
        for name in ["E", "Px", "Py", "Qx", "Qy"] {
            sig.extend(hex(rows[name]));
        }
        sig.extend(j.to_le_bytes());
        assert_eq!(
            prism_verify::verify::<L>(&key, rows["message"].as_bytes(), &sig),
            Err(prism_verify::VerifyError::Length)
        );
        #[cfg(feature = "experiments")]
        assert_eq!(
            verify_unsalted::<L>(&key, rows["message"].as_bytes(), &sig, L::CHALLENGE_BITS),
            Ok(())
        );
    }
    #[test]
    fn archived_hashes_match_every_original_and_default_verifier_rejects_old_wire() {
        for p in files() {
            let text = std::fs::read_to_string(p).unwrap();
            let rows: std::collections::HashMap<_, _> =
                text.lines().filter_map(|s| s.split_once(' ')).collect();
            match rows["level"] {
                "NIST-I" => check::<Level1>(&rows),
                "NIST-III" => check::<Level3>(&rows),
                "NIST-V" => check::<Level5>(&rows),
                _ => panic!("row"),
            }
        }
    }
}
