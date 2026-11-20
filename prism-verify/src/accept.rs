// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Two adapted chains and the complete image check of the Sage model.
use crate::mark::Stage;
use crate::relation::KeyBasis;
use crate::{signature::AffinePoint, VerifyError};
use prism_field::{Field, ParameterSet};
use prism_isogeny::dual_chain::{canonical_four, product_null, Couple, DualChain};
use prism_isogeny::jac::{add, dbl, dbl_iter, lift_basis, to_xz, Jac};
use prism_isogeny::modular::{challenge_matrices, Mat4, Residue};
use prism_isogeny::montgomery::{Montgomery, Xz};
use prism_isogeny::pairing::{mul, weil};
use prism_isogeny::theta_base::base_change_theta;

fn affine<L: ParameterSet>(point: AffinePoint<L>) -> Jac<L::Ext> {
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

/// `[2^k] X` for every `k` a scalar of this set can use.
///
/// The ladders of one verification share their base points: each kernel
/// generator is a combination of the same four, and the image check scales
/// two of them again. Doubling each base point once and adding from the
/// table replaces a run of doublings per ladder. `times` adds the entries
/// in the order [`mul`] meets them, so it returns the same coordinates.
///
/// Verifier only, variable time: the additions made depend on the scalar.
/// Never call with a secret scalar.
struct Doublings<F> {
    curve: Montgomery<F>,
    table: Vec<Jac<F>>,
}

impl<F: Field> Doublings<F> {
    /// `bits` is the width of the widest scalar, rounded up to a byte.
    fn new(curve: Montgomery<F>, point: Jac<F>, bits: u32) -> Self {
        let mut table = Vec::with_capacity(bits as usize);
        let mut addend = point;
        for _ in 0..bits {
            table.push(addend);
            addend = dbl(curve, addend);
        }
        Self { curve, table }
    }

    /// `[2^k] X`.
    fn doubled(&self, k: u32) -> Jac<F> {
        self.table[k as usize]
    }

    /// `[scalar] X`, `scalar` little-endian. A zero scalar costs nothing.
    fn times(&self, scalar: &[u8]) -> Jac<F> {
        let mut result = Jac::infinity();
        for (place, byte) in scalar.iter().enumerate() {
            for bit in 0..8 {
                if byte >> bit & 1 == 1 {
                    let Some(entry) = self.table.get(place * 8 + bit) else {
                        // A set bit above the table. No caller has one, and
                        // the ladder answers for a scalar of any width.
                        return mul(self.curve, self.table[0], scalar);
                    };
                    result = add(self.curve, result, *entry);
                }
            }
        }
        result
    }
}

/// The two columns of `matrix` that name a chain's kernel, as scalars.
fn columns<L: ParameterSet>(matrix: Mat4<L>) -> [[Vec<u8>; 4]; 2] {
    let rows = matrix.rows();
    core::array::from_fn(|j| rows.map(|row| row[j + 2].to_le_bytes()))
}

/// Whether `scalar`, little-endian, is `2^bits − 1`.
fn is_minus_one(scalar: &[u8], bits: u32) -> bool {
    let (full, rest) = ((bits / 8) as usize, bits % 8);
    let top = if rest == 0 { 0 } else { 0xffu8 >> (8 - rest) };
    scalar.len() >= full + usize::from(rest != 0)
        && scalar[..full].iter().all(|byte| *byte == 0xff)
        && scalar[full..]
            .iter()
            .enumerate()
            .all(|(place, byte)| *byte == if place == 0 { top } else { 0 })
}

/// The chain kernel: two generators, each a couple of points, one on each
/// curve. `basis` is `[P₁, R₁, P₂, R₂′]`, the first of each pair on the
/// key's curve and the second on the response's.
///
/// Each basis point must have order dividing `2^order_bits`. The order
/// checks before the pairing establish that for the key's basis and the
/// signature points, and the other two are multiples of those. A
/// coefficient that is `2^order_bits − 1` is then `−1`, and its term is
/// the negated point rather than a sum of every table entry. Which
/// coefficients are `0`, `1` or `−1` is read off each scalar, never
/// assumed from its place in the matrix; a zero adds nothing and a one
/// adds the point once.
///
/// Verifier only, variable time. Never call with a secret scalar.
fn kernel<F: Field>(
    curves: [Montgomery<F>; 2],
    basis: &[Doublings<F>; 4],
    columns: &[[Vec<u8>; 4]; 2],
    doubles: u32,
    order_bits: u32,
) -> [Couple<F>; 2] {
    let term = |table: &Doublings<F>, scalar: &[u8]| {
        if is_minus_one(scalar, order_bits) {
            negative(table.doubled(0))
        } else {
            table.times(scalar)
        }
    };
    core::array::from_fn(|j| {
        let c = &columns[j];
        core::array::from_fn(|i| {
            let point = add(
                curves[i],
                term(&basis[i], &c[i]),
                term(&basis[i + 2], &c[i + 2]),
            );
            dbl_iter(curves[i], point, doubles)
        })
    })
}

/// The kernel by one full ladder per coefficient, as it was built before
/// the ladders shared their doublings. The reference for [`kernel`].
#[cfg(test)]
fn kernel_by_ladders<F: Field>(
    curves: [Montgomery<F>; 2],
    basis: [Jac<F>; 4],
    columns: &[[Vec<u8>; 4]; 2],
    doubles: u32,
) -> [Couple<F>; 2] {
    core::array::from_fn(|j| {
        let c = &columns[j];
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

/// The theta base change of one chain: `M0·M` modulo 4, at `zeta`.
///
/// `None` when [`base_change_theta`] finds no usable shift. No signature
/// reaches that: `zeta` is the Weil pairing of an order-four basis, so
/// `zeta⁴ = 1`, and the search fails only when `zeta² = −3`. Separately,
/// the tests below try each of the 256 matrices this can build at the
/// roots of `−3`, and none fails there either.
fn change<L: ParameterSet>(
    matrix: Mat4<L>,
    m0: [[u8; 4]; 4],
    zeta: L::Ext,
) -> Option<[[L::Ext; 4]; 4]> {
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

/// The same point of the Kummer line. `(0 : 0)` is no point, and it is
/// equal to nothing here, itself included, because it cross-multiplies
/// equal to everything.
///
/// No caller passes it today. The image comes from
/// `DualChain::dual_eval`, each of whose points has a nonzero coordinate
/// or is infinity as `(1 : 0)`, and the expected point comes from `to_xz`,
/// which gives `(X : Z^2)` with `Z` nonzero or `(1 : 0)`. The refusal is
/// kept so that a change to either does not turn a failed evaluation
/// into an accepted image.
fn same_x<F: Field>(p: Xz<F>, q: Xz<F>) -> bool {
    !(p.x.is_zero() && p.z.is_zero() || q.x.is_zero() && q.z.is_zero())
        && p.x.mul(q.z).same(q.x.mul(p.z))
}

/// What the pairing check and the chains both start from.
pub(crate) struct Lifted<F> {
    /// The key's curve as [`lift_basis`] normalizes it, for the pairing.
    pub vk_curve: Montgomery<F>,
    /// The key's curve, then the response's.
    pub curves: [Montgomery<F>; 2],
    /// The key's basis as full points.
    pub vk: [Jac<F>; 2],
    /// `[s]` of the two signature points.
    pub scaled: [Jac<F>; 2],
}

/// Lift the key's basis and scale the signature points, once. `None` when
/// the basis does not lift.
///
/// Verifier only, variable time. Never call with a secret scalar.
pub(crate) fn lift<L: ParameterSet>(
    basis: &KeyBasis<L>,
    sig: L::Ext,
    p: AffinePoint<L>,
    q: AffinePoint<L>,
    scalar: &[u8],
) -> Option<Lifted<L::Ext>> {
    let vk = basis.coefficient();
    let (vk_curve, p1, p2) = lift_basis(
        Montgomery::normalized(vk),
        basis.p(),
        basis.difference(),
        basis.q(),
    )?;
    let curves = [Montgomery::normalized(vk), Montgomery::normalized(sig)];
    Stage::Scale.enter();
    let scaled = [
        mul(curves[1], affine(p), scalar),
        mul(curves[1], affine(q), scalar),
    ];
    Some(Lifted {
        vk_curve,
        curves,
        vk: [p1, p2],
        scaled,
    })
}

/// Four points, one per column of a challenge matrix.
type Basis<F> = [Jac<F>; 4];

/// The challenge as a residue and the kernel basis `[P₁, R₁, P₂, R₂′]`,
/// with `R₂′ = [q⁻¹] R₂`.
fn basis_points<L: ParameterSet>(
    lifted: &Lifted<L::Ext>,
    scalar: &[u8],
) -> Result<(Residue<L>, Basis<L::Ext>), VerifyError> {
    let [p1, p2] = lifted.vk;
    let [r1, r2] = lifted.scaled;
    let s = Residue::<L>::from_le_bytes(scalar);
    let q_inverse = s.multiply(s).negate().inv().ok_or(VerifyError::Kernel)?;
    let r2_scaled = mul(lifted.curves[1], r2, &q_inverse.to_le_bytes());
    Ok((s, [p1, r1, p2, r2_scaled]))
}

/// A table for each basis point, on its own curve.
fn doublings<L: ParameterSet>(
    curves: [Montgomery<L::Ext>; 2],
    points: [Jac<L::Ext>; 4],
) -> [Doublings<L::Ext>; 4] {
    // A residue modulo `2^e` is this many bytes, and the challenge is shorter.
    let bits = 8 * L::E.div_ceil(8);
    core::array::from_fn(|i| Doublings::new(curves[i % 2], points[i], bits))
}

struct Built<F> {
    chains: [DualChain<F>; 2],
    /// Points on the key's curve paired with their expected images on the
    /// response's, each already multiplied by the challenge.
    images: [(Jac<F>, [Jac<F>; 2]); 3],
}

/// Both chains, and the three points whose images are compared.
///
/// Verifier only, variable time. Never call with a secret scalar.
fn build<L: ParameterSet>(
    lifted: &Lifted<L::Ext>,
    scalar: &[u8],
) -> Result<Built<L::Ext>, VerifyError> {
    Stage::Kernel.enter();
    let Lifted {
        curves,
        vk: [p1, p2],
        scaled: [r1, r2],
        ..
    } = *lifted;
    let (s, points) = basis_points::<L>(lifted, scalar)?;
    let basis = doublings::<L>(curves, points);
    let four_vk = [basis[0].doubled(L::E - 2), basis[2].doubled(L::E - 2)];
    let (t1, mt) = canonical_four(curves[0], four_vk[0], four_vk[1]).ok_or(VerifyError::Kernel)?;
    let (u1, mu) = canonical_four(
        curves[1],
        basis[1].doubled(L::E - 2),
        basis[3].doubled(L::E - 2),
    )
    .ok_or(VerifyError::Kernel)?;
    let zeta = weil(curves[0], four_vk[0], four_vk[1], 2).ok_or(VerifyError::Kernel)?;
    let m0 = [
        [mt[0][0], 0, mt[1][0], 0],
        [0, mu[0][0], 0, mu[1][0]],
        [mt[0][1], 0, mt[1][1], 0],
        [0, mu[0][1], 0, mu[1][1]],
    ];
    let (_, m1, m2) = challenge_matrices(s).ok_or(VerifyError::Kernel)?;
    let product = product_null(t1, u1);
    // Each chain's kernel and base change are built directly before it, in
    // this order, so a refusal in the long chain costs nothing of the short.
    let (kernel1, change1) = (
        kernel(curves, &basis, &columns(m1), 0, L::E),
        change(m1, m0, zeta).ok_or(VerifyError::Kernel)?,
    );
    Stage::Chain1.enter();
    let chain1 =
        DualChain::new(L::E - 2, curves, kernel1, product, change1).ok_or(VerifyError::Kernel)?;
    Stage::Kernel.enter();
    let (kernel2, change2) = (
        kernel(curves, &basis, &columns(m2), 1, L::E),
        change(m2, m0, zeta).ok_or(VerifyError::Kernel)?,
    );
    Stage::Chain2.enter();
    let chain2 =
        DualChain::new(L::E - 3, curves, kernel2, product, change2).ok_or(VerifyError::Kernel)?;
    // The image of `X` is compared with `[s]X` on the key's curve and `[s]Y`
    // on the response's, for `(X, Y)` the two basis pairs and their
    // difference. `[s]` of the signature points is already known, and `[s]`
    // of a difference is the difference of the two.
    Stage::Image.enter();
    let (s_p1, s_p2) = (basis[0].times(scalar), basis[2].times(scalar));
    Ok(Built {
        chains: [chain1, chain2],
        images: [
            (p1, [s_p1, r1]),
            (p2, [s_p2, r2]),
            (
                add(curves[0], p1, negative(p2)),
                [
                    add(curves[0], s_p1, negative(s_p2)),
                    add(curves[1], r1, negative(r2)),
                ],
            ),
        ],
    })
}

/// The two chains, their meeting, and the three images. The pairing check
/// has passed before this is called.
///
/// Verifier only, variable time. Never call with a secret scalar.
pub(crate) fn check<L: ParameterSet>(
    lifted: &Lifted<L::Ext>,
    scalar: &[u8],
) -> Result<(), VerifyError> {
    let Built {
        chains: [chain1, chain2],
        images,
    } = build::<L>(lifted, scalar)?;
    Stage::Codomain.enter();
    if !chain1.matches_dual(&chain2) {
        return Err(VerifyError::Codomain);
    }
    Stage::Image.enter();
    for (x, expected) in images {
        let middle = chain1
            .eval([x, Jac::infinity()])
            .ok_or(VerifyError::Kernel)?;
        let image = chain2.dual_eval(middle).ok_or(VerifyError::Kernel)?;
        if !same_x(image[0], to_xz(expected[0])) || !same_x(image[1], to_xz(expected[1])) {
            return Err(VerifyError::Image);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{basis::basis_hint, challenge::ChallengeScalar, identity::KeyIdentity};
    use prism_field::Salt;
    use prism_field::{P248, P324, P376, P500, P664};
    use prism_isogeny::theta::{projectively_same, ThetaPoint};

    fn decimal<F: Field>(text: &str) -> F {
        let mut value = F::zero();
        for c in text.bytes() {
            value = value
                .mul(F::from_u64(10))
                .add(F::from_u64(u64::from(c - b'0')));
        }
        value
    }
    fn ext<L: ParameterSet>(text: &str) -> L::Ext {
        let text = text.replace(' ', "");
        if let Some((im, re)) = text.split_once("*i+") {
            L::ext_from_base(decimal::<L::Base>(im))
                .mul(L::ext_i())
                .add(L::ext_from_base(decimal::<L::Base>(re)))
        } else {
            L::ext_from_base(decimal::<L::Base>(&text))
        }
    }
    /// What the pairing check and the chains start from, for an honest
    /// vector: the key's basis lifted, the signature's points multiplied
    /// by the challenge, and the challenge.
    fn honest_lift<L: ParameterSet>(
        vector: &prism_vectors::corpus::Vector<'_>,
    ) -> (Lifted<L::Ext>, ChallengeScalar) {
        let key = vector.public_key().expect("the vector's public key");
        let id = KeyIdentity::<L>::decode(&key).unwrap();
        let s = ChallengeScalar::derive(
            &id,
            vector.message().expect("the vector's message"),
            L::Salt::from_bytes(&vector.salt().expect("the vector's salt")).unwrap(),
        );
        let f = |name| L::ext_decode(&vector.bytes(name).expect(name)).unwrap();
        let hint = basis_hint::<L>(id.coefficient()).unwrap();
        let basis = KeyBasis::<L>::of_key(id.coefficient(), hint).unwrap();
        let lifted = lift::<L>(
            &basis,
            f("E"),
            AffinePoint {
                x: f("Px"),
                y: f("Py"),
            },
            AffinePoint {
                x: f("Qx"),
                y: f("Qy"),
            },
            s.as_bytes(),
        )
        .unwrap();
        (lifted, s)
    }

    fn known<L: ParameterSet>(primary: prism_vectors::corpus::Primary) {
        let nulls = prism_vectors::model::nulls(primary);
        let (lifted, s) = honest_lift::<L>(&primary.vector());
        let built = build::<L>(&lifted, s.as_bytes()).unwrap();
        for (chain, key) in built.chains.iter().zip(["chain1", "chain2"]) {
            let section = nulls.split_once(&format!("\"{key}\"")).unwrap().1;
            let end = section.find("\"chain2\"").unwrap_or(section.len());
            let numbers: Vec<_> = section[..end]
                .split('"')
                .enumerate()
                .filter_map(|(i, s)| (i % 2 == 1).then_some(s))
                .collect();
            assert_eq!(numbers.len(), chain.nulls().len() * 4);
            for (step, (got, want)) in chain
                .nulls()
                .iter()
                .zip(numbers.chunks_exact(4))
                .enumerate()
            {
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
        use prism_vectors::corpus::WithCorpus;
        prism_vectors::for_each_corpus_set!(Set, {
            for primary in <Set as WithCorpus>::PRIMARY {
                known::<Set>(primary);
            }
        });
    }

    /// xorshift64*: a fixed stream, so a failure names its case.
    struct Stream(u64);

    impl Stream {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }
        fn scalar(&mut self, len: usize) -> Vec<u8> {
            let mut bytes = vec![0u8; len];
            for chunk in bytes.chunks_mut(8) {
                let word = self.next().to_le_bytes();
                chunk.copy_from_slice(&word[..chunk.len()]);
            }
            bytes
        }
    }

    /// Equal as points: the same affine coordinates, or both infinity.
    fn same_point<F: Field>(a: Jac<F>, b: Jac<F>) -> bool {
        if a.is_infinity() || b.is_infinity() {
            return a.is_infinity() && b.is_infinity();
        }
        let (az2, bz2) = (a.z.square(), b.z.square());
        a.x.mul(bz2).same(b.x.mul(az2)) && a.y.mul(bz2).mul(b.z).same(b.y.mul(az2).mul(a.z))
    }

    fn identical<F: Field>(new: [Couple<F>; 2], old: [Couple<F>; 2], case: &str) {
        for (j, (new, old)) in new.iter().zip(&old).enumerate() {
            for (i, (new, old)) in new.iter().zip(old).enumerate() {
                assert!(same_point(*new, *old), "{case}: generator {j}, curve {i}");
            }
        }
    }

    /// The kernel from shared doublings against one ladder per coefficient,
    /// on the vector's own matrices and on `random` sets of scalars.
    fn kernels_agree<L: ParameterSet>(text: &str, name: &str, random: u32) {
        let (lifted, s) = honest_lift::<L>(&prism_vectors::corpus::Vector::new(text));
        let (residue, points) = basis_points::<L>(&lifted, s.as_bytes()).unwrap();
        let tables = doublings::<L>(lifted.curves, points);
        let (_, m1, m2) = challenge_matrices(residue).unwrap();
        for (matrix, doubles) in [(m1, 0), (m2, 1)] {
            let c = columns(matrix);
            identical(
                kernel(lifted.curves, &tables, &c, doubles, L::E),
                kernel_by_ladders(lifted.curves, points, &c, doubles),
                &format!("{name}: its own matrix, {doubles} doublings"),
            );
        }
        let len = (L::E as usize).div_ceil(8);
        let mut stream = Stream(0x4b45_524e_454c + name.len() as u64);
        // Minus one exactly, which takes the negation; all ones to the byte,
        // which is minus one only where `e` is a multiple of eight.
        let minus_one = Residue::<L>::one().negate().to_le_bytes();
        assert!(is_minus_one(&minus_one, L::E));
        let special = [
            vec![0u8; len],
            vec![1u8],
            minus_one,
            vec![0xff; len],
            Vec::new(),
        ];
        for n in 0..random {
            // Every third set mixes in zero, one, minus one and the empty
            // scalar, the coefficients a matrix is most likely to hold.
            let c: [[Vec<u8>; 4]; 2] = core::array::from_fn(|j| {
                core::array::from_fn(|i| {
                    if n.is_multiple_of(3) && (i + j + n as usize).is_multiple_of(2) {
                        special[(i + 2 * j + n as usize) % special.len()].clone()
                    } else {
                        stream.scalar(len)
                    }
                })
            });
            identical(
                kernel(lifted.curves, &tables, &c, n % 2, L::E),
                kernel_by_ladders(lifted.curves, points, &c, n % 2),
                &format!("{name}: random set {n}"),
            );
        }
    }

    #[test]
    fn the_shared_doublings_give_the_ladders_kernel_on_every_vector() {
        let files = prism_vectors::corpus::files();
        assert_eq!(files.len(), prism_vectors::corpus::HONEST_VECTORS);
        for path in files {
            let text = std::fs::read_to_string(&path).unwrap();
            let vector = prism_vectors::corpus::Vector::new(&text);
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            // Random sets on the six primary vectors; the rest check their
            // own two matrices.
            let random = if prism_vectors::corpus::Primary::by_file_name(&name).is_some() {
                6
            } else {
                0
            };
            let label = vector.level().expect("the vector's level");
            match crate::set::CommittedSet::from_label(label) {
                Ok(crate::set::CommittedSet::P248) => kernels_agree::<P248>(&text, &name, random),
                Ok(crate::set::CommittedSet::P376) => kernels_agree::<P376>(&text, &name, random),
                Ok(crate::set::CommittedSet::P500) => kernels_agree::<P500>(&text, &name, random),
                Err(_) => panic!("{name}: label {label}"),
            }
        }
    }

    /// The pairing check and then the chains, as the relation runs them:
    /// `None` when the pairing refuses, and otherwise what the chains say.
    fn after_the_lift<L: ParameterSet>(
        lifted: &Lifted<L::Ext>,
        scalar: &ChallengeScalar,
    ) -> Option<Result<(), VerifyError>> {
        crate::relation::pairing_holds::<L>(lifted, scalar.as_bytes())
            .then(|| check::<L>(lifted, scalar.as_bytes()))
    }

    /// The verdict with the key's lifted pair as it is, with both points
    /// negated, and with one negated.
    fn the_sign_of_the_lift<L: ParameterSet>(text: &str, name: &str) {
        let (lifted, s) = honest_lift::<L>(&prism_vectors::corpus::Vector::new(text));
        let [p1, p2] = lifted.vk;
        assert_eq!(after_the_lift::<L>(&lifted, &s), Some(Ok(())), "{name}");
        let both = Lifted {
            vk: [negative(p1), negative(p2)],
            ..lifted
        };
        assert_eq!(
            after_the_lift::<L>(&both, &s),
            Some(Ok(())),
            "{name}: both points negated"
        );
        // Not vacuous: the relative sign of the two is what the lift fixes,
        // and with it wrong the pairing is the inverse and refuses.
        let one = Lifted {
            vk: [negative(p1), p2],
            ..lifted
        };
        assert_eq!(
            after_the_lift::<L>(&one, &s),
            None,
            "{name}: one point negated"
        );
    }

    /// `jac::lift_basis` takes a square root for the `y` of the key's
    /// first point, and the other root gives the negative of both lifted
    /// points. The verdict does not depend on which: with both negated,
    /// every committed honest signature is still accepted. With one
    /// negated it is refused at the pairing, so the lift's relative sign
    /// is not free.
    #[test]
    fn the_verdict_does_not_depend_on_the_sign_of_the_lifted_basis() {
        let files = prism_vectors::corpus::files();
        assert_eq!(files.len(), prism_vectors::corpus::HONEST_VECTORS);
        for path in files {
            let text = std::fs::read_to_string(&path).unwrap();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let label = prism_vectors::corpus::Vector::new(&text)
                .level()
                .expect("the vector's level");
            match crate::set::CommittedSet::from_label(label) {
                Ok(crate::set::CommittedSet::P248) => the_sign_of_the_lift::<P248>(&text, &name),
                Ok(crate::set::CommittedSet::P376) => the_sign_of_the_lift::<P376>(&text, &name),
                Ok(crate::set::CommittedSet::P500) => the_sign_of_the_lift::<P500>(&text, &name),
                Err(_) => panic!("{name}: label {label}"),
            }
        }
    }

    #[test]
    fn minus_one_is_recognized_exactly() {
        assert!(is_minus_one(&[0xff, 0xff], 16));
        assert!(is_minus_one(&[0xff, 0x0f], 12));
        assert!(is_minus_one(&[0xff, 0x0f, 0], 12));
        assert!(is_minus_one(&[0xff, 0xff, 0], 16));
        for (scalar, bits) in [
            (&[0xff, 0xff][..], 12),
            (&[0xff, 0x0f, 1], 12),
            (&[0xff, 0x07], 12),
            (&[0xfe, 0x0f], 12),
            (&[0xff], 12),
            (&[0xff], 16),
            (&[], 8),
            (&[0xff, 0xff, 1], 16),
        ] {
            assert!(!is_minus_one(scalar, bits), "{scalar:?} at {bits} bits");
        }
    }

    /// Equality of `x`-only points is equality up to a common factor,
    /// with the points whose `x` or `z` is zero as points like any other,
    /// and `(0 : 0)` equal to nothing.
    #[test]
    fn two_x_only_points_are_the_same_up_to_scale_and_the_null_pair_is_none() {
        type F = <P248 as ParameterSet>::Ext;
        let at = |x: u64, z: u64| Xz {
            x: F::from_u64(x),
            z: F::from_u64(z),
        };
        let null = at(0, 0);
        // The same point, written twice and scaled.
        for (left, right) in [
            (at(3, 1), at(3, 1)),
            (at(3, 1), at(21, 7)),
            (at(6, 4), at(9, 6)),
            // `x = 0`, the point `(0, 0)` of order two.
            (at(0, 1), at(0, 5)),
            // Infinity.
            (at(1, 0), at(4, 0)),
        ] {
            assert!(same_x(left, right) && same_x(right, left));
        }
        for (left, right) in [
            (at(3, 1), at(4, 1)),
            (at(3, 1), at(3, 2)),
            (at(0, 1), at(1, 0)),
            (at(0, 1), at(1, 1)),
            (at(1, 0), at(1, 1)),
        ] {
            assert!(!same_x(left, right) && !same_x(right, left));
        }
        for point in [at(3, 1), at(0, 1), at(1, 0), null] {
            assert!(!same_x(null, point) && !same_x(point, null));
        }
    }

    #[test]
    fn a_table_multiplies_as_the_ladder_does() {
        let vector = prism_vectors::corpus::Primary::Lvl1.vector();
        let f = |row| P248::ext_decode(&vector.bytes(row).expect(row)).unwrap();
        let curve = Montgomery::normalized(f("E"));
        let point = Jac {
            x: f("Px"),
            y: f("Py"),
            z: <P248 as ParameterSet>::Ext::one(),
        };
        let table = Doublings::new(curve, point, 248);
        assert!(table.times(&[]).is_infinity() && table.times(&[0; 31]).is_infinity());
        let mut stream = Stream(7);
        for len in [1usize, 2, 31, 32] {
            // 32 bytes is wider than the table: a set bit above it sends
            // the whole scalar to the ladder.
            let scalar = stream.scalar(len);
            let (new, old) = (table.times(&scalar), mul(curve, point, &scalar));
            assert!(new.x.same(old.x) && new.y.same(old.y) && new.z.same(old.z));
        }
        // The first bit above the table, alone and with the bits below it.
        for low in [0u8, 0xff] {
            let mut scalar = [low; 32];
            scalar[31] = 1;
            let (new, old) = (table.times(&scalar), mul(curve, point, &scalar));
            assert!(new.x.same(old.x) && new.y.same(old.y) && new.z.same(old.z));
        }
        // Wider bytes with no bit above the table are the same scalar.
        let scalar = stream.scalar(31);
        let padded = [&scalar[..], &[0; 4]].concat();
        let (short, long) = (table.times(&scalar), table.times(&padded));
        assert!(short.x.same(long.x) && short.y.same(long.y) && short.z.same(long.z));
        let doubled = table.doubled(5);
        let old = dbl_iter(curve, point, 5);
        assert!(doubled.x.same(old.x) && doubled.y.same(old.y) && doubled.z.same(old.z));
    }

    /// The eight matrices `canonical_four` can return.
    fn canonical_forms() -> Vec<[[u8; 2]; 2]> {
        (0..4)
            .flat_map(|d| [[[1, 0], [d, 1]], [[d, 3], [1, 0]]])
            .collect()
    }

    /// Every base change `build` can ask for. Modulo 4 the challenge
    /// matrices depend only on `s` modulo 4, which is 1 or 3, and `M0` is
    /// one of 8 × 8 pairs of canonical forms: 2 × 64 × 2 = 256 matrices.
    /// Each has a base change at both primitive fourth roots of unity, and
    /// also at a root of `−3`, the one `zeta` at which the search can fail
    /// at all: none of the 256 is a matrix it fails on.
    fn every_reachable_base_change_exists<L: ParameterSet>() {
        let root = L::Ext::from_u64(3).negate().sqrt().unwrap();
        let zetas = [L::ext_i(), L::ext_i().negate(), root, root.negate()];
        let mut tried = 0;
        for s in [1, 3] {
            let (_, m1, m2) = challenge_matrices(Residue::<L>::from_u64(s)).unwrap();
            for mt in canonical_forms() {
                for mu in canonical_forms() {
                    let m0 = [
                        [mt[0][0], 0, mt[1][0], 0],
                        [0, mu[0][0], 0, mu[1][0]],
                        [mt[0][1], 0, mt[1][1], 0],
                        [0, mu[0][1], 0, mu[1][1]],
                    ];
                    for matrix in [m1, m2] {
                        for zeta in zetas {
                            assert!(change(matrix, m0, zeta).is_some(), "{} s={s}", L::NAME);
                        }
                        tried += 1;
                    }
                }
            }
        }
        assert_eq!(tried, 256);
    }

    #[test]
    fn no_base_change_the_verifier_can_ask_for_is_refused() {
        every_reachable_base_change_exists::<P248>();
        every_reachable_base_change_exists::<P324>();
        every_reachable_base_change_exists::<P376>();
        every_reachable_base_change_exists::<P500>();
        every_reachable_base_change_exists::<P664>();
    }
}
