# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Why is H1's trace multimodal? Group H1 traces by the XOF-chosen row
permutation (first two rows) of the mask U (24 possibilities) and by U's largest entry size.
Three p500 challenge degrees x 300 simulated public keys."""
import json, secrets
import numpy as np
from lp4d import *
from run_4d import t1_target

name, e, f = 'p500', 500, 27
p = prime_of(e, f)
out = []
for t in range(3):
    T = t1_target(e); B0, _ = embed_D0(T, p)
    for _ in range(300):
        seed = seed_from_pk(secrets.token_bytes(128))
        U = unimodular_from_xof(seed, T)
        z = xof_ints(seed, T, b'H1', 13, 129)[12]; perm = list(range(4))
        for i in range(3, 0, -1):
            j = z % (i + 1); z //= (i + 1); perm[i], perm[j] = perm[j], perm[i]
        perm_code = perm[0] * 6 + perm[1] * 1 + 100 * 0  # encodes the permutation's first two rows
        _, tr = lll_gram(gram_norm(matmul(U, B0), p))
        out.append(dict(t=t, it=tr['iterations'], lead=perm_code,
                        umax=max(abs(x).bit_length() for r in U for x in r)))
it = np.array([o['it'] for o in out]); lead = np.array([o['lead'] for o in out]); tt = np.array([o['t'] for o in out])
def eta2(y, g):
    m = y.mean(); return float(sum((g == k).sum() * (y[g == k].mean() - m) ** 2 for k in np.unique(g)) / ((y - m) ** 2).sum())
res = dict(eta2_by_perm_first_rows=round(eta2(it, lead), 3), eta2_by_target=round(eta2(it, tt), 3),
           mean_by_perm_first_rows={int(k): round(float(it[lead == k].mean()), 1) for k in np.unique(lead)},
           n=len(out))
print(json.dumps(res)); json.dump(res, open('results/h1_modes_p500.json', 'w'), indent=1)
