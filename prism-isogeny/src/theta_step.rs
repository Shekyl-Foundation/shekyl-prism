// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// The (2,2) step in this file is a port of `theta_isogeny_compute`,
// `theta_isogeny_compute_4`, `theta_isogeny_compute_2`, `theta_isogeny_eval`,
// and `apply_isomorphism` in
// `external/PRISM_v2/src/hd/ref/lvlx/theta_isogenies.c` (Apache-2.0; Antonin
// Leroux and the SQIsign authors). See `NOTICE`.

//! One (2,2)-isogeny in theta coordinates.
//!
//! The verification chain asks for the 8-torsion form
//! ([`isogeny_compute`]). The 4-torsion and 2-torsion forms exist because
//! the C chain uses them when the kernel is not given with two extra
//! doublings. Signature verification in the pinned tree sets that flag, so
//! those two forms are not on its path. A missing square root is a refusal
//! here; the C signing path does not check the root it takes.

use prism_field::Field;

use crate::theta::{hadamard, to_squared_theta, ThetaPoint};

/// A 4×4 change of theta basis. Row `i` is the output coordinate.
#[derive(Clone, Copy, Debug)]
pub struct BasisChange<F> {
    pub rows: [[F; 4]; 4],
}

impl<F: Field> BasisChange<F> {
    pub fn identity() -> Self {
        let zero = F::zero();
        let one = F::one();
        Self {
            rows: [
                [one, zero, zero, zero],
                [zero, one, zero, zero],
                [zero, zero, one, zero],
                [zero, zero, zero, one],
            ],
        }
    }

    pub fn zero() -> Self {
        let zero = F::zero();
        Self {
            rows: [[zero; 4]; 4],
        }
    }
}

/// `M · P`. When `include_t` is false the `t` column is omitted, which is the
/// C path for a point whose `t` coordinate is known to be zero.
pub fn apply_isomorphism<F: Field>(
    matrix: &BasisChange<F>,
    point: ThetaPoint<F>,
    include_t: bool,
) -> ThetaPoint<F> {
    let coords = point.coords();
    let last = if include_t { 4 } else { 3 };
    let mut out = [F::zero(); 4];
    for (out_i, row) in out.iter_mut().zip(matrix.rows) {
        for (coord, entry) in coords.iter().take(last).zip(row) {
            *out_i = out_i.add(entry.mul(*coord));
        }
    }
    ThetaPoint::from_coords(out)
}

/// One (2,2)-step. `codomain` is its theta null point.
#[derive(Clone, Copy, Debug)]
pub struct IsogenyStep<F> {
    pub kernel_1: ThetaPoint<F>,
    pub kernel_2: ThetaPoint<F>,
    pub domain_hadamard: bool,
    pub codomain_hadamard: bool,
    pub precomputation: ThetaPoint<F>,
    pub codomain: ThetaPoint<F>,
}

fn prepare_kernel<F: Field>(point: ThetaPoint<F>, domain_hadamard: bool) -> ThetaPoint<F> {
    let point = if domain_hadamard {
        hadamard(point)
    } else {
        point
    };
    to_squared_theta(point)
}

/// `(2,2)`-isogeny of kernel `[4](T1, T2)` from points of order 8.
///
/// `domain_hadamard` selects dual coordinates on the domain and
/// `codomain_hadamard` selects them on the codomain, as the C booleans do.
/// `check_isotropy` is the verification chain's extra check that the
/// 4-torsion below the kernel is isotropic. A zero projective factor is a
/// refusal: the C comment records that as an unexpected split.
pub fn isogeny_compute<F: Field>(
    kernel_1: ThetaPoint<F>,
    kernel_2: ThetaPoint<F>,
    domain_hadamard: bool,
    codomain_hadamard: bool,
    check_isotropy: bool,
) -> Option<IsogenyStep<F>> {
    let first = prepare_kernel(kernel_1, domain_hadamard);
    let second = prepare_kernel(kernel_2, domain_hadamard);
    if second.coords().iter().any(|coord| coord.is_zero()) || first.x.is_zero() || first.y.is_zero()
    {
        return None;
    }

    let t1 = first.x.mul(second.y);
    let t2 = first.y.mul(second.x);
    let second_zt = second.z.mul(second.t);
    let mut codomain = ThetaPoint {
        x: second.x.mul(t1),
        y: second.y.mul(t2),
        z: second.z.mul(t1),
        t: second.t.mul(t2),
    };
    let precomputation = ThetaPoint {
        x: second_zt.mul(first.y),
        y: second_zt.mul(first.x),
        z: codomain.t,
        t: codomain.z,
    };

    if check_isotropy && !isotropic(first, second, precomputation) {
        return None;
    }
    if codomain_hadamard {
        codomain = hadamard(codomain);
    }
    Some(IsogenyStep {
        kernel_1,
        kernel_2,
        domain_hadamard,
        codomain_hadamard,
        precomputation,
        codomain,
    })
}

