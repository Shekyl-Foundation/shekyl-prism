"""Exhaustive toy-prime check: every left O0-ideal of norm N <= NMAX (as O0*N + O0*alpha, alpha over O0/N O0),
for toy primes p = 3 mod 4, including N even, composite, p | N.  Per ideal: several Z[i]-bases (Gaussian-Euclid,
reduced, random unimodular twists, unit twists), all 4 orientations of beta = eps*u, beta = v, and the
j-closure identities (S, Gram = D * Omega * S^T)."""
import sys, json, random, itertools
from check_lib import *
from sq import j_act, coords_in_basis, jidentities

def ideals_for(p, N):
    seen = {}
    for c in itertools.product(range(N), repeat=4):
        alpha = tuple(sum(ci * Oi[k] for ci, Oi in zip(c, O0)) for k in range(4))
        # O0 element numerators: sum c_i O0_i (numerator coordinates of basis element i)
        gens = [(2 * N, 0, 0, 0), alpha]
        H = ideal_from_gens(p, gens)
        if lat_index(H) % 4: continue
        k = lat_index(H) // 4
        n = isqrt(k)
        if n * n != k: continue
        seen[H] = n
    return seen

if __name__ == "__main__":
    p = int(sys.argv[1]); NMAX = int(sys.argv[2])
    rng = random.Random(p * 1000 + NMAX)
    tot = {}; fails = []; n_ideals = 0; jstats = [0, 0]; byN = {}
    for N0 in range(1, NMAX + 1):
        ids = ideals_for(p, N0)
        for H, n in ids.items():
            assert is_left_O0_ideal(p, list(H)) and ideal_norm(H) == n
            n_ideals += 1; byN[int(n)] = byN.get(int(n), 0) + 1
            u, v = gaussian_basis_of(H)
            bases = [(u, v)]
            ru, rv, _, _ = lagrange(p, u, v); bases.append((ru, rv))
            for _ in range(4):
                bases.append(unimodular(rng, p, u, v, kmax=2))
                bases.append(unimodular(rng, p, ru, rv, kmax=2))
            for (bu, bv) in bases:
                if bu == (0,0,0,0) or bv == (0,0,0,0): continue
                if gaussian_lattice([bu, bv]) != H: fails.append(f"p={p} basis does not span H"); continue
                rec, f = check_basis(p, n, bu, bv, f"p={p} N={n}")
                add(tot, rec); fails += f
                ok, why = jidentities(p, n, bu, bv)
                jstats[0 if ok else 1] += 1
                if not ok: fails.append(f"p={p} N={n} j-identities: {why}")
    print(json.dumps(dict(p=p, NMAX=NMAX, ideals=n_ideals, byN=byN, tot=tot, j_ok=jstats[0], j_bad=jstats[1], fails=fails[:20], nfails=len(fails))))
