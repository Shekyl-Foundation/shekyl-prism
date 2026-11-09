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
    if !text.len().is_multiple_of(2) {
        return Err("scalar hex has an odd length".to_string());
    }
    if text.is_empty() {
        return Err("scalar hex is empty".to_string());
    }
    (0..text.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&text[index..index + 2], 16)
                .map_err(|_| "scalar hex is not hexadecimal".to_string())
        })
        .collect()
}
