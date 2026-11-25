// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//
// Cell potential for the main phase of the Z[i]-Lagrange contract.
//
// State: eps = (x, y), the residual of the current step, in the closed square
// [-1/2, 1/2]^2, and lam = Delta/A^2 in [0, 2^-LAM].  If the step swaps,
// r = lam + |eps|^2, the next exact quotient is q = (x, -y)/r, the next
// rounded quotient is mu = floor(q + 1/2) componentwise (the contract), and
// the next residual is eps' = q - mu.  Lemma 1: if the next step also swaps,
// |mu|^2 >= 2.
//
// All interval arithmetic is exact integer arithmetic (i128, overflow checks
// on): cell corners are multiples of 2^-30, r is in units of 2^-60, quotient
// enclosures are exact rationals, and successor boxes are rounded outward to
// multiples of 2^-40.  Only the search (value iteration) uses floating point;
// `check` re-derives every successor set and verifies the certificate
//     rhi(c) * g(c') <= rho * g(c)  for every c' in succ(c)
//     rhi(c) * Gmax  <= rho * g(c)  for wild c
//     g(c) >= 1
// in integers.
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;

const CS: u32 = 30; // cell coordinate scale 2^-30
const RS: u32 = 40; // successor box scale 2^-40
const MAXL: u32 = 28;
const MAXMU: i128 = 49;
const GS: u32 = 32; // g scale 2^-32

#[derive(Clone, Copy)]
struct Node {
    l: u32,
    i: u32,
    j: u32,
    kids: u32, // 0 = leaf, else index of first of 4 consecutive kids
}

struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    fn new(l0: u32) -> Tree {
        let mut t = Tree { nodes: vec![Node { l: 0, i: 0, j: 0, kids: 0 }] };
        let mut frontier = vec![0usize];
        for _ in 0..l0 {
            let mut nf = vec![];
            for &n in &frontier {
                let k = t.split(n);
                for d in 0..4 {
                    nf.push(k + d);
                }
            }
            frontier = nf;
        }
        t
    }
    fn split(&mut self, n: usize) -> usize {
        let nd = self.nodes[n];
        assert!(nd.kids == 0 && nd.l < MAXL);
        let k = self.nodes.len();
        for di in 0..2 {
            for dj in 0..2 {
                self.nodes.push(Node { l: nd.l + 1, i: 2 * nd.i + di, j: 2 * nd.j + dj, kids: 0 });
            }
        }
        self.nodes[n].kids = k as u32;
        k
    }
    fn leaves(&self) -> Vec<usize> {
        (0..self.nodes.len()).filter(|&n| self.nodes[n].kids == 0).collect()
    }
    // closed box in units 2^-s
    fn bx(&self, n: usize, s: u32) -> (i128, i128, i128, i128) {
        let nd = self.nodes[n];
        let w = 1i128 << (s - nd.l);
        let h = 1i128 << (s - 1);
        (nd.i as i128 * w - h, (nd.i as i128 + 1) * w - h, nd.j as i128 * w - h, (nd.j as i128 + 1) * w - h)
    }
    fn query(&self, b: (i128, i128, i128, i128), out: &mut Vec<usize>) {
        let mut st = vec![0usize];
        while let Some(n) = st.pop() {
            let (xl, xh, yl, yh) = self.bx(n, RS);
            if xl > b.1 || xh < b.0 || yl > b.3 || yh < b.2 {
                continue;
            }
            let k = self.nodes[n].kids as usize;
            if k == 0 {
                out.push(n);
            } else {
                for d in 0..4 {
                    st.push(k + d);
                }
            }
        }
    }
}

fn sq_iv(lo: i128, hi: i128) -> (i128, i128) {
    if lo >= 0 {
        (lo * lo, hi * hi)
    } else if hi <= 0 {
        (hi * hi, lo * lo)
    } else {
        (0, std::cmp::max(lo * lo, hi * hi))
    }
}

// rationals n/d, d > 0
fn lt(a: (i128, i128), b: (i128, i128)) -> bool {
    a.0 * b.1 < b.0 * a.1
}
fn fhu(a: (i128, i128)) -> i128 {
    (2 * a.0 + a.1).div_euclid(2 * a.1)
}
fn floor_div(n: i128, d: i128) -> i128 {
    n.div_euclid(d)
}
fn ceil_div(n: i128, d: i128) -> i128 {
    -((-n).div_euclid(d))
}

