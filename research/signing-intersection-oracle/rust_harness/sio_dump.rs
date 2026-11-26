// Local extraction harness for the signing-intersection oracle. Not committed to dev.
// Reads: set n xn yn u0..u3 v0..v3 scalar xq yq   (hex, '-' sign, little-endian-free big hex)
// Prints: set status ... where status is admit=0 | admit=1 inter=none | inter=ok/inter=marked
use prism_field::{P248, P324, P376, P500, P664};
use prism_int::SecretInt;
use shekyl_prism::hazmat::ideal::{self, IdealSet};
use std::io::BufRead;

type Sc<const W: usize> = SecretInt<W>;

fn mag(hex: &str) -> Vec<u8> {
    hex.trim_start_matches('-')
        .as_bytes()
        .rchunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}
fn sc<const W: usize>(hex: &str) -> Sc<W> {
    let x = Sc::from_le_bytes(&mag(hex));
    if hex.starts_with('-') { x.neg() } else { x }
}
fn show<const W: usize>(x: &Sc<W>) -> String {
    match x.declassify() {
        Ok(v) => v.public_hex().replace("0x", ""),
        Err(_) => "MARKED".to_string(),
    }
}

fn run<S: IdealSet<W, D> + prism_field::ParameterSet, const W: usize, const D: usize>(f: &[&str]) -> String {
    let n: Sc<W> = sc(f[1]);
    let root = [sc::<W>(f[2]), sc::<W>(f[3])];
    let u: [Sc<W>; 4] = std::array::from_fn(|i| sc(f[4 + i]));
    let v: [Sc<W>; 4] = std::array::from_fn(|i| sc(f[8 + i]));
    let mut ch = mag(f[12]);
    ch.resize((S::CHALLENGE_BITS as usize).div_ceil(8), 0);
    let qroot = [sc::<W>(f[13]), sc::<W>(f[14])];
    let hnf = ideal::GenericHnf::from_norm_and_root(&n, &root);
    let g = match ideal::GuardedIdeal::<S, W, D>::admit(n, hnf.columns(), u, v) {
        None => return "admit=0".to_string(),
        Some(g) => g,
    };
    match g.intersection(&ch, &qroot) {
        None => "admit=1 inter=none".to_string(),
        Some(i) => {
            let vals: Vec<String> = i.u.iter().chain(i.v.iter()).chain([&i.norm]).map(show).collect();
            let marked = vals.iter().any(|s| s == "MARKED");
            format!("admit=1 inter={} {}", if marked { "marked" } else { "ok" }, vals.join(" "))
        }
    }
}

fn main() {
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.is_empty() { continue; }
        let out = match f[0] {
            "p248" => run::<P248, 11, 22>(&f[0..]),
            "p324" => run::<P324, 14, 28>(&f[0..]),
            "p376" => run::<P376, 16, 32>(&f[0..]),
            "p500" => run::<P500, 21, 42>(&f[0..]),
            "p664" => run::<P664, 27, 54>(&f[0..]),
            s => panic!("set {s}"),
        };
        println!("{} {}", f[0], out);
    }
}
