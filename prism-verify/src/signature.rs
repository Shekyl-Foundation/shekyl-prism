// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The signature's byte layout, and the sign normalization of `y(P_sig)`.

use core::cmp::Ordering;

use prism_field::{ChallengeIndex, Field, Level};

use crate::VerifyError;

/// Position of each field element in the signature's prefix.
///
/// The layout is the table in `docs/VERIFIER_SPEC.md`:
/// `A_sig ‖ x(P) ‖ y(P) ‖ x(Q) ‖ y(Q) ‖ j`.
mod at {
    pub(super) const CURVE: usize = 0;
    pub(super) const P_X: usize = 1;
    pub(super) const P_Y: usize = 2;
    pub(super) const Q_X: usize = 3;
    pub(super) const Q_Y: usize = 4;
    /// Field elements in the layout. `Level::SIGNATURE_ELEMENTS` agrees with
    /// this, which [`super::Response::decode`] asserts at compile time.
    pub(super) const COUNT: usize = 5;
}

/// A point of the signature's basis image, in affine coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AffinePoint<L: Level> {
    pub x: L::Ext,
    pub y: L::Ext,
}

/// A decoded signature.
///
/// Every field is canonical, and `p.y` has passed the sign normalization.
/// `curve`, `p.x` and `q` are read by Algorithm 3 steps 3–8, which
/// `docs/VERIFIER_SPEC.md` specifies and this crate does not yet run; they
/// are decoded either way, because a non-canonical element anywhere in the
/// signature is a refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Response<L: Level> {
    /// `A_sig`, the Montgomery coefficient of `E_sig`.
    pub curve: L::Ext,
    pub p: AffinePoint<L>,
    pub q: AffinePoint<L>,
    /// The challenge index the signer searched for.
    pub index: ChallengeIndex,
}

impl<L: Level> Response<L> {
    /// Decode `signature`: `Length`, then `Canonical`.
    pub fn decode(signature: &[u8]) -> Result<Self, VerifyError> {
        const {
            assert!(
                L::SIGNATURE_ELEMENTS == at::COUNT,
                "the level's element count is not this module's layout"
            );
        }
        if signature.len() != L::SIGNATURE_LEN {
            return Err(VerifyError::Length);
        }
        Ok(Self {
            curve: element::<L>(signature, at::CURVE)?,
            p: AffinePoint {
                x: element::<L>(signature, at::P_X)?,
                y: element::<L>(signature, at::P_Y)?,
            },
            q: AffinePoint {
                x: element::<L>(signature, at::Q_X)?,
                y: element::<L>(signature, at::Q_Y)?,
            },
            index: index::<L>(signature)?,
        })
    }

    /// The bytes [`Self::decode`] accepts. The layout lives in one place, so
    /// a sample built here and a signature read off the wire agree.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(L::SIGNATURE_LEN);
        for element in [self.curve, self.p.x, self.p.y, self.q.x, self.q.y] {
            out.extend_from_slice(&L::ext_encode(element));
        }
        out.extend_from_slice(&self.index.to_canonical_bytes());
        out
    }
}

fn element<L: Level>(signature: &[u8], position: usize) -> Result<L::Ext, VerifyError> {
    let at = position * L::FP2_LEN;
    signature
        .get(at..at + L::FP2_LEN)
        .and_then(L::ext_decode)
        .ok_or(VerifyError::Canonical)
}

fn index<L: Level>(signature: &[u8]) -> Result<ChallengeIndex, VerifyError> {
    let at = L::SIGNATURE_ELEMENTS * L::FP2_LEN;
    let bytes: [u8; ChallengeIndex::WIDTH] = signature
        .get(at..)
        .and_then(|tail| tail.try_into().ok())
        .ok_or(VerifyError::Length)?;
    Ok(ChallengeIndex::from_canonical_bytes(bytes))
}

/// `true` when `y` is the representative Algorithm 2 line 8 normalizes to.
///
/// `y = 0` is an order-2 point: it cannot be a basis image, and it is its own
/// negation, so no inequality would separate it.
pub fn canonical_sign<L: Level>(y: L::Ext) -> bool {
    !y.is_zero() && ext_less::<L>(y, y.negate())
}

/// `<_{F_{p^2}}`: the real part as an integer in `[0, p)`, then the
/// imaginary part.
pub fn ext_less<L: Level>(left: L::Ext, right: L::Ext) -> bool {
    let (left_re, left_im) = L::ext_parts(left);
    let (right_re, right_im) = L::ext_parts(right);
    match little_endian_cmp(&L::base_encode(left_re), &L::base_encode(right_re)) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => {
            little_endian_cmp(&L::base_encode(left_im), &L::base_encode(right_im)) == Ordering::Less
        }
    }
}

/// Integer order of two little-endian byte strings of the same length.
fn little_endian_cmp(left: &[u8], right: &[u8]) -> Ordering {
    for (left, right) in left.iter().rev().zip(right.iter().rev()) {
        match left.cmp(right) {
            Ordering::Equal => (),
            unequal => return unequal,
        }
    }
    Ordering::Equal
}
