// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Generate ephemeral research keys, sign, and verify. No secrets are printed.
use prism_signer::{SecretSeed, Signer};
use prism_verify::{verify_set, Set};
use std::{path::PathBuf, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: roundtrip <backend-directory>")?,
    );
    for (set, name) in [
        (Set::P248, "lvl1"),
        (Set::P376, "lvl3"),
        (Set::P500, "lvl5"),
    ] {
        let signer = Signer::new(
            dir.join(format!("prism_signer_{name}")),
            set,
            SecretSeed::generate()?,
        );
        let before = Instant::now();
        let key = signer.public_key()?;
        let keygen = before.elapsed();
        let before = Instant::now();
        let sig = signer.sign(b"shekyl-prism-roundtrip")?;
        let sign = before.elapsed();
        let before = Instant::now();
        verify_set(set, &key, b"shekyl-prism-roundtrip", &sig)?;
        let verify = before.elapsed();
        println!("set={set} pk_bytes={} sig_bytes={} keygen_ms={} sign_including_keygen_ms={} verify_ms={} accepted=true",key.len(),sig.len(),keygen.as_millis(),sign.as_millis(),verify.as_millis());
    }
    Ok(())
}
