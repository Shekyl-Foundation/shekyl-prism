// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Experimental prime-degree PRISM verifier at `e′ = e − 2`: what a
//! signature is checked for, in what order, and what each refusal means.
//!
//! # What `Ok(())` means
//!
//! Canonical decoding and public checks precede two adapted theta chains,
//! dual-codomain identification and both components of the image check.
//! `Ok(())` means this research relation holds; it is not a claim of a
//! reviewed SUF-CMA security proof. See `docs/SECURITY_REVIEW.md`.
//!
//! Everything here is variable time and takes public values only: a key,
//! a message and a signature. There is no secret and no randomness.
//!
//! # The two entries
//!
//! - [`verify`], with [`set::verify_set`] for a set named at run time:
//!   three byte strings, every check on every call. It is the
//!   definition. Its refusal order and its operation counts are held
//!   fixed, and the committed count records are taken through it.
//! - [`prepared`]: the same checks cut where the inputs separate.
//!   [`prepared::PreparedKey::prepare`] runs every check on the key
//!   alone, [`prepared::checked_response`] every check on the signature
//!   alone, and [`prepared::verify_prepared`] the checks that need both
//!   and the message. The typed interface of the `shekyl-prism` crate is
//!   built on these three.
//!
//! The two accept the same inputs. With the prepared entry taken as its
//! three calls in the order key, signature, verification, and its
//! refusal as the first of them:
//!
//! 1. the prepared entry accepts exactly when [`verify`] accepts;
//! 2. if `prepare` accepts the key, the prepared entry's refusal is
//!    [`verify`]'s;
//! 3. if `prepare` refuses the key, [`verify`] refuses too, and may name
//!    another error. It names `Length` when the signature's length is
//!    wrong, whatever the key is. Under a key whose only fault is
//!    `Basis` it names a fault of the signature (`Canonical`, `Sign`,
//!    `Singular`, `Model`) first; and when that fault is that the named
//!    points are no basis, it names `Composite` if the challenge degree
//!    is composite and otherwise `Kernel`.
//!
//! `shekyl-prism/tests/support/entries.rs` holds the rule on every input
//! the front crate's tests verify.
//!
//! # The checks of the byte-level entry
//!
//! One row for each refusal [`verify`] can make, in the order it makes
//! them. The table is written from the code. "Sender's cost" says whether
//! the work this step does depends on bytes the sender of the key or the
//! signature chooses; where it says no, every input that passes the
//! step has paid the same for it. An input the step refuses may have
//! paid less: a step that makes several tests stops at the first that
//! fails (row 16, for one, is three tests, and its refusals cost a
//! third, two thirds or all of it).
//!
//! | # | Step | What is tested | Refusal | Sender's cost |
//! | --- | --- | --- | --- | --- |
//! | 1 | lengths | the key and the signature each have the set's length; both before either is read | `Length` | no |
//! | 2 | key | both halves of `A` are below `p` | `Canonical` | no |
//! | 3 | key | `A ≠ 0` | `Origin` | no |
//! | 4 | key | `A² ≠ 4` | `Singular` | no |
//! | 5 | signature | each of `A_sig`, `x(P)`, `y(P)`, `x(Q)`, `y(Q)`, in that order, has both halves below `p` | `Canonical` | no |
//! | 6 | signature | `0 < y(P) < −y(P)` | `Sign` | no |
//! | 7 | response curve | `A_sig² ≠ 4` | `Singular` | no |
//! | 8 | response curve | not one of the classes with extra automorphisms: `A_sig ≠ 0`, `A_sig² ≠ 3`, `2·A_sig² ≠ 9` | `Model` | no |
//! | 9 | response curve | `A_sig` is not above `−A_sig` | `Model` | no |
//! | 10 | response curve | `A_sig² − 4` is a square | `Model` | yes: a square root, paid only by a curve that passed 9 |
//! | 11 | response curve | neither coefficient that a rational point of order 2 moves the curve to, nor its negative, is below `A_sig` | `Model` | yes: an inversion, then up to two more square roots and two more inversions |
//! | 12 | key | an index within the search bound names a point of the key's curve | `Basis` | **yes**: one to `SEARCH_BOUND` Legendre symbols, and the key chooses how many |
//! | 13 | challenge | the degree `2^{2e′−1} − s²` passes Baillie–PSW, for `s` the hash of key, salt and message | `Composite` | yes: the hash is linear in the message's length, and the test stops at the first small factor, or after its Miller–Rabin half, or runs both halves |
//! | 14 | key's points | the index point, its partner and their difference can be computed | `Kernel` | no |
//! | 15 | signature's points | `P` and `Q` satisfy the response curve's equation | `Kernel` | no |
//! | 16 | signature's points | each has order exactly `2^e`, and the two span the `2^e`-torsion | `Kernel` | no |
//! | 17 | key's points | each of the two basis points has order exactly `2^e`, and the two span | `Kernel` | no |
//! | 18 | lift | the key's three points lift to full points with consistent signs | `Pairing` | no |
//! | 19 | pairing | `e([s]P, [s]Q) = e(P_vk, Q_vk)^q`, and both pairings have a value | `Pairing` | slightly: `[s]` and the power follow the bits of `s` |
//! | 20 | kernel | `−s²` has an inverse modulo `2^e` | `Kernel` | no |
//! | 21 | kernel | the key's 4-torsion has a canonical basis | `Kernel` | no |
//! | 22 | kernel | the response's 4-torsion has a canonical basis | `Kernel` | no |
//! | 23 | kernel | the Weil pairing of the key's 4-torsion basis has a value | `Kernel` | no |
//! | 24 | kernel | the two challenge matrices exist | `Kernel` | no |
//! | 25 | kernel | the long chain's change of theta basis exists | `Kernel` | no |
//! | 26 | long chain | every step of the `e − 2` steps has a codomain | `Kernel` | slightly: the kernel's ladders follow the matrix entries |
//! | 27 | kernel | the short chain's change of theta basis exists | `Kernel` | no |
//! | 28 | short chain | every step of the `e − 3` steps has a codomain | `Kernel` | slightly, as 26 |
//! | 29 | codomain | the two chains end on dual theta structures of one surface | `Codomain` | no |
//! | 30 | image, three times | the point can be carried through the long chain | `Kernel` | no |
//! | 31 | image, three times | and back through the dual of the short chain | `Kernel` | no |
//! | 32 | image, three times | both components are the expected multiples by `s` | `Image` | no |
//!
//! Rows 30 to 32 run in turn for each of the three points `P`, `Q` and
//! `P − Q`, so a later point's `Kernel` can follow an earlier point's
//! passing comparison.
//!
//! What the order is for: everything through row 11 is decoding and a
//! few field operations, and runs before the search of row 12, the one
//! step whose length a sender chooses outright. The hash and primality
//! of row 13 come before any curve arithmetic, so a signature whose salt
//! does not name a prime degree costs no doubling. From row 14 on, the
//! cost of a refusal is set by how far it gets, and
//! `docs/benchmarks/negative-verdicts.tsv` records it for each committed
//! negative.
//!
//! ## Refusals the code spells and this entry cannot reach
//!
//! - `Length` inside [`identity::KeyIdentity::decode`] and
//!   [`signature::Response::decode`]: row 1 has tested both lengths.
//!   The prepared entry reaches them, having no row 1.
//! - `Length` for the salt: a string of the salt's length is a salt.
//! - `Model` where 2 has no inverse, and where the square root of
//!   `α² − 1` is zero: the fields have odd characteristic, and the
//!   second needs `A_sig = ±2`, which row 7 refused.
//! - `Origin` and `Basis` at the head of the relation: rows 3 and 12
//!   refused both, and the relation's second search of the same key
//!   finds what the first found.
//! - Row 20: `s` is odd, so `−s²` is a unit.
//! - Rows 25 and 27: the change of theta basis is refused only when the
//!   pairing of the order-four basis squares to `−3`. That pairing is a
//!   fourth root of unity, so its square is `±1`.
//!
//! Some rows of the table refuse a value the earlier rows leave no known
//! way to produce. They stay, so that a value this crate did not expect
//! is refused and not used. The ledger names them
//! (`docs/lab/LEDGER.md`, O23).
//!
//! # The prepared entry's order
//!
//! | Call | Rows, in order | Refusals |
//! | --- | --- | --- |
//! | [`prepared::PreparedKey::prepare`] | the key's length, 2, 3, 4, 12, then 14 and 17 | `Length`, `Canonical`, `Origin`, `Singular`, `Basis`; rows 14 and 17 are `Basis` here, not `Kernel` |
//! | [`prepared::checked_response`] | the signature's length, 5 to 11 | `Length`, `Canonical`, `Sign`, `Singular`, `Model` |
//! | [`prepared::verify_prepared`] | 13, 15, 16, 18 to 32 | `Composite`, `Kernel`, `Pairing`, `Codomain`, `Image` |
//!
//! The cost order differs with it: the search of row 12 and the test of
//! row 17, the dearest of the key's checks, run when the key is prepared
//! and never again. That suits a key validated once and kept, and not a
//! key built from every incoming message.
//!
//! # The five sets
//!
//! | Set | Honest signatures to test with | What holds it |
//! | --- | --- | --- |
//! | `p248` | yes, committed | acceptance and every reachable refusal |
//! | `p324` | none | refusals only |
//! | `p376` | yes, committed | acceptance and every reachable refusal |
//! | `p500` | yes, committed | acceptance and every reachable refusal |
//! | `p664` | none | refusals only |
//!
//! At `p324` and `p664` no signer exists, so nothing shows that
//! [`verify`] accepts anything, and row 32 is not reached. The field,
//! the torsion, the pairing, the challenge and primality are tested
//! there against independent implementations, and the refusals reachable
//! from public data are reached, `Codomain` among them
//! (`docs/lab/LEDGER.md`, O14).
//!
//! # The modules
//!
//! Each public check owns a module: [`identity`] decodes the key,
//! [`signature`] decodes the response and has the sign rule, [`basis`]
//! re-derives the C reference's basis index, [`challenge`] derives the
//! challenge scalar, and [`prime`] decides its degree. [`set`] names a
//! parameter set at run time. The relation itself, rows 14 to 32, is
//! private: it is reached through the two entries and no other way.
//!
//! The functions in those modules are steps, not entries. Each one's
//! documentation says what it assumes of its arguments, what it returns
//! when the assumption fails, and which call establishes it. Only
//! [`verify`], [`set::verify_set`] and the three calls of [`prepared`]
//! taken together check everything.
//!
//! # An example
//!
//! A committed signature at `p248` is accepted. With one byte changed,
//! the top byte of `x(P)`, it is refused at row 5: no element of the
//! field has that byte.
//!
//! ```
//! use prism_field::{ParameterSet, P248};
//! use prism_vectors::corpus::Primary;
//! use prism_verify::{verify, VerifyError};
//!
//! let vector = Primary::Lvl1.vector();
//! let key = vector.public_key().unwrap();
//! let message = vector.message().unwrap();
//! let mut signature = vector.signature().unwrap();
//! assert_eq!(verify::<P248>(&key, message, &signature), Ok(()));
//!
//! // `x(P)` is the second field element of the signature.
//! signature[2 * P248::FP2_LEN - 1] = 0xff;
//! assert_eq!(
//!     verify::<P248>(&key, message, &signature),
//!     Err(VerifyError::Canonical)
//! );
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod accept;
pub mod basis;
pub mod challenge;
pub mod identity;
mod mark;
mod model;
pub mod prepared;
pub mod prime;
mod relation;
#[cfg(feature = "research-reduced-width")]
pub mod research;
pub mod set;
pub mod signature;
// The stage names are the counter's and not part of the library: the
// module exists only when counting is asked for. `mark` is what the
// verifier calls, with or without it.
#[cfg(feature = "op-count")]
pub mod stage;