fn isotropic<F: Field>(first: ThetaPoint<F>, second: ThetaPoint<F>, pre: ThetaPoint<F>) -> bool {
    let pairs = [
        (first.x, pre.x, first.y, pre.y),
        (first.z, pre.z, first.t, pre.t),
        (second.x, pre.x, second.z, pre.z),
        (second.y, pre.y, second.t, pre.t),
    ];
    pairs
        .into_iter()
        .all(|(left_a, left_b, right_a, right_b)| left_a.mul(left_b).same(right_a.mul(right_b)))
}

/// `(2,2)`-isogeny from the 4-torsion above the kernel. Not used when the
/// chain is given two extra doublings, which is how verification calls it.
pub fn isogeny_compute_4<F: Field>(
    domain_null: ThetaPoint<F>,
    kernel: ThetaPoint<F>,
    domain_hadamard: bool,
    codomain_hadamard: bool,
) -> Option<IsogenyStep<F>> {
    let first = prepare_kernel(kernel, domain_hadamard);
    let second = prepare_kernel(domain_null, domain_hadamard);
    let sqrt_xy = second.x.mul(second.y).sqrt()?;
    let sqrt_xz = second.x.mul(second.z).sqrt()?;

    let mut codomain_y = sqrt_xy.mul(sqrt_xz);
    let pre_t = codomain_y.mul(first.z);
    codomain_y = codomain_y.mul(first.x);
    let codomain_t = first.z.mul(sqrt_xy).mul(second.x);
    let codomain_x_base = first.x.mul(second.x);
    let codomain_z = codomain_x_base.mul(second.z);
    let codomain_x = codomain_x_base.mul(sqrt_xz);

    let pre_x_base = first.x.mul(second.t);
    let pre_z = pre_x_base.mul(second.y).mul(sqrt_xz);
    let pre_y = pre_x_base.mul(second.z).mul(sqrt_xy);
    let pre_x = pre_x_base.mul(second.z).mul(second.y);
    let mut codomain = ThetaPoint {
        x: codomain_x,
        y: codomain_y,
        z: codomain_z,
        t: codomain_t,
    };
    if codomain_hadamard {
        codomain = hadamard(codomain);
    }
    Some(IsogenyStep {
        kernel_1: kernel,
        kernel_2: kernel,
        domain_hadamard,
        codomain_hadamard,
        precomputation: ThetaPoint {
            x: pre_x,
            y: pre_y,
            z: pre_z,
            t: pre_t.mul(second.y),
        },
        codomain,
    })
}

/// `(2,2)`-isogeny from the kernel itself. Same calling convention as
/// [`isogeny_compute_4`]: verification does not use it.
pub fn isogeny_compute_2<F: Field>(
    domain_null: ThetaPoint<F>,
    kernel: ThetaPoint<F>,
    domain_hadamard: bool,
    codomain_hadamard: bool,
) -> Option<IsogenyStep<F>> {
    let second = prepare_kernel(domain_null, domain_hadamard);
    let mut codomain = ThetaPoint {
        x: second.x,
        y: second.x.mul(second.y).sqrt()?,
        z: second.x.mul(second.z).sqrt()?,
        t: second.x.mul(second.t).sqrt()?,
    };
    let pre_x_base = second.z.mul(second.t);
    let precomputation = ThetaPoint {
        x: pre_x_base.mul(second.y),
        y: pre_x_base.mul(codomain.y),
        z: second.t.mul(codomain.z).mul(second.y),
        t: second.z.mul(codomain.t).mul(second.y),
    };
    if codomain_hadamard {
        codomain = hadamard(codomain);
    }
    Some(IsogenyStep {
        kernel_1: kernel,
        kernel_2: kernel,
        domain_hadamard,
        codomain_hadamard,
        precomputation,
        codomain,
    })
}

