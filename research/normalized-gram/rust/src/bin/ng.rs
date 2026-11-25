// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! ng check | bench <set> <reps> | memcheck <set> [control]
//!
//! check: every line of vectors_ng/<set>.txt and vectors_ng/toy.txt (written
//! by gen_ng.py). Rejects must fail every constructor. Accepted inputs: the
//! constructors must reproduce the Python normalized Gram; the normalized
//! reducer, Rick's combined candidate (a60fd06, numerator Gram) and the
//! original exp64 (1db834d) must end on the Python final pair; the two
//! carried-Gram reducers must reproduce the Python physical decision
//! traces of their own scaling, and both must compose to the same logical
//! (exact) trace as the oracle.
//! bench: TSC medians of setup, reduction and both, per variant, on the
//! honest signing lifts and on the pre-reduced inputs, native schedule.

use ng_xcheck::ngram::{self, Gram};
use ng_xcheck::sets::{from_limbs_signed, to_limbs, SetInfo, SETS};
use ng_xcheck::zi::Vec4;
use ng_xcheck::{orig_exp, with_set, zi_exp};
use num_bigint::BigInt;
use num_traits::Zero;

struct Line {
    p: BigInt,
    cls: String,
    kind: String,
    shape: String,
    t: BigInt,
    u: [BigInt; 4],
    v: [BigInt; 4],
    steps: usize,
    accept: bool,
    g: [BigInt; 4],
    nbig_num: u64,
    nbig_norm: u64,
    phys_num: u64,
    phys_norm: u64,
    logical: u64,
    fu: [BigInt; 4],
    fv: [BigInt; 4],
}

fn hx(s: &str) -> BigInt {
    if let Some(r) = s.strip_prefix('-') {
        -BigInt::parse_bytes(r.as_bytes(), 16).unwrap()
    } else {
        BigInt::parse_bytes(s.as_bytes(), 16).unwrap()
    }
}

fn four(f: &[&str]) -> [BigInt; 4] {
    [hx(f[0]), hx(f[1]), hx(f[2]), hx(f[3])]
}

fn load(file: &str) -> Vec<Line> {
    std::fs::read_to_string(format!("vectors_ng/{file}.txt"))
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split(' ').collect();
            let accept = f[14] == "1";
            let z = || [BigInt::zero(), BigInt::zero(), BigInt::zero(), BigInt::zero()];
            Line {
                p: hx(f[0]),
                cls: f[1].into(),
                kind: f[2].into(),
                shape: f[3].into(),
                t: hx(f[4]),
                u: four(&f[5..9]),
                v: four(&f[9..13]),
                steps: f[13].parse().unwrap(),
                accept,
                g: if accept { four(&f[15..19]) } else { z() },
                nbig_num: if accept { f[19].parse().unwrap() } else { 0 },
                nbig_norm: if accept { f[20].parse().unwrap() } else { 0 },
                phys_num: if accept { f[21].parse().unwrap() } else { 0 },
                phys_norm: if accept { f[22].parse().unwrap() } else { 0 },
                logical: if accept { f[23].parse().unwrap() } else { 0 },
                fu: if accept { four(&f[24..28]) } else { z() },
                fv: if accept { four(&f[28..32]) } else { z() },
            }
        })
        .collect()
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn v4<const C: usize>(a: &[BigInt; 4]) -> Vec4<C> {
    [to_limbs(&a[0]), to_limbs(&a[1]), to_limbs(&a[2]), to_limbs(&a[3])]
}

fn big4<const C: usize>(a: &Vec4<C>) -> [BigInt; 4] {
    [from_limbs_signed(&a[0]), from_limbs_signed(&a[1]), from_limbs_signed(&a[2]), from_limbs_signed(&a[3])]
}

fn gram_big<const L: usize>(g: &Gram<L>) -> [BigInt; 4] {
    big4::<L>(g)
}

/// Trace recorder: physical string and logical (composed) steps.
#[derive(Default)]
struct Trace {
    phys: String,
    logical: Vec<(BigInt, BigInt, u8)>,
    acc: (BigInt, BigInt),
    nbig: u64,
}

impl Trace {
    fn push(&mut self, m0: i64, m1: i64, s: u64, big: u64, sw: u64) {
        let swb = (sw & 1) as u8;
        self.phys.push_str(&format!("{m0},{m1},{s},{big},{swb};"));
        self.acc.0 += BigInt::from(m0) << s;
        self.acc.1 += BigInt::from(m1) << s;
        self.nbig += big;
        if big == 0 {
            let a = std::mem::take(&mut self.acc);
            self.logical.push((a.0, a.1, swb));
        }
    }
    fn logical_digest(&self) -> u64 {
        let mut l = self.logical.clone();
        while l.last().is_some_and(|(a, b, w)| a.is_zero() && b.is_zero() && *w == 0) {
            l.pop();
        }
        fnv(&l.iter().map(|(a, b, w)| format!("{a},{b},{w};")).collect::<String>())
    }
}