use prism_field::ParameterSet;

use basis::basis_hint;
use challenge::ChallengeScalar;
use identity::KeyIdentity;
use prepared::CheckedResponse;

/// Why a signature was refused.
///
/// The row numbers are those of the table in the crate documentation,
/// which is the order [`verify`] refuses in. The prepared entry runs the
/// same checks in the order of its three calls. The enum's derived order
/// is the order of the variants below and is not a precedence rule:
/// `Singular` can be either curve's, and `Kernel` is given before the
/// pairing and after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VerifyError {
    /// A byte string is not this set's length (row 1).
    ///
    /// In [`verify`]: the key's or the signature's, tested together
    /// before either is read, so a wrong signature length is `Length`
    /// whatever the key is. In the prepared entry each call tests its
    /// own input, so a key with another fault and a signature of the
    /// wrong length are named for the key there and `Length` here.
    ///
    /// With the feature `research-reduced-width`, the two functions of
    /// `research` also give it for a challenge width or scalar they do
    /// not take.
    Length,
    /// A field element is not the canonical representative in `0..p`
    /// (rows 2 and 5): the key's coefficient, or the first of the
    /// signature's five elements, in layout order, that is out of range.
    Canonical,
    /// `A = 0`, the starting curve, as a key (row 3). Its endomorphism
    /// ring is public. A response curve with `A_sig = 0` is `Model`.
    Origin,
    /// `A² = 4`: the Montgomery curve is singular. The key's (row 4) or
    /// the response's (row 7).
    Singular,
    /// `y(P)` is zero or not strictly below `y(−P)` in `<_{F_{p^2}}`
    /// (row 6).
    Sign,
    /// The response curve's coefficient is not the one permitted for its
    /// isomorphism class (rows 8 to 11). Four causes: the class has
    /// extra automorphisms (`A_sig = 0`, `A_sig² = 3` or `2·A_sig² = 9`);
    /// `−A_sig` is below `A_sig`; `A_sig² − 4` is not a square, so the
    /// curve's points of order 2 are not all rational; or a coefficient
    /// one of those points moves the curve to, or its negative, is below
    /// `A_sig`.
    Model,
    /// The key gives no usable basis of its `2^e`-torsion, by one of two
    /// causes.
    ///
    /// No basis index within the set's search bound meets the C search
    /// (row 12). The search is the reference's, including the indices its
    /// `start = 128` fallback covers, so this refuses only keys whose
    /// smallest index exceeds the bound. The documentation of [`basis`]
    /// gives the heuristic estimate of how rare that is, and
    /// `docs/VERIFIER_SPEC.md` records the bound and what it costs. Both
    /// entries give this refusal for it.
    ///
    /// Or the search finds an index and the points it names are not a
    /// basis: they cannot be computed, one of them has not order `2^e`,
    /// or the two do not span (rows 14 and 17). The search looks at
    /// abscissas and does not see this. It is so for a curve whose
    /// `2^e`-torsion is not rational, which is no key.
    /// [`prepared::PreparedKey::prepare`] refuses such a key with this
    /// error. The byte-level [`verify`] reaches the same test later, in
    /// the relation, and names it [`Self::Kernel`]; before that it
    /// names any fault of the signature in rows 5 to 11, and
    /// [`Self::Composite`] when the challenge degree is composite.
    Basis,
    /// `2^{2e′−1} − s²` is not a Baillie–PSW prime, `e′ = e − 2` (row
    /// 13). The same in both entries.
    Composite,
    /// A point or a construction the relation needs is not what it must
    /// be. Before the pairing:
    ///
    /// - a signature point is not on the response curve (row 15);
    /// - a signature point has not order exactly `2^e`, or the two do
    ///   not span the torsion (row 16);
    /// - in [`verify`] only, the points the key's index names cannot be
    ///   computed or are not a basis (rows 14 and 17), which the
    ///   prepared entry names [`Self::Basis`] when the key is prepared.
    ///
    /// After the pairing:
    ///
    /// - the 4-torsion of either curve has no canonical basis, its
    ///   pairing has no value, the challenge matrices or a change of
    ///   theta basis do not exist (rows 20 to 25 and 27);
    /// - a step of either chain has no codomain (rows 26 and 28);
    /// - a point cannot be carried through the long chain or back
    ///   through the dual of the short one (rows 30 and 31).
    ///
    /// A signature whose two points are a basis and pass the pairing can
    /// still end here: the committed negative `identity-response-q-negated`
    /// does, at row 26.
    Kernel,
    /// `e(R₁, R₂) ≠ e(P₁, P₂)^q`, with `R_i = [s]` of the signature
    /// points (row 19). This pins the determinant of the basis change. It
    /// runs after the order and span checks and before either chain.
    ///
    /// The same error is given when either pairing has no value, and
    /// when the key's basis does not lift to full points (row 18). No
    /// input is known for the lift's refusal.
    Pairing,
    /// The adapted chains do not meet in dual theta structures (row 29).
    Codomain,
    /// The composed chain does not have the challenge-bound images (row
    /// 32).
    Image,
}

