// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! rlc check | bench <set> <reps> | dudect <set> <n> <impl>
//! Research driver: cross-check against the Python model's vectors, cycle
//! counts, and a dudect-style fixed-vs-random Welch t-test.

use num_bigint::BigInt;
use rust_lagrange_ct::sets::{from_limbs_signed, from_limbs_unsigned, to_limbs, SetInfo, SETS};
use rust_lagrange_ct::{exact, with_set, zi};
use std::io::Write;

struct Row {
    kind: String,
    nb: u64,
    t: BigInt,
    x: BigInt,
    y: BigInt,
    nu_exact: BigInt,
    nu_p64: BigInt,
    lam1: BigInt,
}

fn hx(s: &str) -> BigInt {
    BigInt::parse_bytes(s.as_bytes(), 16).unwrap()
}

fn load(path: &str) -> Vec<Row> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split(' ').collect();
            Row {
                kind: f[0].to_string(),
                nb: u64::from_str_radix(f[1], 16).unwrap(),
                t: hx(f[2]),
                x: hx(f[3]),
                y: hx(f[4]),
                nu_exact: hx(f[5]),
                nu_p64: hx(f[6]),
                lam1: hx(f[7]),
            }
        })
        .collect()
}

fn info(name: &str) -> &'static SetInfo {
    SETS.iter().find(|s| s.name == name).expect("set")
}

#[inline(always)]
fn rdtsc() -> u64 {
    // lfence; rdtsc; lfence  (serialises around the timed region)
    unsafe {
        core::arch::x86_64::_mm_lfence();
        let t = core::arch::x86_64::_rdtsc();
        core::arch::x86_64::_mm_lfence();
        t
    }
}

fn pin(cpu: usize) {
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(cpu, &mut set);
        let r = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        eprintln!("pinned to cpu {cpu}: {}", if r == 0 { "ok" } else { "FAILED" });
    }
}

struct Inp<const C: usize> {
    t: [u64; C],
    x: [u64; C],
    y: [u64; C],
}

fn prep<const C: usize>(si: &SetInfo, r: &Row) -> Inp<C> {
    assert!(r.t.bits() <= si.t_bits_max(), "T longer than the public bound");
    Inp { t: to_limbs(&r.t), x: to_limbs(&r.x), y: to_limbs(&r.y) }
}

// ------------------------------------------------------------- check
fn check_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo) -> bool {
    assert_eq!((si.c, si.n, si.pl), (C, N, PL));
    assert!(C * 64 >= si.t_bits_max() as usize + 3 && N == 2 * C);
    let p = si.p();
    assert!(p.bits() as usize <= 64 * PL);
    let pl: [u64; PL] = to_limbs(&p);
    let mut rows = load(&format!("vectors/{}_check.txt", si.name));
    let pool = load(&format!("vectors/{}_pool.txt", si.name));
    rows.extend(pool.into_iter().take(501));
    let (mut n, mut ok_p64, mut ok_exact, mut ok_lam, mut ok_lll, mut ok_gauss, mut ok_q) =
        (0, 0, 0, 0, 0, 0, 0);
    let (mut n_lam, mut n_lll_short) = (0, 0);
    let mut maxbits_t = 0;
    for r in &rows {
        let inp = prep::<C>(si, r);
        let (u, nu) = zi::reduce::<C, N, PL>(&inp.t, &inp.x, &inp.y, &pl, si.steps);
        let nu_b = from_limbs_unsigned(&nu);
        let ub: Vec<BigInt> = u.iter().map(from_limbs_signed).collect();
        let nrd_u = &ub[0] * &ub[0] + &ub[1] * &ub[1] + &p * (&ub[2] * &ub[2] + &ub[3] * &ub[3]);
        // u is in the ideal: nrd divisible by T, nonzero, and equals the CT norm
        let in_ideal = nrd_u == nu_b && (&nu_b % &r.t) == BigInt::from(0) && nu_b > BigInt::from(0);
        n += 1;
        ok_q += in_ideal as usize;
        ok_p64 += (nu_b == r.nu_p64) as usize;
        ok_exact += (nu_b == r.nu_exact) as usize;
        let (g_nu, _) = exact::gauss(&r.t, &r.x, &r.y, &p);
        ok_gauss += (g_nu == nu_b) as usize;
        if r.lam1 > BigInt::from(0) {
            n_lam += 1;
            ok_lam += (nu_b == r.lam1) as usize;
            let (b1, _) = exact::lll(&exact::gram(&r.t, &r.x, &r.y, &p));
            ok_lll += (b1 == r.lam1) as usize;
            n_lll_short += (b1 > r.lam1) as usize;
        }
        maxbits_t = maxbits_t.max(r.t.bits());
    }
    println!(
        "{} rows={} ct==py_p64 {}/{} ct==py_exact {}/{} ct==lambda1 {}/{} ct==rust_exact_gauss {}/{} \
         ct_in_ideal {}/{} rust_lll_b1==lambda1 {}/{} (lll b1 > lambda1: {}) maxbits(T)={} bound={}",
        si.name, n, ok_p64, n, ok_exact, n, ok_lam, n_lam, ok_gauss, n, ok_q, n, ok_lll, n_lam,
        n_lll_short, maxbits_t, si.t_bits_max()
    );
    ok_p64 == n && ok_lam == n_lam && ok_q == n
}

// ------------------------------------------------------------- bench
fn median(v: &mut [u64]) -> u64 {
    v.sort_unstable();
    v[v.len() / 2]
}

