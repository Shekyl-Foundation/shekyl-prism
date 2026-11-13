// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The research adapter against the production signer, on the real backend.
use prism_compare::native::{self, Unsalted};
use prism_field::{Level, Level1, Level3, Level5};
use prism_signer::{limits, SecretSeed, Signer};
use prism_verify::Nist;

fn agree<L: Level>(dir: &std::path::Path, nist: Nist, name: &str) {
    let backend = dir.join(name);
    let seed = SecretSeed::from_bytes([0x42; limits::STREAM_LEN]);
    let signer = Signer::new(
        &backend,
        nist,
        SecretSeed::from_bytes([0x42; limits::STREAM_LEN]),
    );
    let session = native::open::<L>(&backend, &seed).expect("research keygen");
    assert_eq!(session.public_key(), signer.public_key().expect("keygen"));
    let randomnesses: [([u8; limits::CALLER_RANDOMNESS_LEN], &[u8]); 2] = [
        (
            [0u8; limits::CALLER_RANDOMNESS_LEN],
            b"shekyl-prism-compare-agreement",
        ),
        ([0x37; limits::CALLER_RANDOMNESS_LEN], b"a second message"),
    ];
    for (randomness, message) in randomnesses {
        // Same seed, message, caller randomness, and full width: the
        // research projection must release the production bytes.
        let research =
            native::sign(&session, message, &randomness, L::CHALLENGE_BITS).expect("research sign");
        let production = signer
            .sign_report(message, &randomness)
            .expect("production sign");
        assert_eq!(research.signature, production.signature, "{nist}");
        assert_eq!(research.report.hash_candidates, production.hash_candidates);
        assert_eq!(
            research.report.prime_candidates,
            production.prime_candidates
        );
        assert_eq!(research.report.backend_errors, production.backend_errors);
        assert_eq!(
            research.report.malformed_responses,
            production.malformed_responses
        );
        assert_eq!(
            research.report.verification_rejections,
            production.verification_rejections
        );
        // The archived loop on the same backend: accepted by the archived
        // derivation, refused as a length by the default verifier.
        let Some(Unsalted::Signed { signature, .. }) =
            native::sign_unsalted(&session, message, L::CHALLENGE_BITS)
        else {
            panic!("{nist} archived signing failed");
        };
        assert_eq!(
            prism_compare::verify_unsalted::<L>(
                session.public_key(),
                message,
                &signature,
                L::CHALLENGE_BITS
            ),
            Ok(())
        );
        assert!(prism_compare::verify_unsalted::<L>(
            session.public_key(),
            b"other",
            &signature,
            L::CHALLENGE_BITS
        )
        .is_err());
        assert_eq!(
            prism_verify::verify::<L>(session.public_key(), message, &signature),
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
