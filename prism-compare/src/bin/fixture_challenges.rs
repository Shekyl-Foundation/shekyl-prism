// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Public test-seed driver. Never supply a production seed on the CLI.
/// Counters one `fixture_challenges` invocation prints. The regenerator
/// walks `limits::SALT_COUNTERS` in steps of this size.
const CHALLENGE_BATCH: u64 = 512;

use prism_field::{Level, Level1, Level3, Level5, Salt};
use prism_signer::limits;
use prism_verify::{ChallengeScalar, KeyIdentity};
fn hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok())
        .collect()
}
fn encode(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}
fn run<L: Level>(key: &[u8], seed: &[u8; limits::STREAM_LEN], start: u64) -> Option<()> {
    let id = KeyIdentity::<L>::decode(key).ok()?;
    let randomness = [0u8; limits::CALLER_RANDOMNESS_LEN];
    for counter in start..(start.saturating_add(CHALLENGE_BATCH)).min(limits::SALT_COUNTERS) {
        let salt = prism_signer::salt_for::<L>(seed, key, b"shekyl-prism-s5", &randomness, counter);
        let s = ChallengeScalar::derive(&id, b"shekyl-prism-s5", salt);
        println!(
            "{} {} {} {}",
            counter,
            encode(salt.as_bytes()),
            encode(s.as_bytes()),
            s.degree_is_prime::<L>()
        );
    }
    Some(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 4 {
        return Err(
            "fixture_challenges <row> <public-key-hex> <PUBLIC-test-seed-hex> <counter>".into(),
        );
    }
    let key = hex(&a[1]).ok_or("key hex")?;
    let bytes = hex(&a[2]).ok_or("seed hex")?;
    let seed: [u8; limits::STREAM_LEN] = bytes.try_into().map_err(|_| "seed length")?;
    let start = a[3].parse()?;
    let ok = match a[0].as_str() {
        "NIST-I" => run::<Level1>(&key, &seed, start),
        "NIST-III" => run::<Level3>(&key, &seed, start),
        "NIST-V" => run::<Level5>(&key, &seed, start),
        _ => None,
    };
    ok.ok_or("row/key refused".into())
}
