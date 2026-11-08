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
        let before = counter();
        let start = Instant::now();
        let outcome = verify_level(level, &public_key, message, &signature);
        let elapsed = start.elapsed();
        let after = counter();
        match outcome {
            Err(VerifyError::Relation) => println!(
                "level={name} outcome=relation wall_ns={} counter_delta={}",
                elapsed.as_nanos(),
                after.wrapping_sub(before)
            ),
            other => {
                refused = true;
                println!(
                    "level={level} refused={other:?} wall_ns={} counter_delta={}",
                    elapsed.as_nanos(),
                    after.wrapping_sub(before)
                );
            }
        }
    }
    if refused {
        std::process::exit(1);
    }
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
