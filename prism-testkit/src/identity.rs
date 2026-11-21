// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! A response built from a public key alone that passes the pairing check.
//!
//! The pairing check asks that `e([s]P_sig, [s]Q_sig) = e(P, Q)^(−s²)`,
//! where `(P, Q)` is the basis of `E[2^e]` the verifier derives from the
//! key. With `s` odd that is `e(P_sig, Q_sig) = e(P, Q)^(−1)`. Anyone can
//! meet it: take the key's own curve as the response curve and answer with
//! any pair whose pairing is the inverse of the basis's. No secret is
//! involved. What such a response does not have is an isogeny behind it,
//! so the verifier goes on past the pairing and refuses there. Where it
//! refuses depends on the pair, and [`Form`] names two:
//!
//! - `(P, −Q)` is refused as `Kernel`, at the start of the chain work;
//! - `(P, P − Q)` runs both chains and is refused as `Codomain`.
//!
//! Nothing here reads a committed vector, so a set that has none (`p324`,
//! `p664`) is served as well as one that has.

use prism_field::{Field, ParameterSet, Salt};
use prism_isogeny::jac::{add, lift_basis, Jac};
use prism_isogeny::montgomery::Montgomery;
use prism_isogeny::torsion::even_basis;
use prism_verify::signature::canonical_sign;
use prism_verify::{
    basis::basis_hint,
    challenge::ChallengeScalar,
    identity::KeyIdentity,
    signature::{AffinePoint, Response},
    VerifyError,
};

/// Which pair of inverse pairing answers for the basis `(P, Q)`.
#[derive(Clone, Copy, Debug)]
pub enum Form {
    /// `(P, −Q)`.
    NegatedQ,
    /// `(P, P − Q)`: `e(P, P − Q) = e(P, −Q)`.
    Difference,
}

impl Form {
    /// Both forms, in the order of the records.
    pub const ALL: [Self; 2] = [Self::NegatedQ, Self::Difference];

    /// The name of the case in the records.
    pub fn name(self) -> &'static str {
        match self {
            Self::NegatedQ => "identity-response-q-negated",
            Self::Difference => "identity-response-q-difference",
        }
    }

    /// The one refusal this form may meet, as a case of the negatives
    /// lists what it allows. It is the same at every set and curve tried.
    /// Both lie past the pairing check.
    pub fn allowed(self) -> &'static [VerifyError] {
        match self {
            Self::NegatedQ => &[VerifyError::Kernel],
            Self::Difference => &[VerifyError::Codomain],
        }
    }
}

/// How many salt counters a search for a prime degree tries. About one
/// degree in a hundred is prime at these sizes, so a search that reaches
/// this has found a broken hash or a broken primality test.
const SALT_COUNTERS: u64 = 65_536;

/// The first salt counter whose challenge degree is prime under `key` and
/// `message`. Anyone can run this search: it needs the key, not a secret.
///
/// # Panics
///
/// If `key` does not decode as a public key of `L`.
pub fn first_prime_salt_counter<L: ParameterSet>(key: &[u8], message: &[u8]) -> u64 {
    let identity = KeyIdentity::<L>::decode(key).expect("a key");
    (0..SALT_COUNTERS)
        .find(|&counter| {
            ChallengeScalar::derive(&identity, message, salt_of::<L>(counter))
                .degree_is_prime::<L>()
        })
        .unwrap_or_else(|| panic!("no prime degree within {SALT_COUNTERS} salts"))
}

/// The salt that encodes `counter`.
pub fn salt_of<L: ParameterSet>(counter: u64) -> L::Salt {
    L::Salt::from_counter(counter).expect("a counter fits every salt")
}

/// A response that passes the pairing check, built from a key alone.
pub struct IdentityResponse<L: ParameterSet> {
    /// The public key: the encoding of the coefficient.
    pub key: Vec<u8>,
    /// On the key's own curve, with the salt of
    /// [`first_prime_salt_counter`].
    pub response: Response<L>,
}

/// The response of the given form under `coefficient` as a key.
///
/// `coefficient` must be a curve the verifier takes both as a key and as a
/// response curve: not the starting curve, not singular, the permitted
/// model of its class, with a basis index inside the bound. A response
/// curve of an honest signature is one, and so is a fixture curve that the
/// model check passes.
///
/// # Panics
///
/// If `coefficient` is not such a curve.
pub fn identity_response<L: ParameterSet>(
    coefficient: L::Ext,
    message: &[u8],
    form: Form,
) -> IdentityResponse<L> {
    let key = L::ext_encode(coefficient);
    let hint = basis_hint::<L>(coefficient).expect("a basis index inside the bound");
    let even = even_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square())
        .expect("a basis of the even torsion");
    // The verifier's own basis as full points: `Q` is the cleared
    // difference, and `even.q` is `P − Q` for the lift.
    let (curve, p, q) = lift_basis(
        Montgomery::normalized(coefficient),
        even.p,
        even.difference,
        even.q,
    )
    .expect("the basis lifts to full points");
    let minus_q = Jac {
        x: q.x,
        y: q.y.negate(),
        z: q.z,
    };
    let second = match form {
        Form::NegatedQ => minus_q,
        Form::Difference => add(curve, p, minus_q),
    };
    let affine = |point: Jac<L::Ext>| {
        let inverse = point.z.inv().expect("not the point at infinity");
        let square = inverse.square();
        AffinePoint::<L> {
            x: point.x.mul(square),
            y: point.y.mul(square).mul(inverse),
        }
    };
    let (mut p, mut q) = (affine(p), affine(second));
    // Negating both points keeps the pairing, and gives `y(P)` the sign
    // the verifier asks for.
    if !canonical_sign::<L>(p.y) {
        p.y = p.y.negate();
        q.y = q.y.negate();
    }
    let salt = salt_of::<L>(first_prime_salt_counter::<L>(&key, message));
    IdentityResponse {
        key,
        response: Response {
            curve: coefficient,
            p,
            q,
            salt,
        },
    }
}
