// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Algorithm 3 steps 3–8.
//!
//! Two chains, both matching `theta_chain_compute_and_eval`. The longer has
//! length `e` and kernel `(P, P_sig), (Q, Q_sig)`. The shorter has length
//! `e − 1` and kernel the doubles. Each chain is evaluated at `(R, 0)`,
//! `(T, 0)` and `(R−T, 0)`, where `(R, T)` is a basis of `E_vk[p+1]`.
//!
//! Using `P_sig` rather than `−P_sig` replaces the kernel by its image under
//! `ι(X, Y) = (X, −Y)`. `ι` fixes every point `(T, 0)`, so the three
//! evaluated points do not see the sign. `docs/VERIFIER_SPEC.md` records this.
//!
//! The chains may order the product factors differently and may return `A`
//! or `−A`. Each factor is normalized so the encoding of `A` is at most the
//! encoding of `−A`, which negates that factor's x-coordinates, and the
//! comparison accepts either order of the factors.

use prism_field::{Field, Level};
use prism_isogeny::chain::{chain, ChainResult};
use prism_isogeny::gluing::{Couple, Kernel};
use prism_isogeny::jac::{add, to_xz, Jac};
use prism_isogeny::montgomery::{a24, ladder, Montgomery, Xz};
use prism_isogeny::torsion::{
    double_times, even_parts_generate, rational_basis, starting_basis, RationalBasis,
};

use crate::basis::basis_hint;
use crate::challenge::ChallengeScalar;
use crate::identity::KeyIdentity;
use crate::signature::{AffinePoint, Response};
use crate::VerifyError;

/// The isogeny relation for a key the public checks have already accepted.
pub fn relate<L: Level>(
    identity: &KeyIdentity<'_, L>,
    response: &Response<L>,
    scalar: &ChallengeScalar,
) -> Result<(), VerifyError> {
    relate_points::<L>(
        identity.coefficient(),
        response.curve,
        response.p,
        response.q,
        scalar.as_bytes(),
    )
}

/// The same relation, including the starting curve. `verify` never calls
/// this with a zero key: that key is [`VerifyError::Origin`] first. The
/// `E0` vectors call it directly.
pub fn relate_points<L: Level>(
    vk: L::Ext,
    sig: L::Ext,
    p_sig: AffinePoint<L>,
    q_sig: AffinePoint<L>,
    scalar: &[u8],
) -> Result<(), VerifyError> {
    relate_len::<L>(vk, sig, p_sig, q_sig, scalar, L::E)
}

pub(crate) fn relate_len<L: Level>(
    vk: L::Ext,
    sig: L::Ext,
    p_sig: AffinePoint<L>,
    q_sig: AffinePoint<L>,
    scalar: &[u8],
    steps: u32,
) -> Result<(), VerifyError> {
    let basis = if vk.is_zero() {
        starting_basis::<L>()
    } else {
        let hint = basis_hint::<L>(vk)?;
        rational_basis::<L>(vk, hint.index(), hint.coefficient_is_square())
    }
    .ok_or(VerifyError::Kernel)?;
    if !on_curve::<L>(sig, p_sig.x, p_sig.y) || !on_curve::<L>(sig, q_sig.x, q_sig.y) {
        return Err(VerifyError::Kernel);
    }
    let p_sig_x = Xz::affine(p_sig.x);
    let q_sig_x = Xz::affine(q_sig.x);
    if !order_two_pow::<L>(sig, p_sig_x) || !order_two_pow::<L>(sig, q_sig_x) {
        return Err(VerifyError::Kernel);
    }
    let signature_difference = difference::<L>(sig, p_sig.x, p_sig.y, q_sig.x, q_sig.y);
    let kernel = Kernel {
        t1: Couple {
            first: basis.even.p,
            second: p_sig_x,
        },
        t2: Couple {
            first: basis.even.q,
            second: q_sig_x,
        },
        difference: Couple {
            first: basis.even.difference,
            second: signature_difference,
        },
    };
    if !even_parts_generate::<L>(sig, p_sig_x, q_sig_x) {
        return Err(VerifyError::Kernel);
    }
    let curves = [Montgomery::normalized(vk), Montgomery::normalized(sig)];
    let long_points = evaluated::<L>(vk, &basis, scalar, true);
    let short_points = evaluated::<L>(vk, &basis, scalar, false);
    let left = chain(steps, curves, kernel, false, &long_points, L::ext_i(), true)
        .ok_or(VerifyError::Kernel)?;
    let right = chain(
        steps - 1,
        curves,
        double_kernel::<L>(vk, sig, kernel),
        false,
        &short_points,
        L::ext_i(),
        true,
    )
    .ok_or(VerifyError::Kernel)?;
    if images_agree::<L>(&left, &right) {
        Ok(())
    } else {
        Err(VerifyError::Image)
    }
}

