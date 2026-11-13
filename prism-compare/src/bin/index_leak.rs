// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! E5: what the carried index of each archived signature shows.
//!
//! No arguments. For every archived unsalted vector, public data alone
//! (key, message, index) gives the number of earlier indices whose degree
//! is prime. The archived signer took the first prime index unless its own
//! final verification refused the response, so a nonzero count is a
//! secret-dependent event made public. Stdout: one `INDEX` line per vector
//! and one `SKIPPED` line per observed count.
use prism_compare::{archived_vectors, unsalted};
use prism_field::{Level, Level1, Level3, Level5};
use std::collections::BTreeMap;

fn earlier_primes<L: Level>(key: &[u8], message: &[u8], index: u16) -> Option<u32> {
    let carried = unsalted::<L>(key, message, index, L::CHALLENGE_BITS)?;
    if !prism_verify::degree_is_prime(L::E - 2, &carried) {
        return None;
    }
    let mut primes = 0;
    for earlier in 0..index {
        let s = unsalted::<L>(key, message, earlier, L::CHALLENGE_BITS)?;
        primes += u32::from(prism_verify::degree_is_prime(L::E - 2, &s));
    }
    Some(primes)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vectors = archived_vectors();
    let mut skipped = BTreeMap::new();
    for v in &vectors {
        let primes = match v.level.as_str() {
            "NIST-I" => earlier_primes::<Level1>(&v.key, &v.message, v.index),
            "NIST-III" => earlier_primes::<Level3>(&v.key, &v.message, v.index),
            "NIST-V" => earlier_primes::<Level5>(&v.key, &v.message, v.index),
            _ => None,
        }
        .ok_or("an archived vector's carried index is not prime")?;
        println!(
            "INDEX file={} level={} carried_index={} earlier_composite={} earlier_prime={primes}",
            v.file,
            v.level,
            v.index,
            u32::from(v.index) - primes
        );
        *skipped.entry(primes).or_insert(0u32) += 1;
    }
    for (count, vectors) in &skipped {
        println!("SKIPPED earlier_prime={count} vectors={vectors}");
    }
    println!("TOTAL vectors={}", vectors.len());
    Ok(())
}
