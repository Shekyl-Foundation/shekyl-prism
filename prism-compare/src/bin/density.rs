// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! E1: prime density of the challenge degree under both hash versions.
//!
//! `density <samples-per-set>`. For each set the key is the archived
//! primary vector's and sample `n` uses the message
//! `shekyl-prism-comparison-e1-v1 || LE64(n)`. The archived derivation is
//! evaluated at index 0 and the salted one at salt `LE64(n)` zero-padded;
//! both use the production projection and the production Baillie-PSW test.
//! Stdout: one `DENSITY` line per set and version, with the prime count
//! of each tenth of the run so the dispersion is visible. `level` is the
//! key committed comparison records match. Its value is the committed
//! label, not the set name.
use prism_compare::{archived_vectors, unsalted};
use prism_field::{Salt, P248, P376, P500};
use prism_verify::{ChallengeScalar, Committed, KeyIdentity};

fn message(n: u64) -> Vec<u8> {
    let mut m = b"shekyl-prism-comparison-e1-v1".to_vec();
    m.extend(n.to_le_bytes());
    m
}
fn set<L: Committed>(samples: u64, salted: bool) -> String {
    let key = archived_vectors()
        .into_iter()
        .find(|v| v.level == L::LABEL && !v.file.contains('/'))
        .expect("an archived primary vector with this committed label")
        .key;
    let id = KeyIdentity::<L>::decode(&key).expect("archived key");
    let mut blocks = [0u64; 10];
    for n in 0..samples {
        let m = message(n);
        let prime = if salted {
            let salt = L::Salt::from_counter(n).expect("the salt holds a counter");
            ChallengeScalar::derive(&id, &m, salt).degree_is_prime::<L>()
        } else {
            let s = unsalted::<L>(&key, &m, 0, L::CHALLENGE_BITS).expect("full width");
            prism_verify::degree_is_prime(L::E - 2, &s)
        };
        blocks[(n * 10 / samples) as usize] += u64::from(prime);
    }
    let primes: u64 = blocks.iter().sum();
    format!(
        "DENSITY level={} hash={} bits={} free_bits={} samples={samples} primes={primes} density={:.6} tenths={}",
        L::LABEL,
        if salted { "challenge-v2-salted" } else { "challenge-v1-unsalted" },
        L::CHALLENGE_BITS,
        L::CHALLENGE_BITS - 1,
        primes as f64 / samples as f64,
        blocks.map(|b| b.to_string()).join(","),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 1 {
        return Err("density <samples-per-set>".into());
    }
    let samples: u64 = a[0].parse()?;
    if samples < 10 {
        return Err("at least 10 samples".into());
    }
    let jobs: Vec<std::thread::JoinHandle<String>> = vec![
        std::thread::spawn(move || set::<P248>(samples, false)),
        std::thread::spawn(move || set::<P248>(samples, true)),
        std::thread::spawn(move || set::<P376>(samples, false)),
        std::thread::spawn(move || set::<P376>(samples, true)),
        std::thread::spawn(move || set::<P500>(samples, false)),
        std::thread::spawn(move || set::<P500>(samples, true)),
    ];
    for job in jobs {
        println!("{}", job.join().map_err(|_| "a density worker panicked")?);
    }
    Ok(())
}
