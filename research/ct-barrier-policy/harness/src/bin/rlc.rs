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

#[cfg(target_arch = "x86_64")]
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

/// aarch64: the virtual counter (only meaningful on real hardware, not
/// under qemu-user, where it is emulated).
#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn rdtsc() -> u64 {
    let t: u64;
    unsafe { core::arch::asm!("isb", "mrs {0}, cntvct_el0", "isb", out(reg) t, options(nostack)) };
    t
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
    let adv = load(&format!("vectors/{}_adv.txt", si.name));
    let n_adv = adv.len();
    rows.extend(adv);
    let (mut n, mut ok_p64, mut ok_exact, mut ok_lam, mut ok_lll, mut ok_gauss, mut ok_q) =
        (0, 0, 0, 0, 0, 0, 0);
    let (mut n_lam, mut n_lll_short) = (0, 0);
    let mut maxbits_t = 0;
    #[allow(unused_mut)]
    let mut gmp_agree = 0;
    for r in &rows {
        let inp = prep::<C>(si, r);
        let (u, nu) = zi::reduce::<C, N, PL>(&inp.t, &inp.x, &inp.y, &pl, si.steps);
        let nu_b = from_limbs_unsigned(&nu);
        let ub: Vec<BigInt> = u.iter().map(from_limbs_signed).collect();
        let nrd_u = &ub[0] * &ub[0] + &ub[1] * &ub[1] + &p * (&ub[2] * &ub[2] + &ub[3] * &ub[3]);
        // u is in the ideal: nrd divisible by T, nonzero, and equals the CT norm
        let in_ideal = nrd_u == nu_b && (&nu_b % &r.t) == BigInt::from(0) && nu_b > BigInt::from(0);
        n += 1;
        // corner-case adversarial lattices are not ideals of norm T: n/a there
        ok_q += (in_ideal || r.nu_p64 == BigInt::from(0)) as usize;
        // adversarial rows carry no p64 reference (0): compare with lambda_1 instead
        ok_p64 += (nu_b == r.nu_p64 || (r.nu_p64 == BigInt::from(0) && nu_b == r.lam1)) as usize;
        ok_exact += (nu_b == r.nu_exact) as usize;
        // RLC_FAST=1 (used under qemu): skip the variable-time baselines
        let fast = std::env::var("RLC_FAST").is_ok();
        if fast {
            ok_gauss += 1;
        } else {
            let (g_nu, _) = exact::gauss(&r.t, &r.x, &r.y, &p);
            ok_gauss += (g_nu == nu_b) as usize;
        }
        if r.lam1 > BigInt::from(0) {
            n_lam += 1;
            ok_lam += (nu_b == r.lam1) as usize;
            let (b1, _) = if fast { (r.lam1.clone(), 0) } else { exact::lll(&exact::gram(&r.t, &r.x, &r.y, &p)) };
            ok_lll += (b1 == r.lam1) as usize;
            n_lll_short += (b1 > r.lam1) as usize;
            #[cfg(feature = "gmp")]
            {
                use rust_lagrange_ct::exact_gmp as eg;
                let (t, x, y, gp) = (eg::conv(&r.t), eg::conv(&r.x), eg::conv(&r.y), eg::conv(&p));
                let (b1g, _) = eg::lll(&eg::gram(&t, &x, &y, &gp));
                let (ng, _) = eg::gauss(&t, &x, &y, &gp);
                gmp_agree += (b1g == eg::conv(&b1) && ng == eg::conv(&r.lam1)) as usize;
            }
        }
        maxbits_t = maxbits_t.max(r.t.bits());
        if r.nu_p64 == BigInt::from(0) {
            println!("  {} {}: exact steps {} schedule {} ct==lambda1 {}", si.name, r.kind, r.nb, si.steps, nu_b == r.lam1);
        }
    }
    println!("  {} adversarial rows: {}", si.name, n_adv);
    println!(
        "{} rows={} ct==py_p64 {}/{} ct==py_exact {}/{} ct==lambda1 {}/{} ct==rust_exact_gauss {}/{} \
         ct_in_ideal {}/{} rust_lll_b1==lambda1 {}/{} (lll b1 > lambda1: {}) maxbits(T)={} bound={} gmp_baselines_agree={}/{}",
        si.name, n, ok_p64, n, ok_exact, n, ok_lam, n_lam, ok_gauss, n, ok_q, n, ok_lll, n_lam,
        n_lll_short, maxbits_t, si.t_bits_max(), gmp_agree, n_lam
    );
    ok_p64 == n && ok_lam == n_lam && ok_q == n
}