impl core::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reason = match self {
            Self::Length => "not this set's length",
            Self::Canonical => "a field element is not canonical",
            Self::Origin => "the starting curve",
            Self::Singular => "a singular Montgomery curve",
            Self::Model => "the response curve is not the unique permitted model",
            Self::Sign => "y(P) is not the normalized representative",
            Self::Basis => "the key gives no basis of its torsion",
            Self::Composite => "the challenge degree is composite",
            Self::Kernel => "the curve, torsion, or adapted chain is invalid",
            Self::Pairing => "the Weil pairing does not match the challenge degree",
            Self::Codomain => "the adapted codomains do not match",
            Self::Image => "the composed images do not match the challenge",
        };
        f.write_str(reason)
    }
}

impl std::error::Error for VerifyError {}

/// Verify `signature` on `message` under `public_key`, at the set `L`.
///
/// The byte-level entry and the definition of what is accepted. It
/// assumes nothing of its three arguments: every check in the table of
/// the crate documentation runs, in that order, on every call, and the
/// first that fails is the refusal. `Ok(())` is returned only after the
/// last image comparison.
pub fn verify<L: ParameterSet>(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), VerifyError> {
    let opened = open::<L>(public_key, signature)?;
    mark::Stage::Challenge.enter();
    let scalar = ChallengeScalar::derive(&opened.identity, message, opened.checked.response().salt);
    accept(&opened, &scalar)
}

