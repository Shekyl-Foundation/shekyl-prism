//! Hill-climb for inputs needing many divsteps (δ₀ = 1/2), at each bit
//! bound of the report; prints the worst found next to the 2026 bound.
//! Variable-time counter: an instrument, not the routine under test.

use safegcd_inverse::num_like::{count, Big};
use safegcd_inverse::rng::Rng;
use safegcd_inverse::divsteps_2026;

fn to_big(v: &[u64]) -> Big {
    Big { mag: v.to_vec(), neg: false }
}
fn le(a: &[u64], b: &[u64]) -> bool {
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i] < b[i];
        }
    }
    true
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let iters: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(20000);
    let restarts: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    let bs: [(&str, u32); 15] = [
        ("P248 n", 128), ("P248 N", 158), ("P248 q", 491),
        ("P324 n", 165), ("P324 N", 195), ("P324 q", 643),
        ("P376 n", 194), ("P376 N", 224), ("P376 q", 747),
        ("P500 n", 255), ("P500 N", 285), ("P500 q", 995),
        ("P664 n", 337), ("P664 N", 367), ("P664 q", 1323),
    ];
    println!("| class | b | random mean | random max | hill-climb max | 2026 bound | slack |");
    println!("|---|---|---|---|---|---|---|");
    for (name, b) in bs {
        let mut rng = Rng(0xad5e ^ b as u64);
        let bound = divsteps_2026(b);
        // Random baseline.
        let (mut sum, mut rmax) = (0u64, 0u32);
        let samples = 2000;
        for _ in 0..samples {
            let mut f = rng.bits(b, true);
            f[0] |= 1;
            let mut g = rng.bits(b, false);
            if !le(&g, &f) { std::mem::swap(&mut f, &mut g); f[0] |= 1; if !le(&g,&f) { continue; } }
            let n = count(&to_big(&f), &to_big(&g), 10 * b).unwrap();
            sum += n as u64;
            rmax = rmax.max(n);
        }
        // Hill-climb: flip 1–3 random bits of f or g, keep if not fewer steps.
        let mut best = 0;
        for _ in 0..restarts {
            let mut f = rng.bits(b, true);
            f[0] |= 1;
            let mut g = rng.bits(b - 1, false);
            g.resize(f.len(), 0);
            let mut cur = count(&to_big(&f), &to_big(&g), 10 * b).unwrap();
            for _ in 0..iters {
                let (mut f2, mut g2) = (f.clone(), g.clone());
                for _ in 0..1 + rng.next() % 3 {
                    let bit = (rng.next() % b as u64) as usize;
                    if rng.next() & 1 == 0 { f2[bit / 64] ^= 1 << (bit % 64); } else { g2[bit / 64] ^= 1 << (bit % 64); }
                }
                f2[0] |= 1;
                if !le(&g2, &f2) || f2.iter().all(|&x| x == 0) { continue; }
                let n = count(&to_big(&f2), &to_big(&g2), 10 * b).unwrap();
                if n >= cur { cur = n; f = f2; g = g2; }
            }
            best = best.max(cur);
        }
        assert!(best <= bound && rmax <= bound);
        println!("| {name} | {b} | {:.1} | {rmax} | {best} | {bound} | {} |", sum as f64 / samples as f64, bound - best);
    }
}
