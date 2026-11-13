// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! E6 and the signer half of E4: full-width signing of the same messages
//! under the archived index protocol and the salted protocol.
//!
//! `unsignable <backend-dir> <level 1|3|5> <first> <count> [key]`. Message
//! `n` is `shekyl-prism-comparison-e6-v1 || LE64(n)` and caller randomness
//! is zero. Without `key` the seed is the public test seed (0x42 repeated).
//! With it, the seed is the first 48 bytes of
//! `SHAKE256("shekyl-prism-comparison-e6-key-v1" || LE64(key))`, and a `KEY`
//! line states the seed and the public key it gives. One `SIGN` line per
//! message records both signers' candidate counts. A message the archived
//! signer cannot sign is retried twice more to show the same failure, and
//! its `UNSIGNABLE` line carries the salted outcome for the same message.
use prism_compare::{
    encode,
    native::{self, Unsalted},
};
use prism_field::{Level, Level1, Level3, Level5};
use prism_signer::{limits, SecretSeed};
use sha3::{
    digest::{ExtendableOutput, Update, XofReader},
    Shake256,
};

/// The seed of a run: the public test seed, or the public seed of key `key`.
fn seed_bytes(key: Option<u64>) -> [u8; limits::STREAM_LEN] {
    let mut bytes = [0x42; limits::STREAM_LEN];
    if let Some(key) = key {
        let mut h = Shake256::default();
        h.update(b"shekyl-prism-comparison-e6-key-v1");
        h.update(&key.to_le_bytes());
        h.finalize_xof().read(&mut bytes);
    }
    bytes
}

fn message(n: u64) -> Vec<u8> {
    let mut m = b"shekyl-prism-comparison-e6-v1".to_vec();
    m.extend(n.to_le_bytes());
    m
}
fn describe(outcome: &Unsalted) -> String {
    match outcome {
        Unsalted::Signed { index, skipped, .. } => {
            format!(
                "signed index={index} hashes={} skipped_primes={skipped}",
                u32::from(*index) + 1
            )
        }
        Unsalted::Backend { index } => format!("backend-error index={index}"),
        Unsalted::Malformed { index } => format!("malformed index={index}"),
        Unsalted::Exhausted => "exhausted".to_string(),
    }
}
fn run<L: Level>(
    dir: &std::path::Path,
    name: &str,
    first: u64,
    count: u64,
    key: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = seed_bytes(key);
    let seed = SecretSeed::from_bytes(bytes);
    let session = native::open::<L>(dir.join(name), &seed)?;
    if let Some(key) = key {
        println!(
            "KEY level={} key={key} seed={} public={}",
            L::NAME,
            encode(&bytes),
            encode(session.public_key())
        );
    }
    let randomness = [0u8; limits::CALLER_RANDOMNESS_LEN];
    let mut unsignable = 0u64;
    for n in first..first + count {
        let m = message(n);
        let old = native::sign_unsalted(&session, &m, L::CHALLENGE_BITS).ok_or("width")?;
        let new = native::sign(&session, &m, &randomness, L::CHALLENGE_BITS);
        let salted = match &new {
            Ok(s) => {
                if prism_verify::verify::<L>(session.public_key(), &m, &s.signature).is_err() {
                    return Err("the default verifier refused a salted signature".into());
                }
                format!(
                    "signed hashes={} primes={} backend_errors={} malformed={} rejections={}",
                    s.report.hash_candidates,
                    s.report.prime_candidates,
                    s.report.backend_errors,
                    s.report.malformed_responses,
                    s.report.verification_rejections
                )
            }
            Err(e) => format!("failed {e}").replace(' ', "-"),
        };
        println!(
            "SIGN level={} n={n} unsalted={} salted={salted}",
            L::NAME,
            describe(&old).replace(' ', ";"),
        );
        if !matches!(old, Unsalted::Signed { .. }) {
            unsignable += 1;
            let again = native::sign_unsalted(&session, &m, L::CHALLENGE_BITS).ok_or("width")?;
            let third = native::sign_unsalted(&session, &m, L::CHALLENGE_BITS).ok_or("width")?;
            println!(
                "UNSIGNABLE level={} n={n} message={} first={} second={} third={} same_on_retry={} salted={salted}",
                L::NAME,
                encode(&m),
                describe(&old).replace(' ', ";"),
                describe(&again).replace(' ', ";"),
                describe(&third).replace(' ', ";"),
                old == again && old == third,
            );
        }
    }
    println!(
        "TRIED level={} first={first} count={count} unsignable={unsignable}",
        L::NAME
    );
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if !(4..=5).contains(&a.len()) {
        return Err("unsignable <backend-dir> <level 1|3|5> <first> <count> [key]".into());
    }
    let key = a.get(4).map(|k| k.parse()).transpose()?;
    let dir = std::path::PathBuf::from(&a[0]);
    let (first, count) = (a[2].parse()?, a[3].parse()?);
    match a[1].as_str() {
        "1" => run::<Level1>(&dir, "prism_signer_lvl1", first, count, key),
        "3" => run::<Level3>(&dir, "prism_signer_lvl3", first, count, key),
        "5" => run::<Level5>(&dir, "prism_signer_lvl5", first, count, key),
        _ => Err("level is 1, 3 or 5".into()),
    }
}