/// A key and response that passed the checks shared by every verifier entry.
///
/// The basis index is not kept here. [`open`] searches for it so a missing
/// basis is refused before the challenge, and the relation searches again:
/// that second search is part of the byte-level entry's pinned cost.
/// [`PreparedKey`] is the owned key that keeps the index and does not
/// search a second time.
pub(crate) struct Opened<'a, L: ParameterSet> {
    pub identity: KeyIdentity<'a, L>,
    pub checked: CheckedResponse<L>,
}

/// Length, decode, sign, model, and basis. The signature's checks are
/// [`prepared::response_checks`], the function the split entry runs. Both
/// lengths come before either decode, so `Length` does not depend on which
/// input is short and a short signature cannot surface as `Origin`. The
/// sign test is a byte comparison on an element already decoded, so it
/// comes before the basis search, whose length an attacker chooses with
/// `A` — up to `SEARCH_BOUND` Legendre symbols.
pub(crate) fn open<'a, L: ParameterSet>(
    public_key: &'a [u8],
    signature: &[u8],
) -> Result<Opened<'a, L>, VerifyError> {
    mark::Stage::Public.enter();
    if public_key.len() != L::PUBLIC_KEY_LEN || signature.len() != L::SIGNATURE_LEN {
        return Err(VerifyError::Length);
    }
    let identity = KeyIdentity::<L>::decode(public_key)?;
    let checked = prepared::response_checks::<L>(signature)?;
    basis_hint::<L>(identity.coefficient())?;
    Ok(Opened { identity, checked })
}

