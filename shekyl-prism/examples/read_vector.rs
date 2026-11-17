// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Read a vector file and verify what it holds.
//!
//! ```text
//! cargo run --release -p shekyl-prism --example read_vector [FILE]
//! ```
//!
//! `FILE` is one of the honest-signature files of the `prism-vectors`
//! crate (`prism-vectors/data/verify/`); without it, the first primary
//! vector is read. A vector file is lines of `name value`; the reader is
//! `prism_vectors::corpus::Vector`. The file names its parameter set by
//! a label, and the set is chosen from it here: the typed interface takes
//! the set as a type, so a caller that learns it at run time matches on
//! what it learned.

use std::path::PathBuf;

use prism_vectors::corpus::{Primary, Vector};
use shekyl_prism::{ParameterSet, PublicKey, Signature, VerifyError, P248, P376, P500};

fn verify<S: ParameterSet>(vector: &Vector<'_>) -> Result<Result<(), VerifyError>, String> {
    let read = |e| format!("the file is not a vector: {e}");
    let key = vector.public_key().map_err(read)?;
    let signature = vector.signature().map_err(read)?;
    let message = vector.message().map_err(read)?;
    println!("set        {}", S::NAME);
    println!("public key {} bytes", key.len());
    println!("signature  {} bytes", signature.len());
    println!("message    {} bytes", message.len());
    Ok((|| {
        let key = PublicKey::<S>::from_bytes(&key)?;
        let signature = Signature::<S>::from_bytes(&signature)?;
        key.verify(message, &signature)
    })())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map_or_else(|| Primary::Lvl1.path(), PathBuf::from);
    println!("file       {}", path.display());
    let text = std::fs::read_to_string(&path)?;
    let vector = Vector::new(&text);
    let level = vector
        .level()
        .map_err(|e| format!("the file is not a vector: {e}"))?;
    let verdict = match level {
        "NIST-I" => verify::<P248>(&vector),
        "NIST-III" => verify::<P376>(&vector),
        "NIST-V" => verify::<P500>(&vector),
        other => Err(format!("no parameter set is labelled {other}")),
    }?;
    match verdict {
        Ok(()) => println!("verdict    accepted"),
        Err(error) => println!("verdict    refused: {error}"),
    }
    Ok(())
}
