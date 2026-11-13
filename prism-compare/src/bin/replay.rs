// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! E3: real-backend collision replay and one-signed-target second-preimage
//! study. The only weakened parameter is the public challenge projection
//! width. Every acceptance below is the real algebraic verifier's.
//!
//! `replay <backend-dir> <trial> <width|all|ci>`; one `RESULT` JSON line per
//! width on stdout. `ci` runs widths 16 and 20. The exit status is nonzero
//! when the unsalted replay or the reduced-width second preimage is not
//! accepted, or when the default full-width verifier accepts either forgery.
use prism_compare::{
    encode,
    native::{verify_salted, Native, Unsalted},
};
use prism_field::{Level, Level1, Salt};
use sha3::{
    digest::{core_api::CoreWrapper, ExtendableOutput, Update, XofReader},
    CShake256, CShake256Core,
};
use std::{collections::HashMap, path::PathBuf, time::Instant};
use zeroize::Zeroizing;

type L = Level1;
struct Template {
    base: CShake256,
    bits: u32,
    legacy: bool,
    prefix: Vec<u8>,
}
impl Template {
    fn new(key: &[u8], salt: Option<&[u8]>, trial: u64, purpose: u8, bits: u32) -> Self {
        let legacy = salt.is_none();
        let domain = if legacy {
            b"shekyl-prism-challenge-v1"
        } else {
            b"shekyl-prism-challenge-v2"
        };
        let mut base: CShake256 = CoreWrapper::from_core(CShake256Core::new(domain));
        base.update(key);
        if let Some(s) = salt {
            base.update(s);
        }
        let mut prefix = b"shekyl-prism-comparison-e3-v1".to_vec();
        prefix.extend(trial.to_le_bytes());
        prefix.push(purpose);
        base.update(&prefix);
        Self {
            base,
            bits,
            legacy,
            prefix,
        }
    }
    fn value(&self, counter: u64) -> u32 {
        let mut h = self.base.clone();
        h.update(&counter.to_le_bytes());
        if self.legacy {
            h.update(&[0, 0]);
        }
        let mut out = [0; 4];
        h.finalize_xof()
            .read(&mut out[..(self.bits as usize).div_ceil(8)]);
        let mask = if self.bits == 32 {
            u32::MAX
        } else {
            (1u32 << self.bits) - 1
        };
        (u32::from_le_bytes(out) & mask) | 1
    }
    fn message(&self, counter: u64) -> Vec<u8> {
        let mut m = self.prefix.clone();
        m.extend(counter.to_le_bytes());
        m
    }
    fn scalar(&self, value: u32) -> Vec<u8> {
        value.to_le_bytes()[..(self.bits as usize).div_ceil(8)].to_vec()
    }
}
struct Collision {
    first: u64,
    second: u64,
    queries: u64,
}
/// What the search knows about a projected value it has already hashed to.
enum Seen {
    /// The counter that first produced it.
    Once(u64),
    /// It was produced twice and its degree is composite, so no pair of
    /// messages on it is of use.
    Composite,
}
/// The first two counters whose projected scalars are equal and whose
/// degree is prime. `queries` is every hash evaluated, composite or not.
fn prime_collision(t: &Template) -> Result<Collision, &'static str> {
    let mut seen = HashMap::new();
    for counter in 0..u64::MAX {
        let value = t.value(counter);
        match seen.get(&value) {
            None => {
                seen.insert(value, Seen::Once(counter));
            }
            Some(Seen::Composite) => {}
            Some(&Seen::Once(first)) => {
                if prism_verify::degree_is_prime(L::E - 2, &t.scalar(value)) {
                    return Ok(Collision {
                        first,
                        second: counter,
                        queries: counter + 1,
                    });
                }
                seen.insert(value, Seen::Composite);
            }
        }
    }
    Err("collision search exhausted its counter")
}
fn run(dir: &std::path::Path, trial: u64, bits: u32) -> Result<(), Box<dyn std::error::Error>> {
    if !(16..=32).contains(&bits) {
        return Err("projection width must be 16..32".into());
    }
    let native = Native::<L>::new(dir.join("prism_signer_lvl1"), Zeroizing::new([0x42; 48]))?;
    let zero = Salt::<L>::from_bytes(&[0; L::SALT_LEN]).ok_or("salt length")?;
    // Unsalted: the attacker collides two messages offline at index 0, the
    // archived signer signs the first, and the same bytes are submitted for
    // the second. A collision the signer cannot sign costs the attacker a
    // fresh search, and those hash queries are counted.
    let started = Instant::now();
    let mut unsalted_queries = 0;
    let mut unsalted_unsignable = 0u32;
    let (m1, m2, archived) = loop {
        let old = Template::new(
            &native.key,
            None,
            trial,
            1 + 16 * unsalted_unsignable as u8,
            bits,
        );
        let collision = prime_collision(&old)?;
        unsalted_queries += collision.queries;
        let m1 = old.message(collision.first);
        match native.sign_unsalted(&m1, bits).ok_or("research width")? {
            Unsalted::Signed {
                signature,
                index: 0,
                skipped: 0,
            } => break (m1, old.message(collision.second), signature),
            _ if unsalted_unsignable < 15 => unsalted_unsignable += 1,
            _ => return Err("the archived signer refused 16 collisions".into()),
        }
    };
    if prism_compare::verify_unsalted::<L>(&native.key, &m1, &archived, bits).is_err() {
        return Err("the archived signer released a refused signature".into());
    }
    let unsalted_accepted =
        prism_compare::verify_unsalted::<L>(&native.key, &m2, &archived, bits).is_ok();
    let unsalted_seconds = started.elapsed().as_secs_f64();
    let mut full = archived[..L::SIGNATURE_ELEMENTS * L::FP2_LEN].to_vec();
    full.extend(zero.as_bytes());
    let default_accepts_unsalted = prism_verify::verify::<L>(&native.key, &m2, &archived).is_ok()
        || prism_verify::verify::<L>(&native.key, &m2, &full).is_ok();

    // Keep the salted v2 domain/framing in the precomputation too. This
    // isolates unknown signer salt from merely changing the hash version.
    let guessed = Template::new(&native.key, Some(zero.as_bytes()), trial, 2, bits);
    let guess = prime_collision(&guessed)?;
    let guessed_first = guessed.message(guess.first);
    let guessed_second = guessed.message(guess.second);
    let signed = native.sign(&guessed_first, &[0; 32], bits)?;
    let actual = prism_verify::Response::<L>::decode(&signed.signature)?;
    let replay = verify_salted::<L>(&native.key, &guessed_second, &signed.signature, bits);
    let salted_replay_ok = replay.is_ok();
    let target = u32::from_le_bytes({
        let mut b = [0; 4];
        b[..signed.scalar.len()].copy_from_slice(&signed.scalar);
        b
    });
    let search = Template::new(&native.key, Some(actual.salt.as_bytes()), trial, 3, bits);
    let started = Instant::now();
    let mut second_queries = 0;
    let forged = (0..u64::MAX)
        .find_map(|n| {
            second_queries = n + 1;
            if n > 0 && n % 100_000_000 == 0 {
                eprintln!("progress trial={trial} width={bits} queries={n}");
            }
            (search.value(n) == target).then(|| search.message(n))
        })
        .ok_or("second-preimage counter exhausted")?;
    let second_preimage_accepted =
        verify_salted::<L>(&native.key, &forged, &signed.signature, bits).is_ok();
    let seconds = started.elapsed().as_secs_f64();
    let default_accepts = default_accepts_unsalted
        || prism_verify::verify::<L>(&native.key, &forged, &signed.signature).is_ok();
    println!("RESULT {{\"trial\":{trial},\"projection_bits\":{bits},\"free_bits\":{},\"public_test_seed_repeat_byte\":66,\"key\":\"{}\",\"unsalted_queries\":{unsalted_queries},\"unsalted_unsignable_collisions\":{unsalted_unsignable},\"unsalted_accepted\":{unsalted_accepted},\"unsalted_seconds\":{unsalted_seconds},\"unsalted_message1\":\"{}\",\"unsalted_message2\":\"{}\",\"unsalted_signature\":\"{}\",\"guessed_salt_queries\":{},\"guessed_salt\":\"{}\",\"actual_salt\":\"{}\",\"salted_precomputed_replay_accepted\":{salted_replay_ok},\"salted_signing_hashes\":{},\"salted_prime_candidates\":{},\"salted_backend_errors\":{},\"salted_malformed\":{},\"salted_rejections\":{},\"second_preimage_queries\":{second_queries},\"second_preimage_accepted\":{second_preimage_accepted},\"second_preimage_seconds\":{seconds},\"signed_message\":\"{}\",\"forged_message\":\"{}\",\"salted_signature\":\"{}\",\"default_verifier_accepted\":{default_accepts}}}",
        bits-1,encode(&native.key),encode(&m1),encode(&m2),encode(&archived),guess.queries,encode(zero.as_bytes()),encode(actual.salt.as_bytes()),signed.counts.hashes,signed.counts.primes,signed.counts.backend_errors,signed.counts.malformed,signed.counts.rejections,encode(&guessed_first),encode(&forged),encode(&signed.signature));
    if !unsalted_accepted {
        return Err("the unsalted collision replay was refused".into());
    }
    if !second_preimage_accepted {
        return Err("the reduced-width second preimage was refused".into());
    }
    if default_accepts {
        return Err("the default full-width verifier accepted a reduced-width forgery".into());
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 3 {
        return Err("replay <backend-dir> <trial-number> <width|all|ci>".into());
    }
    let dir = PathBuf::from(&a[0]);
    let trial = a[1].parse()?;
    if a[2] == "all" || a[2] == "ci" {
        let widths: &[u32] = if a[2] == "ci" {
            &[16, 20]
        } else {
            &[16, 20, 24, 28, 32]
        };
        for bits in widths {
            run(&dir, trial, *bits)?;
        }
    } else {
        run(&dir, trial, a[2].parse()?)?;
    }
    Ok(())
}