/// Primality, then the relation. The challenge hash has already run.
pub(crate) fn accept<L: ParameterSet>(
    opened: &Opened<'_, L>,
    scalar: &ChallengeScalar,
) -> Result<(), VerifyError> {
    prime_then::<L>(scalar, || {
        relation::relate(&opened.identity, opened.checked.response(), scalar)
    })
}

/// `Composite` unless the challenge degree is prime, then `relation`.
/// Both entries end here; they differ in the relation they pass, which
/// searches for the key's basis index or is given it.
pub(crate) fn prime_then<L: ParameterSet>(
    scalar: &ChallengeScalar,
    relation: impl FnOnce() -> Result<(), VerifyError>,
) -> Result<(), VerifyError> {
    mark::Stage::Challenge.enter();
    if !scalar.degree_is_prime::<L>() {
        return Err(VerifyError::Composite);
    }
    relation()
}

#[cfg(test)]
mod tests {
    use super::*;
    use basis::{BasisHint, C_HINT_MAX_INDEX};
    use prime::is_baillie_psw;
    use prism_field::{Field, ParameterSet, Salt, P248, P376, P500};
    use set::{
        verify_set, Committed, CommittedSet, NoCorpus, Set, UnknownCommittedLabel, UnknownSet,
    };
    use signature::{canonical_sign, ext_less, AffinePoint, Response};

    type Ext1 = <P248 as ParameterSet>::Ext;

    fn key<L: ParameterSet>(coefficient: u64) -> Vec<u8> {
        L::ext_encode(L::Ext::from_u64(coefficient))
    }

    /// The message the samples of [`prepared`] are under.
    const SAMPLE_MESSAGE: &[u8] = b"shekyl-prism";

    /// The committed C-generated key of a set and the hint the C derived
    /// for it, from `prism_vectors::hints`.
    struct ReferenceKey {
        public_key: Vec<u8>,
        hint: BasisHint,
    }

    fn reference_key(set: CommittedSet) -> ReferenceKey {
        let row =
            prism_vectors::hints::row(set.label()).unwrap_or_else(|err| panic!("{set}: {err}"));
        ReferenceKey {
            public_key: row.public_key.clone(),
            hint: BasisHint::from_c_byte(row.hint)
                .unwrap_or_else(|| panic!("{set}: hint byte {}", row.hint)),
        }
    }

    /// The first primary vector of a set: the honest signature whose
    /// response curve the samples borrow.
    fn primary(set: CommittedSet) -> prism_vectors::corpus::Primary {
        use prism_vectors::corpus::WithCorpus;
        match set {
            CommittedSet::P248 => P248::PRIMARY[0],
            CommittedSet::P376 => P376::PRIMARY[0],
            CommittedSet::P500 => P500::PRIMARY[0],
        }
    }

    /// A public key, a message and a signature that pass every public
    /// check and stop at `Kernel`.
    struct Sample {
        public_key: Vec<u8>,
        message: &'static [u8],
        signature: Vec<u8>,
    }

    /// A sample for the tests of the public checks, under the set's
    /// committed C-generated key.
    ///
    /// It is computed here and not committed as data: its salt is the
    /// first counter whose challenge degree is prime under that key and
    /// [`SAMPLE_MESSAGE`], which takes the verifier's own challenge and
    /// primality code to find, and only these tests use it. The response
    /// carries an admissible curve, the response curve of the set's first
    /// primary vector, and point coordinates that are not on it, so
    /// verification passes the sign and degree checks and stops at
    /// `Kernel`.
    fn prepared(set: CommittedSet) -> Sample {
        match set {
            CommittedSet::P248 => sample::<P248>(set),
            CommittedSet::P376 => sample::<P376>(set),
            CommittedSet::P500 => sample::<P500>(set),
        }
    }

    fn sample<L: ParameterSet>(set: CommittedSet) -> Sample {
        let key = reference_key(set);
        let id = KeyIdentity::<L>::decode(&key.public_key).expect("the committed key decodes");
        let salt = (0u64..65_536)
            .find_map(|counter| {
                let salt = L::Salt::from_counter(counter)?;
                ChallengeScalar::derive(&id, SAMPLE_MESSAGE, salt)
                    .degree_is_prime::<L>()
                    .then_some(salt)
            })
            .expect("a salt of prime degree within the search");
        let honest = primary(set)
            .vector()
            .signature()
            .expect("the primary vector reads");
        let response = Response::<L> {
            curve: Response::<L>::decode(&honest)
                .expect("the honest signature decodes")
                .curve,
            p: AffinePoint {
                x: L::Ext::zero(),
                y: L::Ext::one(),
            },
            q: AffinePoint {
                x: L::Ext::zero(),
                y: L::Ext::zero(),
            },
            salt,
        };
        Sample {
            public_key: key.public_key,
            message: SAMPLE_MESSAGE,
            signature: response.encode(),
        }
    }

