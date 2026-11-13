// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Time a verification at NIST I, III, and V.
//!
//! Times the complete successful two-chain verification path. Only a run
//! on an identified Raspberry Pi 4 Model B is the floor-device measurement.
//! Userspace counters are cntvct_el0 on aarch64 and rdtsc on x86_64.
//!
//! After the verifications it times one extension-field multiplication,
//! squaring and inversion per level. With the operation counts of
//! `docs/benchmarks/op-counts.txt` these turn a count into an estimate for
//! the machine the run was on.

use std::process::ExitCode;
use std::time::Instant;

use prism_field::{Field, Level, Level1, Level3, Level5, FIELD_BACKEND};
use prism_verify::{reference::honest, verify_level, Nist, VectorError, VerifyError};

/// Runs before the clock starts, to settle the frequency and the caches.
const WARMUP: usize = 5;
/// Odd, so the median is a sample rather than a mean of two.
const SAMPLES: usize = 31;

// Every timed invocation must accept the honest signature.

fn main() -> ExitCode {
    println!("field-backend={FIELD_BACKEND}");
    println!("counter={}", counter_name());
    let mut refused = false;
    for level in Nist::ALL {
        match measure(level) {
            Ok(timing) => println!(
                "level={level} outcome=accepted samples={SAMPLES} median_ns={} spread_ns={} median_counter={} counter_spread={}",
                timing.median_ns, timing.spread_ns, timing.median_counter, timing.counter_spread
            ),
            Err(err) => {
                refused = true;
                println!("level={level} refused=\"{err}\"");
            }
        }
    }
    for level in Nist::ALL {
        let Ok(sample) = honest(level) else {
            continue;
        };
        match level {
            Nist::I => field_operations::<Level1>(level, &sample.public_key),
            Nist::III => field_operations::<Level3>(level, &sample.public_key),
            Nist::V => field_operations::<Level5>(level, &sample.public_key),
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
    Prepare(VectorError),
    /// A timed invocation refused the honest signature.
    Refused(VerifyError),
}

impl std::fmt::Display for BenchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepare(err) => write!(f, "{err}"),
            Self::Refused(err) => write!(f, "{err}"),
        }
    }
}

fn measure(level: Nist) -> Result<Timing, BenchError> {
    let sample = honest(level).map_err(BenchError::Prepare)?;
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

/// Operations in one timed batch of multiplications or squarings.
const FIELD_BATCH: u32 = 10_000;
/// Inversions in one timed batch; an inversion is some hundreds of
/// multiplications.
const INVERSION_BATCH: u32 = 200;

/// Time one extension-field operation, starting from the sample key's
/// coefficient. Each batch is a dependent chain, so no operation can be
/// reordered or dropped, and the figure includes no loop but its own.
fn field_operations<L: Level>(level: Nist, key: &[u8]) {
    let Some(start) = L::ext_decode(key) else {
        println!("field level={level} refused=\"the sample key does not decode\"");
        return;
    };
    let per_op = |batch: u32, step: &dyn Fn(L::Ext) -> L::Ext| {
        let mut walls = Vec::with_capacity(SAMPLES);
        let mut counters = Vec::with_capacity(SAMPLES);
        for sample in 0..WARMUP + SAMPLES {
            let mut x = std::hint::black_box(start);
            let before = counter();
            let clock = Instant::now();
            for _ in 0..batch {
                x = step(x);
            }
            let elapsed = clock.elapsed().as_nanos();
            let delta = counter().wrapping_sub(before);
            std::hint::black_box(x);
            if sample >= WARMUP {
                walls.push(elapsed);
                counters.push(u128::from(delta));
            }
        }
        let (ns, _) = median_spread(&mut walls);
        let (ticks, _) = median_spread(&mut counters);
        let batch = f64::from(batch);
        (ns as f64 / batch, ticks as f64 / batch)
    };
    let fixed = std::hint::black_box(start);
    for (name, batch, step) in [
        (
            "mul",
            FIELD_BATCH,
            &(|x: L::Ext| x.mul(fixed)) as &dyn Fn(L::Ext) -> L::Ext,
        ),
        ("square", FIELD_BATCH, &|x: L::Ext| x.square()),
        // `x ↦ 1/x + c` keeps the chain off the two-cycle of a bare inverse.
        ("inv", INVERSION_BATCH, &|x: L::Ext| {
            x.inv().map_or(fixed, |y| y.add(fixed))
        }),
    ] {
        let (ns, ticks) = per_op(batch, step);
        println!(
            "field level={level} op={name} batch={batch} samples={SAMPLES} ns_per_op={ns:.1} counter_per_op={ticks:.1}"
        );
    }
}

fn expected(outcome: Result<(), VerifyError>) -> Result<(), BenchError> {
    outcome.map_err(BenchError::Refused)
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

/// `cntvct_el0` and `rdtsc` have no safe wrapper. This is the crate's only
/// `unsafe_code` allow; the workspace lint is `forbid` everywhere else.
#[allow(unsafe_code)]
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
