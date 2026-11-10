// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The first prime challenge degrees at `e′ = e − 2`, without a signature.
//!
//! Each sample is one decoded public key and one message. The key bytes are
//! a fixed permutation of the sample number, retried until the key decoder
//! accepts them, so a run is the same run again. The message is
//! `shekyl-prism-small-extension` and the sample number. The degree is the
//! smallest prime `q_j` from [`smallest_at_exponent`] at exponent `e − 2`.
//!
//! One argument is the sample count per level. Stdout is
//! `<level> <exponent> <j> <s hex> <q hex>`, little-endian.

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use prism_field::{Level, Level1, Level3, Level5};
use prism_verify::{
    challenge_degree, smallest_at_exponent, ChallengeScalar, IndexError, KeyIdentity,
};

const MESSAGE_PREFIX: &[u8] = b"shekyl-prism-small-extension";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let count = match args.next() {
        Some(text) => match text.parse::<u64>() {
            Ok(count) if count > 0 => count,
            _ => {
                eprintln!("sample_e_prime: the count is a positive integer");
                return ExitCode::FAILURE;
            }
        },
        None => {
            eprintln!("sample_e_prime: pass the sample count per level");
            return ExitCode::FAILURE;
        }
    };
    if args.next().is_some() {
        eprintln!("sample_e_prime: one argument, the sample count");
        return ExitCode::FAILURE;
    }
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    if sample_level::<Level1>(count, &mut out).is_err()
        || sample_level::<Level3>(count, &mut out).is_err()
        || sample_level::<Level5>(count, &mut out).is_err()
    {
        return ExitCode::FAILURE;
    }
    if out.flush().is_err() {
        eprintln!("sample_e_prime: could not write stdout");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn sample_level<L: Level>(count: u64, out: &mut impl Write) -> Result<(), ()> {
    let mut draw = 0u64;
    let mut produced = 0u64;
    while produced < count {
        let Some(key) = key_for::<L>(draw) else {
            draw = draw.checked_add(1).ok_or_else(exhausted)?;
            continue;
        };
        let message = message_for(produced);
        match smallest_at_exponent::<L>(&key, &message, L::E - 2) {
            Ok((index, scalar)) => {
                write_sample::<L>(produced, &index.to_string(), &scalar, out)?;
                produced += 1;
                draw = draw.checked_add(1).ok_or_else(exhausted)?;
                eprintln!("{name} {produced}/{count}", name = L::NAME);
            }
            Err(IndexError::Exhausted) => {
                // This key and message have no prime degree in the index
                // range. The next draw is a different key.
                draw = draw.checked_add(1).ok_or_else(exhausted)?;
            }
            Err(IndexError::Identity(err)) => {
                eprintln!("sample_e_prime: a key that decoded was refused: {err}");
                return Err(());
            }
        }
    }
    Ok(())
}

fn write_sample<L: Level>(
    produced: u64,
    index: &str,
    scalar: &ChallengeScalar,
    out: &mut impl Write,
) -> Result<(), ()> {
    let degree = challenge_degree(L::E - 2, scalar.as_bytes()).ok_or_else(|| {
        eprintln!("sample_e_prime: {produced} named a prime the degree constructor refuses");
    })?;
    writeln!(
        out,
        "{name} {exponent} {index} {scalar} {degree}",
        name = L::NAME,
        exponent = L::E - 2,
        scalar = hex(scalar.as_bytes()),
        degree = hex(&degree),
    )
    .map_err(|_| eprintln!("sample_e_prime: could not write stdout"))
}

/// Key bytes from `draw`, or `None` when this level's decoder refuses them.
fn key_for<L: Level>(draw: u64) -> Option<Vec<u8>> {
    let mut state = draw.wrapping_add(1);
    let mut bytes = vec![0u8; L::PUBLIC_KEY_LEN];
    for byte in &mut bytes {
        // A fixed 64-bit mixer. The same `draw` is the same key.
        state = state
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0xBF58_476D_1CE4_E5B9);
        *byte = (state >> 56) as u8;
    }
    KeyIdentity::<L>::decode(&bytes).ok()?;
    Some(bytes)
}

fn message_for(produced: u64) -> Vec<u8> {
    let mut message = MESSAGE_PREFIX.to_vec();
    message.extend_from_slice(&produced.to_le_bytes());
    message
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0xf) as usize] as char);
    }
    out
}

fn exhausted() {
    eprintln!("sample_e_prime: the key draw overflowed");
}
