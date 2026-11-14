// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Print the order-`2^e` basis Rust derives from `A`, for the C differential.
//!
//! `even_basis <I|III|V> <A-hex>` prints four affine x-coordinates: `P`,
//! the cleared point `R`, `Q = difference(P, R)` (the field the verifier
//! uses), and `[f]` of the difference of the uncleared points. The fourth
//! value is the rejected variant: it equals `Q` on only about half of keys.
//! `sample <I|III|V> <count>` prints that many coefficients that have a
//! basis.

use std::env;
use std::process::ExitCode;

use prism_field::{Field, Level, Level1, Level3, Level5};
use prism_isogeny::torsion::{even_basis, mul_odd_then_double};
use prism_verify::basis::basis_hint;
use sha3::{Digest, Sha3_256};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        eprintln!("usage: even_basis <I|III|V> <A-hex> | sample <I|III|V> <count>");
        return ExitCode::from(2);
    };
    if command == "sample" {
        let Some(level) = args.next() else {
            return ExitCode::from(2);
        };
        let Some(count) = args.next().and_then(|text| text.parse::<u32>().ok()) else {
            return ExitCode::from(2);
        };
        return match level.as_str() {
            "I" => sample::<Level1>(count),
            "III" => sample::<Level3>(count),
            "V" => sample::<Level5>(count),
            _ => ExitCode::from(2),
        };
    }
    let Some(coefficient) = args.next() else {
        return ExitCode::from(2);
    };
    match command.as_str() {
        "I" => show::<Level1>(&coefficient),
        "III" => show::<Level3>(&coefficient),
        "V" => show::<Level5>(&coefficient),
        _ => ExitCode::from(2),
    }
}

fn show<L: Level>(hex: &str) -> ExitCode {
    let Ok(bytes) = decode_hex(hex) else {
        eprintln!("A is not hex");
        return ExitCode::FAILURE;
    };
    let Some(coefficient) = L::ext_decode(&bytes) else {
        eprintln!("A does not decode");
        return ExitCode::FAILURE;
    };
    let Ok(hint) = basis_hint::<L>(coefficient) else {
        eprintln!("no basis index");
        return ExitCode::FAILURE;
    };
    let Some(even) = even_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square())
    else {
        eprintln!("no even basis");
        return ExitCode::FAILURE;
    };
    let Some(rejected) = mul_odd_then_double::<L>(coefficient, even.full_difference, L::ODD, 0)
    else {
        eprintln!("no cleared image of the uncleared difference");
        return ExitCode::FAILURE;
    };
    for point in [even.p, even.q, even.difference, rejected] {
        let Some(x) = point.x_affine() else {
            eprintln!("a basis point is infinite");
            return ExitCode::FAILURE;
        };
        println!("{}", hex_encode(&L::ext_encode(x)));
    }
    ExitCode::SUCCESS
}

fn sample<L: Level>(count: u32) -> ExitCode {
    let mut produced = 0u32;
    let mut counter = 0u64;
    while produced < count {
        counter += 1;
        let mut bytes = vec![0u8; L::FP2_LEN];
        let mut filled = 0;
        let mut block = 0u64;
        while filled < bytes.len() {
            let mut hasher = Sha3_256::new();
            hasher.update(L::NAME.as_bytes());
            hasher.update(counter.to_le_bytes());
            hasher.update(block.to_le_bytes());
            let digest = hasher.finalize();
            let take = (bytes.len() - filled).min(digest.len());
            bytes[filled..filled + take].copy_from_slice(&digest[..take]);
            filled += take;
            block += 1;
        }
        let Some(coefficient) = L::ext_decode(&bytes) else {
            continue;
        };
        if coefficient.square().same(L::Ext::from_u64(4)) || coefficient.is_zero() {
            continue;
        }
        if basis_hint::<L>(coefficient).is_err() {
            continue;
        }
        println!("{}", hex_encode(&L::ext_encode(coefficient)));
        produced += 1;
    }
    ExitCode::SUCCESS
}

fn decode_hex(text: &str) -> Result<Vec<u8>, ()> {
    if !text.is_ascii() || !text.len().is_multiple_of(2) {
        return Err(());
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).map_err(|_| ()))
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
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
