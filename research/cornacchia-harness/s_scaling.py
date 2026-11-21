# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Extra experiment: does a *bigger / fully random* unimodular mask S erase the
dependence of the Lagrange trace on the lattice (M, u)?  Not in 2023/807; ours.
Fixed panel: 30 primes M (384 bits), 400 draws each, S uniform-ish in SL2(Z)
with entries of t bits, t in {32, 128, 256, 384, 768}. Reports slope/r of
per-M mean B steps on L steps, and ICC (share of B-variance explained by M)."""
import secrets, json, gmpy2, numpy as np
from scipy import stats
from cornacchia_variants import sqrt_minus_one, lattice_plain, lattice_randomized
from run_experiments import rand_prime_1mod4

def random_sl2(t):
    while True:
        a = secrets.randbits(t) | (1 << (t - 1)); c = secrets.randbits(t) | (1 << (t - 1))
        g, s, tt = gmpy2.gcdext(a, c)
        if g == 1: break
    b, d = int(-tt), int(s)              # a*d - b*c = 1
    k = secrets.randbits(t) - (1 << (t - 1))
    b, d = b + k * a, d + k * c          # still det 1, entries ~2t bits max
    if secrets.randbelow(2): a, b, c, d = c, d, -a, -b   # random row swap (det kept)
    return ((a, b), (c, d))

out = {}
Ms = [rand_prime_1mod4(384) for _ in range(30)]
us = [sqrt_minus_one(M)[0] for M in Ms]
Ls = [lattice_plain(M, u)[1]['steps'] for M, u in zip(Ms, us)]
for t in [32, 128, 256, 384, 768]:
    B = np.array([[lattice_randomized(M, u, random_sl2(t))[1]['steps'] for _ in range(400)]
                  for M, u in zip(Ms, us)])
    means = B.mean(1); lr = stats.linregress(Ls, means)
    icc = means.var(ddof=1) / B.var(ddof=1)
    kw = stats.kruskal(*B)
    out[t] = dict(S_bits=t, B_mean=float(B.mean()), B_sd=float(B.std()),
                  sd_within=float(B.std(1).mean()), slope=float(lr.slope), r=float(lr.rvalue),
                  icc=float(icc), kruskal_H=float(kw.statistic), kruskal_p=float(kw.pvalue))
    print(out[t], flush=True)
json.dump(out, open('results/s_scaling_384.json', 'w'), indent=1)
