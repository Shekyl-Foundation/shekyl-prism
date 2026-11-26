"""Composite odd key norms at the real sets (t49u item 2).  usage: composite_sweep.py SET COUNT SEED [RUST_BIN]
Builds COUNT admitted-by-construction keys with composite N (several shapes, N close to 2^B), a random challenge
(prime q, root), runs the oracle checks of check.py and the prototype, and records per row the factorization,
the guard margin and the outcome."""
import random, sys, json
from sage.all import factor, RR
from common import *
import check

name, count, seed = sys.argv[1], int(sys.argv[2]), sys.argv[3]
BIN = sys.argv[4] if len(sys.argv) > 4 else "/workspace/wt-dev2/target/release/examples/sio_dump"
f, e, p, L, B = params(name)
rng = random.Random(f"comp-{name}-{seed}")
from sage.all import crt

def prime_power_root(l, k):
    M = l ** k
    for _ in range(500):
        x = rng.randrange(M)
        t = Mod(-p - x * x, M)
        if t.is_square():
            y = int(t.sqrt())
            if (x * x + y * y + p) % M == 0:
                return x, y
    return None

def build_norm(shape):
    """returns (list of (prime, exponent), N) with odd prime factors, and a root by CRT"""
    if shape == "semiprime":
        a = rng.randrange(2, B - 2)
        fs = [(rand_prime_bits(rng, max(2, a)) if a > 2 else 3, 1), (rand_prime_bits(rng, B - 1 - a), 1)]
    elif shape == "three":
        a, b = rng.randrange(3, B // 3), rng.randrange(3, B // 3)
        fs = [(rand_prime_bits(rng, a), 1), (rand_prime_bits(rng, b), 1), (rand_prime_bits(rng, B - 1 - a - b), 1)]
    elif shape == "many":
        fs = [(rand_prime_bits(rng, rng.randrange(4, 12)), 1) for _ in range(rng.randrange(4, 8))]
        rest = B - 1 - sum(l.bit_length() for l, _ in fs)
        fs.append((rand_prime_bits(rng, max(rest, 3)), 1))
    elif shape == "small-times-large":
        fs = [(rng.choice([3, 5, 7, 11, 13, 17, 19, 23]), 1)]
        fs.append((rand_prime_bits(rng, B - 1 - fs[0][0].bit_length()), 1))
    elif shape == "square":
        l = rand_prime_bits(rng, (B - 1) // 2); fs = [(l, 2)]
    elif shape == "cube":
        l = rand_prime_bits(rng, (B - 1) // 3); fs = [(l, 3)]
    elif shape == "square-times-prime":
        a = rng.randrange(8, (B - 1) // 2 - 4)
        l = rand_prime_bits(rng, a); m = rand_prime_bits(rng, B - 1 - 2 * a)
        fs = [(l, 2), (m, 1)]
    elif shape == "small-power":
        l = rng.choice([3, 5, 7]); k = int((B - 1) / RR(l).log2())
        fs = [(l, k)]
    else:
        raise ValueError(shape)
    fs = sorted(set(fs))
    if len({l for l, _ in fs}) != len(fs):
        return None
    mods, xs, ys = [], [], []
    for l, k in fs:
        if l == 2 or p % l == 0:
            return None
        r = prime_power_root(l, k)
        if r is None:
            return None
        mods.append(l ** k); xs.append(r[0]); ys.append(r[1])
    N = 1
    for m in mods: N *= m
    if N % 2 == 0 or N >= (1 << B) or N.bit_length() < B - 12:
        return None
    x = int(crt(xs, mods)) if len(mods) > 1 else xs[0]
    y = int(crt(ys, mods)) if len(mods) > 1 else ys[0]
    return fs, N, lift_root(x, y, N)

shapes = ["semiprime", "semiprime", "three", "many", "small-times-large", "square", "cube", "square-times-prime", "small-power"]
rows, meta = [], []
tries = 0
while len(rows) < count and tries < 20 * count:
    tries += 1
    shape = shapes[len(rows) % len(shapes)] if tries < 5 * count else rng.choice(shapes)
    b = build_norm(shape)
    if b is None:
        continue
    fs, N, rs = b
    u, v = reduced_basis(p, N, rs)
    if rng.random() < 0.5:
        g = (rng.randrange(-8, 9), rng.randrange(-8, 9))
        while g != (0, 0) and not guard_ok(p, N, shear(u, v, g)):
            g = (g[0] // 2, g[1] // 2)
        v = shear(u, v, g)
    s, q, rq = challenge(rng, p, e)
    C = N4(p, v)
    margin = RR(C * C) / RR(p * (1024 * N) ** 2)   # < 1 passes the guard
    rows.append((f"{shape}-{len(rows)}", row(name, N, rs, u, v, s, rq)))
    meta.append({"shape": shape, "factors": [[str(l), k] for l, k in fs], "N_bits": N.bit_length(), "guard_ratio_sq": float(margin),
                 "composite": True})
path = f"/tmp/comp_{name}_{seed}.tsv"
open(path, "w").write("\n".join(c + "\t" + r for c, r in rows) + "\n")
rep, mism = check.check_set(name, BIN, path)
out = []
for m, r in zip(meta, rep):
    m.update({"case": r["case"], "oracle_admit": r["oracle_admit"], "rust_admit": r["rust_admit"], "inter": r.get("rust_inter"),
              "agree": r["ok"], "all_routes": bool(r.get("routes")) and all(r["routes"].values())})
    out.append(m)
json.dump(out, open(f"results_composite_{name}.json", "w"), indent=1)
adm = [m for m in out if m["rust_admit"]]
print(name, "rows", len(out), "admitted", len(adm), "ok-intersections", sum(1 for m in adm if m["inter"] == "ok"),
      "all agree", all(m["agree"] for m in out), "shapes admitted", sorted({m["shape"] for m in adm}))