    #[test]
    fn public_refusals_precede_the_expensive_checks() {
        // The declared order is the documented one, and `Sign` precedes the
        // basis search an attacker can lengthen.
        let order = [
            VerifyError::Length,
            VerifyError::Canonical,
            VerifyError::Origin,
            VerifyError::Singular,
            VerifyError::Sign,
            VerifyError::Model,
            VerifyError::Basis,
            VerifyError::Composite,
            VerifyError::Kernel,
            VerifyError::Pairing,
            VerifyError::Codomain,
            VerifyError::Image,
        ];
        let mut sorted = order;
        sorted.sort_unstable();
        assert_eq!(order, sorted);
        assert!(VerifyError::Sign < VerifyError::Basis);
        assert!(VerifyError::Basis < VerifyError::Composite);
    }

    #[test]
    fn a_short_signature_is_a_length_refusal() {
        let err = verify::<P248>(&[0u8; P248::PUBLIC_KEY_LEN], b"m", &[0u8; 4]);
        assert_eq!(err, Err(VerifyError::Length));
        // A wrong length is refused before the key is looked at, so the
        // starting curve does not shadow it.
        let err = verify::<P248>(&[0u8; P248::PUBLIC_KEY_LEN], b"m", &[]);
        assert_eq!(err, Err(VerifyError::Length));
    }

    #[test]
    fn salt_prism_lengths_are_not_this_encoding() {
        for n in [260usize, 388, 516] {
            assert_ne!(n, P248::SIGNATURE_LEN);
            assert_ne!(n, P376::SIGNATURE_LEN);
            assert_ne!(n, P500::SIGNATURE_LEN);
        }
        assert_eq!(P248::SIGNATURE_LEN, 352);
        assert_eq!(P376::SIGNATURE_LEN, 528);
        assert_eq!(P500::SIGNATURE_LEN, 704);
        assert_eq!(P376::CHAIN_STEPS, P376::E - 2);
    }

    #[test]
    fn the_starting_curve_is_refused() {
        let pk = vec![0u8; P248::PUBLIC_KEY_LEN];
        let sig = vec![0u8; P248::SIGNATURE_LEN];
        assert_eq!(
            verify::<P248>(&pk, b"shekyl-prism", &sig),
            Err(VerifyError::Origin)
        );
    }

    #[test]
    fn a_singular_curve_is_refused() {
        // `A = 2` is a square root of four, so the curve is singular.
        let sig = vec![0u8; P248::SIGNATURE_LEN];
        assert_eq!(
            verify::<P248>(&key::<P248>(2), b"shekyl-prism", &sig),
            Err(VerifyError::Singular)
        );
    }

    #[test]
    fn a_noncanonical_key_is_refused() {
        let mut pk = vec![0xffu8; P248::PUBLIC_KEY_LEN];
        pk[P248::FP_LEN - 1] = 0xff;
        let sig = vec![0u8; P248::SIGNATURE_LEN];
        assert_eq!(
            verify::<P248>(&pk, b"shekyl-prism", &sig),
            Err(VerifyError::Canonical)
        );
    }

    #[test]
    fn a_noncanonical_signature_is_refused_before_the_basis_search() {
        // The key is canonical, non-zero and nonsingular, so nothing above
        // `Canonical` can shadow the refusal; every field element of the
        // signature is out of range. The basis search is the next check
        // after the decode, so a `Canonical` answer here is what keeps a
        // junk signature from paying for up to `SEARCH_BOUND` Legendre
        // symbols.
        let pk = key::<P248>(3);
        let sig = vec![0xffu8; P248::SIGNATURE_LEN];
        assert_eq!(
            verify::<P248>(&pk, b"shekyl-prism", &sig),
            Err(VerifyError::Canonical)
        );
    }

    #[test]
    fn zero_y_is_refused() {
        let pk = key::<P248>(3);
        assert_eq!(pk.len(), P248::PUBLIC_KEY_LEN);
        let sig = vec![0u8; P248::SIGNATURE_LEN];
        assert_eq!(
            verify::<P248>(&pk, b"shekyl-prism", &sig),
            Err(VerifyError::Sign)
        );
    }

    #[test]
    fn the_sign_order_is_the_integer_order() {
        let one = Ext1::one();
        assert!(ext_less::<P248>(one, one.negate()));
        assert!(!ext_less::<P248>(one.negate(), one));
        let i = P248::ext_i();
        assert!(ext_less::<P248>(i, i.negate()));
        assert!(!canonical_sign::<P248>(Ext1::zero()));
        assert!(canonical_sign::<P248>(one));
        assert!(!canonical_sign::<P248>(one.negate()));
    }

