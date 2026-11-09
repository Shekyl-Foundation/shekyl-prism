// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Tell `scripts/differential_prime.py` whether each challenge degree is prime.
//!
//! Stdin lines are `<exponent> <little-endian scalar hex>`. A `#` line is a
//! comment. Stdout repeats the line with `prime` or `composite`.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use prism_verify::degree_is_prime;

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let mut failed = false;
    for (number, line) in stdin.lock().lines().enumerate() {
        let Ok(line) = line else {
            eprintln!("degree-check: could not read stdin");
            return ExitCode::FAILURE;
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match judge(line) {
            Ok(verdict) => {
                if writeln!(stdout, "{line} {verdict}").is_err() {
                    eprintln!("degree-check: could not write stdout");
                    return ExitCode::FAILURE;
                }
            }
            Err(err) => {
                eprintln!("degree-check: line {}: {err}", number + 1);
                failed = true;
            }
        }
    }
    if stdout.flush().is_err() {
        return ExitCode::FAILURE;
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn judge(line: &str) -> Result<&'static str, String> {
    let mut parts = line.split_whitespace();
    let exponent = parts.next().ok_or("missing exponent")?;
    let hex = parts.next().ok_or("missing scalar")?;
    if parts.next().is_some() {
        return Err("trailing fields".to_string());
    }
    let exponent: u32 = exponent
        .parse()
        .map_err(|_| "exponent is not a u32".to_string())?;
    let scalar = decode_hex(hex)?;
    let verdict = if degree_is_prime(exponent, &scalar) {
        "prime"
    } else {
        "composite"
    };
    Ok(verdict)
}

fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return Err("scalar hex is empty".to_string());
    }
    if !bytes.len().is_multiple_of(2) {
        return Err("scalar hex has an odd length".to_string());
    }
    // Byte pairs, not `str` slices: a multibyte character is still even in
    // bytes and slicing it would panic on a char boundary.
    bytes
        .chunks_exact(2)
        .map(|pair| Ok(hex_value(pair[0])? << 4 | hex_value(pair[1])?))
        .collect()
}

fn hex_value(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("scalar hex is not hexadecimal".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::judge;

    #[test]
    fn a_multibyte_scalar_is_refused() {
        let err = judge("4 a€").expect_err("a split character is not hex");
        assert_eq!(err, "scalar hex is not hexadecimal");
    }

    #[test]
    fn one_is_a_prime_degree_at_exponent_four() {
        assert_eq!(judge("4 01").expect("1 is hex"), "prime");
    }
}
