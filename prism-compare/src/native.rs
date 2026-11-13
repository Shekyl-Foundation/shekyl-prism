// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Research policies on the production signing session.
//! Reduced width and the archived index loop stay here. The salt KDF,
//! native process, and candidate budget stay in `prism-signer`.

use prism_field::{Level, Salt};
use prism_signer::{limits, Attempt, SecretSeed, Session, SigningReport};
use prism_verify::{ChallengeScalar, KeyIdentity};
use std::path::Path;

/// A reduced-width signature and the report of the search that found it.
pub struct Signed {
    pub signature: Vec<u8>,
    pub scalar: Vec<u8>,
    pub report: SigningReport,
}

/// What the archived index signer did with one message.
#[derive(Debug, PartialEq, Eq)]
pub enum Unsalted {
    /// `signature` is the archived wire: five elements and the LE16 index.
    /// `skipped` counts earlier prime indices whose response was refused.
    Signed {
        signature: Vec<u8>,
        index: u16,
        skipped: u32,
    },
    /// The native backend failed at this prime index. The archived signer
    /// returned the error; the same streams fail again on every retry.
    Backend {
        index: u16,
    },
    Malformed {
        index: u16,
    },
    /// More than [`limits::PRIME_CANDIDATES`] prime indices, or none accepted.
    Exhausted,
}

/// Open the production session. The caller keeps `seed` alive.
pub fn open<'a, L: Level>(
    program: impl AsRef<Path>,
    seed: &'a SecretSeed,
) -> Result<Session<'a, L>, prism_signer::Error> {
    Session::<L>::open(program, seed)
}

/// Salted signing at projection width `bits`. Full width matches [`Session::sign`].
pub fn sign<L: Level>(
    session: &Session<L>,
    message: &[u8],
    randomness: &[u8; limits::CALLER_RANDOMNESS_LEN],
    bits: u32,
) -> Result<Signed, prism_signer::Error> {
    if bits == 0 || bits > L::CHALLENGE_BITS {
        return Err(prism_signer::Error::Malformed);
    }
    let public_key = session.public_key().to_vec();
    let identity =
        KeyIdentity::<L>::decode(&public_key).map_err(|_| prism_signer::Error::Malformed)?;
    let report = session.search(
        message,
        randomness,
        |salt| {
            let scalar = ChallengeScalar::derive_width(&identity, message, salt, bits)
                .expect("the width was checked before the search");
            if scalar.degree_is_prime::<L>() {
                Attempt::Prime(scalar.as_bytes().to_vec())
            } else {
                Attempt::Composite
            }
        },
        |signature| {
            prism_verify::research::verify_width::<L>(&public_key, message, signature, bits).is_ok()
        },
    )?;
    let scalar = {
        let response = prism_verify::Response::<L>::decode(&report.signature)
            .map_err(|_| prism_signer::Error::Malformed)?;
        ChallengeScalar::derive_width(&identity, message, response.salt, bits)
            .ok_or(prism_signer::Error::Malformed)?
            .as_bytes()
            .to_vec()
    };
    Ok(Signed {
        signature: report.signature.clone(),
        scalar,
        report,
    })
}

/// The archived signing loop of `archive/unsalted-index-2026-11-12`
/// (`prism-signer` `sign_level`), on the same backend and the same
/// scalar-keyed stream: ascending index, first prime degree, a backend
/// or malformed result ends the call, a refused response moves on.
pub fn sign_unsalted<L: Level>(
    session: &Session<L>,
    message: &[u8],
    bits: u32,
) -> Option<Unsalted> {
    let zero = L::Salt::zero();
    let prefix = L::SIGNATURE_ELEMENTS * L::FP2_LEN;
    let mut primes = 0u32;
    for index in 0..=u16::MAX {
        let scalar = super::unsalted::<L>(session.public_key(), message, index, bits)?;
        if !prism_verify::degree_is_prime(L::E - 2, &scalar) {
            continue;
        }
        primes += 1;
        if primes > limits::PRIME_CANDIDATES {
            return Some(Unsalted::Exhausted);
        }
        let mut signature = match session.candidate(&scalar, zero) {
            Ok(signature) => signature,
            Err(prism_signer::CandidateError::Backend) => return Some(Unsalted::Backend { index }),
            Err(prism_signer::CandidateError::Malformed) => {
                return Some(Unsalted::Malformed { index })
            }
        };
        signature.truncate(prefix);
        signature.extend(index.to_le_bytes());
        if super::verify_unsalted::<L>(session.public_key(), message, &signature, bits).is_ok() {
            return Some(Unsalted::Signed {
                signature,
                index,
                skipped: primes - 1,
            });
        }
    }
    Some(Unsalted::Exhausted)
}