struct Img {
    rhi: i128, // units 2^-60
    succ: Option<Vec<usize>>, // None = wild
}

fn image(t: &Tree, n: usize, lamu: i128) -> Img {
    let (xl, xh, yl, yh) = t.bx(n, CS);
    let (x2l, x2h) = sq_iv(xl, xh);
    let (y2l, y2h) = sq_iv(yl, yh);
    let rl = x2l + y2l;
    let rh = x2h + y2h + lamu;
    if rl == 0 {
        return Img { rhi: rh, succ: None };
    }
    let one = 1i128 << (2 * CS);
    let rhc = std::cmp::min(rh, one);
    // q0 = x * 2^30 / r, q1 = -y * 2^30 / r: corners of the enclosure
    let corners = |a: i128, b: i128| -> ((i128, i128), (i128, i128)) {
        let c = [(a << CS, rl), (a << CS, rhc), (b << CS, rl), (b << CS, rhc)];
        let mut lo = c[0];
        let mut hi = c[0];
        for &v in &c[1..] {
            if lt(v, lo) {
                lo = v;
            }
            if lt(hi, v) {
                hi = v;
            }
        }
        (lo, hi)
    };
    let (q0l, q0h) = corners(xl, xh);
    let (q1l, q1h) = corners(-yh, -yl);
    let (m0l, m0h, m1l, m1h) = (fhu(q0l), fhu(q0h), fhu(q1l), fhu(q1h));
    if (m0h - m0l + 1) * (m1h - m1l + 1) > MAXMU {
        return Img { rhi: rh, succ: None };
    }
    let half = 1i128 << (RS - 1);
    let mut out = vec![];
    for m0 in m0l..=m0h {
        for m1 in m1l..=m1h {
            if m0 * m0 + m1 * m1 <= 1 {
                continue;
            }
            let a = std::cmp::max(-half, floor_div((q0l.0 - m0 * q0l.1) << RS, q0l.1));
            let b = std::cmp::min(half, ceil_div((q0h.0 - m0 * q0h.1) << RS, q0h.1));
            let c = std::cmp::max(-half, floor_div((q1l.0 - m1 * q1l.1) << RS, q1l.1));
            let d = std::cmp::min(half, ceil_div((q1h.0 - m1 * q1h.1) << RS, q1h.1));
            if a > b || c > d {
                continue;
            }
            t.query((a, b, c, d), &mut out);
        }
    }
    out.sort_unstable();
    out.dedup();
    Img { rhi: rh, succ: Some(out) }
}

struct Built {
    leaves: Vec<usize>,
    pos: HashMap<usize, usize>,
    imgs: Vec<Img>,
}

fn build(t: &Tree, lamu: i128) -> Built {
    let leaves = t.leaves();
    let pos: HashMap<usize, usize> = leaves.iter().enumerate().map(|(a, &n)| (n, a)).collect();
    let nth = 8;
    let chunk = (leaves.len() + nth - 1) / nth;
    let mut imgs: Vec<Img> = Vec::with_capacity(leaves.len());
    std::thread::scope(|s| {
        let hs: Vec<_> = leaves
            .chunks(chunk.max(1))
            .map(|ch| s.spawn(move || ch.iter().map(|&n| image(t, n, lamu)).collect::<Vec<Img>>()))
            .collect();
        for h in hs {
            imgs.extend(h.join().unwrap());
        }
    });
    Built { leaves, pos, imgs }
}