#[derive(Default, Debug)]
struct Tally {
    n: usize,
    rejects_ok: usize,
    rejects: usize,
    ctor_ok: usize,
    ctor_restoring_ok: usize,
    ctor_general_ok: usize,
    norm_final: usize,
    norm_phys: usize,
    norm_nbig: usize,
    comb_final: usize,
    comb_phys: usize,
    comb_nbig: usize,
    logical_same: usize,
    rick_reduce_exp: usize,
    rick_reduce_exp_n: usize,
    orig_final: usize,
    inv_ok: usize,
    phys_differ: usize,
    bad: Vec<String>,
}

fn check_line<const C: usize, const N: usize, const PL: usize>(l: &Line, t: &mut Tally) {
    let p: [u64; PL] = to_limbs(&l.p);
    let tt: [u64; C] = to_limbs(&l.t);
    let u0: Vec4<C> = v4(&l.u);
    let v0: Vec4<C> = v4(&l.v);
    let tinv = ngram::inv_pow2(&tt);
    if !l.accept {
        t.rejects += 1;
        let (_, _, _, ok1) = ngram::gram_hnf::<C, N, PL>(&tt, &v0[0], &v0[1], &p, &tinv);
        let (_, _, _, ok2) = ngram::gram_hnf_restoring::<C, N, PL>(&tt, &v0[0], &v0[1], &p);
        let (_, ok3) = ngram::gram_general::<C, N, PL>(&tt, &u0, &v0, &p, &tinv);
        if ok1 == 0 && ok2 == 0 && ok3 == 0 {
            t.rejects_ok += 1;
        } else {
            t.bad.push(format!("reject {} accepted", l.kind));
        }
        return;
    }
    t.n += 1;
    // Hensel inverse: T T^-1 = 1, and the warm composition N^-1 q^-1 = (N q)^-1
    // (here with q = 2^(2e-5) - 3^2-shaped odd b = 2^61 - 9 times the low limb, odd).
    {
        let mut one = [0u64; C];
        one[0] = 1;
        let mut b = [0u64; C];
        b[0] = ((1u64 << 61) - 9).wrapping_mul(tt[0] | 1) | 1;
        b[C - 1] = tt[0] >> 1;
        let tb = ngram::mul_lo(&tt, &b);
        if l.t.bit(0) && ngram::mul_lo(&tt, &tinv) == one
            && ngram::mul_lo(&tinv, &ngram::inv_pow2(&b)) == ngram::inv_pow2(&tb)
        {
            t.inv_ok += 1;
        } else {
            eprintln!("FAIL inverse {}", l.kind);
            t.bad.push(format!("inverse {}", l.kind));
        }
    }
    let mut ok_all = true;
    let mut fail = |what: &str, ok: bool, cnt: &mut usize| {
        if ok {
            *cnt += 1;
        } else {
            ok_all = false;
            eprintln!("FAIL {} {} {}", l.cls, l.kind, what);
        }
    };
    // constructors
    let (g, okc) = if l.shape == "hnf" {
        let (_, _, g, ok) = ngram::gram_hnf::<C, N, PL>(&tt, &v0[0], &v0[1], &p, &tinv);
        let (_, _, g2, ok2) = ngram::gram_hnf_restoring::<C, N, PL>(&tt, &v0[0], &v0[1], &p);
        fail("restoring ctor", ok2 == u64::MAX && g2 == g, &mut t.ctor_restoring_ok);
        (g, ok)
    } else {
        t.ctor_restoring_ok += 1;
        let (g, ok) = ngram::gram_general::<C, N, PL>(&tt, &u0, &v0, &p, &tinv);
        (g, ok)
    };
    let (gg, okg) = ngram::gram_general::<C, N, PL>(&tt, &u0, &v0, &p, &tinv);
    fail("general ctor", okg == u64::MAX && gram_big(&gg) == l.g, &mut t.ctor_general_ok);
    fail("ctor", okc == u64::MAX && gram_big(&g) == l.g, &mut t.ctor_ok);
    // normalized reducer
    let (mut u, mut v, mut gn) = (u0, v0, g);
    ngram::order(&mut u, &mut v, &mut gn);
    let mut tr = Trace::default();
    ngram::reduce_loop::<C, C, _>(&mut u, &mut v, &mut gn, l.steps, |a, b, s, g, w| tr.push(a, b, s, g, w));
    fail("norm final", big4(&u) == l.fu && big4(&v) == l.fv, &mut t.norm_final);
    fail("norm phys", fnv(&tr.phys) == l.phys_norm, &mut t.norm_phys);
    fail("norm nbig", tr.nbig == l.nbig_norm, &mut t.norm_nbig);
    // combined candidate on the numerator Gram
    let (mut uc, mut vc) = (u0, v0);
    let mut gc: Gram<N> = ngram::setup_numerator::<C, N, PL>(&mut uc, &mut vc, &p);
    let mut tc = Trace::default();
    ngram::reduce_loop::<C, N, _>(&mut uc, &mut vc, &mut gc, l.steps, |a, b, s, g, w| tc.push(a, b, s, g, w));
    fail("comb final", big4(&uc) == l.fu && big4(&vc) == l.fv, &mut t.comb_final);
    fail("comb phys", fnv(&tc.phys) == l.phys_num, &mut t.comb_phys);
    fail("comb nbig", tc.nbig == l.nbig_num, &mut t.comb_nbig);
    let same = tr.logical_digest() == l.logical && tc.logical_digest() == l.logical;
    fail("logical", same, &mut t.logical_same);
    if fnv(&tr.phys) != fnv(&tc.phys) {
        t.phys_differ += 1;
    }
    // Rick's unmodified a60fd06 reduce_exp on the HNF lift
    if l.shape == "hnf" {
        t.rick_reduce_exp_n += 1;
        let (ur, _, nb) = zi_exp::reduce_exp::<C, N, PL>(&u0[0], &v0[0], &v0[1], &p, l.steps);
        fail("rick reduce_exp", big4(&ur) == l.fu && nb == l.nbig_num, &mut t.rick_reduce_exp);
    }
    // original exp64 (1db834d)
    let (uo, vo) = orig_exp::reduce_exp_uv::<C, N, PL>(&u0, &v0, &p, l.steps);
    fail("orig final", big4(&uo) == l.fu && big4(&vo) == l.fv, &mut t.orig_final);
    if !ok_all {
        t.bad.push(format!("{} {}", l.cls, l.kind));
    }
}

