// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! xq check [sets] | bench <set> <reps>
//! check: the exp64 rule in Rust against the Python model (bit-exact u,
//! nrd(u), big sub-step count), and the prototype's own schedule (64-bit
//! clamp, T_prov, exact cleanup) against the Python clamp64 model.
//! bench: cycles of the prototype schedule vs the exp64 schedule (its
//! T_exp64 count, no cleanup) on the honest vectors, pinned to one core.

use cqp_xcheck::sets::{from_limbs_signed, from_limbs_unsigned, to_limbs, SetInfo, SETS};
use cqp_xcheck::{with_set, zi, zi_exp};
use num_bigint::BigInt;

struct Row {
    kind: String,
    t: BigInt,
    x: BigInt,
    y: BigInt,
    steps: usize,
    nu: BigInt,
    u: [BigInt; 4],
    nbig: u64,
    nu_clamp: BigInt,
    nu_exact: BigInt,
}

fn hx(s: &str) -> BigInt {
    if let Some(r) = s.strip_prefix('-') {
        -BigInt::parse_bytes(r.as_bytes(), 16).unwrap()
    } else {
        BigInt::parse_bytes(s.as_bytes(), 16).unwrap()
    }
}

fn load(name: &str) -> Vec<Row> {
    std::fs::read_to_string(format!("vectors/{name}.txt"))
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split(' ').collect();
            Row {
                kind: f[0].to_string(),
                t: hx(f[1]),
                x: hx(f[2]),
                y: hx(f[3]),
                steps: f[4].parse().unwrap(),
                nu: hx(f[5]),
                u: [hx(f[6]), hx(f[7]), hx(f[8]), hx(f[9])],
                nbig: f[10].parse().unwrap(),
                nu_clamp: hx(f[11]),
                nu_exact: hx(f[12]),
            }
        })
        .collect()
}

fn info(name: &str) -> &'static SetInfo {
    SETS.iter().find(|s| s.name == name).expect("set")
}

fn check_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo) -> bool {
    let p = si.p();
    let pl: [u64; PL] = to_limbs(&p);
    let rows = load(si.name);
    let (mut ok_exp, mut ok_big, mut ok_clamp, mut exp_eq_exact, mut proto_eq_exact) = (0, 0, 0, 0, 0);
    let mut bad: Vec<String> = vec![];
    for r in &rows {
        assert!(r.t.bits() <= si.t_bits_max());
        let (t, x, y): ([u64; C], [u64; C], [u64; C]) = (to_limbs(&r.t), to_limbs(&r.x), to_limbs(&r.y));
        let (u, nu, nbig) = zi_exp::reduce_exp::<C, N, PL>(&t, &x, &y, &pl, r.steps);
        let ub: Vec<BigInt> = u.iter().map(from_limbs_signed).collect();
        let nub = from_limbs_unsigned(&nu);
        let e = nub == r.nu && ub.iter().zip(r.u.iter()).all(|(a, b)| a == b);
        ok_exp += e as usize;
        ok_big += (nbig == r.nbig) as usize;
        exp_eq_exact += (nub == r.nu_exact) as usize;
        let (_, nup) = zi::reduce::<C, N, PL>(&t, &x, &y, &pl, si.steps);
        let nupb = from_limbs_unsigned(&nup);
        ok_clamp += (nupb == r.nu_clamp) as usize;
        proto_eq_exact += (nupb == r.nu_exact) as usize;
        if !e || nupb != r.nu_exact {
            bad.push(format!("{}(exp64 match {}, proto==exact {})", r.kind, e, nupb == r.nu_exact));
        }
    }
    let n = rows.len();
    println!(
        "{} rows={} rust_exp64==py_exp64(u, nrd u) {}/{} big-substep count {}/{} rust_exp64==exact {}/{} \
         prototype==py_clamp64+cleanup {}/{} prototype==exact {}/{}",
        si.name, n, ok_exp, n, ok_big, n, exp_eq_exact, n, ok_clamp, n, proto_eq_exact, n
    );
    println!("  {} rows where exp64 mismatches or the prototype misses the exact output: {:?}", si.name, bad);
    ok_exp == n && ok_big == n && ok_clamp == n && exp_eq_exact == n
}

#[inline(always)]
fn rdtsc() -> u64 {
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

fn stats(v: &mut Vec<u64>) -> (u64, f64) {
    v.sort_unstable();
    let mean = v.iter().sum::<u64>() as f64 / v.len() as f64;
    (v[v.len() / 2], mean)
}

fn bench_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo, reps: usize) {
    let p = si.p();
    let pl: [u64; PL] = to_limbs(&p);
    let rows: Vec<Row> = load(si.name).into_iter().filter(|r| r.kind == "honest").collect();
    let inps: Vec<([u64; C], [u64; C], [u64; C], usize)> =
        rows.iter().map(|r| (to_limbs(&r.t), to_limbs(&r.x), to_limbs(&r.y), r.steps)).collect();
    let mut cp = vec![];
    let mut ce = vec![];
    let mut ce_same = vec![];
    for k in 0..reps + 10 {
        let (t, x, y, st) = &inps[k % inps.len()];
        let a = rdtsc();
        let o = zi::reduce::<C, N, PL>(t, x, y, &pl, si.steps);
        let b = rdtsc();
        std::hint::black_box(o);
        let c0 = rdtsc();
        let o2 = zi_exp::reduce_exp::<C, N, PL>(t, x, y, &pl, *st);
        let c1 = rdtsc();
        std::hint::black_box(o2);
        let d0 = rdtsc();
        let o3 = zi_exp::reduce_exp::<C, N, PL>(t, x, y, &pl, si.steps);
        let d1 = rdtsc();
        std::hint::black_box(o3);
        if k >= 10 {
            cp.push(b - a);
            ce.push(c1 - c0);
            ce_same.push(d1 - d0);
        }
    }
    let (mp, ap) = stats(&mut cp);
    let (me, ae) = stats(&mut ce);
    let (ms, _) = stats(&mut ce_same);
    // set,prototype_steps,prototype_median,exp64_steps,exp64_median,ratio,per_step_ratio
    println!(
        "{},{},{},{},{},{:.4},{:.4},{:.0},{:.0}",
        si.name, si.steps + 1, mp, inps[0].3, me, me as f64 / mp as f64, ms as f64 / mp as f64, ap, ae
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("check") => {
            let names: Vec<&str> = if args.len() > 2 { args[2..].iter().map(|s| s.as_str()).collect() } else { SETS.iter().map(|s| s.name).collect() };
            let mut all = true;
            for n in names {
                let si = info(n);
                all &= with_set!(si.name, check_set, si);
            }
            println!("ALL {}", if all { "OK" } else { "FAILED" });
        }
        Some("bench") => {
            pin(std::env::var("RLC_CPU").ok().and_then(|s| s.parse().ok()).unwrap_or(6));
            let si = info(&args[2]);
            let reps: usize = args[3].parse().unwrap();
            with_set!(si.name, bench_set, si, reps);
        }
        _ => eprintln!("usage: xq check [sets] | bench <set> <reps>"),
    }
}
