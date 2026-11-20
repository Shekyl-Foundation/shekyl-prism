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
//! signature, and a key whose named points are no basis. Those are the
//! cases where the entries may name different refusals: whenever the key
//! is refused, and only then.

#[path = "support/entries.rs"]
mod entries;

use entries::{both, both_at, Tally};
use prism_field::{Field, ParameterSet, P248, P324, P376, P500, P664};
use prism_vectors::corpus::Primary;
use prism_verify::{set::Set, VerifyError};

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
/// and otherwise decodes the key first and names the key's fault too,
/// unless that fault is `Basis`: it reaches the key's basis only after
/// the signature's own checks, so it names the signature's fault. So the
/// entries differ where the key's fault is not `Length` and the
/// signature's is, and on every signature fault under a `Basis` key.
///
/// The `Basis` key is `1 + i`, whose curve does not have its
/// `2^e`-torsion rational at any of the five sets. A key refused as
/// `Basis` because the search finds no index is not known: that fails
/// with probability at most about `2^-128` (the documentation of
/// `prism_verify::basis`, "The search bound").
fn both_faulty<L: ParameterSet>() -> Tally {
    let keys = [
        (vec![1u8; L::PUBLIC_KEY_LEN - 1], VerifyError::Length),
        (vec![0xffu8; L::PUBLIC_KEY_LEN], VerifyError::Canonical),
        (vec![0u8; L::PUBLIC_KEY_LEN], VerifyError::Origin),
        (L::ext_encode(L::Ext::from_u64(2)), VerifyError::Singular),
        (
            L::ext_encode(L::Ext::from_u64(1).add(L::ext_i())),
            VerifyError::Basis,
        ),
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
            } else if *key_fault == VerifyError::Basis {
                *signature_fault
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
                inputs: 15,
                accepted: 0,
                key_refused: 15,
                // `Canonical`, `Origin`, `Singular` and `Basis` keys, each
                // with the signature of the wrong length, and the `Basis`
                // key with the other two.
                different: 6
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

/// The keys `a + i`, `a` from 1 to 199, each with a committed signature
/// of the set that is good in itself, and what the two entries say.
///
/// These coefficients are curves without rational `2^e`-torsion: they
/// decode, the search finds an index, and the points it names are no
/// basis. `PublicKey::from_bytes` refuses such a key as `Basis`. The
/// byte-level entry refuses it too, and names what it reaches first: the
/// challenge is hashed from the key, so nearly always `Composite`, and
/// `Kernel` on the few whose degree is prime.
fn keys_without_a_basis<L: ParameterSet>(set: Set) -> (Tally, usize) {
    let rows = prism_vectors::refusals::rows().expect("the committed file parses");
    let row = rows
        .iter()
        .find(|row| row.set.parse::<Set>().expect("the row's set") == set)
        .expect("a committed refusal at every set");
    let mut tally = Tally::default();
    let mut kernel = 0;
    for real in 1u64..=199 {
        let key = L::ext_encode(L::Ext::from_u64(real).add(L::ext_i()));
        let outcome = both::<L>(&key, &row.message, &row.signature);
        if outcome.key_refused {
            assert_eq!(outcome.typed, Err(VerifyError::Basis), "{}", L::NAME);
            match outcome.byte {
                Err(VerifyError::Composite) => {}
                Err(VerifyError::Kernel) => kernel += 1,
                other => panic!("{}: {real} + i: {other:?}", L::NAME),
            }
        }
        tally.add(outcome);
    }
    (tally, kernel)
}

/// All 199 are refused as keys at every set, each under another name by
/// the byte-level entry. The second number is how many of them the
/// byte-level entry carries as far as `Kernel`: those whose challenge
/// degree under the committed salt happens to be prime.
#[test]
fn a_key_whose_points_are_no_basis_is_refused_by_both_entries() {
    let all = || Tally {
        inputs: 199,
        accepted: 0,
        key_refused: 199,
        different: 199,
    };
    assert_eq!(keys_without_a_basis::<P248>(Set::P248), (all(), 2));
    assert_eq!(keys_without_a_basis::<P324>(Set::P324), (all(), 1));
    assert_eq!(keys_without_a_basis::<P376>(Set::P376), (all(), 0));
    assert_eq!(keys_without_a_basis::<P500>(Set::P500), (all(), 0));
    assert_eq!(keys_without_a_basis::<P664>(Set::P664), (all(), 0));
}
