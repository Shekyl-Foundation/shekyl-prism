// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Time a verification at every parameter set.
//!
//! Three kinds of row, each with its own leading words so that no script
//! can read one kind as another:
//!
//! - `set=… outcome=accepted`: the complete successful two-chain
//!   verification of a committed honest signature, at the sets that have
//!   one;
//! - `set=… outcome=refused error=…`: the verification of a committed
//!   response that passes the pairing check and is refused after it, at
//!   every set. `error=Codomain` runs both chains, so it is nearly all of
//!   the work of an accepted verification; `error=Kernel` stops at the
//!   start of the chain work. Neither is an accepted verification;
//! - `search set=…`: what the search for the salt of that refused response
//!   costs whoever sends it, per salt tried, with the salts whose challenge
//!   degree is composite kept apart from the prime ones. A prime one costs
//!   several composite ones, so one figure per attempt would depend on how
//!   many attempts a search happened to need.
//!
//! Every timed call's result is compared with the pinned one, warm-ups
//! included. A row is printed only if all of them agreed. Otherwise a line
//! without `outcome=` says what happened and the run exits non-zero.
//!
//! Only a run on an identified Raspberry Pi 4 Model B is the floor-device
//! measurement. Userspace counters are cntvct_el0 on aarch64 and rdtsc on
//! x86_64.
//!
//! After the verifications it times one extension-field multiplication,
//! squaring and inversion per set. With the operation counts of
//! `docs/benchmarks/op-counts.txt` these turn a count into an estimate for
//! the machine the run was on.

use std::fmt;
use std::process::ExitCode;
use std::time::Instant;

use prism_field::{Field, ParameterSet, Salt, FIELD_BACKEND, P248, P324, P376, P500, P664};
use prism_vectors::corpus::{Malformed as MalformedVector, Primary, WithCorpus};
use prism_vectors::refusals::{self, Malformed as MalformedRefusals, Refusal, Verdict};
use prism_verify::{verify_set, ChallengeScalar, CommittedSet, KeyIdentity, Set, VerifyError};

/// Runs before the clock starts, to settle the frequency and the caches.
const WARMUP: usize = 5;
/// Odd, so the median is a sample rather than a mean of two.
const SAMPLES: usize = 31;

/// The refusals timed at every set, in the order their rows are printed.
/// `Codomain` is the one that does the work of both chains.
const REFUSALS: [Verdict; 2] = [Verdict::Codomain, Verdict::Kernel];

/// The verifier's error for a verdict of the committed refusal vectors.
/// The vectors crate does not depend on the verifier, so it names a
/// refusal by a type of its own.
const fn error_of(verdict: Verdict) -> VerifyError {
    match verdict {
        Verdict::Kernel => VerifyError::Kernel,
        Verdict::Codomain => VerifyError::Codomain,
    }
}

/// The honest signature a set's accept row verifies: its first primary
/// vector.
const fn honest(set: CommittedSet) -> Primary {
    let [primary, _] = match set {
        CommittedSet::P248 => P248::PRIMARY,
        CommittedSet::P376 => P376::PRIMARY,
        CommittedSet::P500 => P500::PRIMARY,
    };
    primary
}

/// Call a function that is generic over the parameter set for a set named
/// at run time.
macro_rules! at_set {
    ($set:expr, $function:ident($($argument:expr),*)) => {
        match $set {
            Set::P248 => $function::<P248>($($argument),*),
            Set::P324 => $function::<P324>($($argument),*),
            Set::P376 => $function::<P376>($($argument),*),
            Set::P500 => $function::<P500>($($argument),*),
            Set::P664 => $function::<P664>($($argument),*),
        }
    };
}

