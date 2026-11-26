"""Oracle-only fixtures (t49u item 1).  Run this BEFORE the Rust harness is built or run.

For each case the inputs (source ideal = norm + root, Gaussian basis, challenge scalar + root) come from the
oracle's own generators.  The expected values are computed from the *ideals alone*:
  T       = N q
  H       = canonical column HNF of I_sec ∩ I_q, where I_sec = O0*N + O0*(x+yi+j) and I_q likewise are built by
            left closure and intersected with the general routes (Sage, PARI dual, kernel, Q3), neither the
            basis (u,v) nor the specialized formula is used.
  verdict = oracle's own admission verdict (reject / challenge-refused / marked / ok).
No prototype code or output is read."""
import random, sys
from common import *

def verdict_of(p, e, B, N, rs, u, v, s, rq):
    ok, why = None, None
    # admission, mathematically (oracle.py)
    adm = None
    import check
    adm, why = check.oracle_admit(p, B, N, rs, u, v)
    if not adm:
        return "reject-admission", why, None
    q, qwhy = check.oracle_challenge(p, e, s, rq)
    if q is None:
        return "challenge-refused", qwhy, None
    if (rq[0] ** 2 + rq[1] ** 2 + p) % q:
        return "marked", "challenge root fails p + x^2 + y^2 = 0 (mod q)", q
    rv = rho(v, rq, q)
    nrm = (rv[0] ** 2 + rv[1] ** 2) % q
    if gcd(nrm, q) != 1:
        return "marked", "rho(v) not a unit mod q", q
    return "ok", "", q

def expected_hnf(p, N, rs, q, rq):
    """Column HNF of I_sec ∩ I_q from the ideals only; checked across the four general routes."""
    G = general_constructions(p, N, rs, q, rq, Hs=hnf_rows(generic_hnf_columns(N, rs)))
    routes = [G[k] for k in ("sage", "pari_dual", "kernel", "q3")]
    assert len(set(routes)) == 1, "general routes disagree"
    M = col_hnf(list(routes[0]))
    assert lattice_of_col_hnf(M) == routes[0]
    # the intersection ideal is generic of norm T: its column HNF is GenericHnf(T, CRT root)
    T = N * q
    rc = crt_root(N, rs, q, rq)
    assert M == [list(r) for r in zip(*generic_hnf_columns(T, rc))], "column HNF != generic shape at CRT root"
    return M