fn order_two_pow<L: Level>(coefficient: L::Ext, point: Xz<L::Ext>) -> bool {
    let full = double_times::<L>(coefficient, point, L::E);
    let almost = double_times::<L>(coefficient, point, L::E - 1);
    full.z.is_zero() && !almost.z.is_zero()
}

fn evaluated<L: Level>(
    vk: L::Ext,
    basis: &RationalBasis<L::Ext>,
    scalar: &[u8],
    long: bool,
) -> [Couple<L::Ext>; 3] {
    let prepare = |point: Xz<L::Ext>| {
        let first = if long {
            double_times::<L>(vk, point, L::E - 1)
        } else {
            scale::<L>(vk, point, scalar)
        };
        Couple {
            first,
            second: Xz::infinity(),
        }
    };
    [
        prepare(basis.r),
        prepare(basis.t),
        prepare(basis.difference),
    ]
}

fn on_curve<L: Level>(coefficient: L::Ext, x: L::Ext, y: L::Ext) -> bool {
    let square = x.mul(x);
    y.mul(y)
        .same(square.mul(x).add(coefficient.mul(square)).add(x))
}

fn difference<L: Level>(
    coefficient: L::Ext,
    px: L::Ext,
    py: L::Ext,
    qx: L::Ext,
    qy: L::Ext,
) -> Xz<L::Ext> {
    let curve = Montgomery::normalized(coefficient);
    to_xz(add(
        curve,
        Jac {
            x: px,
            y: py,
            z: L::Ext::one(),
        },
        Jac {
            x: qx,
            y: qy.negate(),
            z: L::Ext::one(),
        },
    ))
}

fn double_kernel<L: Level>(vk: L::Ext, sig: L::Ext, kernel: Kernel<L::Ext>) -> Kernel<L::Ext> {
    let once = |curve, point: Xz<L::Ext>| double_times::<L>(curve, point, 1);
    Kernel {
        t1: Couple {
            first: once(vk, kernel.t1.first),
            second: once(sig, kernel.t1.second),
        },
        t2: Couple {
            first: once(vk, kernel.t2.first),
            second: once(sig, kernel.t2.second),
        },
        difference: Couple {
            first: once(vk, kernel.difference.first),
            second: once(sig, kernel.difference.second),
        },
    }
}

fn scale<L: Level>(coefficient: L::Ext, point: Xz<L::Ext>, scalar: &[u8]) -> Xz<L::Ext> {
    let Some(constant) = a24(coefficient) else {
        return Xz::infinity();
    };
    let mut bits = Vec::new();
    for byte in scalar.iter().rev() {
        for shift in (0..8).rev() {
            bits.push((byte >> shift) & 1 == 1);
        }
    }
    while bits.first() == Some(&false) {
        bits.remove(0);
    }
    if bits.is_empty() {
        return Xz::infinity();
    }
    ladder(constant, bits.into_iter(), point)
}

struct Factor<F> {
    curve: Montgomery<F>,
    points: [Xz<F>; 3],
}

fn canonical_factor<L: Level>(
    curve: Montgomery<L::Ext>,
    points: [Xz<L::Ext>; 3],
) -> Factor<L::Ext> {
    let Some(inverse) = curve.c.inv() else {
        return Factor { curve, points };
    };
    let affine = curve.a.mul(inverse);
    let negative = affine.negate();
    if L::ext_encode(negative) < L::ext_encode(affine) {
        Factor {
            curve: Montgomery::normalized(negative),
            points: points.map(|point| Xz {
                x: point.x.negate(),
                z: point.z,
            }),
        }
    } else {
        Factor {
            curve: Montgomery::normalized(affine),
            points,
        }
    }
}

fn side<L: Level>(result: &ChainResult<L::Ext>) -> [Factor<L::Ext>; 2] {
    let points = |which: usize| {
        [
            if which == 0 {
                result.images[0].first
            } else {
                result.images[0].second
            },
            if which == 0 {
                result.images[1].first
            } else {
                result.images[1].second
            },
            if which == 0 {
                result.images[2].first
            } else {
                result.images[2].second
            },
        ]
    };
    [
        canonical_factor::<L>(result.codomain[0], points(0)),
        canonical_factor::<L>(result.codomain[1], points(1)),
    ]
}

