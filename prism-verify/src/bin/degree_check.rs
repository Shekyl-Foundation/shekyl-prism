// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Tell `scripts/differential_prime.py` whether each challenge degree is prime.
//!
//! Stdin lines are `<set> <exponent> <little-endian scalar hex>`. A `#` line
//! is a comment. Stdout repeats the line with `prime` or `composite`.
//!
//! The set names the width of the integers the test runs on, which is the
//! width the verifier uses for that set. The exponent is free: the script
//! sends each set its own chain exponent, and a table of small known
//! degrees to every set.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
use prism_verify::{degree_is_prime, Set};

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
    let set: Set = parts
        .next()
        .ok_or("missing set")?
        .parse()
        .map_err(|_| "the set is not one this build has".to_string())?;
    let exponent = parts.next().ok_or("missing exponent")?;
    let hex = parts.next().ok_or("missing scalar")?;
    if parts.next().is_some() {
        return Err("trailing fields".to_string());
    }
    let exponent: u32 = exponent
        .parse()
        .map_err(|_| "exponent is not a u32".to_string())?;
    let scalar = decode_hex(hex)?;
    // One arm for each set, at that set's width. No set is reached by a
    // default.
    let prime = match set {
        Set::P248 => at::<P248>(exponent, &scalar),
        Set::P324 => at::<P324>(exponent, &scalar),
        Set::P376 => at::<P376>(exponent, &scalar),
        Set::P500 => at::<P500>(exponent, &scalar),
        Set::P664 => at::<P664>(exponent, &scalar),
    };
    let verdict = if prime { "prime" } else { "composite" };
    Ok(verdict)
}

/// The verdict at the width of `L`.
fn at<L: ParameterSet>(exponent: u32, scalar: &[u8]) -> bool {
    degree_is_prime::<L::Degree>(exponent, scalar)
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
    use prism_verify::Set;

    #[test]
    fn a_multibyte_scalar_is_refused() {
        let err = judge("p248 4 a€").expect_err("a split character is not hex");
        assert_eq!(err, "scalar hex is not hexadecimal");
    }

    /// `2^7 − 1 = 127` is prime, and it is at every set: the set names a
    /// width, and the verdict does not depend on the width.
    #[test]
    fn one_is_a_prime_degree_at_exponent_four_at_every_set() {
        for set in Set::ALL {
            let line = format!("{set} 4 01");
            assert_eq!(judge(&line).expect("1 is hex"), "prime", "{set}");
        }
    }

    #[test]
    fn a_line_without_a_set_this_build_has_is_refused() {
        for line in ["4 01", "p999 4 01", "P248 4 01"] {
            let err = judge(line).expect_err("no such set");
            assert_eq!(err, "the set is not one this build has", "{line}");
        }
        assert_eq!(judge("p248 4").expect_err("no scalar"), "missing scalar");
        assert_eq!(
            judge("p248 4 01 01").expect_err("too many"),
            "trailing fields"
        );
    }
}
