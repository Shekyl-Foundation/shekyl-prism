// Copyright (c) 2026, The Shekyl Foundation
// All rights reserved.
// BSD-3-Clause

use quorum_approval_experiment::{fixed_pair_digest, Collector, Payment, Policy};
use shekyl_prism::{verify_tagged, AcceptList, SetId, P248, P324, P376, P500, P664};
use std::{fs, path::Path};

#[test]
fn verified_replacement_and_binding() {
    for set in [
        SetId::of::<P248>(),
        SetId::of::<P376>(),
        SetId::of::<P500>(),
    ] {
        let dir = Path::new("fixtures").join(set.name());
        let enrollment = fs::read(dir.join("enrollment.bin")).unwrap();
        let key_len = set.tagged_public_key_len();
        let keys: [&[u8]; 3] = enrollment
            .chunks_exact(key_len)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        assert_eq!(enrollment.len(), 3 * key_len);
        let policy = Policy::enroll(set, keys).unwrap();
        assert!(Policy::enroll(set, [keys[0], keys[1], keys[0]]).is_err());
        let payment = Payment::fixture();
        let frames = [0, 1, 2].map(|i| fs::read(dir.join(format!("{i}.bin"))).unwrap());
        for (first, second) in [(0, 1), (0, 2), (1, 2)] {
            let mut collector = Collector::new(&policy, &payment);
            assert!(collector.admit(&frames[second]).unwrap());
            assert!(!collector.admit(&frames[second]).unwrap());
            assert!(collector.witness().is_none());
            assert!(collector.admit(&frames[first]).unwrap());
            let witness = collector.witness().unwrap();
            assert_eq!(
                witness,
                [frames[first].as_slice(), &frames[second]].concat()
            );
            policy.verify_witness(&payment, &witness).unwrap();
        }
        let digest = payment.digest(&policy);
        let fixed = fs::read(dir.join("alice-fixed-ab.sig")).unwrap();
        let accept = AcceptList::new(&[set]).unwrap();
        verify_tagged(
            &accept,
            keys[0],
            &fixed_pair_digest(&digest, [0, 1]),
            &fixed,
        )
        .unwrap();
        assert!(verify_tagged(
            &accept,
            keys[0],
            &fixed_pair_digest(&digest, [0, 2]),
            &fixed
        )
        .is_err());
        let [alice, bob, carol] = &frames;
        let mut collector = Collector::new(&policy, &payment);
        assert!(collector.admit(alice).unwrap());
        assert!(collector.admit(&bob[..bob.len() / 2]).is_err());
        let mut relabelled = alice.clone();
        relabelled[0] = 1;
        assert!(collector.admit(&relabelled).is_err());
        let mut corrupt = bob.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(collector.admit(&corrupt).is_err());
        assert!(collector.witness().is_none());
        assert!(collector.admit(carol).unwrap());
        let witness = collector.witness().unwrap();
        for invalid in [
            [alice.as_slice(), alice].concat(),
            [carol.as_slice(), alice].concat(),
            [alice.as_slice(), bob, carol].concat(),
            witness[..witness.len() - 1].to_vec(),
        ] {
            assert!(policy.verify_witness(&payment, &invalid).is_err());
        }
        // Each otherwise valid witness is tied to the authorized effects and the policy.
        for changed in [
            Payment {
                network: [9; 32],
                ..payment.clone()
            },
            Payment {
                input: [9; 32],
                ..payment.clone()
            },
            Payment {
                recipient: [9; 32],
                ..payment.clone()
            },
            Payment {
                amount: 969,
                ..payment.clone()
            },
            Payment {
                fee: 31,
                ..payment.clone()
            },
            Payment {
                expiry_height: 12_346,
                ..payment.clone()
            },
        ] {
            assert!(policy.verify_witness(&changed, &witness).is_err());
        }
        // Keep the same revealed keys and slots, change only the absent member.
        let mut altered_key = keys[1].to_vec();
        altered_key[key_len - 1] ^= 1;
        let replacement = Policy::enroll(set, [keys[0], &altered_key, keys[2]]).unwrap();
        assert!(replacement.verify_witness(&payment, &witness).is_err());
        let ab = [alice.as_slice(), bob].concat();
        assert_ne!(ab, witness);
        println!(
            "{}: all three coalitions verified; one approval {} B, witness {} B",
            set.name(),
            alice.len(),
            witness.len()
        );
    }
}

#[test]
fn verifier_refuses_synthetic_approvals_at_all_five_sets() {
    for set in [
        SetId::of::<P248>(),
        SetId::of::<P324>(),
        SetId::of::<P376>(),
        SetId::of::<P500>(),
        SetId::of::<P664>(),
    ] {
        let keys = [0u8, 1, 2].map(|n| {
            let mut key = vec![0; set.tagged_public_key_len()];
            key[..4].copy_from_slice(&set.to_bytes());
            key[4] = n;
            key
        });
        let policy = Policy::enroll(set, [&keys[0], &keys[1], &keys[2]]).unwrap();
        let mut collector = Collector::new(&policy, &Payment::fixture());
        let mut syntactic = vec![0];
        syntactic.extend(&keys[0]);
        syntactic.extend(set.to_bytes());
        syntactic.resize(policy.approval_len(), 0);
        assert_eq!(collector.admit(&syntactic), Err("invalid signature"));
        assert!(collector.witness().is_none());
    }
}