fn factor_agrees<L: Level>(left: &Factor<L::Ext>, right: &Factor<L::Ext>) -> bool {
    left.curve.a.same(right.curve.a)
        && left
            .points
            .iter()
            .zip(right.points)
            .all(|(left_point, right_point)| {
                left_point
                    .x
                    .mul(right_point.z)
                    .same(right_point.x.mul(left_point.z))
            })
}

fn images_agree<L: Level>(left: &ChainResult<L::Ext>, right: &ChainResult<L::Ext>) -> bool {
    if left.images.len() != 3 || right.images.len() != 3 {
        return false;
    }
    let left = side::<L>(left);
    let right = side::<L>(right);
    (factor_agrees::<L>(&left[0], &right[0]) && factor_agrees::<L>(&left[1], &right[1]))
        || (factor_agrees::<L>(&left[0], &right[1]) && factor_agrees::<L>(&left[1], &right[0]))
}

#[cfg(test)]
mod e0_vectors {
    use super::relate_points;
    use crate::signature::{canonical_sign, AffinePoint};
    use crate::verify;
    use crate::VerifyError;
    use prism_field::{Field, Level, Level1};
    use prism_isogeny::endomorphism::{apply, multiply};
    use prism_isogeny::jac::{lift_basis, Jac};
    use prism_isogeny::montgomery::Montgomery;
    use prism_isogeny::torsion::starting_basis;

    fn decimal(text: &str) -> Vec<u8> {
        let mut bytes = vec![0u8];
        for digit in text.bytes() {
            let mut carry = u16::from(digit - b'0');
            for byte in &mut bytes {
                carry += u16::from(*byte) * 10;
                *byte = carry as u8;
                carry >>= 8;
            }
            while carry > 0 {
                bytes.push(carry as u8);
                carry >>= 8;
            }
        }
        bytes
    }

    fn field<'a>(text: &'a str, label: &str) -> &'a str {
        text.lines()
            .find_map(|line| line.strip_prefix(label))
            .unwrap()
            .trim()
    }

    fn affine<L: Level>(point: Jac<L::Ext>) -> (L::Ext, L::Ext) {
        let z = point.z.inv().expect("finite");
        let z2 = z.mul(z);
        (point.x.mul(z2), point.y.mul(z2.mul(z)))
    }

    fn signature<L: Level>(block: &str) -> (AffinePoint<L>, AffinePoint<L>, Vec<u8>) {
        let basis = starting_basis::<L>().expect("E0 basis");
        let (_, p, q) = lift_basis(
            Montgomery::normalized(L::Ext::zero()),
            basis.even.p,
            basis.even.q,
            basis.even.difference,
        )
        .expect("lift");
        let a = decimal(field(block, "a "));
        let b = decimal(field(block, "b "));
        let t = decimal(field(block, "t "));
        let c: i64 = field(block, "c ").parse().unwrap();
        let d: i64 = field(block, "d ").parse().unwrap();
        let p = multiply::<L>(apply::<L>(p, &a, &b, c, d), &t);
        let q = multiply::<L>(apply::<L>(q, &a, &b, c, d), &t);
        let (px, py) = affine::<L>(p);
        let (qx, qy) = affine::<L>(q);
        let mut p: AffinePoint<L> = AffinePoint { x: px, y: py };
        let mut q: AffinePoint<L> = AffinePoint { x: qx, y: qy };
        if !canonical_sign::<L>(p.y) {
            p.y = p.y.negate();
            q.y = q.y.negate();
        }
        (p, q, decimal(field(block, "s ")))
    }

    fn exercise<L: Level>(block: &str) {
        let (p, q, s) = signature::<L>(block);
        let zero = L::Ext::zero();
        // The reference chain refuses this graph too: an endomorphism of norm
        // 2^{2e−1} − s^2 is not the kernel `fixed_degree_isogeny` evaluates.
        assert_eq!(
            relate_points::<L>(zero, zero, p, q, &s),
            Err(VerifyError::Kernel)
        );
    }

    #[test]
    fn a_prime_norm_endomorphism_of_the_starting_curve_does_not_split() {
        let text = include_str!("../testdata/e0_endomorphisms.txt");
        let block = text.split("NIST-").find(|block| !block.is_empty()).unwrap();
        exercise::<Level1>(block);
        for length in [260usize, 388, 516] {
            let err = verify::<Level1>(&[0u8; Level1::PUBLIC_KEY_LEN], b"m", &vec![0u8; length]);
            assert_eq!(err, Err(VerifyError::Length));
        }
    }
}
