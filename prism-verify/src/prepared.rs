// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Verification split by object: a key checked once, a response checked
//! once, and the checks that need both.
//!
//! [`crate::verify`] takes three byte strings and runs every check on
//! every call. A caller that verifies many signatures under one key pays
//! for the key's checks each time, and the test that the key's points
//! are a basis is the dearest of them. The three functions here are the
//! same checks cut where the inputs separate:
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
//! The signature's checks, the test of the key's basis, and primality
//! followed by the relation, are each one function that [`crate::verify`]
//! runs as well, so the two entries cannot come to check different
//! things.
//!
//! # The order of refusals
//!
//! [`crate::verify`] checks both lengths, decodes the key, then decodes
//! and checks the signature, and only then searches for the key's basis;
//! it tests that basis after the challenge's primality and the
//! signature's points, and names a failure `Kernel`. Here the key's
//! checks, the search and the test among them, are finished before the
//! signature is looked at, and a failure of the test is `Basis`. The two
//! entries accept the same inputs. They name the same refusal whenever
//! the key is one [`PreparedKey::prepare`] accepts; a key it refuses the
//! other entry refuses too, and may name differently.
//! `docs/VERIFIER_SPEC.md` states the rule and
//! `shekyl-prism/tests/support/entries.rs` holds it on every input the
//! front crate's tests verify.

use core::marker::PhantomData;

use prism_field::ParameterSet;

use crate::basis::{basis_hint, BasisHint};
use crate::challenge::ChallengeScalar;
use crate::identity::KeyIdentity;
use crate::relation::KeyBasis;
use crate::signature::{canonical_sign, Response};
use crate::{mark, model, relation, VerifyError};

/// A public key that passed every check a key can pass alone, with the
/// basis those checks found and tested.
///
/// The only way to make one is [`Self::prepare`], so a value of this type
/// is a key of the right length, canonically encoded, on neither excluded
/// curve, whose basis index is within the search bound and whose named
/// points are a basis of the `2^e`-torsion of its curve.
///
/// What it stores is the key's bytes, that index and that basis. The
/// coefficient is the decode of those bytes whenever it is read, so the
/// challenge and the relation see one coefficient. The checks ran in
/// [`Self::prepare`]; the read does not square again. The basis is kept
/// because it is the thing the test was run on: the relation takes these
/// three points as they are, and a basis computed again at verification
/// would be a second value that nothing had tested. It is the
/// coefficient and three points, seven field elements. The parameter set is not in the bytes; [`Self::prepare`]
/// checked them against `L`, and the marker keeps that check in the type.
///
/// The three stored things are set in one place, [`Self::prepare`], from
/// one decode of the bytes, and nothing changes any of them afterwards:
/// the fields are private and no method takes `&mut self`.
///
/// Equality is not implemented. The key is its encoding: compare
/// [`Self::as_bytes`]. The index and the basis are functions of that
/// encoding.
#[derive(Clone)]
pub struct PreparedKey<L: ParameterSet> {
    bytes: Vec<u8>,
    hint: BasisHint,
    basis: KeyBasis<L>,
    set: PhantomData<L>,
}

/// The set's name, the encoding in hex and the basis index. The basis is
/// a function of those and holds field elements, which do not print.
impl<L: ParameterSet> core::fmt::Debug for PreparedKey<L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PreparedKey<{}>(", L::NAME)?;
        for byte in &self.bytes {
            write!(f, "{byte:02x}")?;
        }
        write!(f, ", {:?})", self.hint)
    }
}

impl<L: ParameterSet> PreparedKey<L> {
    /// Decode `public_key`, search for its basis, and test it.
    ///
    /// It assumes nothing of the bytes: it is the first call of the
    /// prepared entry, and every check on a key alone is in it.
    ///
    /// `Length`, then `Canonical`, then `Origin`, then `Singular`, then
    /// `Basis`: [`KeyIdentity::decode`], then [`basis_hint`], then the
    /// test that the points the index names are a basis of the
    /// `2^e`-torsion. `Basis` is either cause: no index, or no basis.
    ///
    /// The search costs up to `SEARCH_BOUND` Legendre symbols,
    /// and whoever chose the key chose how many. The basis and its test
    /// cost the same for every key of a set: about `6e` doublings on the
    /// key's curve, which at `p248` is some 6,000 multiplications and
    /// 3,000 squarings in the extension field, 3.9% of the products of a
    /// verification (`docs/BENCHMARKING.md`). [`crate::verify`] runs its
    /// cheap checks on the signature before the search and the
    /// challenge's primality before the test; this function has no
    /// signature to look at. Call it on a key that is kept, not once for
    /// every incoming message: a kept key pays this once, and every
    /// verification under it is that much cheaper.
    pub fn prepare(public_key: &[u8]) -> Result<Self, VerifyError> {
        mark::Stage::Public.enter();
        let identity = KeyIdentity::<L>::decode(public_key)?;
        let hint = basis_hint::<L>(identity.coefficient())?;
        mark::Stage::Torsion.enter();
        let basis = KeyBasis::of_key(identity.coefficient(), hint)?;
        Ok(Self {
            bytes: public_key.to_vec(),
            hint,
            basis,
            set: PhantomData,
        })
    }

