// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Verify with the typed interface: one key, kept, and several signatures.
//!
//! ```text
//! cargo run --release -p shekyl-prism --example verify
//! ```
//!
//! The inputs are the committed primary vectors of the `prism-vectors`
//! crate. The function `check` is all a caller needs: it is generic over
//! the parameter set, and each step's error says which input is at fault.

use prism_vectors::corpus::{Primary, WithCorpus};
use shekyl_prism::{ParameterSet, PublicKey, Signature, VerifyError, P248, P376, P500};

/// Which step refused, and why.
#[derive(Debug)]
enum Refused {
    Key(VerifyError),
    Signature(VerifyError),
    Verification(VerifyError),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Key(error) => write!(f, "not a key: {error}"),
            Self::Signature(error) => write!(f, "not a signature: {error}"),
            Self::Verification(error) => write!(f, "refused: {error}"),
        }
    }
}

/// The typed path, start to finish.
fn check<S: ParameterSet>(key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), Refused> {
    // The key's checks, the basis search among them, run once, here.
    let key = PublicKey::<S>::from_bytes(key).map_err(Refused::Key)?;
    // The signature's checks run here and need no key.
    let signature = Signature::<S>::from_bytes(signature).map_err(Refused::Signature)?;
    // What is left needs all three.
    key.verify(message, &signature)
        .map_err(Refused::Verification)
}

fn outcome(result: Result<(), Refused>) -> String {
    match result {
        Ok(()) => "accepted".into(),
        Err(refused) => refused.to_string(),
    }
}

fn set<S: WithCorpus>() -> Result<(), Box<dyn std::error::Error>> {
    for primary in S::PRIMARY {
        let vector = primary.vector();
        let (key, message, signature) =
            (vector.public_key()?, vector.message()?, vector.signature()?);
        println!(
            "{} {}: key {} bytes, signature {} bytes: {}",
            S::NAME,
            primary.file_name(),
            PublicKey::<S>::LEN,
            Signature::<S>::LEN,
            outcome(check::<S>(&key, message, &signature))
        );
        // The same signature under another message is still a signature,
        // and is refused by the verification.
        println!(
            "{} {}: under another message: {}",
            S::NAME,
            primary.file_name(),
            outcome(check::<S>(&key, b"another message", &signature))
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    set::<P248>()?;
    set::<P376>()?;
    set::<P500>()?;
    // A key is kept and used again: the search ran once.
    let vector = Primary::Lvl1.vector();
    let key = PublicKey::<P248>::from_bytes(&vector.public_key()?)?;
    let signature = Signature::<P248>::from_bytes(&vector.signature()?)?;
    for _ in 0..3 {
        key.verify(vector.message()?, &signature)?;
    }
    println!("p248: one key, three verifications: Ok");
    Ok(())
}
