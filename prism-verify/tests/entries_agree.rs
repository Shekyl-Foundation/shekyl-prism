// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The two entries agree: the cases the corpus does not reach.
//!
//! `support/entries.rs` states the rule and holds it on one input, and
//! every test that verifies goes through it. `acceptance.rs` walks the
//! 113 honest signatures and the negatives derived from them and pins
//! their counts; `no_corpus_refusals.rs` builds every refusal that needs
//! no signer, at all five sets; `identity_refusals.rs` reads the
//! committed identity refusals.
//!
//! This file pins the committed identity refusals as a family, and builds
//! what no committed input is: a faulty key together with a faulty
//! signature, the one case where the entries may name different
//! refusals.

#[path = "support/entries.rs"]
mod entries;

use entries::{both, both_at, Tally};
use prism_field::{Field, ParameterSet, P248, P324, P376, P500, P664};
use prism_vectors::corpus::Primary;
use prism_verify::{Set, VerifyError};

#[test]
fn the_identity_refusals_are_refused_alike_by_both_entries() {
    let mut tally = Tally::default();
    let rows = prism_vectors::refusals::rows().expect("the committed file parses");
    for row in rows {
        let set: Set = row.set.parse().expect("the row's set");
        tally.add(both_at(set, &row.public_key, &row.message, &row.signature));
    }
    assert_eq!(
        tally,
        Tally {
            inputs: 2 * Set::ALL.len(),
            accepted: 0,
            key_refused: 0,
            different: 0
        }
    );
}

/// A key and a signature that are both faulty, in every pairing of the
/// faults that need no signer, at one set.
///
/// The typed path names the key's fault. The byte-level entry names
/// `Length` when the signature's length is wrong, whatever the key is,
/// and otherwise decodes the key first and names the key's fault too. So
/// the entries differ exactly where the key's fault is not `Length` and
/// the signature's is.
///
/// A key refused as `Basis` would differ from every earlier fault of the
/// signature as well. No such key is known: the search fails with
/// probability about `2^-63` (`prism_verify::search_bound`).
fn both_faulty<L: ParameterSet>() -> Tally {
    let keys = [
        (vec![1u8; L::PUBLIC_KEY_LEN - 1], VerifyError::Length),
        (vec![0xffu8; L::PUBLIC_KEY_LEN], VerifyError::Canonical),
        (vec![0u8; L::PUBLIC_KEY_LEN], VerifyError::Origin),
        (L::ext_encode(L::Ext::from_u64(2)), VerifyError::Singular),
    ];
    let signatures = [
        (vec![0u8; L::SIGNATURE_LEN - 1], VerifyError::Length),
        (vec![0xffu8; L::SIGNATURE_LEN], VerifyError::Canonical),
        // Every element zero: `y(P) = 0` is not a normalized sign.
        (vec![0u8; L::SIGNATURE_LEN], VerifyError::Sign),
    ];
    let mut tally = Tally::default();
    for (key, key_fault) in &keys {
        for (signature, signature_fault) in &signatures {
            let outcome = both::<L>(key, b"both faulty", signature);
            assert!(outcome.key_refused);
            assert_eq!(outcome.typed, Err(*key_fault), "{}", L::NAME);
            let byte = if *signature_fault == VerifyError::Length {
                VerifyError::Length
            } else {
                *key_fault
            };
            assert_eq!(outcome.byte, Err(byte), "{}", L::NAME);
            tally.add(outcome);
        }
    }
    tally
}

#[test]
fn a_faulty_key_with_a_faulty_signature_is_where_the_entries_may_differ() {
    for tally in [
        both_faulty::<P248>(),
        both_faulty::<P324>(),
        both_faulty::<P376>(),
        both_faulty::<P500>(),
        both_faulty::<P664>(),
    ] {
        assert_eq!(
            tally,
            Tally {
                inputs: 12,
                accepted: 0,
                key_refused: 12,
                // `Canonical`, `Origin` and `Singular` keys, each with the
                // signature of the wrong length.
                different: 3
            }
        );
    }
}

/// With a valid signature, each faulty key is refused alike: the one
/// fault is the key's, and both entries name it.
#[test]
fn a_faulty_key_alone_is_refused_alike() {
    let vector = Primary::Lvl1.vector();
    let message = vector.message().expect("the vector's message").to_vec();
    let signature = vector.signature().expect("the vector's signature");
    for (key, fault) in [
        (vec![1u8; P248::PUBLIC_KEY_LEN - 1], VerifyError::Length),
        (vec![0xffu8; P248::PUBLIC_KEY_LEN], VerifyError::Canonical),
        (vec![0u8; P248::PUBLIC_KEY_LEN], VerifyError::Origin),
        (
            P248::ext_encode(<P248 as ParameterSet>::Ext::from_u64(2)),
            VerifyError::Singular,
        ),
    ] {
        let outcome = both::<P248>(&key, &message, &signature);
        assert!(outcome.key_refused);
        assert_eq!(outcome.byte, Err(fault));
        assert_eq!(outcome.typed, Err(fault));
    }
}
