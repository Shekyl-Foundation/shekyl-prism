// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The isogeny engine of the PRISM verifier: Montgomery curves, their
//! `2^e`-torsion and Weil pairing, and chains of (2,2)-isogenies between
//! products of two curves, in theta coordinates.
//!
//! # What it is for
//!
//! A PRISM signature is a curve and two points that claim to be the
//! image of the key's `2^e`-torsion basis under an isogeny of a degree
//! the message fixes. The verifier (`prism-verify`) decides whether the
//! claim holds. Every curve operation it needs is here. This crate
//! decides nothing: it has no notion of a key, a signature or a verdict.
//!
//! # Read this before calling anything
//!
//! Most functions here are correct only under conditions the caller must
//! have established: that a point is on the curve it is used with, that
//! it has a given order, that two points span, that a curve's
//! `2^e`-torsion is rational. **Almost none of those conditions is
//! tested here.** A function whose assumption fails does one of three
//! things, and its documentation says which:
//!
//! - it returns `None`;
//! - it returns a wrong value that a later check will catch;
//! - it returns a wrong value that looks like a right one.
//!
//! Each public item states what it assumes, which of the three happens
//! otherwise, and which check in `prism-verify` makes the assumption
//! true before the verifier's call. That last part is a pointer for the
//! reader, not the contract: another caller has to establish the
//! assumption itself.
//!
//! Everything here is variable time and is for public values only.
//!
//! # The path of one verification
//!
//! `prism_verify::verify` calls these modules, in this order.
//!
//! 1. [`torsion`]: from the key's coefficient, the points the basis
//!    index names, with the odd cofactor cleared, and the canonical
//!    second point ([`torsion::even_basis`]). The verifier then tests
//!    that they, and the signature's two points, have order exactly
//!    `2^e` and are independent, with [`torsion::double_times`].
//! 2. [`jac`]: the key's basis lifted from x-coordinates to full points
//!    ([`jac::lift_basis`]), and the group law on full points that
//!    everything after uses.
//! 3. [`pairing`]: the signature's points multiplied by the challenge
//!    ([`pairing::mul`]), then the Weil pairing of each pair
//!    ([`pairing::weil`]) and the comparison `e(sP', sQ') = e(P, Q)^q`.
//! 4. [`modular`]: the challenge as an integer modulo `2^e`, and the two
//!    4×4 matrices whose columns are the coefficients of the two kernels
//!    ([`modular::challenge_matrices`]).
//! 5. [`dual_chain`] and [`theta_base`]: the 4-torsion of each curve put
//!    in canonical position ([`dual_chain::canonical_four`]), the theta
//!    null point of the product ([`dual_chain::product_null`]), and its
//!    change of basis for each kernel
//!    ([`theta_base::base_change_theta`]).
//! 6. [`dual_chain::DualChain`]: the two chains, of `e − 2` and `e − 3`
//!    steps; the test that they end on the same surface; and three
//!    points carried forward through the first and back through the
//!    dual of the second, to be compared with their expected images.
//!
//! Under those: [`montgomery`] is the x-only arithmetic step 1 runs on,
//! and [`theta`] is the theta point type the chains return. The
//! (2,2)-isogeny step itself and the theta doubling are private modules
//! that [`dual_chain`] drives.
//!
//! # What is not on that path
//!
//! - [`chain`], with [`gluing`] and a private splitting module: the C
//!   reference's x-only chain, which ends by splitting its codomain into
//!   two curves. It is compared with the pinned C tree on a committed
//!   chain, and the verifier's tests use it to record where a single
//!   chain stops on a signature of prime degree. The verifier does not
//!   call it.
//! - [`endomorphism`]: the action of an endomorphism on the starting
//!   curve, with which the tests build inputs that need no signer.
//! - [`torsion::rational_basis`] and [`torsion::starting_basis`]: a
//!   basis of the whole `(p+1)`-torsion. The verifier needs the
//!   `2^e`-torsion only.
//! - `pairing::oracle`, behind the feature `oracle`: the affine Miller
//!   loop that [`pairing::weil`] replaced, kept as its reference and
//!   compiled only for the comparison. Nothing shipped enables the
//!   feature.
//!
//! # Where this is in the paper
//!
//! The paper is ePrint 2026/443, in the repository as
//! `docs/papers/2026-443_PRISM_with _salt.pdf`. Its section 5 and
//! Algorithm 3 (`VerIsogenyStrong`) define the verification, and
//! section 2.2 (Kani's lemma) is why an isogeny between two curves can
//! be checked through a (2,2)-isogeny between products. Algorithm 3,
//! line 3, "compute `(P, Q) = E[2^e]`", is step 1 above. Its lines 4
//! and 5, "try to compute the isogeny", are steps 5 and 6. The paper
//! says what is computed there and not how, and it has no section for
//! theta coordinates, gluing or the pairing. Those are ports of
//! reference code, which each module names at the top of its file: the
//! C tree in `external/PRISM_v2` and the Sage package in
//! `external/Theta_dim4`. The relation this verifier checks in step 6
//! is not Algorithm 3's lines 4 to 8 as printed;
//! `docs/VERIFIER_SPEC.md`, "Acceptance relation", states it.
//!
//! # An example
//!
//! The first thing the verifier does with a signature is test its
//! points. Here that is done for one point of a committed signature at
//! `p248`: it is on the signature's curve, and it has order exactly
//! `2^e`.
//!
//! ```
//! use prism_field::{Field, ParameterSet, P248};
//! use prism_isogeny::montgomery::{Montgomery, Xz};
//! use prism_isogeny::{jac, torsion};
//! use prism_vectors::corpus::Primary;
//!
//! let vector = Primary::Lvl1.vector();
//! let element = |name: &'static str| {
//!     P248::ext_decode(&vector.bytes(name).unwrap()).unwrap()
//! };
//! // The signature's curve y² = x³ + A x² + x, and its first point.
//! let (a, x, y) = (element("E"), element("Px"), element("Py"));
//!
//! // On the curve: y is one of the two square roots of x³ + A x² + x.
//! let root = jac::recover_y(Montgomery::normalized(a), x).expect("x is on the curve");
//! assert!(root.same(y) || root.same(y.negate()));
//!
//! // Of order exactly 2^e: e doublings reach infinity, and e − 1 do not.
//! let point = Xz::affine(x);
//! assert!(torsion::double_times::<P248>(a, point, P248::E).is_infinity());
//! assert!(!torsion::double_times::<P248>(a, point, P248::E - 1).is_infinity());
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod chain;
pub mod dual_chain;
pub mod endomorphism;
pub mod gluing;
pub mod jac;
pub mod modular;
pub mod montgomery;
pub mod pairing;
mod splitting;
pub mod theta;
pub mod theta_base;
mod theta_step;
pub mod torsion;
#[cfg(test)]
mod weierstrass;