fn check_file<const C: usize, const N: usize, const PL: usize>(file: &str) -> bool {
    let mut t = Tally::default();
    for l in load(file) {
        check_line::<C, N, PL>(&l, &mut t);
    }
    let all = t.bad.is_empty();
    println!(
        "{file}: accepted {} | ctor {} restoring {} general {} | normalized final {} phys {} nbig {} | combined final {} phys {} nbig {} | logical equal {} | a60fd06 reduce_exp {}/{} | original final {} | inverse checks {} | inputs whose physical traces differ {} | rejects refused {}/{} | {}",
        t.n, t.ctor_ok, t.ctor_restoring_ok, t.ctor_general_ok, t.norm_final, t.norm_phys, t.norm_nbig, t.comb_final,
        t.comb_phys, t.comb_nbig, t.logical_same, t.rick_reduce_exp, t.rick_reduce_exp_n, t.orig_final, t.inv_ok, t.phys_differ,
        t.rejects_ok, t.rejects, if all { "OK" } else { "FAILED" }
    );
    if !all {
        println!("  bad: {:?}", t.bad);
    }
    all
}

fn check_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo) -> bool {
    check_file::<C, N, PL>(si.name)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn rdtsc() -> u64 {
    // SAFETY: lfence and rdtsc have no memory effects.
    unsafe {
        core::arch::x86_64::_mm_lfence();
        let t = core::arch::x86_64::_rdtsc();
        core::arch::x86_64::_mm_lfence();
        t
    }
}

fn pin(cpu: usize) {
    // SAFETY: plain libc affinity call on this thread.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(cpu, &mut set);
        let r = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        eprintln!("pinned to cpu {cpu}: {}", if r == 0 { "ok" } else { "FAILED" });
    }
}

fn median(v: &mut [u64]) -> u64 {
    v.sort_unstable();
    v[v.len() / 2]
}

const VARIANTS: [&str; 15] = [
    "orig_setup", "orig_reduce", "orig_full",
    "comb_setup", "comb_reduce", "comb_full",
    "norm_setup_cold", "norm_setup_warm", "norm_setup_restoring", "norm_setup_general", "norm_reduce", "norm_full",
    "prered_comb_setup", "prered_norm_setup_general", "inv_pow2",
];

