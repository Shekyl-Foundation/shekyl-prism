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
//! These are low-level, and each takes and returns a type that holds what
//! was checked: a [`PreparedKey`] and a [`CheckedResponse`] can only be
//! made by the function that checks them. The typed interface of the
//! `shekyl-prism` crate (`PublicKey`, `Signature`) wraps the two.
//!
//! The signature's checks, and primality followed by the relation, are
//! each one function that [`crate::verify`] runs as well, so the two
//! entries cannot come to check different things.
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

/// A decoded signature that passed every check a signature can pass alone.
///
/// The only way to make one is [`checked_response`], so a value of this
/// type is a response of the right length, canonically encoded, whose
/// first point has the normalized sign and whose curve is the one
/// permitted model of its class. Those checks are what make an accepted
/// signature's bytes unique. [`Response`] has public fields and records
/// none of this; this type has one private field and no other
/// constructor, and [`verify_prepared`] takes nothing else.
///
/// Equality is not implemented, for the reason given at [`PreparedKey`].
/// Compare [`Response::encode`] of [`Self::response`].
#[derive(Clone, Copy)]
pub struct CheckedResponse<L: ParameterSet> {
    response: Response<L>,
}

/// The set's name and the encoding in hex.
impl<L: ParameterSet> core::fmt::Debug for CheckedResponse<L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "CheckedResponse<{}>(", L::NAME)?;
        for byte in self.response.encode() {
            write!(f, "{byte:02x}")?;
        }
        f.write_str(")")
    }
}

impl<L: ParameterSet> CheckedResponse<L> {
    /// The decoded response, to read. A copy of it changed by hand is a
    /// [`Response`] again and has to be encoded and checked to come back.
    pub fn response(&self) -> &Response<L> {
        &self.response
    }
}

/// The signature's checks, in one place: both entries run this function.
///
/// `Length`, then `Canonical` ([`Response::decode`]), then `Sign`, then
/// `Singular` or `Model`. It marks no stage; each entry has entered the
/// public stage before it.
pub(crate) fn response_checks<L: ParameterSet>(
    signature: &[u8],
) -> Result<CheckedResponse<L>, VerifyError> {
    let response = Response::<L>::decode(signature)?;
    if !canonical_sign::<L>(response.p.y) {
        return Err(VerifyError::Sign);
    }
    model::check::<L>(response.curve)?;
    Ok(CheckedResponse { response })
}

/// Decode `signature` and run every check a signature can pass alone.
///
/// `Length`, then `Canonical`, then `Sign`, then `Model` (or `Singular`,
/// for a response curve that is singular): [`Response::decode`], the sign
/// rule on `y(P)`, and the permitted model of the response curve. These
/// are the checks [`crate::verify`] runs on the signature, by the same
/// function.
pub fn checked_response<L: ParameterSet>(
    signature: &[u8],
) -> Result<CheckedResponse<L>, VerifyError> {
    mark::Stage::Public.enter();
    response_checks::<L>(signature)
}

/// The checks that need the key, the message and the response together:
/// the challenge, the primality of its degree, and the relation.
///
/// `Composite`, then the relation's refusals (`Kernel`, `Pairing`,
/// `Codomain`, `Image`). The basis index is the one `key` carries; it is
/// not searched for again. Primality and then the relation is one
/// function, which [`crate::verify`] runs too.
///
/// Both arguments are valid by their types: a [`PreparedKey`] and a
/// [`CheckedResponse`] can each be made only by the function that checks
/// them. This function assumes nothing else.
pub fn verify_prepared<L: ParameterSet>(
    key: &PreparedKey<L>,
    message: &[u8],
    response: &CheckedResponse<L>,
) -> Result<(), VerifyError> {
    mark::Stage::Challenge.enter();
    let response = response.response();
    let scalar = ChallengeScalar::derive(&key.identity(), message, response.salt);
    crate::prime_then::<L>(&scalar, || {
        relation::relate_prepared::<L>(key.coefficient, key.hint, response, scalar.as_bytes())
    })
}
