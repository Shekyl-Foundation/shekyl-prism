// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Verification split by object: a key checked once, a response checked
//! once, and the checks that need both.
//!
//! [`crate::verify`] takes three byte strings and runs every check on
//! every call. A caller that verifies many signatures under one key pays
//! for the key's checks each time, and the search for the key's basis is
//! the dearest of them. The three functions here are the same checks cut
//! where the inputs separate:
//!
//! - [`PreparedKey::prepare`]: every check that depends on the key alone;
//! - [`checked_response`]: every check that depends on the signature alone;
//! - [`verify_prepared`]: the challenge, its degree and the relation.
//!
//! These are low-level. The typed interface of the `shekyl-prism` crate
//! (`PublicKey`, `Signature`) is built on them and is where the invariants
//! they assume are held by a type. Each function says which it assumes.
//!
//! # The order of refusals
//!
//! [`crate::verify`] checks both lengths, decodes the key, then decodes
//! and checks the signature, and only then searches for the key's basis.
//! Here the key's checks, the search among them, are finished before the
//! signature is looked at. When exactly one of the two inputs is faulty
//! the two entries name the same fault. When both are, they can name
//! different ones; `docs/VERIFIER_SPEC.md` states the rule and
//! `tests/support/entries.rs` holds it on every input the tests verify.

use prism_field::ParameterSet;

use crate::basis::{basis_hint, BasisHint};
use crate::challenge::ChallengeScalar;
use crate::identity::KeyIdentity;
use crate::signature::{canonical_sign, Response};
use crate::{mark, model, relation, VerifyError};

/// A public key that passed every check a key can pass alone, with the
/// basis index the search found.
///
/// The only way to make one is [`Self::prepare`], so a value of this type
/// is a key of the right length, canonically encoded, on neither excluded
/// curve, whose basis index is within the search bound.
///
/// Equality is not implemented: the decoded coefficient is a field
/// element, whose `==` is not a comparison of values. Compare
/// [`Self::as_bytes`].
#[derive(Clone, Debug)]
pub struct PreparedKey<L: ParameterSet> {
    bytes: Vec<u8>,
    coefficient: L::Ext,
    hint: BasisHint,
}

impl<L: ParameterSet> PreparedKey<L> {
    /// Decode `public_key` and search for its basis.
    ///
    /// `Length`, then `Canonical`, then `Origin`, then `Singular`, then
    /// `Basis`: [`KeyIdentity::decode`], then [`basis_hint`].
    ///
    /// The search costs up to [`crate::search_bound`] Legendre symbols,
    /// and whoever chose the key chose how many. [`crate::verify`] runs
    /// its cheap checks on the signature before that search; this function
    /// has no signature to look at. Call it on a key that is kept, not
    /// once for every incoming message.
    pub fn prepare(public_key: &[u8]) -> Result<Self, VerifyError> {
        mark::Stage::Public.enter();
        let identity = KeyIdentity::<L>::decode(public_key)?;
        let hint = basis_hint::<L>(identity.coefficient())?;
        Ok(Self {
            bytes: public_key.to_vec(),
            coefficient: identity.coefficient(),
            hint,
        })
    }

    /// The bytes the key was prepared from, which are its canonical
    /// encoding and what the challenge hash absorbs.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// `A`.
    pub fn coefficient(&self) -> L::Ext {
        self.coefficient
    }

    /// The basis index [`basis_hint`] derived for `A`.
    pub fn hint(&self) -> BasisHint {
        self.hint
    }

    /// The key as the challenge hash takes it, without decoding it again.
    fn identity(&self) -> KeyIdentity<'_, L> {
        KeyIdentity::from_checked(&self.bytes, self.coefficient)
    }
}

/// Decode `signature` and run every check a signature can pass alone.
///
/// `Length`, then `Canonical`, then `Sign`, then `Model` (or `Singular`,
/// for a response curve that is singular): [`Response::decode`], the sign
/// rule on `y(P)`, and the permitted model of the response curve. These
/// are the checks [`crate::verify`] runs on the signature, in its order.
///
/// The value returned is a plain [`Response`] with public fields. Nothing
/// in its type records that it was checked: [`verify_prepared`] assumes
/// it, and `shekyl_prism::Signature` is the type that holds it.
pub fn checked_response<L: ParameterSet>(signature: &[u8]) -> Result<Response<L>, VerifyError> {
    mark::Stage::Public.enter();
    let response = Response::<L>::decode(signature)?;
    if !canonical_sign::<L>(response.p.y) {
        return Err(VerifyError::Sign);
    }
    model::check::<L>(response.curve)?;
    Ok(response)
}

/// The checks that need the key, the message and the response together:
/// the challenge, the primality of its degree, and the relation.
///
/// `Composite`, then the relation's refusals (`Kernel`, `Pairing`,
/// `Codomain`, `Image`). The basis index is the one `key` carries; it is
/// not searched for again.
///
/// # What this assumes
///
/// `response` passed the sign rule and the model check, which is what
/// [`checked_response`] returns and what `shekyl_prism::Signature::from_bytes`
/// establishes. This function does not repeat them, and what it returns
/// for a response that did not pass them is not specified: those checks
/// are what make an accepted signature's bytes unique, and the model
/// check is also what refuses a singular response curve and the curves
/// with extra automorphisms.
///
/// Nothing is assumed of `key` beyond its type.
pub fn verify_prepared<L: ParameterSet>(
    key: &PreparedKey<L>,
    message: &[u8],
    response: &Response<L>,
) -> Result<(), VerifyError> {
    mark::Stage::Challenge.enter();
    let scalar = ChallengeScalar::derive(&key.identity(), message, response.salt);
    if !scalar.degree_is_prime::<L>() {
        return Err(VerifyError::Composite);
    }
    relation::relate_prepared::<L>(key.coefficient, key.hint, response, scalar.as_bytes())
}
