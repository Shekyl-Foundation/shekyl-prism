// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The research adapter against the production signer, on the real backend.
use prism_compare::native::{Native, Unsalted};
use prism_field::{Level, Level1, Level3, Level5};
use prism_signer::{SecretSeed, Signer};
use prism_verify::Nist;
use zeroize::Zeroizing;

fn agree<L: Level>(dir: &std::path::Path, nist: Nist, name: &str) {
    let backend = dir.join(name);
    let signer = Signer::new(&backend, nist, SecretSeed::from_bytes([0x42; 48]));
    let native = Native::<L>::new(&backend, Zeroizing::new([0x42; 48])).expect("research keygen");
    assert_eq!(native.key, signer.public_key().expect("keygen"));
    for (message, randomness) in [
        (&b"shekyl-prism-compare-agreement"[..], [0u8; 32]),
        (&b"a second message"[..], [0x37; 32]),
    ] {
        // Same seed, message and caller randomness: the research signer at
        // the production width must release the production bytes, so only
        // the projection width separates an experiment from the protocol.
        let research = native
            .sign(message, &randomness, L::CHALLENGE_BITS)
            .expect("research sign");
        let production = signer
            .sign_report(message, &randomness)
            .expect("production sign");
        assert_eq!(research.signature, production.signature, "{nist}");
        assert_eq!(
            research.counts.hashes,
            u64::from(production.hash_candidates)
        );
        assert_eq!(research.counts.primes, production.prime_candidates);
        // The archived loop on the same backend: accepted by the archived
        // derivation, refused as a length by the default verifier.
        let Some(Unsalted::Signed { signature, .. }) =
            native.sign_unsalted(message, L::CHALLENGE_BITS)
        else {
            panic!("{nist} archived signing failed");
        };
        assert_eq!(
            prism_compare::verify_unsalted::<L>(
                &native.key,
                message,
                &signature,
                L::CHALLENGE_BITS
            ),
            Ok(())
        );
        assert!(prism_compare::verify_unsalted::<L>(
            &native.key,
            b"other",
            &signature,
            L::CHALLENGE_BITS
        )
        .is_err());
        assert_eq!(
            prism_verify::verify::<L>(&native.key, message, &signature),
            Err(prism_verify::VerifyError::Length)
        );
    }
}

#[test]
#[ignore = "requires the pinned native backend; set PRISM_SIGNER_BACKEND_DIR"]
fn research_signing_is_production_signing_at_full_width() {
    let dir = std::path::PathBuf::from(
        std::env::var_os("PRISM_SIGNER_BACKEND_DIR").expect("set PRISM_SIGNER_BACKEND_DIR"),
    );
    agree::<Level1>(&dir, Nist::I, "prism_signer_lvl1");
    agree::<Level3>(&dir, Nist::III, "prism_signer_lvl3");
    agree::<Level5>(&dir, Nist::V, "prism_signer_lvl5");
}
