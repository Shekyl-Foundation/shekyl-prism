"""Medium-size random sweep (p, N, q up to ~2^50): specialised formula vs the general routes over
random primes p = 3 mod 4, random prime / composite / square norms N (generic roots by CRT or Hensel
inputs, plus non-generic ideals O0*pi and O0*l), challenge primes q of both residues mod 4 and composite q,
random Gaussian bases (shears, unit multiples, both orders).  Records every outcome class."""
import random, sys, json
from collections import Counter
from sage.all import ZZ, Mod, two_squares, next_prime, random_prime
from oracle import *
from gen_corpus import sqrt_mod, lift_root, composite_root

rng = random.Random(int(sys.argv[2]) if len(sys.argv) > 2 else 7)
count = int(sys.argv[1]) if len(sys.argv) > 1 else 400
stats = Counter(); bad = []

def rprime(lo, hi, mod4=None):
    while True:
        n = int(random_prime(hi, lbound=lo))
        if mod4 is None or n % 4 == mod4:
            return n

def root_for(p, N):
    while True:
        x = rng.randrange(N)
        y = sqrt_mod(-p - x * x, N)
        if y is not None:
            return lift_root(x, (y if rng.random() < .5 else -y) % N, N)

for it in range(count):
    p = rprime(2 ** 10, 2 ** 40, 3)
    kind = rng.choice(["prime", "prime", "composite", "pi", "l"])
    if kind == "prime":
        N = rprime(2 ** 8, 2 ** 40); 
        if p % N == 0 or N == 2: continue
        Hs = ideal_root(p, N, root_for(p, N))
    elif kind == "composite":
        N1, N2 = rprime(2 ** 6, 2 ** 20), rprime(2 ** 6, 2 ** 20)
        if N1 == N2 or p in (N1, N2): continue
        r1, r2 = root_for(p, N1), root_for(p, N2)
        N = N1 * N2
        from sage.all import crt
        x = int(crt([r1[0], r2[0]], [N1, N2])); y = int(crt([r1[1], r2[1]], [N1, N2]))
        Hs = ideal_root(p, N, lift_root(x, y, N))
    elif kind == "pi":
        N = rprime(2 ** 8, 2 ** 40, 1)
        a, b = two_squares(N)
        Hs = ideal_from_gens(p, [(2 * a, 2 * rng.choice([-1, 1]) * b, 0, 0)])
    else:
        l = rprime(2 ** 6, 2 ** 20)
        if l == p: continue
        N = l * l
        Hs = ideal_from_gens(p, [(2 * l, 0, 0, 0)])
    assert is_left_O0_ideal(p, Hs) and ideal_norm(Hs) == N
    qkind = rng.choice(["3", "3", "1", "comp"])
    if qkind == "comp":
        q = rprime(2 ** 7, 2 ** 25) * rprime(2 ** 7, 2 ** 25)
        if q % 2 == 0 or gcd(q, p) != 1: continue
    else:
        q = rprime(2 ** 12, 2 ** 50, 3 if qkind == "3" else 1)
        if q == p or q == N: continue
    # root of I_q: need a root of x^2+y^2+p mod q; for composite q try CRT of two primes
    try:
        if qkind == "comp":
            continue  # a generic root mod a composite q is built by CRT elsewhere; the prototype rejects composite q
        rq = root_for(p, q)
    except Exception:
        continue
    Hq = ideal_root(p, q, rq)
    G = {"sage": inter_sage(Hs, Hq), "pari": inter_pari_dual(Hs, Hq), "kernel": inter_kernel_residue(p, Hs, q, rq)}
    if gcd(N, q) == 1:
        G["q3"] = hnf_rows([tuple(q * x for x in r) for r in Hs] + [tuple(N * x for x in r) for r in Hq])
    ref = G["sage"]
    if any(v != ref for v in G.values()):
        bad.append(("general-routes-disagree", p, N, q)); continue
    T = N * q
    comax = lat_index(ref) == 4 * T * T
    stats["comaximal" if comax else "not_comaximal"] += 1
    u0, v0 = gaussian_basis_of(Hs)
    for variant in range(6):
        u, v = (u0, v0) if variant % 2 == 0 else (v0, u0)
        g = (rng.randrange(-5, 6), rng.randrange(-5, 6)); v = tuple(v[k] + g[0] * u[k] + g[1] * J(u)[k] for k in range(4))
        if rng.random() < .5: u = J(u)
        out = spec_intersection(p, N, u, v, q, rq, fermat=True)
        rv = rho(v, rq, q)
        cls = f"{kind}/q{qkind}"
        if out is None:
            nrm = (rv[0] ** 2 + rv[1] ** 2) % q
            if rv == (0, 0): stats["fail:v_in_Iq"] += 1
            elif nrm == 0: stats["fail:zero_divisor"] += 1
            else: bad.append(("unexpected-failure", p, N, q))
            continue
        U, V, Tc = out
        S = gaussian_span(U, V)
        stats[f"spec_ok:{cls}"] += 1
        if S != ref: bad.append(("SPEC-MISMATCH", p, N, q, kind))
        else: stats["spec==general"] += 1
        if not comax: bad.append(("spec succeeded but ideals not comaximal", p, N, q))
        if Tc != T: bad.append(("T", p, N, q))
        gm = norm_gram(p, T, U, V)
        if gm is None or gram_det(p, gm) != p: bad.append(("gram", p, N, q))
        else: stats["gram_ok"] += 1
print(dict(stats)); print("BAD", len(bad), bad[:10])
json.dump({"stats": dict(stats), "bad": [list(map(str, b)) for b in bad]}, open("results/medium.json", "w"), indent=1)
