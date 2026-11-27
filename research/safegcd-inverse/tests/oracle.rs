//! Correctness of `inv_mod_ct` against a BigInt oracle and against
//! `prism_int::Int::inv_mod`, at every Shekyl set and modulus class.

use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer;
use num_traits::{One, Zero};
use prism_int::Int;
use safegcd_inverse::rng::Rng;
use safegcd_inverse::*;

fn to_big(v: &[u64]) -> BigUint {
    BigUint::from_slice(&v.iter().flat_map(|&x| [x as u32, (x >> 32) as u32]).collect::<Vec<_>>())
}
fn from_big(b: &BigUint, n: usize) -> Vec<u64> {
    let mut v = b.to_u64_digits();
    v.resize(n.max(v.len()), 0);
    v
}
fn oracle(x: &BigUint, m: &BigUint) -> Option<BigUint> {
    let (xi, mi) = (BigInt::from_biguint(Sign::Plus, x.clone()), BigInt::from_biguint(Sign::Plus, m.clone()));
    let eg = xi.extended_gcd(&mi);
    if !eg.gcd.is_one() {
        return None;
    }
    let r = eg.x.mod_floor(&mi);
    Some(r.to_biguint().unwrap())
}

#[derive(Default, Debug)]
struct Tally {
    total: u64,
    invertible: u64,
    non_invertible: u64,
    prism_int_compared: u64,
}

/// One case: CT result against the oracle, and against prism-int at `W` limbs.
fn check<const K: usize, const W: usize>(x: &BigUint, m: &BigUint, jumps: u32, t: &mut Tally, what: &str) {
    let xs = S62::<K>::from_u64_limbs(&from_big(x, K + 2));
    let ms = S62::<K>::from_u64_limbs(&from_big(m, K + 2));
    let got = inv_mod_ct(&xs, &ms, jumps);
    let want = oracle(x, m);
    let mut out = vec![0u64; K + 2];
    got.value.to_u64_limbs(&mut out);
    let got_big = to_big(&out);
    match &want {
        Some(w) => {
            assert_eq!(got.ok, u64::MAX, "{what}: flag for a unit, x={x:x} m={m:x}");
            assert_eq!(&got_big, w, "{what}: value, x={x:x} m={m:x}");
            t.invertible += 1;
        }
        None => {
            assert_eq!(got.ok, 0, "{what}: flag for a non-unit, x={x:x} m={m:x}");
            assert!(got_big.is_zero(), "{what}: value not zeroed for a non-unit");
            t.non_invertible += 1;
        }
    }
    // prism-int's variable-time inverse at the signer width W, when m fits.
    if m.bits() < 64 * W as u64 - 2 && m > &BigUint::one() {
        let xi = Int::<W>::from_le_limbs(&from_big(x, W)[..W], false).unwrap();
        let mi = Int::<W>::from_le_limbs(&from_big(m, W)[..W], false).unwrap();
        let r = xi.inv_mod(&mi).unwrap();
        match (r, &want) {
            (Some(r), Some(w)) => {
                let mut bytes = vec![0u8; 8 * W];
                r.to_le_bytes(&mut bytes).unwrap();
                assert_eq!(&BigUint::from_bytes_le(&bytes), w, "{what}: prism-int disagrees");
            }
            (None, None) => {}
            (r, w) => panic!("{what}: prism-int {:?} vs oracle {:?}", r.is_some(), w.is_some()),
        }
        t.prism_int_compared += 1;
    }
    t.total += 1;
}

fn random_odd(rng: &mut Rng, bits: u32) -> BigUint {
    let mut v = rng.bits(bits, true);
    v[0] |= 1;
    to_big(&v)
}
fn below(rng: &mut Rng, m: &BigUint) -> BigUint {
    to_big(&rng.bits(m.bits() as u32 + 8, false)) % m
}

/// The cases at bit bound `b`: random, edge, non-unit, adversarial moduli.
fn run_class<const K: usize, const W: usize>(name: &str, b: u32, seed: u64, randoms: u32) -> Tally {
    let jumps = jumps(divsteps_2026(b));
    assert!(62 * K as u32 >= b + 3);
    let mut rng = Rng(seed);
    let mut t = Tally::default();
    // Random odd moduli with exactly b bits, random operands.
    for _ in 0..randoms {
        let m = random_odd(&mut rng, b);
        let x = below(&mut rng, &m);
        check::<K, W>(&x, &m, jumps, &mut t, name);
    }
    // Edge operands on random moduli and on structured moduli.
    let two_b = BigUint::one() << b;
    let structured = [
        two_b.clone() - 1u32,                       // 2^b − 1 (all ones)
        (BigUint::one() << (b - 1)) + 1u32,         // 2^(b−1) + 1
        BigUint::from(3u32),
        BigUint::from(5u32),
        BigUint::from(7u32),
        BigUint::from(0xffff_fff1u32),               // a small odd modulus
        random_odd(&mut rng, b),
        random_odd(&mut rng, b / 2),
    ];
    for m in structured.iter() {
        let mm1 = m - 1u32;
        let mut xs = vec![BigUint::zero(), BigUint::one(), mm1.clone()];
        if m > &BigUint::from(3u32) {
            xs.push(BigUint::from(2u32));
            xs.push(m - 2u32);
            xs.push(m >> 1); // (m−1)/2
            xs.push((m >> 1) + 1u32);
        }
        for s in 3u32..40 {
            xs.push(BigUint::from(s) % m);
        }
        for x in xs {
            check::<K, W>(&x, m, jumps, &mut t, name);
        }
    }
    // Non-units: composite m = a·c, x a multiple of a (and of c, and of both).
    for _ in 0..(randoms / 4).max(64) {
        let abits = 3 + (rng.next() % (b as u64 - 6)) as u32;
        let a = random_odd(&mut rng, abits);
        let c = random_odd(&mut rng, b - abits);
        let m = &a * &c;
        if m.bits() > b as u64 || m <= BigUint::one() {
            continue;
        }
        for x in [(&a * below(&mut rng, &c)) % &m, (&c * below(&mut rng, &a)) % &m, &a % &m, &c % &m] {
            check::<K, W>(&x, &m, jumps, &mut t, name);
        }
    }
    // Small-factor moduli: 3^k·r, 5·r with x a multiple of 3 or 5.
    for f in [3u32, 5, 9, 15, 27] {
        let r = random_odd(&mut rng, b - 6);
        let m = &r * f;
        for x in [BigUint::from(f), BigUint::from(f) * 2u32, below(&mut rng, &m) * f % &m] {
            check::<K, W>(&x, &m, jumps, &mut t, name);
        }
    }
    eprintln!("{name}: {t:?}");
    t
}

