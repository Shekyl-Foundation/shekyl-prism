//! Cost of the fixed-count divstep inverse against the two inverses the
//! code base has today, at every set:
//!   - `prism_int::Int::inv_mod` (Euclid, variable time), at the signer width W;
//!   - the Stage 2d Fermat inverse modulo q in Montgomery form (dev
//!     `shekyl-prism/src/ideal/mod.rs`, `Modulus` and the `q − 2` ladder),
//!     copied verbatim below so it runs on the same prism-int.
//! Usage: bench [ROUNDS]. Prints a Markdown table, ns per inverse (median of rounds).

use prism_int::{Int, SecretInt};
use safegcd_inverse::rng::Rng;
use safegcd_inverse::*;
use std::hint::black_box;
use std::time::Instant;

// ---- dev 8232184 shekyl-prism/src/ideal/geometry.rs `inverse_odd`, verbatim.
pub fn inverse_odd<const W: usize>(t: &SecretInt<W>) -> SecretInt<W> {
    let max = SecretInt::from_public(&Int::<W>::from_le_limbs(&[u64::MAX; W], false).unwrap());
    let mut x = SecretInt::one();
    for _ in 0..(W * 64).next_power_of_two().trailing_zeros() {
        let diff = SecretInt::from_u64(2).sub(&t.mul_low(&x));
        let complement = max.sub(&diff.abs()).add(&SecretInt::one());
        x = x.mul_low(&SecretInt::select(&diff, &complement, &diff.is_negative()));
    }
    x
}

