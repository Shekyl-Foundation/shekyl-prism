// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Two adapted chains and the complete image check of the Sage model.
use crate::{AffinePoint, VerifyError};
use prism_field::{Field, Level};
use prism_isogeny::dual_chain::{canonical_four, product_null, Couple, DualChain};
use prism_isogeny::jac::{add, dbl_iter, lift_basis, to_xz, Jac};
use prism_isogeny::modular::{challenge_matrices, Mat4, Residue};
use prism_isogeny::montgomery::{Montgomery, Xz};
use prism_isogeny::pairing::{mul, weil};
use prism_isogeny::theta_base::base_change_theta;
use prism_isogeny::torsion::EvenBasis;

fn affine<L: Level>(point: AffinePoint<L>) -> Jac<L::Ext> {
    Jac {
        x: point.x,
        y: point.y,
        z: L::Ext::one(),
    }
}

fn negative<F: Field>(p: Jac<F>) -> Jac<F> {
    Jac {
        y: p.y.negate(),
        ..p
    }
}

fn kernel<F: Field>(
    curves: [Montgomery<F>; 2],
    basis: [Jac<F>; 4],
    matrix: Mat4,
    doubles: u32,
) -> [Couple<F>; 2] {
    let rows = matrix.rows();
    core::array::from_fn(|j| {
        let c = rows.map(|row| row[j + 2].to_le_bytes());
        core::array::from_fn(|i| {
            let point = add(
                curves[i],
                mul(curves[i], basis[i], &c[i]),
                mul(curves[i], basis[i + 2], &c[i + 2]),
            );
            dbl_iter(curves[i], point, doubles)
        })
    })
}

fn change<F: Field>(matrix: Mat4, m0: [[u8; 4]; 4], zeta: F) -> [[F; 4]; 4] {
    let rows = matrix.rows().map(|row| row.map(|v| v.to_le_bytes()[0] & 3));
    let result = core::array::from_fn(|i| {
        core::array::from_fn(|j| {
            (0..4)
                .map(|k| u16::from(m0[i][k]) * u16::from(rows[k][j]))
                .sum::<u16>() as u8
                & 3
        })
    });
    base_change_theta(result, zeta)
}

fn same_x<F: Field>(p: Xz<F>, q: Xz<F>) -> bool {
    !(p.x.is_zero() && p.z.is_zero() || q.x.is_zero() && q.z.is_zero())
        && p.x.mul(q.z).same(q.x.mul(p.z))
}

struct Built<F> {
    chains: [DualChain<F>; 2],
    curves: [Montgomery<F>; 2],
    pairs: [(Jac<F>, Jac<F>); 3],
}

fn build<L: Level>(
    vk: L::Ext,
    even: &EvenBasis<L::Ext>,
    sig: L::Ext,
    p: AffinePoint<L>,
    q: AffinePoint<L>,
    scalar: &[u8],
) -> Result<Built<L::Ext>, VerifyError> {
    let (_, p1, p2) = lift_basis(Montgomery::normalized(vk), even.p, even.difference, even.q)
        .ok_or(VerifyError::Kernel)?;
    let curves = [Montgomery::normalized(vk), Montgomery::normalized(sig)];
    let ps = affine(p);
    let qs = affine(q);
    let r1 = mul(curves[1], ps, scalar);
    let r2 = mul(curves[1], qs, scalar);
    let s = Residue::from_le_bytes(L::E, scalar);
    let q_inverse = s.multiply(s).negate().inv().ok_or(VerifyError::Kernel)?;
    let r2_scaled = mul(curves[1], r2, &q_inverse.to_le_bytes());
    let basis = [p1, r1, p2, r2_scaled];
    let (t1, mt) = canonical_four(
        curves[0],
        dbl_iter(curves[0], p1, L::E - 2),
        dbl_iter(curves[0], p2, L::E - 2),
    )
    .ok_or(VerifyError::Kernel)?;
    let (u1, mu) = canonical_four(
        curves[1],
        dbl_iter(curves[1], r1, L::E - 2),
        dbl_iter(curves[1], r2_scaled, L::E - 2),
    )
    .ok_or(VerifyError::Kernel)?;
    let zeta = weil(
        curves[0],
        dbl_iter(curves[0], p1, L::E - 2),
        dbl_iter(curves[0], p2, L::E - 2),
        2,
    )
    .ok_or(VerifyError::Kernel)?;
    let m0 = [
        [mt[0][0], 0, mt[1][0], 0],
        [0, mu[0][0], 0, mu[1][0]],
        [mt[0][1], 0, mt[1][1], 0],
        [0, mu[0][1], 0, mu[1][1]],
    ];
    let (_, m1, m2) = challenge_matrices(s).ok_or(VerifyError::Kernel)?;
    let product = product_null(t1, u1);
    let chain1 = DualChain::new(
        L::E - 2,
        curves,
        kernel(curves, basis, m1, 0),
        product,
        change(m1, m0, zeta),
    )
    .ok_or(VerifyError::Kernel)?;
    let chain2 = DualChain::new(
        L::E - 3,
        curves,
        kernel(curves, basis, m2, 1),
        product,
        change(m2, m0, zeta),
    )
    .ok_or(VerifyError::Kernel)?;
    Ok(Built {
        chains: [chain1, chain2],
        curves,
        pairs: [
            (p1, ps),
            (p2, qs),
            (
                add(curves[0], p1, negative(p2)),
                add(curves[1], ps, negative(qs)),
            ),
        ],
    })
}

