// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Algorithm 3 steps 3–8.
//!
//! The `(2^e, 2^e)` isogeny and the `(2^{e−1}, 2^{e−1})` isogeny are two
//! calls to the chain that matches `theta_chain_compute_and_eval`. Step 8
//! holds when those calls land on the same product and take the doubled
//! basis point and the challenge multiple of it to the same image.

use prism_field::{Field, Level};
use prism_isogeny::chain::chain;
use prism_isogeny::gluing::{Couple, Kernel};
use prism_isogeny::jac::{add, to_xz, Jac};
use prism_isogeny::montgomery::{a24, ladder, Montgomery, Xz};
use prism_isogeny::torsion::{double_times, even_basis};

use crate::basis::basis_hint;
use crate::challenge::ChallengeScalar;
use crate::identity::KeyIdentity;
use crate::signature::Response;
use crate::VerifyError;

/// The isogeny relation. `Ok(())` is step 8 holding.
pub fn relate<L: Level>(
    identity: &KeyIdentity<'_, L>,
    response: &Response<L>,
    scalar: &ChallengeScalar,
) -> Result<(), VerifyError> {
    let coefficient = identity.coefficient();
    let hint = basis_hint::<L>(coefficient)?;
    let basis = even_basis::<L>(coefficient, hint.index(), hint.coefficient_is_square())
        .ok_or(VerifyError::Kernel)?;
    if !on_curve::<L>(response.curve, response.p.x, response.p.y)
        || !on_curve::<L>(response.curve, response.q.x, response.q.y)
    {
        return Err(VerifyError::Kernel);
    }
    let signature_difference = difference::<L>(
        response.curve,
        response.p.x,
        response.p.y,
        response.q.x,
        response.q.y,
    );
    let kernel = Kernel {
        t1: Couple {
            first: basis.p,
            second: Xz::affine(response.p.x),
        },
        t2: Couple {
            first: basis.q,
            second: Xz::affine(response.q.x),
        },
        difference: Couple {
            first: basis.difference,
            second: signature_difference,
        },
    };
    let curves = [
        Montgomery::normalized(coefficient),
        Montgomery::normalized(response.curve),
    ];
    let doubled = Couple {
        first: double_times::<L>(coefficient, basis.full_p, L::E - 1),
        second: Xz::infinity(),
    };
    let scaled = Couple {
        first: scale::<L>(coefficient, basis.full_p, scalar.as_bytes()),
        second: Xz::infinity(),
    };
    let left = chain(L::E, curves, kernel, false, &[doubled], L::ext_i(), true)
        .ok_or(VerifyError::Kernel)?;
    let right = chain(
        L::E - 1,
        curves,
        doubled_kernel::<L>(coefficient, response.curve, kernel),
        false,
        &[scaled],
        L::ext_i(),
        true,
    )
    .ok_or(VerifyError::Kernel)?;
    if same_curve(left.codomain[0], right.codomain[0])
        && same_curve(left.codomain[1], right.codomain[1])
        && same_point(left.images[0].first, right.images[0].first)
        && same_point(left.images[0].second, right.images[0].second)
    {
        Ok(())
    } else {
        Err(VerifyError::Image)
    }
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
    let sum = add(
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
    );
    to_xz(sum)
}

fn doubled_kernel<L: Level>(vk: L::Ext, sig: L::Ext, kernel: Kernel<L::Ext>) -> Kernel<L::Ext> {
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

fn same_curve<F: Field>(left: Montgomery<F>, right: Montgomery<F>) -> bool {
    left.a.mul(right.c).same(right.a.mul(left.c))
}

fn same_point<F: Field>(left: Xz<F>, right: Xz<F>) -> bool {
    left.x.mul(right.z).same(right.x.mul(left.z))
}