fn bench_set<const C: usize, const N: usize, const PL: usize>(si: &SetInfo, reps: usize) {
    let p: [u64; PL] = to_limbs(&si.p());
    let lines = load(si.name);
    let honest: Vec<&Line> = lines.iter().filter(|l| l.kind == "lift_honest").collect();
    let prered: Vec<&Line> = lines.iter().filter(|l| l.cls == "prered-guard256").collect();
    let steps = honest[0].steps;
    let mut cyc: Vec<Vec<u64>> = vec![vec![]; VARIANTS.len()];
    let warm = 10;
    for k in 0..reps + warm {
        let l = honest[k % honest.len()];
        let lp = prered[k % prered.len()];
        let t: [u64; C] = to_limbs(&l.t);
        let u0: Vec4<C> = v4(&l.u);
        let v0: Vec4<C> = v4(&l.v);
        let (x, y) = (v0[0], v0[1]);
        let tp: [u64; C] = to_limbs(&lp.t);
        let up: Vec4<C> = v4(&lp.u);
        let vp: Vec4<C> = v4(&lp.v);
        // cached factor inverses for the warm path (data-independent cost)
        let ninv = ngram::inv_pow2(&t);
        let mut qinv = [0u64; C];
        qinv[0] = 1;
        let mut tm = [0u64; 15];
        for j in 0..VARIANTS.len() {
            let which = (j + k) % VARIANTS.len(); // rotate the order
            let (a, b);
            match which {
                0 => {
                    let (mut u, mut v) = (u0, v0);
                    a = rdtsc();
                    let r = orig_exp::setup_uv::<C, N, PL>(&mut u, &mut v, &p);
                    b = rdtsc();
                    std::hint::black_box((r, u, v));
                }
                1 => {
                    let (mut u, mut v) = (u0, v0);
                    let (mut nu, mut nv) = orig_exp::setup_uv::<C, N, PL>(&mut u, &mut v, &p);
                    a = rdtsc();
                    orig_exp::reduce_loop::<C, N, PL>(&mut u, &mut v, &mut nu, &mut nv, &p, steps);
                    b = rdtsc();
                    std::hint::black_box((u, v, nu, nv));
                }
                2 => {
                    a = rdtsc();
                    let r = orig_exp::reduce_exp_uv::<C, N, PL>(&u0, &v0, &p, steps);
                    b = rdtsc();
                    std::hint::black_box(r);
                }
                3 => {
                    let (mut u, mut v) = (u0, v0);
                    a = rdtsc();
                    let g = ngram::setup_comb::<C, N, PL>(&mut u, &mut v, &p);
                    b = rdtsc();
                    std::hint::black_box((g, u, v));
                }
                4 => {
                    let (mut u, mut v) = (u0, v0);
                    let mut g = ngram::setup_comb::<C, N, PL>(&mut u, &mut v, &p);
                    a = rdtsc();
                    ngram::reduce_comb::<C, N>(&mut u, &mut v, &mut g, steps);
                    b = rdtsc();
                    std::hint::black_box((u, v, g));
                }
                5 => {
                    a = rdtsc();
                    let r = ngram::full_comb::<C, N, PL>(&t, &x, &y, &p, steps);
                    b = rdtsc();
                    std::hint::black_box(r);
                }
                6 => {
                    a = rdtsc();
                    let r = ngram::setup_norm_cold::<C, N, PL>(&t, &x, &y, &p);
                    b = rdtsc();
                    std::hint::black_box(r);
                }
                7 => {
                    a = rdtsc();
                    let r = ngram::setup_norm_warm::<C, N, PL>(&t, &x, &y, &p, &ninv, &qinv);
                    b = rdtsc();
                    std::hint::black_box(r);
                }
                8 => {
                    a = rdtsc();
                    let r = ngram::setup_norm_restoring::<C, N, PL>(&t, &x, &y, &p);
                    b = rdtsc();
                    std::hint::black_box(r);
                }
                9 => {
                    a = rdtsc();
                    let r = ngram::setup_norm_general::<C, N, PL>(&t, &u0, &v0, &p);
                    b = rdtsc();
                    std::hint::black_box(r);
                }
                10 => {
                    let (mut u, mut v, mut g, _) = ngram::setup_norm_cold::<C, N, PL>(&t, &x, &y, &p);
                    a = rdtsc();
                    ngram::reduce_norm::<C>(&mut u, &mut v, &mut g, steps);
                    b = rdtsc();
                    std::hint::black_box((u, v, g));
                }
                11 => {
                    a = rdtsc();
                    let r = ngram::full_norm::<C, N, PL>(&t, &x, &y, &p, steps);
                    b = rdtsc();
                    std::hint::black_box(r);
                }
                12 => {
                    let (mut u, mut v) = (up, vp);
                    a = rdtsc();
                    let g = ngram::setup_comb::<C, N, PL>(&mut u, &mut v, &p);
                    b = rdtsc();
                    std::hint::black_box((g, u, v));
                }
                13 => {
                    a = rdtsc();
                    let r = ngram::setup_norm_general::<C, N, PL>(&tp, &up, &vp, &p);
                    b = rdtsc();
                    std::hint::black_box(r);
                }
                _ => {
                    a = rdtsc();
                    let r = ngram::inv_pow2(std::hint::black_box(&t));
                    b = rdtsc();
                    std::hint::black_box(r);
                }
            }
            tm[which] = b - a;
        }
        if k >= warm {
            for j in 0..VARIANTS.len() {
                cyc[j].push(tm[j]);
            }
        }
    }
    let meds: Vec<String> = cyc.iter_mut().map(|v| median(v).to_string()).collect();
    println!("{},{},{},{}", si.name, steps, reps, meds.join(","));
}