/// Image of one theta point under a computed step.
pub fn isogeny_eval<F: Field>(step: &IsogenyStep<F>, point: ThetaPoint<F>) -> ThetaPoint<F> {
    let mut image = if step.domain_hadamard {
        to_squared_theta(hadamard(point))
    } else {
        to_squared_theta(point)
    };
    let scale = step.precomputation.coords();
    let mut coords = image.coords();
    for (coord, factor) in coords.iter_mut().zip(scale) {
        *coord = coord.mul(factor);
    }
    image = ThetaPoint::from_coords(coords);
    if step.codomain_hadamard {
        hadamard(image)
    } else {
        image
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_isomorphism, isogeny_compute, BasisChange};
    use crate::theta::{projectively_same, ThetaPoint};
    use prism_field::Field;

    fn point<F: Field>(coords: [u64; 4]) -> ThetaPoint<F> {
        ThetaPoint::from_coords(coords.map(F::from_u64))
    }

    /// A stand-in field. Arithmetic is the integer ring modulo 1019, which is
    /// enough for the projective identities and is not a NIST prime.
    #[derive(Clone, Copy, Debug)]
    struct Toy(i64);
    const P: i64 = 1019;

    impl Toy {
        fn norm(self) -> i64 {
            self.0.rem_euclid(P)
        }
    }

    impl Field for Toy {
        fn zero() -> Self {
            Self(0)
        }
        fn one() -> Self {
            Self(1)
        }
        fn from_u64(v: u64) -> Self {
            Self((v % P as u64) as i64)
        }
        fn add(self, rhs: Self) -> Self {
            Self(self.norm() + rhs.norm())
        }
        fn sub(self, rhs: Self) -> Self {
            Self(self.norm() - rhs.norm())
        }
        fn square(self) -> Self {
            self.mul(self)
        }
        fn mul(self, rhs: Self) -> Self {
            Self(self.norm() * rhs.norm())
        }
        fn negate(self) -> Self {
            Self(-self.norm())
        }
        fn inv(self) -> Option<Self> {
            if self.norm() == 0 {
                None
            } else {
                Some(Self(mod_pow(self.norm(), P - 2)))
            }
        }
        fn sqrt(self) -> Option<Self> {
            if !self.is_square() {
                return None;
            }
            Some(Self(mod_pow(self.norm(), (P + 1) / 4)))
        }
        fn is_square(self) -> bool {
            let n = self.norm();
            n == 0 || mod_pow(n, (P - 1) / 2) == 1
        }
        fn is_zero(self) -> bool {
            self.norm() == 0
        }
        fn same(self, rhs: Self) -> bool {
            self.norm() == rhs.norm()
        }
    }

    fn mod_pow(mut base: i64, mut exp: i64) -> i64 {
        let mut acc = 1i64;
        base = base.rem_euclid(P);
        while exp > 0 {
            if exp % 2 == 1 {
                acc = (acc * base).rem_euclid(P);
            }
            base = (base * base).rem_euclid(P);
            exp /= 2;
        }
        acc
    }

    #[test]
    fn the_identity_matrix_fixes_a_point_and_scaling_a_kernel_does_not_move_the_codomain() {
        let matrix = BasisChange::<Toy>::identity();
        let value = point::<Toy>([3, 5, 7, 11]);
        assert!(projectively_same(
            value,
            apply_isomorphism(&matrix, value, true)
        ));
        assert!(isogeny_compute(
            point::<Toy>([0, 0, 0, 0]),
            point::<Toy>([0, 0, 0, 0]),
            false,
            false,
            false
        )
        .is_none());
        let first = point::<Toy>([2, 3, 5, 7]);
        let second = point::<Toy>([11, 13, 17, 19]);
        let step = isogeny_compute(first, second, false, false, false).expect("factors");
        let factor = Toy::from_u64(4);
        let scaled = ThetaPoint {
            x: second.x.mul(factor),
            y: second.y.mul(factor),
            z: second.z.mul(factor),
            t: second.t.mul(factor),
        };
        let scaled_step = isogeny_compute(first, scaled, false, false, false).expect("scaled");
        assert!(projectively_same(step.codomain, scaled_step.codomain));
    }
}
