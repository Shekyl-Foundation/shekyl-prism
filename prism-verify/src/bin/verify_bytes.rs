// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Streaming differential driver. Each line is level, key hex, message hex
//! (use '-' for empty), signature hex. Outputs exactly one verdict per line.
use prism_verify::{verify_level, Nist};
use std::io::{self, BufRead};
fn hex(s: &str) -> Option<Vec<u8>> {
    if s == "-" {
        return Some(Vec::new());
    }
    if !s.len().is_multiple_of(2) {
        return None;
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok())
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let line = line?;
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 4 {
            return Err("expected level key message signature".into());
        }
        let level = fields[0].parse::<Nist>()?;
        let pk = hex(fields[1]).ok_or("key hex")?;
        let msg = hex(fields[2]).ok_or("message hex")?;
        let sig = hex(fields[3]).ok_or("signature hex")?;
        match verify_level(level, &pk, &msg, &sig) {
            Ok(()) => println!("Ok"),
            Err(err) => println!("{err:?}"),
        }
    }
    Ok(())
}
