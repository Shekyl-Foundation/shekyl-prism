// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Every refusal of the tag layer costs no field operation.
//!
//! ```text
//! cargo test --locked -p shekyl-prism --test tag_layer_counts \
//!     --features prism-verify/op-count -- --nocapture
//! ```
//!
//! This test needs the operation counter, which is a feature of the
//! layer crates that no manifest enables. It requires the feature
//! (`required-features` in this crate's manifest), so a plain
//! `cargo test` does not build it and CI names it. With the feature off
//! it would count nothing and prove nothing.
//!
//! The inputs are built over an honest key and signature, so a tag layer
//! that went on to the bare bytes would decode field elements, search
//! for a basis and count. Two controls show the counter is live: the
//! same bytes accepted, and the same bytes refused by the verifier after
//! the tag layer.

use prism_field::ops::{self, Slot};
use prism_vectors::corpus::Primary;
use shekyl_prism::{
    verify_tagged, AcceptList, PublicKey, SetId, Signature, TaggedError, VerifyError, P248, P500,
};

/// Field operations `run` counted, in both fields, over every stage.
fn counted<T>(run: impl FnOnce() -> T) -> (T, u64) {
    let _ = ops::take();
    let value = run();
    let total = ops::take().iter().fold(0, |sum, slot: &Slot| {
        let one = |c: ops::Counts| c.mul + c.square + c.inv + c.sqrt + c.is_square;
        sum + one(slot.ext) + one(slot.base)
    });
    (value, total)
}

fn under(id: [u8; 4], bare: &[u8]) -> Vec<u8> {
    [&id[..], bare].concat()
}

#[test]
fn every_tag_layer_refusal_costs_no_field_operation() {
    let vector = Primary::Lvl1.vector();
    let (bare_key, message, bare_signature) = (
        vector.public_key().unwrap(),
        vector.message().unwrap(),
        vector.signature().unwrap(),
    );
    let p248 = SetId::of::<P248>();
    let p500 = SetId::of::<P500>();
    let key = under(p248.to_bytes(), &bare_key);
    let signature = under(p248.to_bytes(), &bare_signature);
    let nothing = [1, 0, 1, 0];
    let accept = AcceptList::new(&[p248]).unwrap();
    let other = AcceptList::new(&[p500]).unwrap();

    // Control: the counter is on and these bytes are worth counting.
    let (accepted, cost) = counted(|| verify_tagged(&accept, &key, message, &signature));
    assert_eq!(accepted, Ok(p248));
    assert!(cost > 0, "an accepted signature counted nothing");
    println!("TAG-LAYER control=accepted ops={cost}");
    // Control: a refusal after the tag layer counts too.
    let (refused, cost) = counted(|| verify_tagged(&accept, &key, b"another message", &signature));
    assert_eq!(refused, Err(TaggedError::Verify(VerifyError::Composite)));
    assert!(cost > 0, "a refusal by the verifier counted nothing");
    println!("TAG-LAYER control=refused-by-the-verifier ops={cost}");

    let mut cases: Vec<(String, Result<SetId, TaggedError>)> = Vec::new();
    let mut zero = |name: String, run: &dyn Fn() -> Result<SetId, TaggedError>| {
        let (result, cost) = counted(run);
        assert!(
            !matches!(result, Ok(_) | Err(TaggedError::Verify(_))),
            "{name}: not a refusal of the tag layer: {result:?}"
        );
        assert_eq!(cost, 0, "{name}: {result:?} cost field operations");
        println!("TAG-LAYER case={name} ops={cost}");
        cases.push((name, result));
    };

    for len in 0..4 {
        zero(format!("key-of-{len}-bytes"), &|| {
            verify_tagged(&accept, &key[..len], message, &signature)
        });
        zero(format!("signature-of-{len}-bytes"), &|| {
            verify_tagged(&accept, &key, message, &signature[..len])
        });
    }
    zero("key-names-no-set".into(), &|| {
        verify_tagged(&accept, &under(nothing, &bare_key), message, &signature)
    });
    zero("signature-names-no-set".into(), &|| {
        verify_tagged(&accept, &key, message, &under(nothing, &bare_signature))
    });
    zero("identifiers-differ".into(), &|| {
        let both = AcceptList::new(&[p248, p500]).unwrap();
        verify_tagged(
            &both,
            &key,
            message,
            &under(p500.to_bytes(), &bare_signature),
        )
    });
    // The whole honest input, under a list that does not hold its set.
    zero("valid-and-not-in-the-list".into(), &|| {
        verify_tagged(&other, &key, message, &signature)
    });
    // The typed readers' tag layer, as an identifier or a refusal.
    zero("typed-key-of-another-set".into(), &|| {
        PublicKey::<P500>::from_tagged(&key).map(|_| p500)
    });
    zero("typed-signature-of-another-set".into(), &|| {
        Signature::<P500>::from_tagged(&signature).map(|_| p500)
    });
    zero("typed-key-names-no-set".into(), &|| {
        PublicKey::<P248>::from_tagged(&under(nothing, &bare_key)).map(|_| p248)
    });
    zero("typed-signature-names-no-set".into(), &|| {
        Signature::<P248>::from_tagged(&under(nothing, &bare_signature)).map(|_| p248)
    });
    zero("typed-key-of-3-bytes".into(), &|| {
        PublicKey::<P248>::from_tagged(&key[..3]).map(|_| p248)
    });
    zero("typed-signature-of-3-bytes".into(), &|| {
        Signature::<P248>::from_tagged(&signature[..3]).map(|_| p248)
    });

    // Eight truncations, two unknown, a mismatch, the list, six typed.
    assert_eq!(cases.len(), 18);
    // Every refusal of the tag layer was met.
    let met = |wanted: fn(&TaggedError) -> bool| {
        cases
            .iter()
            .any(|(_, result)| result.as_ref().is_err_and(wanted))
    };
    assert!(met(|e| matches!(e, TaggedError::TooShort { .. })));
    assert!(met(|e| matches!(e, TaggedError::UnknownSet { .. })));
    assert!(met(|e| matches!(e, TaggedError::Mismatch { .. })));
    assert!(met(|e| matches!(e, TaggedError::WrongSet { .. })));
    assert!(met(|e| matches!(e, TaggedError::NotAccepted(_))));
}