fn bench_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo, reps: usize) {
    let p = si.p();
    let pl: [u64; PL] = to_limbs(&p);
    let pool = load(&format!("vectors/{}_pool.txt", si.name));
    let inps: Vec<Inp<C>> = pool.iter().map(|r| prep::<C>(si, r)).collect();
    let grams: Vec<_> = pool.iter().map(|r| exact::gram(&r.t, &r.x, &r.y, &p)).collect();
    let m = pool.len();
    // warm-up
    for i in 0..20 {
        std::hint::black_box(zi::reduce::<C, N, PL>(&inps[i].t, &inps[i].x, &inps[i].y, &pl, si.steps));
        std::hint::black_box(exact::lll(&grams[i]));
    }
    let (mut c_ct, mut c_lll, mut c_g) = (vec![], vec![], vec![]);
    let mut lll_steps = 0usize;
    let mut g_steps = 0usize;
    for k in 0..reps {
        let i = k % m;
        let r = &pool[i];
        let a = rdtsc();
        let o = zi::reduce::<C, N, PL>(&inps[i].t, &inps[i].x, &inps[i].y, &pl, si.steps);
        let b = rdtsc();
        std::hint::black_box(o);
        c_ct.push(b - a);
        let a = rdtsc();
        let o = exact::lll(&grams[i]);
        let b = rdtsc();
        lll_steps += o.1;
        std::hint::black_box(o);
        c_lll.push(b - a);
        let a = rdtsc();
        let o = exact::gauss(&r.t, &r.x, &r.y, &p);
        let b = rdtsc();
        g_steps += o.1;
        std::hint::black_box(o);
        c_g.push(b - a);
    }
    let mean = |v: &[u64]| v.iter().sum::<u64>() as f64 / v.len() as f64;
    let (mct, mlll, mg) = (mean(&c_ct), mean(&c_lll), mean(&c_g));
    let p999 = |v: &mut Vec<u64>| {
        v.sort_unstable();
        v[(v.len() as f64 * 0.999) as usize]
    };
    let (dct, dlll, dg) = (median(&mut c_ct), median(&mut c_lll), median(&mut c_g));
    let (pct, plll) = (p999(&mut c_ct), p999(&mut c_lll));
    println!(
        "{},{},{},{:.0},{},{},{:.0},{},{:.0},{},{:.1},{:.1},{:.4},{:.4},{:.4},{:.3}",
        si.name, reps, dct, mct, pct, dlll, mlll, plll, mg, dg,
        lll_steps as f64 / reps as f64, g_steps as f64 / reps as f64,
        dct as f64 / dlll as f64, mct / mlll, pct as f64 / plll as f64, dct as f64 / dg as f64
    );
}

// ------------------------------------------------------------- dudect
fn dudect_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo, n: usize, imp: &str) {
    let p = si.p();
    let pl: [u64; PL] = to_limbs(&p);
    let pool = load(&format!("vectors/{}_pool.txt", si.name));
    assert_eq!(pool[0].kind, "fixed");
    let inps: Vec<Inp<C>> = pool.iter().map(|r| prep::<C>(si, r)).collect();
    let grams: Vec<_> = pool.iter().map(|r| exact::gram(&r.t, &r.x, &r.y, &p)).collect();
    let m = pool.len();
    // class sequence from a fixed xorshift (public, reproducible)
    let mut s: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let path = format!("results/raw/dudect_{}_{}.csv", si.name, imp);
    std::fs::create_dir_all("results/raw").unwrap();
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    for k in 0..n + 50 {
        let r = next();
        let class = (r & 1) as usize; // 0 fixed, 1 random
        let i = if class == 0 { 0 } else { 1 + ((r >> 1) as usize % (m - 1)) };
        let cyc = match imp {
            "ct" => {
                let a = rdtsc();
                let o = zi::reduce::<C, N, PL>(&inps[i].t, &inps[i].x, &inps[i].y, &pl, si.steps);
                let b = rdtsc();
                std::hint::black_box(o);
                b - a
            }
            "lll" => {
                let a = rdtsc();
                let o = exact::lll(&grams[i]);
                let b = rdtsc();
                std::hint::black_box(o);
                b - a
            }
            "gauss" => {
                let r = &pool[i];
                let a = rdtsc();
                let o = exact::gauss(&r.t, &r.x, &r.y, &p);
                let b = rdtsc();
                std::hint::black_box(o);
                b - a
            }
            _ => panic!("impl"),
        };
        if k >= 50 {
            writeln!(out, "{class},{cyc},{}", pool[i].nb).unwrap();
        }
    }
    eprintln!("wrote {path}");
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let cpu = std::env::var("RLC_CPU").ok().and_then(|s| s.parse().ok());
    match a.get(1).map(|s| s.as_str()) {
        Some("check") => {
            let mut all = true;
            for si in SETS.iter() {
                if a.len() > 2 && !a[2..].iter().any(|s| s == si.name) {
                    continue;
                }
                all &= with_set!(si.name, check_set, si);
            }
            println!("ALL {}", if all { "PASS" } else { "FAIL" });
            std::process::exit(if all { 0 } else { 1 });
        }
        Some("bench") => {
            if let Some(c) = cpu {
                pin(c);
            }
            let si = info(&a[2]);
            let reps: usize = a[3].parse().unwrap();
            with_set!(si.name, bench_set, si, reps);
        }
        Some("dudect") => {
            if let Some(c) = cpu {
                pin(c);
            }
            let si = info(&a[2]);
            let n: usize = a[3].parse().unwrap();
            with_set!(si.name, dudect_set, si, n, &a[4]);
        }
        _ => eprintln!("usage: rlc check [sets] | bench <set> <reps> | dudect <set> <n> <ct|lll|gauss>"),
    }
}