// ---- dev 8232184 shekyl-prism/src/ideal/mod.rs `Modulus` (new, mul), verbatim.
struct Modulus<const W: usize, const D: usize> {
    q: SecretInt<W>,
    inverse: SecretInt<W>,
    one: SecretInt<W>,
    #[allow(dead_code)]
    r2: SecretInt<W>,
}
impl<const W: usize, const D: usize> Modulus<W, D> {
    fn new(q: &Int<W>) -> Self {
        let one = Int::<W>::one();
        let r = one.shl((64 * W - 1) as u32).unwrap().rem_euclid(q).unwrap();
        let r = r.shl(1).unwrap().rem_euclid(q).unwrap();
        let mut r2 = r.clone();
        for _ in 0..64 * W {
            r2 = r2.shl(1).unwrap();
            if &r2 >= q {
                r2 = r2.sub(q).unwrap();
            }
        }
        let q = SecretInt::from_public(q);
        let inverse = inverse_odd(&q);
        Self { q, inverse, one: SecretInt::from_public(&r), r2: SecretInt::from_public(&r2) }
    }
    fn mul(&self, x: &SecretInt<W>, y: &SecretInt<W>) -> SecretInt<W> {
        let t = x.resize::<D>().mul(&y.resize());
        let lo = t.truncate::<W>().mul_low(&self.inverse);
        let d = t.sub(&self.q.resize::<D>().mul(&lo.resize()));
        let z = d.shr((64 * W) as u32).resize::<W>();
        SecretInt::select(&z, &z.add(&self.q), &z.is_negative())
    }
    /// The Stage 2d ladder: `norm^(q−2)` with the public exponent's bits.
    fn fermat_inverse(&self, norm: &SecretInt<W>, exponent: &Int<W>) -> SecretInt<W> {
        let mut inv = self.one.clone();
        for bit in (0..exponent.bit_length()).rev() {
            inv = self.mul(&inv, &inv);
            if exponent.shr(bit).is_odd() {
                inv = self.mul(&inv, norm);
            }
        }
        inv
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn odd_modulus(rng: &mut Rng, bits: u32, limbs: usize) -> Vec<u64> {
    let mut m = rng.bits(bits, true);
    m[0] |= 1;
    m.resize(limbs, 0);
    m
}
/// A value below m: same bit count minus one, so it is < m.
fn below(rng: &mut Rng, bits: u32, limbs: usize) -> Vec<u64> {
    let mut x = rng.bits(bits - 1, false);
    x.resize(limbs, 0);
    x
}

const INPUTS: usize = 64;

fn time<F: FnMut(usize)>(rounds: usize, reps: usize, mut f: F) -> f64 {
    let mut per = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let t0 = Instant::now();
        for r in 0..reps {
            f(r % INPUTS);
        }
        per.push(t0.elapsed().as_nanos() as f64 / reps as f64);
    }
    median(per)
}

/// One modulus class at one set: CT divstep (K limbs, bound for b) vs
/// Int::inv_mod at W limbs, and (q only) the Fermat ladder at (W, D).
fn class<const K: usize, const W: usize, const D: usize>(set: &str, cls: &str, b: u32, fermat: bool, rounds: usize) {
    let mut rng = Rng(0xbe4c ^ b as u64);
    let m: Vec<u64> = odd_modulus(&mut rng, b, W);
    let xs: Vec<Vec<u64>> = (0..INPUTS).map(|_| below(&mut rng, b, W)).collect();
    let jumps = jumps(divsteps_2026(b));
    let jumps19 = safegcd_inverse::jumps(divsteps_2019(b));
    let ms = S62::<K>::from_u64_limbs(&m);
    let reps = 200;
    let ct = time(rounds, reps, |i| {
        let x = S62::<K>::from_u64_limbs(black_box(&xs[i]));
        let r = inv_mod_ct(&x, black_box(&ms), black_box(jumps));
        let mut out = [0u64; W];
        r.value.to_u64_limbs(&mut out);
        black_box((out, r.ok));
    });
    let ct19 = time(rounds, reps, |i| {
        let x = S62::<K>::from_u64_limbs(black_box(&xs[i]));
        let r = inv_mod_ct(&x, black_box(&ms), black_box(jumps19));
        black_box(r.ok);
    });
    let mi = Int::<W>::from_le_limbs(&m, false).unwrap();
    let xi: Vec<Int<W>> = xs.iter().map(|x| Int::<W>::from_le_limbs(x, false).unwrap()).collect();
    let euclid = time(rounds, reps, |i| {
        black_box(black_box(&xi[i]).inv_mod(black_box(&mi)).unwrap());
    });
    let fermat_ns = if fermat {
        let ring = Modulus::<W, D>::new(&mi);
        let exponent = mi.sub(&Int::from_u64(2)).unwrap();
        let xsec: Vec<SecretInt<W>> = xi.iter().map(SecretInt::from_public).collect();
        // q here is a random odd modulus of the right width, not a prime: the
        // ladder's cost depends only on q's bit length and popcount, not primality.
        let fr = (5 * rounds / 10).max(3);
        Some(time(fr, 10, |i| {
            black_box(ring.fermat_inverse(black_box(&xsec[i]), &exponent));
        }))
    } else {
        None
    };
    let f = fermat_ns.map_or("–".to_string(), |v| format!("{:.0}", v));
    let sp = fermat_ns.map_or("–".to_string(), |v| format!("{:.1}×", v / ct));
    println!(
        "| {set} | {cls} | {b} | {K} | {jumps} ({}) | {:.0} | {:.0} | {:.0} | {f} | {:.2}× | {sp} |",
        62 * jumps, ct, ct19, euclid, euclid / ct
    );
}

fn main() {
    let rounds: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(21);
    println!("| set | modulus | b | K (62-bit limbs) | jumps (divsteps) | divstep 2026 ns | divstep 2019-count ns | Int::inv_mod ns | Fermat (Stage 2d) ns | Euclid / divstep | Fermat / divstep |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|");
    class::<8, 11, 22>("P248", "q", 491, true, rounds);
    class::<3, 11, 22>("P248", "n", 128, false, rounds);
    class::<11, 14, 28>("P324", "q", 643, true, rounds);
    class::<3, 14, 28>("P324", "n", 165, false, rounds);
    class::<13, 16, 32>("P376", "q", 747, true, rounds);
    class::<4, 16, 32>("P376", "n", 194, false, rounds);
    class::<17, 21, 42>("P500", "q", 995, true, rounds);
    class::<5, 21, 42>("P500", "n", 255, false, rounds);
    class::<22, 27, 54>("P664", "q", 1323, true, rounds);
    class::<6, 27, 54>("P664", "n", 337, false, rounds);
}
