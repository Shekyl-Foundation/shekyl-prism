// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// The chain driver is a port of `_theta_chain_compute_impl` in
// `external/PRISM_v2/src/hd/ref/lvlx/theta_isogenies.c` (Apache-2.0; Antonin
// Leroux and the SQIsign authors). See `NOTICE`. Randomisation is the
// signing path and is not ported. Repeated Jacobian doubling is `DBL` in a
// loop; the reference switches to a Weierstrass formula for long runs, which
// is the same group law.

//! A (2,2)-isogeny chain from an elliptic product back to an elliptic product.
//!
//! `steps` is the C argument `n`. With `extra_torsion`, the kernel points
//! have order `2^{n+2}` and every step is the 8-torsion formula. Without it,
//! they have order `2^n` and the last two steps are the 4-torsion and
//! 2-torsion formulas. Verification in the pinned tree sets the flag.
//! The codomain is refused unless it splits, and when the flag is set the
//! vanishing structure must be index 8.

use prism_field::Field;

use crate::gluing::{gluing_compute, gluing_eval, gluing_eval_special, Couple, CoupleJac, Kernel};
use crate::jac::{dbl_iter, lift_basis};
use crate::montgomery::Montgomery;
use crate::splitting::{elliptic_product, montgomery_couple, splitting_compute};
use crate::theta::{double_iter, ThetaConstants, ThetaPoint};
use crate::theta_step::{
    apply_isomorphism, isogeny_compute, isogeny_compute_2, isogeny_compute_4, isogeny_eval,
};

/// The split codomain and the images of the input points.
#[derive(Clone, Debug)]
pub struct ChainResult<F> {
    pub codomain: [Montgomery<F>; 2],
    pub images: Vec<Couple<F>>,
}

fn double_jac<F: Field>(
    curves: [Montgomery<F>; 2],
    point: CoupleJac<F>,
    times: u32,
) -> CoupleJac<F> {
    CoupleJac {
        first: dbl_iter(curves[0], point.first, times),
        second: dbl_iter(curves[1], point.second, times),
    }
}

fn one_factor_infinite<F: Field>(point: Couple<F>) -> bool {
    point.first.is_infinity() ^ point.second.is_infinity()
}