// value iteration in log space; returns (v, converged, policy)
fn solve(b: &Built, rho: f64, iters: usize) -> (Vec<f64>, bool, Vec<i64>) {
    let n = b.leaves.len();
    let lr: Vec<f64> = b.imgs.iter().map(|im| ((im.rhi as f64) / ((1u128 << 60) as f64) / rho).ln()).collect();
    let succ: Vec<Option<Vec<usize>>> =
        b.imgs.iter().map(|im| im.succ.as_ref().map(|s| s.iter().map(|x| b.pos[x]).collect())).collect();
    let mut v = vec![0f64; n];
    let mut pol = vec![-1i64; n];
    let mut hist: Vec<f64> = vec![];
    for it in 0..iters {
        let gm = v.iter().cloned().fold(0f64, f64::max);
        hist.push(gm);
        if it >= 1500 && gm > 1.0 && gm - hist[it - 1000] > 1e-3 {
            return (v, false, pol);
        }
        let mut nv = vec![0f64; n];
        let mut diff = 0f64;
        for a in 0..n {
            let (best, arg) = match &succ[a] {
                None => (gm, -2i64),
                Some(s) if s.is_empty() => (f64::NEG_INFINITY, -1),
                Some(s) => {
                    let mut bb = f64::NEG_INFINITY;
                    let mut ar = -1i64;
                    for &c in s {
                        if v[c] > bb {
                            bb = v[c];
                            ar = c as i64;
                        }
                    }
                    (bb, ar)
                }
            };
            let x = (lr[a] + best).max(0.0);
            pol[a] = if lr[a] + best > 0.0 { arg } else { -1 };
            nv[a] = x;
            diff = diff.max((x - v[a]).abs());
        }
        v = nv;
        if diff < 1e-14 {
            return (v, true, pol);
        }
        if gm > 60.0 {
            return (v, false, pol);
        }
    }
    (v, false, pol)
}

fn parse_rho(s: &str) -> (i128, i128) {
    let p: Vec<&str> = s.split('/').collect();
    (p[0].parse().unwrap(), p[1].parse().unwrap())
}

fn main() {
    let a: Vec<String> = env::args().collect();
    let mode = a[1].as_str();
    let (rn, rd) = parse_rho(&a[2]);
    let lam: u32 = a[3].parse().unwrap();
    let lamu = 1i128 << (2 * CS - lam);
    let rho = rn as f64 / rd as f64;
    if mode == "search" {
        let l0: u32 = a[4].parse().unwrap();
        let maxc: usize = a[5].parse().unwrap();
        let outp = &a[6];
        let frac: usize = if a.len() > 7 { a[7].parse().unwrap() } else { 400 };
        let mut t = Tree::new(l0);
        let t0 = std::time::Instant::now();
        let mut round = 0;
        loop {
            round += 1;
            let b = build(&t, lamu);
            let (v, ok, pol) = solve(&b, rho * (1.0 - 1e-7), 20000);
            let gm = v.iter().cloned().fold(0f64, f64::max);
            let nw = b.imgs.iter().filter(|im| im.succ.is_none()).count();
            eprintln!("round {round}: {} cells ({nw} wild), ok={ok}, logG={gm:.5}, {:.1}s", b.leaves.len(), t0.elapsed().as_secs_f64());
            if ok {
                write_cert(&t, &b, &v, &a[2], lam, outp);
                eprintln!("wrote {outp}");
                return;
            }
            if b.leaves.len() > maxc {
                eprintln!("gave up");
                std::process::exit(2);
            }
            // hot cells: the policy cycle reached from each of the top cells, plus the top cells
            let n = b.leaves.len();
            let mut idx: Vec<usize> = (0..n).collect();
            idx.sort_by(|&x, &y| v[y].partial_cmp(&v[x]).unwrap());
            let mut hot = std::collections::HashSet::new();
            let top = std::cmp::max(1, n / frac);
            for &s in idx.iter().take(top) {
                hot.insert(s);
                let mut c = s as i64;
                let mut seen = std::collections::HashSet::new();
                while c >= 0 && seen.insert(c) {
                    hot.insert(c as usize);
                    c = pol[c as usize];
                }
            }
            let mut nsplit = 0;
            for h in hot {
                let node = b.leaves[h];
                if t.nodes[node].l < MAXL {
                    t.split(node);
                    nsplit += 1;
                }
            }
            if nsplit == 0 {
                eprintln!("max depth");
                std::process::exit(3);
            }
        }
    } else if mode == "check" {
        let inp = &a[4];
        check(inp, rn, rd, lamu);
    } else if mode == "ratio" {
        // smallest rho' = k/10^6 for which the certificate's g passes at this LAM
        // (rho argument ignored); prints it, then `check` must be run with it.
        let inp = &a[4];
        check_ratio(inp, lamu);
    }
}

