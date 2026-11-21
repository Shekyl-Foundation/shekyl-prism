// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Both entries on one input, held to the rule that relates them.
//!
//! The verifier has two entries. `prism_verify::verify` takes three byte
//! strings. The typed interface of the `shekyl-prism` crate makes a
//! `PublicKey`, then a `Signature`, then verifies. [`both`] runs the two
//! on the same bytes and fails the test unless:
//!
//! 1. the typed path accepts exactly when the byte-level entry accepts;
//! 2. if `PublicKey::from_bytes` succeeds, the typed path's refusal is the
//!    byte-level entry's;
//! 3. if `PublicKey::from_bytes` fails, the byte-level entry also refuses.
//!    It may name another error: it looks at both lengths and decodes the
//!    signature before it searches for the key's basis, and the typed path
//!    finishes the key first.
//!
//! It also runs the tagged entry, `shekyl_prism::verify_tagged`, on the
//! same bytes with the set's identifier in front of the key and of the
//! signature and an accept list of that one set, and fails the test
//! unless:
//!
//! 4. the tagged entry returns the set exactly when the byte-level entry
//!    accepts, and otherwise the byte-level entry's error, wrapped and
//!    unaltered.
//!
//! So every input any test builds through this module is one more input
//! on which tagged equals bare.
//!
//! [`verify`] and [`verify_set`] have the signatures of the functions of
//! those names in `prism_verify` and return what the byte-level entry
//! returned, so a test that calls them through this module tests what it
//! tested before and holds the rule on every input it builds.

use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
use prism_verify::{set::Set, VerifyError};
use shekyl_prism::{verify_tagged, AcceptList, PublicKey, SetId, Signature, TaggedError};

/// What the two entries said of one input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// `prism_verify::verify`.
    pub byte: Result<(), VerifyError>,
    /// The typed path: the first refusal among the key's construction,
    /// the signature's construction and the verification, in that order.
    pub typed: Result<(), VerifyError>,
    /// Whether `PublicKey::from_bytes` refused the key.
    pub key_refused: bool,
}

impl Outcome {
    /// Whether this is the one case where the entries may differ, and do.
    pub fn differs(&self) -> bool {
        self.byte != self.typed
    }
}

/// What a family of inputs came to.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// Inputs run through both entries.
    pub inputs: usize,
    /// Accepted, by both.
    pub accepted: usize,
    /// `PublicKey::from_bytes` refused.
    pub key_refused: usize,
    /// The entries named different refusals. Only possible among
    /// `key_refused`.
    pub different: usize,
}

impl Tally {
    /// Count one outcome.
    #[allow(dead_code)]
    pub fn add(&mut self, outcome: Outcome) {
        self.inputs += 1;
        self.accepted += usize::from(outcome.byte.is_ok());
        self.key_refused += usize::from(outcome.key_refused);
        self.different += usize::from(outcome.differs());
    }
}

/// The typed path, and whether the key was refused.
fn typed<L: ParameterSet>(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> (Result<(), VerifyError>, bool) {
    let key = match PublicKey::<L>::from_bytes(public_key) {
        Ok(key) => key,
        Err(error) => return (Err(error), true),
    };
    assert_eq!(key.as_bytes(), public_key, "a key's encoding is its bytes");
    let signature = match Signature::<L>::from_bytes(signature) {
        Ok(checked) => {
            assert_eq!(
                checked.as_bytes(),
                signature,
                "a signature's encoding is its bytes"
            );
            checked
        }
        Err(error) => return (Err(error), false),
    };
    (key.verify(message, &signature), false)
}

/// `bare` with the identifier of `L` in front. Written out here, and not
/// with `to_tagged`, because most inputs are not a key or a signature.
pub fn tag<L: ParameterSet>(bare: &[u8]) -> Vec<u8> {
    [&SetId::of::<L>().to_bytes()[..], bare].concat()
}

/// The tagged entry on the same three inputs, accepting the set `L` alone.
fn tagged<L: ParameterSet>(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<SetId, TaggedError> {
    let accept = AcceptList::new(&[SetId::of::<L>()]).expect("one set");
    verify_tagged(
        &accept,
        &tag::<L>(public_key),
        message,
        &tag::<L>(signature),
    )
}

/// Run the entries and hold the rules.
pub fn both<L: ParameterSet>(public_key: &[u8], message: &[u8], signature: &[u8]) -> Outcome {
    let byte = prism_verify::verify::<L>(public_key, message, signature);
    let (typed, key_refused) = typed::<L>(public_key, message, signature);
    let outcome = Outcome {
        byte,
        typed,
        key_refused,
    };
    assert_eq!(
        byte.is_ok(),
        typed.is_ok(),
        "{}: one entry accepts and the other refuses: {outcome:?}",
        L::NAME
    );
    assert_eq!(
        tagged::<L>(public_key, message, signature),
        byte.map(|()| SetId::of::<L>()).map_err(TaggedError::Verify),
        "{}: tagged is not bare",
        L::NAME
    );
    if key_refused {
        // One line for each input of the case where the entries may
        // differ, so a run with `--nocapture` can be counted.
        eprintln!(
            "ENTRIES key-refused set={} byte={byte:?} typed={typed:?}",
            L::NAME
        );
        assert!(
            byte.is_err(),
            "{}: the key is refused and the byte-level entry accepts",
            L::NAME
        );
    } else {
        assert_eq!(
            byte,
            typed,
            "{}: the key is accepted and the entries name different refusals",
            L::NAME
        );
    }
    outcome
}

/// `prism_verify::verify`, with the typed path held to it.
#[allow(dead_code)]
pub fn verify<L: ParameterSet>(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    both::<L>(public_key, message, signature).byte
}

/// [`both`] at a set named by value.
#[allow(dead_code)]
pub fn both_at(set: Set, public_key: &[u8], message: &[u8], signature: &[u8]) -> Outcome {
    match set {
        Set::P248 => both::<P248>(public_key, message, signature),
        Set::P324 => both::<P324>(public_key, message, signature),
        Set::P376 => both::<P376>(public_key, message, signature),
        Set::P500 => both::<P500>(public_key, message, signature),
        Set::P664 => both::<P664>(public_key, message, signature),
    }
}

/// `prism_verify::set::verify_set`, with the typed path held to it.
#[allow(dead_code)]
pub fn verify_set(
    set: Set,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    let outcome = both_at(set, public_key, message, signature);
    assert_eq!(
        prism_verify::set::verify_set(set, public_key, message, signature),
        outcome.byte,
        "{set}: verify_set is not verify at that set"
    );
    outcome.byte
}