macro_rules! class {
    ($test:ident, $name:expr, $b:expr, $k:expr, $w:expr, $n:expr) => {
        #[test]
        fn $test() {
            let t = run_class::<$k, $w>($name, $b, 0x5afe_9cd0 ^ $b as u64, $n);
            assert!(t.invertible > 0 && t.non_invertible > 0);
        }
    };
}

// q: challenge degree (2e − 5 bits), signer width W.
class!(q_p248, "P248 q", 491, 8, 11, 4000);
class!(q_p324, "P324 q", 643, 11, 14, 4000);
class!(q_p376, "P376 q", 747, 13, 16, 4000);
class!(q_p500, "P500 q", 995, 17, 21, 4000);
class!(q_p664, "P664 q", 1323, 22, 27, 4000);
// n: qlapoti norm bound (L+1)/2 + 2 bits.
class!(n_p248, "P248 n", 128, 3, 11, 4000);
class!(n_p324, "P324 n", 165, 3, 14, 4000);
class!(n_p376, "P376 n", 194, 4, 16, 4000);
class!(n_p500, "P500 n", 255, 5, 21, 4000);
class!(n_p664, "P664 n", 337, 6, 27, 4000);
// N: secret key norm bound ⌈L/2⌉ + 32 (no inverse site today; covered anyway).
class!(nn_p248, "P248 N", 158, 3, 11, 2000);
class!(nn_p324, "P324 N", 195, 4, 14, 2000);
class!(nn_p376, "P376 N", 224, 4, 16, 2000);
class!(nn_p500, "P500 N", 285, 5, 21, 2000);
class!(nn_p664, "P664 N", 367, 6, 27, 2000);

/// The EUROCRYPT 2026 worst-case example: exactly 590 divsteps at b = 256.
#[test]
fn paper_590_example() {
    let f = BigUint::parse_bytes(b"eec9f80577a885d22f8d37c1946187e26805ea27b26c5ae10aa38a02e2ea3157", 16).unwrap();
    let g = BigUint::parse_bytes(b"eb40350da50b11d23183ae8e88ffced0ad11263b6d62cde5e5dc1e934ef8229c", 16).unwrap();
    let big = |v: &BigUint| num_like::Big { mag: v.to_u64_digits(), neg: false };
    assert_eq!(divsteps_to_zero_vartime(&big(&f), &big(&g), 10_000), Some(590));
    assert_eq!(divsteps_2026(256), 590);
    // The CT routine with the bound's jumps inverts it; with one jump fewer
    // (558 divsteps < 590) it must fail to finish, which shows the count is
    // load-bearing rather than slack.
    let mut t = Tally::default();
    check::<5, 5>(&g, &f, jumps(590), &mut t, "paper");
    let xs = S62::<5>::from_u64_limbs(&from_big(&g, 7));
    let ms = S62::<5>::from_u64_limbs(&from_big(&f, 7));
    assert_eq!(inv_mod_ct(&xs, &ms, jumps(590) - 1).ok, 0, "558 divsteps should not finish");
}

/// The bound holds exhaustively for every odd f ≤ 2^b, 0 ≤ g ≤ f, b ≤ 11,
/// with the var-time counter (an independent check of the δ = 1/2 step).
#[test]
fn exhaustive_small_b() {
    for b in 1u32..=11 {
        let mut worst = 0;
        let lim = 1u64 << b;
        let mut f = 1u64;
        while f <= lim {
            for g in 0..=f {
                let n = num_like::count(
                    &num_like::Big { mag: vec![f], neg: false },
                    &num_like::Big { mag: vec![g], neg: false },
                    100_000,
                )
                .unwrap();
                worst = worst.max(n);
            }
            f += 2;
        }
        assert!(worst <= divsteps_2026(b), "b={b}: worst {worst} > bound {}", divsteps_2026(b));
        eprintln!("b={b}: worst {worst}, bound {}", divsteps_2026(b));
    }
}
