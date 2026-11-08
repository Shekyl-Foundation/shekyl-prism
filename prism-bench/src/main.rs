// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Time the verification walk at NIST I, III, and V.
//!
//! The level III line is the consensus gate, and it is meaningful on the
//! floor device (Raspberry Pi 4 Model B). Levels I and V are the same run.
//! The counter is the userspace timer, named below: `cntvct_el0` on aarch64
//! and `rdtsc` on x86_64. It is not the Intel cycle count in Table 5.

use prism_field::FIELD_BACKEND;
use prism_verify::time_open;
use std::time::Instant;

fn main() {
    println!("field-backend={FIELD_BACKEND}");
    println!("counter={}", counter_name());
    let mut refused = false;
    for level in ["I", "III", "V"] {
        let before = counter();
        let start = Instant::now();
        let outcome = time_open(level);
        let elapsed = start.elapsed();
        let after = counter();
        match outcome {
            Ok((name, kind)) => println!(
                "level={name} outcome={kind} wall_ns={} counter_delta={}",
                elapsed.as_nanos(),
                after.wrapping_sub(before)
            ),
            Err(err) => {
                refused = true;
                println!(
                    "level={level} refused={err:?} wall_ns={} counter_delta={}",
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