/// Evaluate a chain of `steps` (2,2)-isogenies.
///
/// `i` is the imaginary unit, which a splitting matrix may use. `verify`
/// enables the isotropy checks the reference's verify entry point sets.
pub fn chain<F: Field>(
    steps: u32,
    curves: [Montgomery<F>; 2],
    kernel: Kernel<F>,
    extra_torsion: bool,
    points: &[Couple<F>],
    i: F,
    verify: bool,
) -> Option<ChainResult<F>> {
    if points.iter().any(|point| !one_factor_infinite(*point)) {
        return None;
    }
    let extra = if extra_torsion { 2 } else { 0 };
    let start = steps.checked_add(extra)?.checked_sub(2)?;
    if start == 0 {
        return None;
    }
    let (curve_1, t1_1, t2_1) = lift_basis(
        curves[0],
        kernel.t1.first,
        kernel.t2.first,
        kernel.difference.first,
    )?;
    let (curve_2, t1_2, t2_2) = lift_basis(
        curves[1],
        kernel.t1.second,
        kernel.t2.second,
        kernel.difference.second,
    )?;
    let curves = [curve_1, curve_2];
    let mut jac_1 = vec![CoupleJac {
        first: t1_1,
        second: t1_2,
    }];
    let mut jac_2 = vec![CoupleJac {
        first: t2_1,
        second: t2_2,
    }];
    let mut todo = vec![start];
    let mut current = 0usize;
    while todo[current] != 1 {
        let previous = todo[current];
        let times = if previous >= 16 {
            previous / 2
        } else {
            previous.checked_sub(1)?
        };
        if times == 0 || times >= previous {
            return None;
        }
        current += 1;
        jac_1.push(double_jac(curves, jac_1[current - 1], times));
        jac_2.push(double_jac(curves, jac_2[current - 1], times));
        todo.push(previous - times);
    }

    let gluing = gluing_compute(curves, jac_1[current], jac_2[current], verify)?;
    let mut images = points
        .iter()
        .map(|point| gluing_eval_special(&gluing, *point))
        .collect::<Option<Vec<_>>>()?;
    let mut theta_1 = Vec::with_capacity(current + 4);
    let mut theta_2 = Vec::with_capacity(current + 4);
    for index in 0..current {
        theta_1.push(gluing_eval(&gluing, jac_1[index]));
        theta_2.push(gluing_eval(&gluing, jac_2[index]));
        todo[index] -= 1;
    }
    let mut null = gluing.codomain;
    let mut current = if current == 0 {
        None
    } else {
        Some(current - 1)
    };
    let mut which = 1u32;
    let mut last_step = None;

    while let Some(mut at) = current {
        if todo[at] == 0 {
            break;
        }
        while todo[at] != 1 {
            let previous = todo[at];
            let times = previous / 2;
            if times == 0 || times >= previous {
                return None;
            }
            at += 1;
            let constants = ThetaConstants::from_null(null);
            let image_1 = double_iter(constants, theta_1[at - 1], times);
            let image_2 = double_iter(constants, theta_2[at - 1], times);
            write_at(&mut theta_1, at, image_1);
            write_at(&mut theta_2, at, image_2);
            if todo.len() == at {
                todo.push(previous - times);
            } else {
                todo[at] = previous - times;
            }
        }
        let (domain_hadamard, codomain_hadamard, check) = if which == steps - 2 {
            (false, false, verify)
        } else if which == steps - 1 {
            (true, false, false)
        } else {
            (false, true, verify)
        };
        let step = isogeny_compute(
            theta_1[at],
            theta_2[at],
            domain_hadamard,
            codomain_hadamard,
            check,
        )?;
        for image in &mut images {
            *image = isogeny_eval(&step, *image);
        }
        null = step.codomain;
        if at > 0 {
            for index in 0..at {
                theta_1[index] = isogeny_eval(&step, theta_1[index]);
                theta_2[index] = isogeny_eval(&step, theta_2[index]);
                todo[index] -= 1;
            }
            current = Some(at - 1);
        } else {
            current = None;
        }
        last_step = Some(step);
        which += 1;
    }

    if !extra_torsion {
        if steps >= 3 {
            let step = last_step.as_ref()?;
            theta_1[0] = isogeny_eval(step, theta_1[0]);
            theta_2[0] = isogeny_eval(step, theta_2[0]);
        }
        let step = isogeny_compute_4(null, theta_1.first().copied()?, false, false)?;
        for image in &mut images {
            *image = isogeny_eval(&step, *image);
        }
        if !theta_1.is_empty() {
            theta_1[0] = isogeny_eval(&step, theta_1[0]);
            theta_2[0] = isogeny_eval(&step, theta_2[0]);
        }
        null = step.codomain;
        let step = isogeny_compute_2(null, theta_1.first().copied()?, true, false)?;
        for image in &mut images {
            *image = isogeny_eval(&step, *image);
        }
        null = step.codomain;
    }

    let expected = extra_torsion.then_some(8);
    let splitting = splitting_compute(null, i, expected)?;
    let product = elliptic_product(splitting.null_point)?;
    let images = images
        .into_iter()
        .map(|image| {
            let moved = apply_isomorphism(&splitting.matrix, image, true);
            let point = montgomery_couple(splitting.null_point, moved)?;
            Some(Couple {
                first: point.first,
                second: point.second,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ChainResult {
        codomain: [
            Montgomery {
                a: product.first.a,
                c: product.first.c,
            },
            Montgomery {
                a: product.second.a,
                c: product.second.c,
            },
        ],
        images,
    })
}

fn write_at<F: Field>(points: &mut Vec<ThetaPoint<F>>, index: usize, point: ThetaPoint<F>) {
    if points.len() == index {
        points.push(point);
    } else {
        points[index] = point;
    }
}
