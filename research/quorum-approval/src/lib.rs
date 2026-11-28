// Copyright (c) 2026, The Shekyl Foundation
// All rights reserved.
// BSD-3-Clause

//! One-input, two-of-three research format. All fixtures and seeds are public.
#![forbid(unsafe_code)]

use sha3::{Digest, Sha3_256};
use shekyl_prism::{verify_tagged, AcceptList, SetId};

pub const MEMBERS: usize = 3;
const THRESHOLD: usize = 2;

fn hash(label: &[u8], body: &[u8]) -> [u8; 32] {
    let mut state = Sha3_256::new();
    state.update(label);
    state.update(body);
    state.finalize().into()
}

pub struct Policy {
    set: SetId,
    keys: [[u8; 32]; MEMBERS],
}

impl Policy {
    pub fn enroll(set: SetId, tagged: [&[u8]; MEMBERS]) -> Result<Self, &'static str> {
        let mut keys = [[0; 32]; MEMBERS];
        for (i, key) in tagged.into_iter().enumerate() {
            if key.len() != set.tagged_public_key_len() || !key.starts_with(&set.to_bytes()) {
                return Err("unexpected enrollment set or key length");
            }
            let digest = hash(b"prism-lab/quorum/key/v1", key);
            if keys[..i].contains(&digest) {
                return Err("duplicate enrolled key");
            }
            keys[i] = digest;
        }
        Ok(Self { set, keys })
    }

    pub fn commitment(&self) -> [u8; 32] {
        let mut policy = self.set.to_bytes().to_vec();
        policy.extend([THRESHOLD as u8, MEMBERS as u8]);
        for key in self.keys {
            policy.extend(key);
        }
        hash(b"prism-lab/quorum/policy/v1", &policy)
    }

    pub fn approval_len(&self) -> usize {
        1 + self.set.tagged_public_key_len() + self.set.tagged_signature_len()
    }

    fn verify_member(&self, msg: &[u8], frame: &[u8]) -> Result<usize, &'static str> {
        if frame.len() != self.approval_len() {
            return Err("incorrect approval length");
        }
        let slot = usize::from(frame[0]);
        let (key, signature) = frame[1..].split_at(self.set.tagged_public_key_len());
        if self.keys.get(slot) != Some(&hash(b"prism-lab/quorum/key/v1", key)) {
            return Err("member key is not committed at this position");
        }
        let accept = AcceptList::new(&[self.set]).expect("one set");
        verify_tagged(&accept, key, msg, signature).map_err(|_| "invalid signature")?;
        Ok(slot)
    }

    pub fn verify_witness(&self, payment: &Payment, witness: &[u8]) -> Result<(), &'static str> {
        let width = self.approval_len();
        if witness.len() != THRESHOLD * width {
            return Err("exactly two complete approvals required");
        }
        let (a, b) = witness.split_at(width);
        if a[0] >= b[0] {
            return Err("member positions must increase");
        }
        let digest = payment.digest(self);
        self.verify_member(&digest, a)?;
        self.verify_member(&digest, b)?;
        Ok(())
    }
}

/// Exactly one input and one recipient, denominated in integer units. No change output.
#[derive(Clone)]
pub struct Payment {
    pub network: [u8; 32],
    pub input: [u8; 32],
    pub recipient: [u8; 32],
    pub amount: u64,
    pub fee: u64,
    pub expiry_height: u64,
}

impl Payment {
    pub fn fixture() -> Self {
        Self {
            network: [1; 32],
            input: [2; 32],
            recipient: [3; 32],
            amount: 970,
            fee: 30,
            expiry_height: 12_345,
        }
    }

    /// The caller supplies the independently resolved policy of the input being spent.
    pub fn digest(&self, policy: &Policy) -> [u8; 32] {
        let mut body = Vec::new();
        body.extend(self.network);
        body.extend(self.input);
        body.extend(policy.commitment());
        body.extend(self.recipient);
        for value in [self.amount, self.fee, self.expiry_height] {
            body.extend(value.to_le_bytes());
        }
        hash(b"prism-lab/quorum/payment/v1", &body)
    }
}

/// Holds verified approvals for one payment. Redelivery of the same bytes is idempotent.
pub struct Collector<'a> {
    policy: &'a Policy,
    msg: [u8; 32],
    slots: [Option<Vec<u8>>; MEMBERS],
}

impl<'a> Collector<'a> {
    pub fn new(policy: &'a Policy, payment: &Payment) -> Self {
        Self {
            policy,
            msg: payment.digest(policy),
            slots: Default::default(),
        }
    }

    pub fn admit(&mut self, frame: &[u8]) -> Result<bool, &'static str> {
        if let Some(Some(prior)) = frame.first().and_then(|i| self.slots.get(usize::from(*i))) {
            if prior == frame {
                return Ok(false);
            }
        }
        let i = self.policy.verify_member(&self.msg, frame)?;
        if self.slots[i].is_some() {
            return Ok(false);
        }
        self.slots[i] = Some(frame.to_vec());
        Ok(true)
    }

    pub fn witness(&self) -> Option<Vec<u8>> {
        let mut approvals = self.slots.iter().flatten();
        let mut result = approvals.next()?.clone();
        result.extend(approvals.next()?);
        Some(result)
    }
}

/// Comparator for a design that commits the two signing slots before anyone approves.
pub fn fixed_pair_digest(payment_digest: &[u8; 32], pair: [u8; 2]) -> [u8; 32] {
    let mut body = payment_digest.to_vec();
    body.extend(pair);
    hash(b"prism-lab/quorum/fixed-pair/v1", &body)
}