def build(name, seed="fu1"):
    f, e, p, L, B = params(name)
    rng = random.Random(f"{name}-{seed}")
    cases = []
    def add(case, N, rs, u, v, s, rq, note=""):
        cases.append((case, N, rs, u, v, s, rq, note))
    def basis(N, rs):
        return reduced_basis(p, N, rs)
    ch = lambda **k: challenge(rng, p, e, **k)
    def prime_cases():
        for i in range(3):
            N = rand_prime_bits(rng, B - 1); rs = secret_root(rng, p, N); u, v = basis(N, rs); s, q, rq = ch()
            add(f"prime-rand{i}", N, rs, u, v, s, rq)
        while True:
            N = rand_prime_bits(rng, B - 2)
            if N % 4 == 1: break
        rs = secret_root(rng, p, N); u, v = basis(N, rs); s, q, rq = ch()
        add("prime-1mod4", N, rs, u, v, s, rq)
        Nmax = int(previous_prime(1 << B)); rs = secret_root(rng, p, Nmax); u, v = basis(Nmax, rs)
        s, q, rq = ch(); add("prime-largest-below-2^B", Nmax, rs, u, v, s, rq)
    prime_cases()
    # composite odd key norms (the t49u question)
    def comp(case, factors):
        rs, N = composite_root(rng, p, factors)
        u, v = basis(N, rs); s, q, rq = ch()
        add(case, N, rs, u, v, s, rq, "N=" + "*".join(str(x.bit_length()) + "b" for x in factors))
    bp = lambda b: rand_prime_bits(rng, b)
    comp("composite-2-balanced", sorted({bp(B // 2 - 1), bp(B - B // 2 - 2)}))
    comp("composite-2-unbalanced-3x-large", sorted({3, bp(B - 3)}))
    comp("composite-3-small-factors-3x5x7x-large", sorted({3, 5, 7, bp(B - 9)}))
    comp("composite-3-balanced", sorted({bp(B // 3), bp(B // 3 + 1), bp(B - 2 * (B // 3) - 3)}))
    comp("composite-4-primes", sorted({bp(B // 4), bp(B // 4 + 1), bp(B // 4 + 2), bp(B - 3 * (B // 4) - 6)}))
    # prime powers: the generic root must exist modulo l^k
    for k, tag in ((2, "square"), (3, "cube")):
        for attempt in range(200):
            l = bp(max(8, (B - 3) // k))
            M = l ** k
            x0 = rng.randrange(M)
            t = Mod(-p - x0 * x0, M)
            if t.is_square():
                y0 = int(t.sqrt())
                if gcd(y0 * y0 + x0 * x0 + p, M) == M:
                    rs = lift_root(x0, y0, M); u, v = basis(M, rs); s, q, rq = ch()
                    add(f"composite-prime-{tag}", M, rs, u, v, s, rq, f"l^{k}, l {l.bit_length()}b")
                    break
    # squarefree part times a prime square
    for attempt in range(200):
        l = bp((B - 3) // 3); m = bp(B - 3 - 2 * ((B - 3) // 3) - 3)
        M2 = l * l * m
        # root mod l^2 by square-root lift, mod m directly, then CRT
        x0 = rng.randrange(l * l)
        t = Mod(-p - x0 * x0, l * l)
        if not t.is_square(): continue
        yl = int(t.sqrt()); xm, ym = secret_root(rng, p, m)
        from sage.all import crt
        x = int(crt([x0, xm % m], [l * l, m])); y = int(crt([yl, ym % m], [l * l, m]))
        rs = lift_root(x, y, M2); u, v = basis(M2, rs); s, q, rq = ch()
        add("composite-l2-times-prime", M2, rs, u, v, s, rq, "l^2*m"); break
    # challenge edge cases on a prime key norm and on a composite key norm
    N = rand_prime_bits(rng, B - 1); rs = secret_root(rng, p, N); u, v = basis(N, rs)
    t = smax_for(e)
    while True:
        s2, q2, r2 = ch(s=t)
        if r2 is not None: break
        t -= 2
    add("challenge-q-max", N, rs, u, v, s2, r2)
    t = 1
    while True:
        s3, q3, r3 = ch(s=t)
        if r3 is not None: break
        t += 2
    add("challenge-q-min", N, rs, u, v, s3, r3)
    s4, q4, r4 = ch()
    add("challenge-root-plus-multiples-of-q", N, rs, u, v, s4, (r4[0] + 3 * q4, r4[1] - 7 * q4))
    add("challenge-root-negated", N, rs, u, v, s4, (-r4[0], -r4[1]))
    # guard boundary basis (last pass, first fail) on a composite norm
    facs = sorted({bp(B // 2 - 1), bp(B - B // 2 - 2)})
    rc, Nc = composite_root(rng, p, facs); uc, vc = basis(Nc, rc); sc_, qc, rqc = ch()
    lo, hi = 0, 1
    while guard_ok(p, Nc, shear(uc, vc, (hi, 0))): lo, hi = hi, hi * 2
    while hi - lo > 1:
        mid = (lo + hi) // 2
        if guard_ok(p, Nc, shear(uc, vc, (mid, 0))): lo = mid
        else: hi = mid
    add("composite-guard-last-pass", Nc, rc, uc, shear(uc, vc, (lo, 0)), sc_, rqc, "composite N, v on the guard boundary")
    add("composite-guard-first-fail", Nc, rc, uc, shear(uc, vc, (hi, 0)), sc_, rqc)
    add("composite-unit-multiples", Nc, rc, J(uc), tuple(-x for x in vc), sc_, rqc)
    # negatives, with oracle-computed verdicts
    add("neg-challenge-composite-q", N, rs, u, v, rng.getrandbits(e - 3) | 1 if False else 3, (1, 1))
    add("neg-challenge-even-scalar", N, rs, u, v, s4 + 1, r4)
    add("neg-challenge-wrong-root", N, rs, u, v, s4, (r4[0] + 2, r4[1]))
    Nover = int(next_prime(1 << B)); ro = secret_root(rng, p, Nover); uo, vo = basis(Nover, ro)
    add("neg-N-above-2^B", Nover, ro, uo, vo, s4, r4)
    add("neg-order-swapped", N, rs, v, u, s4, r4)
    add("neg-basis-wrong-ideal-root", N, secret_root(rng, p, N), u, v, s4, r4)
    return cases

if __name__ == "__main__":
    os.makedirs("fixtures_oracle", exist_ok=True)
    for name in SETS:
        f, e, p, L, B = params(name)
        lines = []
        counts = {}
        for (case, N, rs, u, v, s, rq, note) in build(name):
            verdict, why, q = verdict_of(p, e, B, N, rs, u, v, s, rq)
            counts[verdict] = counts.get(verdict, 0) + 1
            inp = " ".join(hxs(x) for x in [N, rs[0], rs[1], *u, *v, s, rq[0], rq[1]])
            if verdict == "ok":
                M = expected_hnf(p, N, rs, q, rq)
                T = N * q
                lines.append("\t".join([case, "ok", inp, hxs(T), " ".join(hxs(M[i][j]) for i in range(4) for j in range(4)), note]))
            else:
                lines.append("\t".join([case, verdict, inp, "-", "-", why or note]))
        open(f"fixtures_oracle/{name}.tsv", "w").write("\n".join(lines) + "\n")
        print(name, len(lines), counts, flush=True)