#[cfg(feature = "memcheck")]
fn memcheck<const C: usize, const N: usize, const PL: usize>(si: &SetInfo, control: bool) {
    unsafe extern "C" {
        fn rlc_secret(p: *mut u64, bytes: usize);
    }
    let p: [u64; PL] = to_limbs(&si.p());
    let lines = load(si.name);
    for kind in ["lift_honest", "lift_crafted_q5"] {
        let l = lines.iter().find(|l| l.kind == kind).expect("missing fixture");
        let mut t: [u64; C] = to_limbs(&l.t);
        let mut x: [u64; C] = to_limbs(&l.v[0]);
        let mut y: [u64; C] = to_limbs(&l.v[1]);
        for w in [&mut t, &mut x, &mut y] {
            // SAFETY: the helper annotates the initialized backing array for Memcheck.
            unsafe { rlc_secret(w.as_mut_ptr(), std::mem::size_of_val(w)) };
        }
        if control && std::hint::black_box(t[0]) & 2 != 0 {
            eprintln!("positive control");
        }
        std::hint::black_box(ngram::full_norm::<C, N, PL>(&t, &x, &y, &p, l.steps));
        std::hint::black_box(ngram::setup_norm_restoring::<C, N, PL>(&t, &x, &y, &p));
    }
    // general constructor on a pre-reduced input, all coordinates secret
    let l = lines.iter().find(|l| l.cls == "prered-guard256").expect("missing prered");
    let mut t: [u64; C] = to_limbs(&l.t);
    let mut u: Vec4<C> = v4(&l.u);
    let mut v: Vec4<C> = v4(&l.v);
    // SAFETY: as above.
    unsafe {
        rlc_secret(t.as_mut_ptr(), std::mem::size_of_val(&t));
        rlc_secret(u.as_mut_ptr() as *mut u64, std::mem::size_of_val(&u));
        rlc_secret(v.as_mut_ptr() as *mut u64, std::mem::size_of_val(&v));
    }
    let (mut uu, mut vv, mut g, ok) = ngram::setup_norm_general::<C, N, PL>(&t, &u, &v, &p);
    ngram::reduce_norm::<C>(&mut uu, &mut vv, &mut g, l.steps);
    std::hint::black_box((uu, vv, g, ok));
}

fn info(name: &str) -> &'static SetInfo {
    SETS.iter().find(|s| s.name == name).expect("set")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("check") => {
            let names: Vec<&str> =
                if args.len() > 2 { args[2..].iter().map(|s| s.as_str()).collect() } else { SETS.iter().map(|s| s.name).collect() };
            let mut all = true;
            for n in names {
                let si = info(n);
                all &= with_set!(si.name, check_set, si);
            }
            // toy half-tie runs at the P248 widths, each with its own p
            all &= check_file::<11, 22, 4>("toy");
            println!("ALL {}", if all { "OK" } else { "FAILED" });
            if !all {
                std::process::exit(1);
            }
        }
        Some("bench") => {
            pin(std::env::var("RLC_CPU").ok().and_then(|s| s.parse().ok()).unwrap_or(6));
            let si = info(&args[2]);
            let reps: usize = args[3].parse().unwrap();
            if args.get(4).is_some_and(|s| s == "header") {
                println!("set,steps,reps,{}", VARIANTS.join(","));
            }
            with_set!(si.name, bench_set, si, reps);
        }
        #[cfg(feature = "memcheck")]
        Some("memcheck") => {
            let si = info(&args[2]);
            let control = args.get(3).is_some_and(|value| value == "control");
            with_set!(si.name, memcheck, si, control);
        }
        _ => eprintln!("usage: ng check [sets] | bench <set> <reps> [header] | memcheck <set> [control]"),
    }
}
