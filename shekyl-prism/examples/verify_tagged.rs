// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Verify tagged bytes under an accept list: the set is read from the
//! bytes, and the list says which sets this program accepts.
//!
//! ```text
//! cargo run --release -p shekyl-prism --example verify_tagged
//! ```
//!
//! The inputs are committed primary vectors of the `prism-vectors`
//! crate, written in the tagged form by the typed interface. The
//! function `check` is all a caller needs.

use prism_vectors::corpus::{Primary, WithCorpus};
use shekyl_prism::{
    verify_tagged, AcceptList, PublicKey, SetId, Signature, TaggedError, P248, P500,
};

/// A key, a message and a signature as they would arrive: the key and
/// the signature each with their set's identifier in front.
struct Received {
    public_key: Vec<u8>,
    message: &'static [u8],
    signature: Vec<u8>,
}

/// One committed vector, tagged by the typed writers.
fn received<S: WithCorpus>(primary: Primary) -> Result<Received, Box<dyn std::error::Error>> {
    let vector = primary.vector();
    Ok(Received {
        public_key: PublicKey::<S>::from_bytes(&vector.public_key()?)?.to_tagged(),
        message: vector.message()?,
        signature: Signature::<S>::from_bytes(&vector.signature()?)?.to_tagged(),
    })
}

/// The tagged path, start to finish. The policy is the caller's list.
fn check(accept: &AcceptList, received: &Received) -> Result<SetId, TaggedError> {
    verify_tagged(
        accept,
        &received.public_key,
        received.message,
        &received.signature,
    )
}

fn outcome(result: Result<SetId, TaggedError>) -> String {
    match result {
        Ok(set) => format!("accepted under {set}"),
        Err(refused) => format!("refused: {refused}"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This program accepts one set, and says so here.
    let accept = AcceptList::new(&[SetId::of::<P500>()])?;
    println!("the list: {accept:?}");

    let at_p500 = received::<P500>(Primary::Lvl5)?;
    println!(
        "a p500 key of {} bytes and signature of {} bytes: {}",
        at_p500.public_key.len(),
        at_p500.signature.len(),
        outcome(check(&accept, &at_p500))
    );

    // An honest signature under a set the list does not hold is refused
    // by the list, before any arithmetic.
    let at_p248 = received::<P248>(Primary::Lvl1)?;
    println!(
        "a p248 key of {} bytes and signature of {} bytes: {}",
        at_p248.public_key.len(),
        at_p248.signature.len(),
        outcome(check(&accept, &at_p248))
    );

    // A list that names both accepts both, and is as strong as p248.
    let both = AcceptList::new(&[SetId::of::<P248>(), SetId::of::<P500>()])?;
    println!(
        "the same p248 input under {both:?}: {}",
        outcome(check(&both, &at_p248))
    );

    // A key of one set with a signature of another is refused as such.
    let mixed = Received {
        public_key: at_p500.public_key.clone(),
        message: at_p500.message,
        signature: at_p248.signature.clone(),
    };
    println!(
        "a p500 key with a p248 signature: {}",
        outcome(check(&both, &mixed))
    );
    Ok(())
}
