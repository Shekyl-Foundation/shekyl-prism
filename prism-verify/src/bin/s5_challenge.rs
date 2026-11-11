// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The challenge scalar the verifier derives, for the Sage accept model.
//!
//! `s5_challenge <I|III|V> <A-hex> <message> <index>` prints the
//! little-endian scalar and `prime` or `composite`. The scalar is
//! [`ChallengeScalar::derive_width`] at `e′ − 1` bits, and the degree is
//! the verifier's own Baillie–PSW test. The model shells out to this so a
//! negative's `s` is the verifier's `s`, not a second hash.

use std::env;
use std::process::ExitCode;

use prism_field::{ChallengeIndex, Level, Level1, Level3, Level5};
use prism_verify::{ChallengeScalar, KeyIdentity};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let (level, coefficient, message, index) =
        match (args.next(), args.next(), args.next(), args.next()) {
            (Some(level), Some(coefficient), Some(message), Some(index)) => {
                (level, coefficient, message, index)
            }
            _ => {
                eprintln!("usage: s5_challenge <I|III|V> <A-hex> <message> <index>");
                return ExitCode::from(2);
            }
        };
    if args.next().is_some() {
        eprintln!("s5_challenge: trailing arguments");
        return ExitCode::from(2);
    }
    let Ok(index) = index.parse::<u16>() else {
        eprintln!("s5_challenge: index is a u16");
        return ExitCode::from(2);
    };
    match level.as_str() {
        "I" | "NIST-I" => emit::<Level1>(&coefficient, &message, index),
        "III" | "NIST-III" => emit::<Level3>(&coefficient, &message, index),
        "V" | "NIST-V" => emit::<Level5>(&coefficient, &message, index),
        _ => {
            eprintln!("level is I, III, or V");
            ExitCode::from(2)
        }
    }
}

fn emit<L: Level>(coefficient: &str, message: &str, index: u16) -> ExitCode {
    let Ok(key) = decode_hex(coefficient) else {
        eprintln!("s5_challenge: A is hex");
        return ExitCode::from(2);
    };
    if key.len() != L::PUBLIC_KEY_LEN {
        eprintln!("s5_challenge: A has the wrong length");
        return ExitCode::from(2);
    }
    let Ok(identity) = KeyIdentity::<L>::decode(&key) else {
        eprintln!("s5_challenge: A is not a key this level accepts");
        return ExitCode::from(1);
    };
    let scalar = ChallengeScalar::derive_width::<L>(
        &identity,
        message.as_bytes(),
        ChallengeIndex::from_canonical_bytes(index.to_le_bytes()),
        L::E - 3,
    );
    let verdict = if scalar.degree_is_prime::<L>() {
        "prime"
    } else {
        "composite"
    };
    let hex: String = scalar
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    println!("{hex} {verdict}");
    ExitCode::SUCCESS
}

fn decode_hex(text: &str) -> Result<Vec<u8>, ()> {
    if !text.len().is_multiple_of(2) {
        return Err(());
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| ()))
        .collect()
}
