"""Comaximal versus coprime (t51u).  Toy p, exhaustive: key norms N in {5, 7, 35, 25?}, challenge degree q in {5, 7}
where gcd(N, q) != 1 is allowed.  Whenever rho(v) is a unit mod q the specialized output must equal I_sec ∩ I_q and
have norm N q (needs only I_sec + I_q = O0).  The coprime formula q I_sec + N I_q and the CRT root at modulus N q
need gcd(N, q) = 1; here they fail."""
import itertools, sys, json
from collections import Counter
from common import *
from toy import all_ideals, root_of

p = int(sys.argv[1]) if len(sys.argv) > 1 else 3
I5, I7 = all_ideals(p, 5), all_ideals(p, 7)
ideals = {5: list(I5), 7: list(I7)}
ideals[35] = sorted({inter_sage(a, b) for a in I5 for b in I7})
stats = Counter(); detail = []
for N in (5, 7, 35):
    for q in (5, 7):
        Iq_all = all_ideals(p, q)
        for Hs in ideals[N]:
            u0, v0 = gaussian_basis_of(Hs)
            assert gaussian_span(u0, v0) == Hs
            for Hq in Iq_all:
                rq = root_of(Hq, q, p)
                if rq is None:
                    continue
                inter = inter_sage(Hs, Hq)
                comax = lat_index(inter) == lat_index(Hs) * lat_index(Hq) // lat_index(L0)
                tag = "gcd=1" if gcd(N, q) == 1 else "gcd>1"
                q3p = hnf_rows([tuple(q * x for x in r) for r in Hs] + [tuple(N * x for x in r) for r in Hq])
                stats[f"PAIRS {tag}, {'comaximal' if comax else 'not comaximal'}: Q3 == intersection" if q3p == inter else f"PAIRS {tag}, {'comaximal' if comax else 'not comaximal'}: Q3 != intersection"] += 1
                stats[f"PAIRS {tag}, {'comaximal' if comax else 'not comaximal'}: index(I_s ∩ I_q) == (Nq)^2 (norm Nq)" if ideal_norm(inter) == N * q else f"PAIRS {tag}, {'comaximal' if comax else 'not comaximal'}: norm of intersection != Nq"] += 1
                for (u, v) in ((u0, v0), (v0, u0)):
                    out = spec_intersection(p, N, u, v, q, rq, fermat=False)
                    if out is None:
                        stats[f"{tag}: spec refuses"] += 1
                        continue
                    U, V, T = out
                    S = gaussian_span(U, V)
                    stats[f"{tag}: spec ok"] += 1
                    if S == inter: stats[f"{tag}: spec lattice == I_sec ∩ I_q"] += 1
                    else: stats[f"{tag}: SPEC LATTICE MISMATCH"] += 1
                    if comax: stats[f"{tag}: comaximal"] += 1
                    else: stats[f"{tag}: NOT comaximal although spec ok"] += 1
                    if ideal_norm(S) == N * q: stats[f"{tag}: norm(S) == N q"] += 1
                    else: stats[f"{tag}: norm(S) != N q"] += 1
                    q3 = hnf_rows([tuple(q * x for x in r) for r in Hs] + [tuple(N * x for x in r) for r in Hq])
                    if q3 == S: stats[f"{tag}: Q3 formula == spec"] += 1
                    else: stats[f"{tag}: Q3 formula != spec"] += 1
                    if gcd(N, q) != 1 and len(detail) < 6 and q3 != S:
                        detail.append({"N": N, "q": q, "p": p, "u": u, "v": v, "U": U, "V": V, "T": T})
for k in sorted(stats): print(k, stats[k])
json.dump({"p": p, "stats": dict(stats), "examples_Q3_differs": detail}, open(f"results_shared_factor_p{p}.json", "w"), indent=1, default=str)