    #[test]
    fn a_bad_sign_is_refused_before_the_challenge_is_hashed() {
        // The key's basis exists and the salt names a prime degree,
        // so the only failing check below `Composite` is the sign test.
        let sample = prepared(CommittedSet::P248);
        let mut response = Response::<P248>::decode(&sample.signature).expect("the sample decodes");
        response.p.y = response.p.y.negate();
        assert_eq!(
            verify::<P248>(&sample.public_key, sample.message, &response.encode()),
            Err(VerifyError::Sign)
        );
        // And with the sign restored, another salt whose degree is composite
        // is what the same signature is refused for.
        response.p.y = response.p.y.negate();
        let composite = (0u64..65_536)
            .find_map(|n| {
                let mut probe = response;
                probe.salt = <P248 as ParameterSet>::Salt::from_counter(n).unwrap();
                (verify::<P248>(&sample.public_key, sample.message, &probe.encode())
                    == Err(VerifyError::Composite))
                .then_some(probe)
            })
            .expect("a composite salt");
        let mut both = composite;
        both.p.y = both.p.y.negate();
        assert_eq!(
            verify::<P248>(&sample.public_key, sample.message, &both.encode()),
            Err(VerifyError::Sign),
            "a signature failing Sign and Composite is refused with Sign"
        );
    }

    #[test]
    fn the_derived_index_equals_the_c_hint() {
        for set in CommittedSet::ALL {
            let row = reference_key(set);
            let derived = match set {
                CommittedSet::P248 => derive_hint::<P248>(&row.public_key),
                CommittedSet::P376 => derive_hint::<P376>(&row.public_key),
                CommittedSet::P500 => derive_hint::<P500>(&row.public_key),
            };
            assert_eq!(derived, Ok(row.hint), "{set}");
            // The byte the C returned round-trips through the typed hint.
            let byte = row.hint.to_c_byte().expect("the C named this index");
            assert_eq!(BasisHint::from_c_byte(byte), Some(row.hint), "{set}");
        }
    }

    fn derive_hint<L: ParameterSet>(public_key: &[u8]) -> Result<BasisHint, VerifyError> {
        let identity = KeyIdentity::<L>::decode(public_key)?;
        basis_hint::<L>(identity.coefficient())
    }

    #[test]
    fn the_pinned_hints_are_the_committed_bytes() {
        // The hint bytes `scripts/export_c_basis_hints.py` wrote from its
        // frozen seed. A regeneration that changes a byte updates this table.
        for (set, byte) in [
            (CommittedSet::P248, 11u8),
            (CommittedSet::P376, 13),
            (CommittedSet::P500, 21),
        ] {
            let row = reference_key(set);
            assert_eq!(row.hint.to_c_byte(), Some(byte), "{set}");
            assert!(row.hint.index() >= 1 && row.hint.index() <= C_HINT_MAX_INDEX);
        }
    }

    #[test]
    fn a_signature_round_trips_through_the_layout() {
        for set in CommittedSet::ALL {
            let sample = prepared(set);
            let decoded = match set {
                CommittedSet::P248 => {
                    Response::<P248>::decode(&sample.signature).map(|r| r.encode())
                }
                CommittedSet::P376 => {
                    Response::<P376>::decode(&sample.signature).map(|r| r.encode())
                }
                CommittedSet::P500 => {
                    Response::<P500>::decode(&sample.signature).map(|r| r.encode())
                }
            };
            assert_eq!(decoded, Ok(sample.signature.clone()), "{set}");
        }
    }

    #[test]
    fn a_carried_salt_reaches_the_real_relation_without_a_search() {
        let sample = prepared(CommittedSet::P248);
        assert_eq!(
            verify_set(
                Set::P248,
                &sample.public_key,
                sample.message,
                &sample.signature
            ),
            Err(VerifyError::Kernel)
        );
        let mut response = Response::<P248>::decode(&sample.signature).unwrap();
        let mut salt = response.salt.as_bytes().to_vec();
        salt[0] ^= 1;
        response.salt = <P248 as ParameterSet>::Salt::from_bytes(&salt).unwrap();
        let id = KeyIdentity::<P248>::decode(&sample.public_key).unwrap();
        let prime =
            ChallengeScalar::derive(&id, sample.message, response.salt).degree_is_prime::<P248>();
        assert_eq!(
            verify::<P248>(&sample.public_key, sample.message, &response.encode()),
            Err(if prime {
                VerifyError::Kernel
            } else {
                VerifyError::Composite
            })
        );
    }