    /// The bytes the key was prepared from, which are its canonical
    /// encoding and what the challenge hash absorbs.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// `A`, the decode of [`Self::as_bytes`].
    ///
    /// [`Self::prepare`] already ran [`KeyIdentity::decode`] on these bytes.
    /// Reading the coefficient decodes them again and does not repeat that
    /// function's checks: the singular test squares, and the typed entry's
    /// cost is held equal to the byte-level entry less the one basis search.
    pub fn coefficient(&self) -> L::Ext {
        self.identity().coefficient()
    }

    /// The basis index [`basis_hint`] derived for [`Self::coefficient`].
    pub fn hint(&self) -> BasisHint {
        self.hint
    }

    /// The key as the challenge hash takes it.
    ///
    /// The coefficient is the decode of the stored bytes, so the hash and
    /// the relation name one key. [`Self::prepare`] is what ran the checks.
    fn identity(&self) -> KeyIdentity<'_, L> {
        KeyIdentity::from_accepted_bytes(&self.bytes)
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
/// Equality is not implemented: a response holds field elements, and
/// their `==` is not a comparison of values. Compare [`Response::encode`]
/// of [`Self::response`].
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
    /// The decoded response, to read: the only reader this type has. A
    /// copy of it changed by hand is a [`Response`] again and has to be
    /// encoded and checked to come back.
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
/// It assumes nothing of the bytes. It does not need a key, and what it
/// returns is not yet a signature of anything: the challenge, its degree
/// and the relation are [`verify_prepared`]'s.
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
/// `Codomain`, `Image`). The key's basis is the one `key` carries: it is
/// not searched for, computed or tested again, and `Kernel` here is a
/// fault of the signature's points alone. Primality and then the relation
/// is one function, which [`crate::verify`] runs too.
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
    // The challenge hashes the key's bytes. The relation takes the basis
    // `prepare` built from those bytes, which carries the coefficient it
    // was tested with.
    let identity = key.identity();
    let scalar = ChallengeScalar::derive(&identity, message, response.salt);
    crate::prime_then::<L>(&scalar, || {
        relation::relate_prepared::<L>(&key.basis, response, scalar.as_bytes())
    })
}

#[cfg(test)]
mod tests {
    use super::{checked_response, PreparedKey};
    use crate::basis::basis_hint;
    use crate::identity::KeyIdentity;
    use prism_field::{Field, ParameterSet, P248};

    fn key() -> Vec<u8> {
        prism_vectors::corpus::Primary::Lvl1
            .vector()
            .public_key()
            .expect("the vector's public key")
    }

    /// The coefficient a prepared key reports is the decode of its bytes,
    /// and the index it carries is the search of that coefficient.
    #[test]
    fn the_coefficient_is_the_bytes_and_the_index_is_that_search() {
        let bytes = key();
        let prepared = PreparedKey::<P248>::prepare(&bytes).expect("an honest key");
        assert_eq!(prepared.as_bytes(), bytes.as_slice());
        let identity = KeyIdentity::<P248>::decode(&bytes).expect("an honest key");
        assert!(prepared.coefficient().same(identity.coefficient()));
        assert_eq!(
            basis_hint::<P248>(identity.coefficient()),
            Ok(prepared.hint())
        );
    }

    /// `Debug` of a prepared key names the set and gives the encoding in
    /// hex and the index: what the key is, and the one searched fact.
    #[test]
    fn a_prepared_key_prints_its_set_its_encoding_and_its_index() {
        let bytes = key();
        let prepared = PreparedKey::<P248>::prepare(&bytes).expect("an honest key");
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            format!("{prepared:?}"),
            format!("PreparedKey<p248>({hex}, {:?})", prepared.hint())
        );
    }

    /// `Debug` of a checked response names the set and gives the
    /// encoding in hex: the bytes that were checked, which is the only
    /// form in which two responses can be compared.
    #[test]
    fn a_checked_response_prints_its_set_and_its_encoding() {
        let signature = prism_vectors::corpus::Primary::Lvl1
            .vector()
            .signature()
            .expect("the vector's signature");
        let checked = checked_response::<P248>(&signature).expect("an honest signature");
        let hex: String = signature.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            format!("{checked:?}"),
            format!("CheckedResponse<p248>({hex})")
        );
        assert_eq!(hex.len(), 2 * <P248 as ParameterSet>::SIGNATURE_LEN);
    }
}
