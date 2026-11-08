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

use std::process::ExitCode;
use std::time::Instant;

use prism_field::FIELD_BACKEND;
use prism_verify::{prepared, verify_level, Nist, PreparedError, VerifyError};

/// Runs before the clock starts, to settle the frequency and the caches.
const WARMUP: usize = 5;
/// Odd, so the median is a sample rather than a mean of two.
const SAMPLES: usize = 31;

/// The outcome every sampled run must report. Nothing in `prism-verify`
/// returns success, so a different outcome means the sample is not timing
/// the public checks.
const EXPECTED: Result<(), VerifyError> = Err(VerifyError::Relation);

fn main() -> ExitCode {
    println!("field-backend={FIELD_BACKEND}");
    println!("counter={}", counter_name());
    let mut refused = false;
    for level in Nist::ALL {
        match measure(level) {
            Ok(timing) => println!(
                "level={level} outcome=relation samples={SAMPLES} median_ns={} spread_ns={} median_counter={} counter_spread={}",
                timing.median_ns, timing.spread_ns, timing.median_counter, timing.counter_spread
            ),
            Err(err) => {
                refused = true;
                println!("level={level} refused=\"{err}\"");
            }
        }
    }
    if refused {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

struct Timing {
    median_ns: u128,
    spread_ns: u128,
    median_counter: u128,
    counter_spread: u128,
}

/// Why a level produced no timing.
enum BenchError {
    /// The sample could not be built, so there is nothing to time.
    Prepare(PreparedError),
    /// A run reported something other than [`EXPECTED`].
    Refused(VerifyError),
    /// The run reported success, which no signature may get.
    Accepted,
}

impl std::fmt::Display for BenchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepare(err) => write!(f, "{err}"),
            Self::Refused(err) => write!(f, "{err}"),
            Self::Accepted => f.write_str("a signature was accepted, which cannot happen"),
        }
    }
}

fn measure(level: Nist) -> Result<Timing, BenchError> {
    let sample = prepared(level).map_err(BenchError::Prepare)?;
    let run = || verify_level(level, &sample.public_key, sample.message, &sample.signature);
    for _ in 0..WARMUP {
        expected(run())?;
    }
    let mut walls = Vec::with_capacity(SAMPLES);
    let mut counters = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let before = counter();
        let start = Instant::now();
        let outcome = run();
        let elapsed = start.elapsed().as_nanos();
        let delta = counter().wrapping_sub(before);
        expected(outcome)?;
        walls.push(elapsed);
        counters.push(u128::from(delta));
    }
    let (median_ns, spread_ns) = median_spread(&mut walls);
    let (median_counter, counter_spread) = median_spread(&mut counters);
    Ok(Timing {
        median_ns,
        spread_ns,
        median_counter,
        counter_spread,
    })
}

fn expected(outcome: Result<(), VerifyError>) -> Result<(), BenchError> {
    match outcome {
        _ if outcome == EXPECTED => Ok(()),
        Ok(()) => Err(BenchError::Accepted),
        Err(err) => Err(BenchError::Refused(err)),
    }
}

/// Median and peak-to-peak spread of a non-empty sample.
fn median_spread(values: &mut [u128]) -> (u128, u128) {
    values.sort_unstable();
    let median = values.get(values.len() / 2).copied().unwrap_or_default();
    let low = values.first().copied().unwrap_or_default();
    let high = values.last().copied().unwrap_or_default();
    (median, high - low)
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
