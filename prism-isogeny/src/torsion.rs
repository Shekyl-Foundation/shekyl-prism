// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// The difference of two x-only points is the formula in
// `external/PRISM_v2/src/ec/ref/lvlx/basis.c` (`difference_point`).

//! A basis of `E[2^e]` from a Montgomery coefficient.
//!
//! The abscissa is the one the basis-index search names. Multiplying by the
//! odd cofactor `f` clears `p + 1 = f·2^e` down to order dividing `2^e`.
//! `Q` is the point above `(0, 0)`, `x(Q) = −x(P) − A`, and `P − Q` is the
//! deterministic difference the reference uses so a later lift can recover
//! `y`.

use prism_field::{Field, Level};

use crate::jac::xdbl_iter;
use crate::montgomery::{a24, ladder, Montgomery, Xz};

/// `(P, Q, P−Q)` in x-only coordinates, each of order dividing `2^e`.
#[derive(Clone, Copy, Debug)]
pub struct EvenBasis<F> {
    /// Order dividing `2^e`, after multiplication by `f`.
    pub p: Xz<F>,
    pub q: Xz<F>,
    pub difference: Xz<F>,
    /// The same points before that multiplication. Their order divides `p+1`.
    pub full_p: Xz<F>,
    pub full_q: Xz<F>,
}

fn conjugate<L: Level>(value: L::Ext) -> L::Ext {
    let (re, im) = L::ext_parts(value);
    L::ext_from_base(re).sub(L::ext_i().mul(L::ext_from_base(im)))
}

/// `x(P)` for the index the basis search settled on.
pub fn basis_abscissa<L: Level>(
    coefficient: L::Ext,
    index: u16,
    coefficient_is_square: bool,
) -> Option<L::Ext> {
    let index = u64::from(index);
    if coefficient_is_square {
        let z = L::Ext::one().add(L::ext_i().mul(L::Ext::from_u64(index)));
        Some(coefficient.mul(z.inv()?).negate())
    } else {
        Some(coefficient.mul(L::Ext::from_u64(index)))
    }
}

/// Deterministic `P − Q` on a normalized Montgomery curve, as `difference_point`.
pub fn difference<L: Level>(
    coefficient: L::Ext,
    p: Xz<L::Ext>,
    q: Xz<L::Ext>,
) -> Option<Xz<L::Ext>> {
    let curve = Montgomery::normalized(coefficient);
    let xx = p.x.mul(q.x);
    let zz = p.z.mul(q.z);
    let mut bxx = xx.sub(zz);
    bxx = bxx.mul(bxx).mul(curve.c);
    let xz = xx.add(zz);
    let px_qz = p.x.mul(q.z);
    let pz_qx = p.z.mul(q.x);
    let mut bzz = px_qz.sub(pz_qx);
    bzz = bzz.mul(bzz).mul(curve.c);
    let mut bxz = xz.mul(px_qz.add(pz_qx)).mul(curve.c);
    let cross = px_qz.mul(pz_qx).mul(curve.a);
    bxz = bxz.add(cross).add(cross);

    // Normalize by `C · conj(C)^2 · conj(P.z)^2 · conj(Q.z)^2`, which makes
    // the denominator a fourth power in `F_p`.
    let mut scale = conjugate::<L>(curve.c);
    scale = scale.mul(scale).mul(curve.c);
    let mut pz = conjugate::<L>(p.z);
    scale = scale.mul(pz.mul(pz));
    pz = conjugate::<L>(q.z);
    scale = scale.mul(pz.mul(pz));
    bxx = bxx.mul(scale);
    bxz = bxz.mul(scale);
    bzz = bzz.mul(scale);

    let disc = bxz.mul(bxz).sub(bxx.mul(bzz));
    let root = disc.sqrt()?;
    Some(Xz {
        x: bxz.add(root),
        z: bzz,
    })
}

fn clear_cofactor<L: Level>(coefficient: L::Ext, point: Xz<L::Ext>) -> Option<Xz<L::Ext>> {
    let constant = a24(coefficient)?;
    let width = usize::try_from(64 - L::ODD.leading_zeros()).unwrap_or(0);
    let bits = (0..width).rev().map(|shift| (L::ODD >> shift) & 1 == 1);
    Some(ladder(constant, bits, point))
}

/// The order-`2^e` basis named by `index`.
pub fn even_basis<L: Level>(
    coefficient: L::Ext,
    index: u16,
    coefficient_is_square: bool,
) -> Option<EvenBasis<L::Ext>> {
    let p = Xz::affine(basis_abscissa::<L>(
        coefficient,
        index,
        coefficient_is_square,
    )?);
    let q = Xz::affine(p.x.add(coefficient).negate());
    let difference = difference::<L>(coefficient, p, q)?;
    Some(EvenBasis {
        p: clear_cofactor::<L>(coefficient, p)?,
        q: clear_cofactor::<L>(coefficient, q)?,
        difference: clear_cofactor::<L>(coefficient, difference)?,
        full_p: p,
        full_q: q,
    })
}

/// `[2^times] point` on the normalized curve. Infinity is `Z = 0`.
pub fn double_times<L: Level>(coefficient: L::Ext, point: Xz<L::Ext>, times: u32) -> Xz<L::Ext> {
    xdbl_iter(Montgomery::normalized(coefficient), point, times)
}

#[cfg(test)]
mod tests {
    use super::even_basis;
    use crate::torsion::double_times;
    use prism_field::{Field, Level, Level1, Level3, Level5};

    fn key<L: Level>(hex: &str) -> L::Ext {
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
            .collect();
        L::ext_decode(&bytes).unwrap()
    }

    fn order_is_two_to_the_e<L: Level>(hex: &str, hint: u8) {
        let coefficient = key::<L>(hex);
        let basis =
            even_basis::<L>(coefficient, u16::from(hint >> 1), hint & 1 == 1).expect(L::NAME);
        let full = double_times::<L>(coefficient, basis.p, L::E);
        assert!(full.z.is_zero(), "{}", L::NAME);
        let almost = double_times::<L>(coefficient, basis.p, L::E - 1);
        assert!(!almost.z.is_zero(), "{}", L::NAME);
        let odd_part = double_times::<L>(coefficient, basis.full_p, L::E);
        assert!(!odd_part.z.is_zero(), "{}", L::NAME);
    }

    #[test]
    fn the_reference_keys_have_a_full_order_basis() {
        order_is_two_to_the_e::<Level1>(
            "e1bea27fb0d6c4f6ee3e8453aa04de40159a4504ccdcd20297e961a683f30003c9318e7f04701121a0d291ee5b1f2a65af8a9c8c514194517b402d4877f7bf01",
            11,
        );
        order_is_two_to_the_e::<Level3>(
            "6d8bff5340b0a6c2e120b56435381403147276e74ccc89fbfead5477453b8aba577d66678b65c320a6f89dab7a9ef3303990b85b491234294c5a5dc9c52396dcde9fa60e735339f2fd50c7df930fee6b313b6c85a7bba7c5da8bf4d90ee3eb2f",
            21,
        );
        order_is_two_to_the_e::<Level5>(
            "9e70ccc3c05b879b170848e55c9811a4146ef12a3c39fcac82a06dc1b4524badf0f8f07b569e0edb63835109563058271e3a92724195121fc00ce08354523801685d9220043cbe9565303c9e8dc6da07cffbddc23a8ccd0a221199fc62d0e7d30577671fb73fa36fb9e7641587c2a0d86213b16c038038e0389c51f647bc0200",
            2,
        );
    }
}
