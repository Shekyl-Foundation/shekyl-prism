// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The challenge scalar the verifier derives, for the Sage accept model.
//!
//! `s5_challenge <I|III|V> <A-hex> <message> <salt-hex>` prints the
//! little-endian scalar and `prime` or `composite`. The scalar is
//! [`ChallengeScalar::derive`] at `e′ − 1` bits, and the degree is
//! the verifier's own Baillie–PSW test. The model shells out to this so a
//! negative's `s` is the verifier's `s`, not a second hash.

use std::env;
use std::process::ExitCode;

use prism_field::{Level, Level1, Level3, Level5, Salt};
use prism_verify::{ChallengeScalar, KeyIdentity};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let (level, coefficient, message, salt_hex) =
        match (args.next(), args.next(), args.next(), args.next()) {
            (Some(level), Some(coefficient), Some(message), Some(salt_hex)) => {
                (level, coefficient, message, salt_hex)
            }
            _ => {
                eprintln!("usage: s5_challenge <I|III|V> <A-hex> <message> <salt-hex>");
                return ExitCode::from(2);
            }
        };
    if args.next().is_some() {
        eprintln!("s5_challenge: trailing arguments");
        return ExitCode::from(2);
    }
    match level.as_str() {
        "I" | "NIST-I" => emit::<Level1>(&coefficient, &message, &salt_hex),
        "III" | "NIST-III" => emit::<Level3>(&coefficient, &message, &salt_hex),
        "V" | "NIST-V" => emit::<Level5>(&coefficient, &message, &salt_hex),
        _ => {
            eprintln!("level is I, III, or V");
            ExitCode::from(2)
        }
    }
}

fn emit<L: Level>(coefficient: &str, message: &str, salt_hex: &str) -> ExitCode {
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
    let Ok(salt_bytes) = decode_hex(salt_hex) else {
        return ExitCode::from(2);
    };
    let Some(salt) = Salt::<L>::from_bytes(&salt_bytes) else {
        return ExitCode::from(2);
    };
    let scalar = ChallengeScalar::derive(&identity, message.as_bytes(), salt);
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
    if !text.is_ascii() || !text.len().is_multiple_of(2) {
        return Err(());
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

#[cfg(test)]
mod hex_input_tests {
    #[test]
    fn malformed_hex_is_refused_without_slicing_a_character() {
        for input in ["0é0", "a€", "é", "f", "gg"] {
            assert!(super::decode_hex(input).is_err(), "{input:?}");
        }
        assert_eq!(super::decode_hex("00aAfF"), Ok(vec![0, 170, 255]));
    }
}