fn main() -> ExitCode {
    println!("field-backend={FIELD_BACKEND}");
    println!("counter={}", counter_name());
    let mut failed = false;
    for set in CommittedSet::ALL {
        match accepted(set) {
            Ok(timing) => println!("set={set} outcome=accepted {timing}"),
            Err(err) => {
                failed = true;
                println!("set={set} refused=\"{err}\"");
            }
        }
    }
    for set in Set::ALL {
        for verdict in REFUSALS {
            let error = error_of(verdict);
            match refused(set, verdict) {
                Ok(timing) => println!("set={set} outcome=refused error={error:?} {timing}"),
                Err(err) => {
                    failed = true;
                    println!("set={set} expected={error:?} found=\"{err}\"");
                }
            }
        }
    }
    for set in Set::ALL {
        let cost = refusals::row(set.name(), Verdict::Codomain)
            .map_err(BenchError::Refusals)
            .and_then(|row| at_set!(set, salt_attempts(row)));
        match cost {
            Ok(cost) => println!("search set={set} {cost}"),
            Err(err) => {
                failed = true;
                println!("search set={set} found=\"{err}\"");
            }
        }
    }
    for set in Set::ALL {
        match field_start(set) {
            Ok(key) => at_set!(set, field_operations(set, &key)),
            Err(err) => {
                failed = true;
                println!("field set={set} refused=\"{err}\"");
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// The median and peak-to-peak spread of the samples of one row, on the
/// wall clock and on the counter.
struct Timing {
    median_ns: u128,
    spread_ns: u128,
    median_counter: u128,
    counter_spread: u128,
}

impl fmt::Display for Timing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "samples={SAMPLES} median_ns={} spread_ns={} median_counter={} counter_spread={}",
            self.median_ns, self.spread_ns, self.median_counter, self.counter_spread
        )
    }
}

/// Why a row produced no timing.
enum BenchError {
    /// The committed honest vector could not be read, so there is nothing
    /// to time.
    Vector(MalformedVector),
    /// The committed refusal vectors could not be read, or have no row
    /// for this set.
    Refusals(MalformedRefusals),
    /// The committed key of a row does not decode, so nothing was timed.
    Key(VerifyError),
    /// A timed verification did not end as pinned. This is what it
    /// returned.
    Verdict(Result<(), VerifyError>),
    /// The salt search's first prime degree was not at the committed
    /// counter. This is the first one it found in its window, if any.
    Search(Option<u64>),
}

impl fmt::Display for BenchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vector(err) => write!(f, "{err}"),
            Self::Refusals(err) => write!(f, "{err}"),
            Self::Key(err) => write!(f, "the committed key does not decode: {err}"),
            Self::Verdict(Ok(())) => f.write_str("accepted"),
            Self::Verdict(Err(err)) => write!(f, "{err}"),
            Self::Search(Some(counter)) => write!(f, "first prime degree at counter {counter}"),
            Self::Search(None) => f.write_str("no prime degree in the window"),
        }
    }
}

/// Time `run`, holding every call, warm-ups included, to `pinned`. The
/// first call that returns anything else ends the row with that value.
fn measure<T: PartialEq>(pinned: &T, run: impl Fn() -> T) -> Result<Timing, T> {
    let held = |outcome: T| {
        if outcome == *pinned {
            Ok(())
        } else {
            Err(outcome)
        }
    };
    for _ in 0..WARMUP {
        held(run())?;
    }
    let mut walls = Vec::with_capacity(SAMPLES);
    let mut counters = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let before = counter();
        let start = Instant::now();
        let outcome = run();
        let elapsed = start.elapsed().as_nanos();
        let delta = counter().wrapping_sub(before);
        held(outcome)?;
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

/// The accepted verification of the set's committed honest signature.
fn accepted(set: CommittedSet) -> Result<Timing, BenchError> {
    let sample = honest(set).vector();
    let public_key = sample.public_key().map_err(BenchError::Vector)?;
    let message = sample.message().map_err(BenchError::Vector)?;
    let signature = sample.signature().map_err(BenchError::Vector)?;
    measure(&Ok(()), || {
        verify_set(set.set(), &public_key, message, &signature)
    })
    .map_err(BenchError::Verdict)
}

/// The verification of the set's committed response that is refused with
/// `verdict`. It is not an accepted verification, and its row says so.
fn refused(set: Set, verdict: Verdict) -> Result<Timing, BenchError> {
    let row = refusals::row(set.name(), verdict).map_err(BenchError::Refusals)?;
    measure(&Err(error_of(verdict)), || {
        verify_set(set, &row.public_key, &row.message, &row.signature)
    })
    .map_err(BenchError::Verdict)
}

/// The salt counters one pass of [`salt_attempts`] tries, from zero. Every
/// committed counter is below it, and it holds several prime degrees at
/// every set.
const SEARCH_WINDOW: u64 = 1024;

/// What one tried salt costs the sender of a refused response, by whether
/// its challenge degree turned out prime.
struct SearchCost {
    /// Salts of the window whose degree is composite, and prime.
    composite: u64,
    prime: u64,
    /// The first counter of prime degree: the committed row's.
    first_prime: u64,
    /// Median over the passes of the mean time of one attempt of each kind.
    composite_ns: u128,
    prime_ns: u128,
}

impl fmt::Display for SearchCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "window={SEARCH_WINDOW} composite={} prime={} first_prime={} samples={SAMPLES} \
             composite_ns_per_attempt={} prime_ns_per_attempt={}",
            self.composite, self.prime, self.first_prime, self.composite_ns, self.prime_ns
        )
    }
}

