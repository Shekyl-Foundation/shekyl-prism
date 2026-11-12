// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

use prism_signer::{SecretSeed, Signer};
use prism_verify::{verify_level, Nist};

#[test]
#[ignore = "requires the pinned native backend; set PRISM_SIGNER_BACKEND_DIR"]
fn deterministic_signing_and_message_binding_at_every_level() {
    let dir = std::path::PathBuf::from(
        std::env::var_os("PRISM_SIGNER_BACKEND_DIR").expect("set PRISM_SIGNER_BACKEND_DIR"),
    );
    for (level, name) in [(Nist::I, "lvl1"), (Nist::III, "lvl3"), (Nist::V, "lvl5")] {
        let signer = Signer::new(
            dir.join(format!("prism_signer_{name}")),
            level,
            SecretSeed::from_bytes([0x42; 48]),
        );
        let key = signer.public_key().expect("keygen");
        let message = b"shekyl-prism-native-roundtrip\0binary";
        let first = signer.sign(message).expect("sign");
        assert_eq!(verify_level(level, &key, message, &first), Ok(()));
        let again = signer.sign(message).expect("repeat sign");
        assert_eq!(first, again, "{level} must be deterministic");
        assert!(verify_level(level, &key, b"different message", &first).is_err());
        let changed = signer.sign(b"a second message").expect("second message");
        assert_eq!(
            verify_level(level, &key, b"a second message", &changed),
            Ok(())
        );
        assert_ne!(first, changed);
        let hedge = signer
            .sign_with_randomness(message, &[0x37; 32])
            .expect("hedged sign");
        assert_ne!(first, hedge, "caller randomness changes the salt");
        assert_eq!(verify_level(level, &key, message, &hedge), Ok(()));
        assert_eq!(
            hedge,
            signer
                .sign_with_randomness(message, &[0x37; 32])
                .expect("same entropy repeats")
        );
    }
}
