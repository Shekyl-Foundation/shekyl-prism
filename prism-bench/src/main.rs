// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Time decode, the challenge hash, and one Baillie–PSW test at NIST I, III, and V.
//!
//! Algorithm 3 is not implemented, so this is not the floor-device gate.
//! That gate is a level III run of Algorithm 3 at `e` steps, on a Raspberry
//! Pi 4 Model B. The counter is the userspace timer, named below:
//! `cntvct_el0` on aarch64 and `rdtsc` on x86_64.

use prism_field::FIELD_BACKEND;
use prism_verify::{prepared, verify_level, VerifyError};
use std::time::Instant;

const WARMUP: usize = 5;
const SAMPLES: usize = 31;

fn main() {
    println!("field-backend={FIELD_BACKEND}");
    println!("counter={}", counter_name());
    let mut refused = false;
    for level in ["I", "III", "V"] {
        let prepared = match prepared(level) {
            Ok(value) => value,
            Err(err) => {
                refused = true;
                println!("level={level} refused={err:?}");
                continue;
            }
        };
        let name = prepared.name;
        let public_key = prepared.public_key;
        let message = prepared.message;
        let signature = prepared.signature;
        let mut warmup_failed = false;
        for _ in 0..WARMUP {
            if verify_level(level, &public_key, message, &signature) != Err(VerifyError::Relation) {
                warmup_failed = true;
                break;
            }
        }
        if warmup_failed {
            refused = true;
            println!("level={level} refused during warm-up");
            continue;
        }
        let mut walls = Vec::with_capacity(SAMPLES);
        let mut counters = Vec::with_capacity(SAMPLES);
        let mut failed = false;
        for _ in 0..SAMPLES {
            let before = counter();
            let start = Instant::now();
            let outcome = verify_level(level, &public_key, message, &signature);
            let elapsed = start.elapsed().as_nanos();
            let delta = counter().wrapping_sub(before);
            if outcome != Err(VerifyError::Relation) {
                failed = true;
                println!("level={level} refused={outcome:?}");
                break;
            }
            walls.push(elapsed);
            counters.push(u128::from(delta));
        }
        if failed || walls.len() != SAMPLES {
            refused = true;
            continue;
        }
        let (median_ns, spread_ns) = median_spread(&mut walls);
        let (median_counter, counter_spread) = median_spread(&mut counters);
        println!(
            "level={name} outcome=relation samples={SAMPLES} median_ns={median_ns} spread_ns={spread_ns} median_counter={median_counter} counter_spread={counter_spread}"
        );
    }
    if refused {
        std::process::exit(1);
    }
}

fn median_spread(values: &mut [u128]) -> (u128, u128) {
    values.sort_unstable();
    let median = values[values.len() / 2];
    let spread = values[values.len() - 1] - values[0];
    (median, spread)
}

fn counter_name() -> &'static str {
    #[cfg(target_arch = "aarch64")]
    {
        "cntvct_el0"
    }
    #[cfg(target_arch = "x86_64")]
    {
        "rdtsc"
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        "none"
    }
}

fn counter() -> u64 {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut value: u64;
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) value);
        value
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        0
    }
}
