"""Exhaustive toy-prime sweep: every left O0-ideal of norm N (prime N by enumeration of
O0*N + O0*alpha over all alpha mod N) against every generic left ideal of norm q.
Compares, lattice for lattice, the specialised construction against four independent
general routes (Sage submodule intersection, PARI dual-lattice route, linear-algebra
kernel of the residue map, and the coprime formula q I_s + N I_q).
"""
import itertools, json, random, sys
from collections import Counter
from oracle import *

def all_ideals(p, N):
    """All left O0-ideals of prime norm N (N odd prime not dividing p), as HNF lattices,
    by exhaustive alpha mod N.  Records, for the generic ones, a root (x,y)."""
    found = {}
    target = N * N * lat_index(L0)
    for coords in itertools.product(range(N), repeat=4):
        if not any(coords):
            continue
        # alpha = c0*e0 + c1*e1 + c2*e2 + c3*e3 in the O0 basis, as a numerator
        alpha = tuple(sum(c * e[k] for c, e in zip(coords, O0)) for k in range(4))
        # norm condition: N | nrd(alpha) = N4/4
        if (N4(p, alpha) // 4) % N:
            continue
        H = ideal_from_gens(p, [(2 * N, 0, 0, 0), alpha])
        if lat_index(H) == target:
            found.setdefault(H, alpha)
    return found

def root_of(H, N, p):
    """Generic root of an ideal lattice H of norm N: the (x,y) with (x,y,1,0) in H, x even,
    y odd, in [0,2N). None if the ideal is not generic."""
    for x in range(0, 2 * N, 2):
        for y in range(1, 2 * N, 2):
            if lat_contains(H, (x, y, 1, 0)):
                return (x, y)
    return None

def run(p, N, q, verbose=False, cap=None):
    Is = all_ideals(p, N)
    Iq = all_ideals(p, q) if q != N else Is
    stats = Counter()
    bad = []
    pairs = [(Hs, Hq) for Hs in Is for Hq in Iq]
    if cap and len(pairs) > cap:
        random.shuffle(pairs); pairs = pairs[:cap]
    for Hs, Hq in pairs:
        rq = root_of(Hq, q, p)
        rs = root_of(Hs, N, p)
        stats["generic_Is" if rs else "nongeneric_Is"] += 1
        stats["pairs"] += 1
        G = {"sage": inter_sage(Hs, Hq), "pari": inter_pari_dual(Hs, Hq)}
        if rq is not None:
            G["kernel"] = inter_kernel_residue(p, Hs, q, rq)
        if gcd(N, q) == 1:
            G["q3"] = hnf_rows([tuple(q * x for x in r) for r in Hs] + [tuple(N * x for x in r) for r in Hq])
        ref = G["sage"]
        for k, val in G.items():
            if val != ref:
                bad.append(("general-mismatch", k, p, N, q))
        T = N * q
        comax = (lat_index(ref) == (T * T) * lat_index(L0))
        stats["comaximal" if comax else "not_comaximal"] += 1
        if rq is None:
            stats["nongeneric_Iq"] += 1
            continue
        if not is_left_O0_ideal(p, ref):
            bad.append(("not-ideal", p, N, q))
        u0, v0 = gaussian_basis_of(Hs)
        assert gaussian_span(u0, v0) == Hs
        # variants: both orders, unit multiples, Gaussian shears of v by multiples of u
        variants = []
        for (u, v) in ((u0, v0), (v0, u0)):
            for a in range(-1, 2):
                for b in range(-1, 2):
                    if (a, b) == (0, 0):
                        continue
                    ju = J(u)
                    vs = tuple(v[k] + a * u[k] + b * ju[k] for k in range(4))
                    variants.append((u, vs))
            variants.append((u, v))
            variants.append((J(u), J(J(v))))  # unit multiples
        for (u, v) in variants:
            assert gaussian_span(u, v) == Hs
            out = spec_intersection(p, N, u, v, q, rq, fermat=False)
            rv = rho(v, rq, q)
            if out is None:
                # classify failure
                nrm = (rv[0] ** 2 + rv[1] ** 2) % q
                if rv == (0, 0):
                    stats["fail_v_in_Iq"] += 1
                elif nrm == 0:
                    stats["fail_zero_divisor(q=1 mod 4)"] += 1
                else:
                    bad.append(("unexpected-failure", p, N, q))
                continue
            U, V, Tc = out
            S = gaussian_span(U, V)
            stats["spec_ok"] += 1
            if S != ref:
                bad.append(("SPEC-LATTICE-MISMATCH", p, N, q, comax))
            else:
                stats["spec_equals_general"] += 1
            if not comax:
                stats["spec_ok_but_not_comaximal"] += 1
            # carried norm
            if ideal_norm(S) != Tc and comax:
                bad.append(("norm-mismatch", p, N, q))
            if ideal_norm(S) != Tc:
                stats["norm_differs"] += 1
            # Gram of the output
            g = norm_gram(p, ideal_norm(S), U, V)
            if g is None:
                bad.append(("gram-inexact", p, N, q))
            elif gram_det(p, g) != p:
                bad.append(("gram-det", p, N, q))
            else:
                stats["gram_ok"] += 1
    return stats, bad

if __name__ == "__main__":
    total = Counter(); allbad = []
    cases = [(3, 5, 7), (3, 5, 11), (3, 7, 11), (3, 5, 19), (7, 5, 3), (7, 11, 3), (11, 5, 7), (3, 5, 13), (3, 13, 7),
             (3, 5, 5), (3, 5, 3), (7, 5, 11), (19, 7, 11), (23, 5, 3), (3, 11, 19), (7, 13, 11)]
    for (p, N, q) in cases:
        st, bad = run(p, N, q, cap=int(sys.argv[1]) if len(sys.argv) > 1 else None)
        print(p, N, q, dict(st), "BAD" if bad else "ok", flush=True)
        total.update(st); allbad += bad
    print("TOTAL", dict(total))
    print("BAD", allbad[:20], len(allbad))