// ------------------------------------------------------------- one
/// Run the reduction once on one input, for control-flow trace
/// comparison under qemu -d exec (the trace is filtered to the address
/// ranges of zi::reduce and ct::exact_round, so the input-dependent
/// parsing here is outside it). `which`: `fixed`, `p<k>` (pool row k),
/// `a<k>` (adversarial row k), `c<k>` (check row k).
fn one_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo, which: &str) {
    let p = si.p();
    let pl: [u64; PL] = to_limbs(&p);
    let (file, k) = match which.as_bytes()[0] {
        b'f' => ("pool", 0usize),
        b'p' => ("pool", which[1..].parse().unwrap()),
        b'a' => ("adv", which[1..].parse().unwrap()),
        b'c' => ("check", which[1..].parse().unwrap()),
        _ => panic!("which"),
    };
    let rows = load(&format!("vectors/{}_{}.txt", si.name, file));
    let r = &rows[k];
    let inp = prep::<C>(si, r);
    let (t, x, y) = (std::hint::black_box(inp.t), std::hint::black_box(inp.x), std::hint::black_box(inp.y));
    let (_u, nu) = zi::reduce::<C, N, PL>(&t, &x, &y, &pl, si.steps);
    let nu_b = from_limbs_unsigned(&std::hint::black_box(nu));
    let want = if r.nu_p64 == BigInt::from(0) { &r.lam1 } else { &r.nu_p64 };
    println!("{} {} {} barrier={} ok={}", si.name, which, r.kind, rust_lagrange_ct::ct::BARRIER, &nu_b == want);
}