    #[test]
    fn a_set_name_parses_or_is_its_own_error() {
        for (name, set) in [
            ("p248", Set::P248),
            ("p324", Set::P324),
            ("p376", Set::P376),
            ("p500", Set::P500),
            ("p664", Set::P664),
        ] {
            assert_eq!(name.parse::<Set>(), Ok(set), "{name}");
            assert_eq!(set.to_string(), name);
            // A name is not a label.
            assert_eq!(
                CommittedSet::from_label(name),
                Err(UnknownCommittedLabel),
                "{name}"
            );
        }
        assert_eq!(Set::ALL.len(), 5);
        for (label, committed, set) in [
            (P248::LABEL, CommittedSet::P248, Set::P248),
            (P376::LABEL, CommittedSet::P376, Set::P376),
            (P500::LABEL, CommittedSet::P500, Set::P500),
        ] {
            assert_eq!(CommittedSet::from_label(label), Ok(committed), "{label}");
            assert_eq!(committed.label(), label);
            assert_eq!(committed.set(), set);
            assert_eq!(set.committed(), Ok(committed));
            assert_eq!(committed.to_string(), set.to_string());
            // A label is not a name.
            assert_eq!(label.parse::<Set>(), Err(UnknownSet), "{label}");
        }
        assert_eq!(
            (P248::LABEL, P376::LABEL, P500::LABEL),
            ("NIST-I", "NIST-III", "NIST-V")
        );
        for text in ["", "248", "P248", "p1024", "1", "I", "iv"] {
            assert_eq!(text.parse::<Set>(), Err(UnknownSet), "{text}");
            assert_eq!(
                CommittedSet::from_label(text),
                Err(UnknownCommittedLabel),
                "{text}"
            );
        }
        // An unknown set is not a length refusal.
        assert_ne!(UnknownSet.to_string(), VerifyError::Length.to_string());
        // Each error names the strings its own parser accepts, and none of
        // the other's.
        for set in Set::ALL {
            assert!(UnknownSet.to_string().contains(set.name()));
            assert!(!UnknownCommittedLabel.to_string().contains(set.name()));
        }
        for set in CommittedSet::ALL {
            assert!(UnknownCommittedLabel.to_string().contains(set.label()));
            assert!(!UnknownSet.to_string().contains(set.label()));
        }
        assert_eq!(
            UnknownSet.to_string(),
            "not the name of a parameter set: expected p248, p324, p376, p500 or p664"
        );
    }

    /// `p324` and `p664` have no vectors, no records and no label. Asking
    /// for either as a set with committed data is its own error, which
    /// names the set, and every other set converts. The exhaustive match is the point: a set
    /// added later must be placed on one side here before this compiles.
    #[test]
    fn a_set_without_a_corpus_says_so_and_is_given_no_label() {
        for set in Set::ALL {
            match set {
                Set::P324 | Set::P664 => {
                    assert_eq!(set.committed(), Err(NoCorpus(set)));
                    assert!(NoCorpus(set).to_string().contains(set.name()));
                }
                Set::P248 | Set::P376 | Set::P500 => {
                    assert_eq!(set.committed().map(CommittedSet::set), Ok(set));
                }
            }
        }
        // No committed label, in either spelling a guess might take.
        for text in ["NIST-II", "p324", "P324", "p664", "P664", "NIST-VII"] {
            assert_eq!(CommittedSet::from_label(text), Err(UnknownCommittedLabel));
        }
        assert_ne!(
            NoCorpus(Set::P324).to_string(),
            UnknownCommittedLabel.to_string()
        );
    }

    #[test]
    fn two_thousand_forty_seven_is_not_a_prime() {
        let is_prime = is_baillie_psw::<<P248 as ParameterSet>::Degree>;
        assert!(!is_prime(&2047u32.to_le_bytes()[..2]));
        assert!(is_prime(&1019u32.to_le_bytes()[..2]));
        assert!(!is_prime(&[1]));
        assert!(!is_prime(&[91]));
        assert!(!is_prime(&[49, 2]));
    }

    #[test]
    fn the_three_field_primes_pass_baillie_psw() {
        // Each at its own set's width, which holds `p` with room: the
        // width is for a degree of about twice the bits.
        fn passes<L: ParameterSet>(cofactor: u128) -> bool {
            is_baillie_psw::<L::Degree>(&prime_bytes(cofactor, L::E))
        }
        assert!(passes::<P248>(5));
        assert!(passes::<P376>(65));
        assert!(passes::<P500>(27));
    }

    /// `cofactor·2^shift − 1`, little-endian.
    fn prime_bytes(cofactor: u128, shift: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; (shift as usize) / 8 + 17];
        let byte = (shift as usize) / 8;
        let bit = shift % 8;
        for (index, value) in cofactor.to_le_bytes().iter().enumerate() {
            let shifted = u16::from(*value) << bit;
            bytes[index + byte] |= shifted as u8;
            if let Some(slot) = bytes.get_mut(index + byte + 1) {
                *slot |= (shifted >> 8) as u8;
            }
        }
        let mut borrow = true;
        for slot in &mut bytes {
            if !borrow {
                break;
            }
            let (value, wrapped) = slot.overflowing_sub(1);
            *slot = value;
            borrow = wrapped;
        }
        while bytes.last() == Some(&0) {
            bytes.pop();
        }
        bytes
    }
}