pub(crate) fn check<L: Level>(
    vk: L::Ext,
    even: &EvenBasis<L::Ext>,
    sig: L::Ext,
    p: AffinePoint<L>,
    q: AffinePoint<L>,
    scalar: &[u8],
) -> Result<(), VerifyError> {
    let Built {
        chains: [chain1, chain2],
        curves,
        pairs,
    } = build::<L>(vk, even, sig, p, q, scalar)?;
    if !chain1.matches_dual(&chain2) {
        return Err(VerifyError::Codomain);
    }
    for (x, y) in pairs {
        let middle = chain1
            .eval([x, Jac::infinity()])
            .ok_or(VerifyError::Kernel)?;
        let image = chain2.dual_eval(middle).ok_or(VerifyError::Kernel)?;
        if !same_x(image[0], to_xz(mul(curves[0], x, scalar)))
            || !same_x(image[1], to_xz(mul(curves[1], y, scalar)))
        {
            return Err(VerifyError::Image);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{basis_hint, ChallengeScalar, KeyIdentity, Salt};
    use prism_field::{Level1, Level3, Level5};
    use prism_isogeny::{
        theta::{projectively_same, ThetaPoint},
        torsion::even_basis,
    };

    fn decimal<F: Field>(text: &str) -> F {
        let mut value = F::zero();
        for c in text.bytes() {
            value = value
                .mul(F::from_u64(10))
                .add(F::from_u64(u64::from(c - b'0')));
        }
        value
    }
    fn ext<L: Level>(text: &str) -> L::Ext {
        let text = text.replace(' ', "");
        if let Some((im, re)) = text.split_once("*i+") {
            L::ext_from_base(decimal::<L::Base>(im))
                .mul(L::ext_i())
                .add(L::ext_from_base(decimal::<L::Base>(re)))
        } else {
            L::ext_from_base(decimal::<L::Base>(&text))
        }
    }
    fn hex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .chunks_exact(2)
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect()
    }
    fn known<L: Level>(vector: &str, nulls: &str) {
        let field = |name: &str| {
            vector
                .lines()
                .find_map(|row| row.strip_prefix(name))
                .unwrap()
        };
        let key = hex(field("A "));
        let id = KeyIdentity::<L>::decode(&key).unwrap();
        let s = ChallengeScalar::derive(
            &id,
            field("message ").as_bytes(),
            Salt::<L>::from_bytes(&hex(field("salt "))).unwrap(),
        );
        let f = |name: &str| L::ext_decode(&hex(field(name))).unwrap();
        let hint = basis_hint::<L>(id.coefficient()).unwrap();
        let even =
            even_basis::<L>(id.coefficient(), hint.index(), hint.coefficient_is_square()).unwrap();
        let built = build::<L>(
            id.coefficient(),
            &even,
            f("E "),
            AffinePoint {
                x: f("Px "),
                y: f("Py "),
            },
            AffinePoint {
                x: f("Qx "),
                y: f("Qy "),
            },
            s.as_bytes(),
        )
        .unwrap();
        for (chain, key) in built.chains.iter().zip(["chain1", "chain2"]) {
            let section = nulls.split_once(&format!("\"{key}\"")).unwrap().1;
            let end = section.find("\"chain2\"").unwrap_or(section.len());
            let numbers: Vec<_> = section[..end]
                .split('"')
                .enumerate()
                .filter_map(|(i, s)| (i % 2 == 1).then_some(s))
                .collect();
            assert_eq!(numbers.len(), chain.nulls.len() * 4);
            for (step, (got, want)) in chain.nulls.iter().zip(numbers.chunks_exact(4)).enumerate() {
                let expected = ThetaPoint::from_coords(core::array::from_fn(|i| ext::<L>(want[i])));
                assert!(
                    projectively_same(*got, expected),
                    "{} {key} step {}",
                    L::NAME,
                    step + 1
                );
            }
        }
    }
    #[test]
    fn both_chains_match_every_sage_null_point() {
        known::<Level1>(
            include_str!("../testdata/s5_lvl1.txt"),
            include_str!("../../prism-isogeny/testdata/s5_model/s5_lvl1_nulls.json"),
        );
        known::<Level1>(
            include_str!("../testdata/s5_lvl1b.txt"),
            include_str!("../../prism-isogeny/testdata/s5_model/s5_lvl1b_nulls.json"),
        );
        known::<Level3>(
            include_str!("../testdata/s5_lvl3.txt"),
            include_str!("../../prism-isogeny/testdata/s5_model/s5_lvl3_nulls.json"),
        );
        known::<Level3>(
            include_str!("../testdata/s5_lvl3b.txt"),
            include_str!("../../prism-isogeny/testdata/s5_model/s5_lvl3b_nulls.json"),
        );
        known::<Level5>(
            include_str!("../testdata/s5_lvl5.txt"),
            include_str!("../../prism-isogeny/testdata/s5_model/s5_lvl5_nulls.json"),
        );
        known::<Level5>(
            include_str!("../testdata/s5_lvl5b.txt"),
            include_str!("../../prism-isogeny/testdata/s5_model/s5_lvl5b_nulls.json"),
        );
    }
}
