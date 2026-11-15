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
//! This is its own file so that a test which has no committed vectors
//! (`p324`) can use it without the rest of the support module.

use prism_field::{Field, ParameterSet, Salt};
use prism_isogeny::jac::{add, lift_basis, Jac};
use prism_isogeny::montgomery::Montgomery;
use prism_isogeny::torsion::even_basis;
use prism_verify::signature::canonical_sign;
use prism_verify::{basis_hint, AffinePoint, ChallengeScalar, KeyIdentity, Response, VerifyError};

/// Which pair of inverse pairing answers for the basis `(P, Q)`.
#[derive(Clone, Copy, Debug)]
pub enum Form {
    /// `(P, −Q)`.
    NegatedQ,
    /// `(P, P − Q)`: `e(P, P − Q) = e(P, −Q)`.
    Difference,
}

impl Form {
    pub const ALL: [Self; 2] = [Self::NegatedQ, Self::Difference];

    /// The name of the case in the records.
    pub fn name(self) -> &'static str {
        match self {
            Self::NegatedQ => "identity-response-q-negated",
            Self::Difference => "identity-response-q-difference",
        }
    }

    /// The refusal this form meets, at every set and on every curve tried.
    /// Both lie past the pairing check.
    pub fn refusal(self) -> &'static [VerifyError] {
        match self {
            Self::NegatedQ => &[VerifyError::Kernel],
            Self::Difference => &[VerifyError::Codomain],
        }
    }
}

/// The key, the response on the key's own curve in the given form, and the
/// counter of the salt: the first counter whose challenge degree is prime
/// under this key and message.
///
/// `coefficient` must be a curve the verifier takes both as a key and as a
/// response curve: not the starting curve, not singular, the permitted
/// model of its class, with a basis index inside the bound. A response
/// curve of an honest signature is one, and so is a fixture curve that the
/// model check passes.
pub fn identity_response<L: ParameterSet>(
    coefficient: L::Ext,
    message: &[u8],
    form: Form,
) -> (Vec<u8>, Response<L>, u64) {
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
    let identity = KeyIdentity::<L>::decode(&key).expect("a key");
    let (counter, salt) = (0u64..65_536)
        .find_map(|counter| {
            let salt = L::Salt::from_counter(counter).expect("a counter salt");
            ChallengeScalar::derive(&identity, message, salt)
                .degree_is_prime::<L>()
                .then_some((counter, salt))
        })
        .expect("a prime degree within 65,536 salts");
    let response = Response {
        curve: coefficient,
        p,
        q,
        salt,
    };
    (key, response, counter)
}