fn write_cert(t: &Tree, b: &Built, v: &[f64], rho_s: &str, lam: u32, path: &str) {
    let mut f = fs::File::create(path).unwrap();
    writeln!(f, "# cellcert certificate: rho {rho_s}, LAM 2^-{lam}; leaf: level i j g_num (g = g_num / 2^{GS})").unwrap();
    writeln!(f, "rho {rho_s}").unwrap();
    writeln!(f, "lam {lam}").unwrap();
    for (a, &n) in b.leaves.iter().enumerate() {
        let nd = t.nodes[n];
        let g = (v[a].exp() * (1.0 + 1e-12) * (1u64 << GS) as f64).ceil() as u64;
        writeln!(f, "{} {} {} {}", nd.l, nd.i, nd.j, g.max(1u64 << GS)).unwrap();
    }
}

fn check_ratio(path: &str, lamu: i128) {
    let (t, gval) = load(path);
    let gmax = *gval.values().max().unwrap() as i128;
    let one60 = 1i128 << 60;
    let b = build(&t, lamu);
    let mut kmax: i128 = 0;
    for (a, &n) in b.leaves.iter().enumerate() {
        let gc = gval[&n] as i128;
        let im = &b.imgs[a];
        let gs = match &im.succ {
            None => gmax,
            Some(sv) => sv.iter().map(|x| gval[x] as i128).max().unwrap_or(0),
        };
        kmax = kmax.max(ceil_div(im.rhi * gs * 1_000_000, gc * one60));
    }
    println!("{}/1000000", kmax);
}

fn load(path: &str) -> (Tree, HashMap<usize, u64>) {
    let s = fs::read_to_string(path).unwrap();
    let mut cells: Vec<(u32, u32, u32, u64)> = vec![];
    for line in s.lines() {
        if line.starts_with('#') || line.starts_with("rho") || line.starts_with("lam") {
            continue;
        }
        let p: Vec<u64> = line.split_whitespace().map(|x| x.parse().unwrap()).collect();
        cells.push((p[0] as u32, p[1] as u32, p[2] as u32, p[3]));
    }
    // rebuild the tree from the leaf set
    let mut t = Tree { nodes: vec![Node { l: 0, i: 0, j: 0, kids: 0 }] };
    let mut gval: HashMap<usize, u64> = HashMap::new();
    for &(l, i, j, g) in &cells {
        let mut n = 0usize;
        for lv in 0..l {
            if t.nodes[n].kids == 0 {
                t.split(n);
            }
            let sh = l - lv - 1;
            let di = (i >> sh) & 1;
            let dj = (j >> sh) & 1;
            n = t.nodes[n].kids as usize + (2 * di + dj) as usize;
        }
        assert!(t.nodes[n].kids == 0);
        gval.insert(n, g);
    }
    assert_eq!(gval.len(), cells.len(), "duplicate cell");
    (t, gval)
}

fn check(path: &str, rn: i128, rd: i128, lamu: i128) {
    let (t, gval) = load(path);
    let cells = gval.len();
    let leaves = t.leaves();
    assert_eq!(leaves.len(), cells, "leaf set is not a partition");
    // area check: the leaves tile the square
    let mut area: u128 = 0;
    for &n in &leaves {
        area += 1u128 << (2 * (MAXL - t.nodes[n].l));
    }
    assert_eq!(area, 1u128 << (2 * MAXL), "leaves do not tile the square");
    let gmax = *gval.values().max().unwrap() as i128;
    let one60 = 1i128 << 60;
    let b = build(&t, lamu);
    let mut worst = 0f64;
    let mut nwild = 0;
    for (a, &n) in b.leaves.iter().enumerate() {
        let gc = gval[&n] as i128;
        assert!(gc >= 1i128 << GS);
        let im = &b.imgs[a];
        let gs = match &im.succ {
            None => {
                nwild += 1;
                gmax
            }
            Some(sv) => sv.iter().map(|x| gval[x] as i128).max().unwrap_or(0),
        };
        // rhi/2^60 * gs <= rn/rd * gc
        let lhs = im.rhi * gs * rd;
        let rhs = rn * gc * one60;
        assert!(lhs <= rhs, "certificate fails at cell {:?}", t.nodes[n].l);
        worst = worst.max(lhs as f64 / rhs as f64);
    }
    println!("OK: {} cells ({nwild} wild), rho {}/{}, lam_max 2^-{}, Gmax = {} / 2^{GS} = {:.6}, tightest ratio {:.9}",
        leaves.len(), rn, rd, (2 * CS) as i128 - lamu.trailing_zeros() as i128, gmax, gmax as f64 / (1u64 << GS) as f64, worst);
}
