"""Corpus generator for the five PRISM sets. Output: corpus/<set>.tsv with
`case<TAB>row` where row = `set n xn yn u0..u3 v0..v3 s xq yq` (hex, '-' sign), the
format of the prototype's ideal corpus plus a leading set name.
Everything here is produced from the mathematics (own prime search, own Gaussian
reduction, own ideal construction); no prototype or research-script code is used.
"""
import random, sys, os
from sage.all import ZZ, Mod, previous_prime, next_prime
from oracle import *

SETS = {"p248": (5, 248), "p324": (3, 324), "p376": (65, 376), "p500": (27, 500), "p664": (17, 664)}

def params(name):
    f, e = SETS[name]
    p = f * (1 << e) - 1
    assert ZZ(p).is_prime()
    L = p.bit_length()
    B = (L + 1) // 2 + 32
    return f, e, p, L, B

def hx(v):
    return ("-" if v < 0 else "") + format(abs(v), "x")

def is_prime(n):
    return n > 1 and ZZ(n).is_pseudoprime()

def rand_prime_bits(rng, bits):
    while True:
        n = rng.getrandbits(bits) | (1 << (bits - 1)) | 1
        if is_prime(n):
            return n

def sqrt_mod(a, n):
    m = Mod(a, n)
    if not m.is_square():
        return None
    return int(m.sqrt())

def lift_root(x, y, T):
    x %= T; y %= T
    x += (x % 2) * T
    y += (1 - y % 2) * T
    return x, y

def secret_root(rng, p, N):
    """A root (x,y) mod N of x^2+y^2+p = 0, prime N, parity-lifted into [0,2N)."""
    while True:
        x = rng.randrange(N)
        y = sqrt_mod(-p - x * x, N)
        if y is None:
            continue
        if rng.random() < 0.5:
            y = (-y) % N
        return lift_root(x, y, N)

def composite_root(rng, p, factors):
    """CRT root for a squarefree composite norm."""
    from sage.all import crt
    T = 1
    for f in factors:
        T *= f
    xs, ys = [], []
    for f in factors:
        x, y = secret_root(rng, p, f)
        xs.append(x % f); ys.append(y % f)
    x = int(crt(xs, factors)); y = int(crt(ys, factors))
    return lift_root(x, y, T), T

def challenge(rng, p, e, s=None, want_prime=True):
    """(s, q, root) with q = 2^(2e-5) - s^2 prime and a root of x^2+y^2+p = 0 mod q."""
    while True:
        ss = s if s is not None else (rng.getrandbits(e - 3) | 1)
        q = (1 << (2 * e - 5)) - ss * ss
        if want_prime and not is_prime(q):
            if s is not None:
                return ss, q, None
            continue
        if not want_prime:
            return ss, q, None
        while True:
            x = rng.randrange(q)
            y = sqrt_mod(-p - x * x, q)
            if y is not None:
                if rng.random() < 0.5:
                    y = (-y) % q
                return ss, q, lift_root(x, y, q)

def smax_for(e):
    return ((1 << (e - 3)) - 1) | 1

def guard_ok(p, N, v):
    c = N4(p, v)
    return c * c <= p * (1024 * N) ** 2

def shear(u, v, g):
    ju = J(u)
    return tuple(v[k] + g[0] * u[k] + g[1] * ju[k] for k in range(4))

def reduced_basis(p, N, root):
    u, v, _, _ = lagrange(p, (2 * N, 0, 0, 0), (root[0], root[1], 1, 0))
    return u, v

def row(name, N, rs, u, v, s, rq):
    vals = [N, rs[0], rs[1], *u, *v, s, rq[0], rq[1]]
    return name + " " + " ".join(hx(x) for x in vals)