/// What the sender of a refused response does to find its salt, one salt
/// at a time: derive the challenge and test its degree. It needs the key
/// and the message, and no secret.
///
/// Each pass tries every counter of the window and times each attempt on
/// its own, so the composite and the prime ones are summed apart. Every
/// pass, warm-ups included, is held to the committed row: its first prime
/// degree must be at the committed counter.
fn salt_attempts<L: ParameterSet>(row: &Refusal) -> Result<SearchCost, BenchError> {
    let identity = KeyIdentity::<L>::decode(&row.public_key).map_err(BenchError::Key)?;
    let mut composite_means = Vec::with_capacity(SAMPLES);
    let mut prime_means = Vec::with_capacity(SAMPLES);
    let mut counts = (0, 0);
    for pass in 0..WARMUP + SAMPLES {
        let (mut composite, mut prime) = (0u64, 0u64);
        let (mut composite_ns, mut prime_ns) = (0u128, 0u128);
        let mut first_prime = None;
        for counter in 0..SEARCH_WINDOW {
            let start = Instant::now();
            let is_prime = L::Salt::from_counter(counter).is_some_and(|salt| {
                ChallengeScalar::derive(&identity, &row.message, salt).degree_is_prime::<L>()
            });
            let elapsed = start.elapsed().as_nanos();
            if is_prime {
                first_prime.get_or_insert(counter);
                prime += 1;
                prime_ns += elapsed;
            } else {
                composite += 1;
                composite_ns += elapsed;
            }
        }
        if first_prime != Some(row.salt_counter) || composite == 0 {
            return Err(BenchError::Search(first_prime));
        }
        if pass >= WARMUP {
            composite_means.push(composite_ns / u128::from(composite));
            prime_means.push(prime_ns / u128::from(prime));
            counts = (composite, prime);
        }
    }
    let (composite_ns, _) = median_spread(&mut composite_means);
    let (prime_ns, _) = median_spread(&mut prime_means);
    Ok(SearchCost {
        composite: counts.0,
        prime: counts.1,
        first_prime: row.salt_counter,
        composite_ns,
        prime_ns,
    })
}

/// The element the field timings of a set start from: the key of the
/// set's honest signature where there is one, as in every earlier record,
/// and the key of its `Codomain` refusal where there is not.
fn field_start(set: Set) -> Result<Vec<u8>, BenchError> {
    match set.committed() {
        Ok(committed) => honest(committed)
            .vector()
            .public_key()
            .map_err(BenchError::Vector),
        Err(_) => refusals::row(set.name(), Verdict::Codomain)
            .map(|row| row.public_key.clone())
            .map_err(BenchError::Refusals),
    }
}

/// Operations in one timed batch of multiplications or squarings.
const FIELD_BATCH: u32 = 10_000;
/// Inversions in one timed batch; an inversion is some hundreds of
/// multiplications.
const INVERSION_BATCH: u32 = 200;

/// Nanoseconds and counter ticks of one application of `step`, the median
/// over the samples of a batch of them. Each batch is a dependent chain
/// from `start`, so no operation can be reordered or dropped. Generic over
/// the step, so each operation gets its own loop and is called directly:
/// the figure includes no loop but its own and no indirect call.
fn per_op<T: Copy>(start: T, batch: u32, step: impl Fn(T) -> T) -> (f64, f64) {
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
}

/// Time one extension-field multiplication, squaring and inversion,
/// starting from the coefficient [`field_start`] names.
fn field_operations<L: ParameterSet>(set: Set, key: &[u8]) {
    let Some(start) = L::ext_decode(key) else {
        println!("field set={set} refused=\"the sample key does not decode\"");
        return;
    };
    let fixed = std::hint::black_box(start);
    let report = |name: &str, batch: u32, (ns, ticks): (f64, f64)| {
        println!(
            "field set={set} op={name} batch={batch} samples={SAMPLES} ns_per_op={ns:.1} counter_per_op={ticks:.1}"
        );
    };
    let mul = per_op(start, FIELD_BATCH, |x: L::Ext| x.mul(fixed));
    report("mul", FIELD_BATCH, mul);
    let square = per_op(start, FIELD_BATCH, |x: L::Ext| x.square());
    report("square", FIELD_BATCH, square);
    // `x ↦ 1/x + c` keeps the chain off the two-cycle of a bare inverse.
    let inv = per_op(start, INVERSION_BATCH, |x: L::Ext| {
        x.inv().map_or(fixed, |y| y.add(fixed))
    });
    report("inv", INVERSION_BATCH, inv);
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
