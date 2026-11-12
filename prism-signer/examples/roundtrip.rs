// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Generate ephemeral research keys, sign, and verify. No secrets are printed.
use prism_signer::{SecretSeed, Signer};
use prism_verify::{verify_level, Nist};
use std::{path::PathBuf, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: roundtrip <backend-directory>")?,
    );
    for (level, name) in [(Nist::I, "lvl1"), (Nist::III, "lvl3"), (Nist::V, "lvl5")] {
        let signer = Signer::new(
            dir.join(format!("prism_signer_{name}")),
            level,
            SecretSeed::generate()?,
        );
        let before = Instant::now();
        let key = signer.public_key()?;
        let keygen = before.elapsed();
        let before = Instant::now();
        let sig = signer.sign(b"shekyl-prism-roundtrip")?;
        let sign = before.elapsed();
        let before = Instant::now();
        verify_level(level, &key, b"shekyl-prism-roundtrip", &sig)?;
        let verify = before.elapsed();
        println!("level={level} pk_bytes={} sig_bytes={} keygen_ms={} sign_including_keygen_ms={} verify_ms={} accepted=true",key.len(),sig.len(),keygen.as_millis(),sign.as_millis(),verify.as_millis());
    }
    Ok(())
}