def build(name, nrand, seed):
    f, e, p, L, B = params(name)
    rng = random.Random(f"{name}-{seed}")
    cases = []
    def add(case, N, rs, u, v, s, rq):
        cases.append((case, row(name, N, rs, u, v, s, rq)))
    ch = lambda **k: challenge(rng, p, e, **k)
    sizes = [B - 1, B - 1, B - 2, L // 2 + 20, L // 2 + 5, L // 2 - 6, L // 2 - 14, 40]
    for i in range(nrand):
        bits = sizes[i % len(sizes)] if i < len(sizes) * 2 else rng.randrange(8, B)
        N = rand_prime_bits(rng, bits)
        rs = secret_root(rng, p, N)
        u, v = reduced_basis(p, N, rs)
        s, q, rq = ch()
        add(f"rand{i}-N{bits}b", N, rs, u, v, s, rq)
    N = rand_prime_bits(rng, B - 1)
    rs = secret_root(rng, p, N)
    u, v = reduced_basis(p, N, rs)
    s, q, rq = ch()
    add("base", N, rs, u, v, s, rq)
    for k in range(4):
        g = (rng.randrange(-6, 7), rng.randrange(-6, 7))
        add(f"shear-small{k}", N, rs, u, shear(u, v, g), s, rq)
    for axis, d in (("re", (1, 0)), ("im", (0, 1)), ("neg-re", (-1, 0))):
        lo, hi = 0, 1
        while guard_ok(p, N, shear(u, v, (hi * d[0], hi * d[1]))):
            lo, hi = hi, hi * 2
        while hi - lo > 1:
            mid = (lo + hi) // 2
            if guard_ok(p, N, shear(u, v, (mid * d[0], mid * d[1]))):
                lo = mid
            else:
                hi = mid
        add(f"guard-last-pass-{axis}", N, rs, u, shear(u, v, (lo * d[0], lo * d[1])), s, rq)
        add(f"guard-first-fail-{axis}", N, rs, u, shear(u, v, (hi * d[0], hi * d[1])), s, rq)
    add("order-swapped", N, rs, v, u, s, rq)
    ju = J(u); jv = J(v); neg = lambda w: tuple(-x for x in w)
    add("unit-neg-u", N, rs, neg(u), v, s, rq)
    add("unit-i-u", N, rs, ju, v, s, rq)
    add("unit-i-v", N, rs, u, jv, s, rq)
    add("unit-neg-both", N, rs, neg(u), neg(v), s, rq)
    rs2 = secret_root(rng, p, N)
    add("wrong-ideal-root", N, rs2, u, v, s, rq)
    add("wrong-norm", N + 2, rs, u, v, s, rq)
    add("root-parity-bad", N, (rs[0] + 1, rs[1]), u, v, s, rq)
    Nmax = int(previous_prime(1 << B))
    rsm = secret_root(rng, p, Nmax)
    um, vm = reduced_basis(p, Nmax, rsm)
    add("N-max-below-2^B", Nmax, rsm, um, vm, s, rq)
    # worst case for output width: largest N, v at the guard boundary, largest q
    ub = []
    for axis, d in (("re", (1, 0)), ("im", (0, 1)), ("diag", (1, 1)), ("anti", (1, -1))):
        lo, hi = 0, 1
        while guard_ok(p, Nmax, shear(um, vm, (hi * d[0], hi * d[1]))):
            lo, hi = hi, hi * 2
        while hi - lo > 1:
            mid = (lo + hi) // 2
            if guard_ok(p, Nmax, shear(um, vm, (mid * d[0], mid * d[1]))):
                lo = mid
            else:
                hi = mid
        ub.append((axis, shear(um, vm, (lo * d[0], lo * d[1]))))
    tq = smax_for(e)
    while True:
        sw, qw, rw = ch(s=tq)
        if rw is not None:
            break
        tq -= 2
    for axis, vv in ub:
        add(f"extreme-Nmax-guard-{axis}-qmax", Nmax, rsm, um, vv, sw, rw)
    Nover = int(next_prime(1 << B))
    rso = secret_root(rng, p, Nover)
    uo, vo = reduced_basis(p, Nover, rso)
    add("N-first-prime-above-2^B", Nover, rso, uo, vo, s, rq)
    for i, fac in enumerate(([rand_prime_bits(rng, B // 2 - 1), rand_prime_bits(rng, B - B // 2 - 2)],
                             [rand_prime_bits(rng, B // 3), rand_prime_bits(rng, B // 3 + 1), rand_prime_bits(rng, B - 2 * (B // 3) - 3)])):
        fac = sorted(set(fac))
        if len(fac) < 2:
            continue
        rc, Nc = composite_root(rng, p, fac)
        uc, vc = reduced_basis(p, Nc, rc)
        add(f"composite-N-{len(fac)}primes", Nc, rc, uc, vc, s, rq)
    for i in range(2):
        while True:
            n1 = rand_prime_bits(rng, B - 2)
            if n1 % 4 == 1:
                break
        r1 = secret_root(rng, p, n1)
        u1, v1 = reduced_basis(p, n1, r1)
        add(f"N-1mod4-{i}", n1, r1, u1, v1, s, rq)
    pr = rand_prime_bits(rng, B // 2 - 1)
    x0, y0 = secret_root(rng, p, pr)
    M2 = pr * pr
    if Mod(-p - x0 * x0, M2).is_square():
        yy = sqrt_mod(-p - x0 * x0, M2)
        rsq = lift_root(x0, yy, M2)
        uq, vq = reduced_basis(p, M2, rsq)
        add("N-prime-square", M2, rsq, uq, vq, s, rq)
    smax = ((1 << (e - 3)) - 1) | 1
    t = smax
    while True:
        s2, q2, r2 = ch(s=t)
        if r2 is not None:
            break
        t -= 2
    add("q-max-s", N, rs, u, v, s2, r2)
    t = 1
    while True:
        s3, q3, r3 = ch(s=t)
        if r3 is not None:
            break
        t += 2
    add("q-min-s", N, rs, u, v, s3, r3)
    t = (1 << (e - 3)) + 1
    while True:
        s4, q4, r4 = ch(s=t)
        if r4 is not None:
            break
        t += 2
    add("s-above-challenge-range", N, rs, u, v, s4, r4)
    t = rng.getrandbits(e - 3) | 1
    while is_prime((1 << (2 * e - 5)) - t * t):
        t = rng.getrandbits(e - 3) | 1
    add("q-composite", N, rs, u, v, t, (1, 1))
    sx, qx, rx = ch()
    add("q-root-wrong", N, rs, u, v, sx, (rx[0] + 2, rx[1]))
    add("q-root-plus-multiples", N, rs, u, v, sx, (rx[0] + 5 * qx, rx[1] - 3 * qx))
    add("q-root-negated", N, rs, u, v, sx, (-rx[0], -rx[1]))
    add("q-root-swap-coords", N, rs, u, v, sx, (rx[1], rx[0]))
    add("q-root-zero", N, rs, u, v, sx, (0, 0))
    add("s-even", N, rs, u, v, sx + 1, rx)
    return cases

if __name__ == "__main__":
    nrand = int(sys.argv[1]) if len(sys.argv) > 1 else 12
    seed = sys.argv[2] if len(sys.argv) > 2 else "1"
    os.makedirs("corpus", exist_ok=True)
    for name in SETS:
        cases = build(name, nrand, seed)
        with open(f"corpus/{name}.tsv", "w") as fh:
            for c, r in cases:
                fh.write(c + "\t" + r + "\n")
        print(name, len(cases), flush=True)