// ------------------------------------------------------------- bench
fn bench_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo, reps: usize) {
    let p = si.p();
    let pl: [u64; PL] = to_limbs(&p);
    let pool = load(&format!("vectors/{}_pool.txt", si.name));
    let inps: Vec<Inp<C>> = pool.iter().map(|r| prep::<C>(si, r)).collect();
    let grams: Vec<_> = pool.iter().map(|r| exact::gram(&r.t, &r.x, &r.y, &p)).collect();
    #[cfg(feature = "gmp")]
    let gp = rust_lagrange_ct::exact_gmp::conv(&p);
    #[cfg(feature = "gmp")]
    let gin: Vec<_> = pool
        .iter()
        .map(|r| {
            use rust_lagrange_ct::exact_gmp::conv;
            let (t, x, y) = (conv(&r.t), conv(&r.x), conv(&r.y));
            let g = rust_lagrange_ct::exact_gmp::gram(&t, &x, &y, &gp);
            (t, x, y, g)
        })
        .collect();
    let m = pool.len();
    let names = ["ct_zi", "lll_num", "gauss_num", "lll_gmp", "gauss_gmp"];
    let mut cyc: Vec<Vec<u64>> = vec![vec![]; 5];
    let mut steps = [0usize; 5];
    for k in 0..reps + 20 {
        let i = k % m;
        let r = &pool[i];
        let mut rec = |j: usize, c: u64, s: usize| {
            if k >= 20 {
                cyc[j].push(c);
                steps[j] += s;
            }
        };
        let a = rdtsc();
        let o = zi::reduce::<C, N, PL>(&inps[i].t, &inps[i].x, &inps[i].y, &pl, si.steps);
        let b = rdtsc();
        std::hint::black_box(o);
        rec(0, b - a, si.steps);
        let a = rdtsc();
        let o = exact::lll(&grams[i]);
        let b = rdtsc();
        rec(1, b - a, o.1);
        std::hint::black_box(o);
        let a = rdtsc();
        let o = exact::gauss(&r.t, &r.x, &r.y, &p);
        let b = rdtsc();
        rec(2, b - a, o.1);
        std::hint::black_box(o);
        #[cfg(feature = "gmp")]
        {
            let a = rdtsc();
            let o = rust_lagrange_ct::exact_gmp::lll(&gin[i].3);
            let b = rdtsc();
            rec(3, b - a, o.1);
            std::hint::black_box(o);
            let a = rdtsc();
            let o = rust_lagrange_ct::exact_gmp::gauss(&gin[i].0, &gin[i].1, &gin[i].2, &gp);
            let b = rdtsc();
            rec(4, b - a, o.1);
            std::hint::black_box(o);
        }
    }
    // name,set,reps,median,mean,p999,cv,mean_steps
    for j in 0..5 {
        let v = &mut cyc[j];
        if v.is_empty() {
            continue;
        }
        let n = v.len() as f64;
        let mean = v.iter().sum::<u64>() as f64 / n;
        let var = v.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / n;
        v.sort_unstable();
        println!(
            "{},{},{},{},{:.0},{},{:.4},{:.1}",
            si.name, names[j], v.len(), v[v.len() / 2], mean, v[(n * 0.999) as usize],
            var.sqrt() / mean, steps[j] as f64 / n
        );
    }
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
    let mut s: u64 = std::env::var("RLC_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(0x9e37_79b9_7f4a_7c15);
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let tag = std::env::var("RLC_TAG").unwrap_or_default();
    let path = format!("results/raw/dudect_{}_{}{}.csv", si.name, imp, tag);
    std::fs::create_dir_all("results/raw").unwrap();
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    for k in 0..n + 50 {
        let r = next();
        let class = (r & 1) as usize; // 0 fixed, 1 random
        let i = if class == 0 { 0 } else { 1 + ((r >> 1) as usize % (m - 1)) };
        let cyc = match imp {
            "ct" => {
                // copy the chosen input to the same stack buffer for both
                // classes, so the timed call reads from the same addresses
                let (t, x, y) = (inps[i].t, inps[i].x, inps[i].y);
                let (t, x, y) = (std::hint::black_box(t), std::hint::black_box(x), std::hint::black_box(y));
                let a = rdtsc();
                let o = zi::reduce::<C, N, PL>(&t, &x, &y, &pl, si.steps);
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
            #[cfg(feature = "gmp")]
            "lll_gmp" => {
                use rust_lagrange_ct::exact_gmp as eg;
                let g = eg::gram(&eg::conv(&pool[i].t), &eg::conv(&pool[i].x), &eg::conv(&pool[i].y), &eg::conv(&p));
                let a = rdtsc();
                let o = eg::lll(&g);
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

// ------------------------------------------------------------- ctgrind
#[cfg(feature = "ctgrind")]
extern "C" {
    fn rlc_secret(p: *mut u8, n: usize);
    fn rlc_public(p: *mut u8, n: usize);
}

/// Run the reduction on `count` vectors with T, x, y marked secret
/// (undefined) for memcheck. A secret-dependent branch or address shows as
/// a memcheck error inside zi::reduce.
#[cfg(feature = "ctgrind")]
fn ctgrind_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo, count: usize) {
    let p = si.p();
    let pl: [u64; PL] = to_limbs(&p);
    let rows = load(&format!("vectors/{}_pool.txt", si.name));
    for r in rows.iter().take(count) {
        let mut inp = prep::<C>(si, r);
        unsafe {
            rlc_secret(inp.t.as_mut_ptr() as *mut u8, 8 * C);
            rlc_secret(inp.x.as_mut_ptr() as *mut u8, 8 * C);
            rlc_secret(inp.y.as_mut_ptr() as *mut u8, 8 * C);
        }
        if std::env::var("RLC_CTGRIND_CONTROL").is_ok() {
            // positive control: a deliberate branch on a secret bit
            if std::hint::black_box(inp.x[0]) & 1 == 1 {
                std::hint::black_box(1u64);
            }
        }
        let (mut u, mut nu) = zi::reduce::<C, N, PL>(&inp.t, &inp.x, &inp.y, &pl, si.steps);
        unsafe {
            rlc_public(nu.as_mut_ptr() as *mut u8, 8 * N);
            rlc_public(u.as_mut_ptr() as *mut u8, 8 * 4 * C);
        }
        assert_eq!(from_limbs_unsigned(&nu), r.nu_p64);
    }
    println!("{} ctgrind: {} runs done", si.name, count);
}

// ------------------------------------------------------------- parts
fn parts_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo, reps: usize) {
    use rust_lagrange_ct::zi::parts::*;
    let p = si.p();
    let pl: [u64; PL] = to_limbs(&p);
    let r = &load(&format!("vectors/{}_pool.txt", si.name))[1];
    let inp = prep::<C>(si, r);
    let mut one = [0u64; C];
    one[0] = 1;
    let u: zi::Vec4<C> = [inp.t, [0; C], [0; C], [0; C]];
    let mut v: zi::Vec4<C> = [inp.x, inp.y, one, [0; C]];
    let nu: [u64; N] = nrd_part::<C, N, PL>(&u, &pl);
    let med = |f: &mut dyn FnMut()| {
        let mut c = vec![];
        for _ in 0..reps {
            let a = rdtsc();
            f();
            c.push(rdtsc() - a);
        }
        c.sort_unstable();
        c[c.len() / 2]
    };
    let h = med(&mut || { std::hint::black_box(herm_part::<C, N, PL>(&v, &u, &pl)); });
    let (h0, _) = herm_part::<C, N, PL>(&v, &u, &pl);
    let q = med(&mut || { std::hint::black_box(quotient_part::<N>(&h0, &nu)); });
    let n = med(&mut || { std::hint::black_box(nrd_part::<C, N, PL>(&v, &pl)); });
    let up = med(&mut || { update_part::<C>(&mut v, &u, std::hint::black_box(3), std::hint::black_box(-2)); std::hint::black_box(&v); });
    let (mut uu, mut vv, mut nnu) = (u, v, nu);
    let mut nnv = nu;
    let cl = med(&mut || { cleanup_part::<C, N, PL>(&mut uu, &mut vv, &mut nnu, &mut nnv, &pl); std::hint::black_box(&uu); });
    let full = h + 2 * q + n + up;
    println!(
        "{},herm={},quotient={} (x2),nrd={},update={},sum_per_step={},herm%={:.0},quot%={:.0},nrd%={:.0},upd%={:.0},cleanup_once={},steps={},est_total={}",
        si.name, h, q, n, up, full,
        100.0 * h as f64 / full as f64, 200.0 * q as f64 / full as f64,
        100.0 * n as f64 / full as f64, 100.0 * up as f64 / full as f64,
        cl, si.steps, full as usize * si.steps + cl as usize
    );
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
        Some("one") => {
            let si = info(&a[2]);
            with_set!(si.name, one_set, si, &a[3]);
        }
        Some("barrier") => println!("{}", rust_lagrange_ct::ct::BARRIER),
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
        Some("parts") => {
            if let Some(c) = cpu {
                pin(c);
            }
            for si in SETS.iter() {
                with_set!(si.name, parts_set, si, 5000);
            }
        }
        #[cfg(feature = "ctgrind")]
        Some("ctgrind") => {
            let si = info(&a[2]);
            let n: usize = a[3].parse().unwrap();
            with_set!(si.name, ctgrind_set, si, n);
        }
        _ => eprintln!("usage: rlc check [sets] | bench <set> <reps> | dudect <set> <n> <ct|lll|gauss>"),
    }
}
