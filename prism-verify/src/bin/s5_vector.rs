// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Public challenge probe for the vector generator; no secret inputs.
use prism_field::{ParameterSet, Salt, P248, P324, P376, P500, P664};
use prism_verify::Set;
use prism_verify::{ChallengeScalar, KeyIdentity};
fn hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok())
        .collect()
}
fn emit<L: ParameterSet>(key: &str, salt: &str) -> Option<()> {
    let key = hex(key)?;
    let id = KeyIdentity::<L>::decode(&key).ok()?;
    let salt = L::Salt::from_bytes(&hex(salt)?)?;
    let s = ChallengeScalar::derive(&id, b"shekyl-prism-s5", salt);
    println!(
        "{} {}",
        s.as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        s.degree_is_prime::<L>()
    );
    Some(())
}
fn main() -> std::process::ExitCode {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 3 {
        eprintln!("s5_vector <p248|p324|p376|p500> <A-hex> <salt-hex>");
        return std::process::ExitCode::from(2);
    }
    let ok = match a[0].parse::<Set>() {
        Ok(Set::P248) => emit::<P248>(&a[1], &a[2]),
        Ok(Set::P324) => emit::<P324>(&a[1], &a[2]),
        Ok(Set::P376) => emit::<P376>(&a[1], &a[2]),
        Ok(Set::P500) => emit::<P500>(&a[1], &a[2]),
        Ok(Set::P664) => emit::<P664>(&a[1], &a[2]),
        Err(_) => None,
    };
    if ok.is_some() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(2)
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn malformed_hex_never_slices_a_character() {
        for s in ["0é0", "a€", "é", "f", "gg"] {
            assert!(super::hex(s).is_none());
        }
    }
}
