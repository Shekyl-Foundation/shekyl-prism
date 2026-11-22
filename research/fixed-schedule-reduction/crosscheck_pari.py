# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Independent check with PARI: lambda_1 from qfminim (after PARI's own
qflllgram) equals (a) our enumeration's lambda_1 and (b) nrd of the
Z[i]-Lagrange output u, on a sample of stream and product lattices."""
from hdl import pari, SETS, gram_norm
from fsr import shortest_norm, zi_gauss_fixed
from run_budget import lattice
from run_fsr import n_bound
from run_fsr import tmax

pari.set_real_precision(1200)
lines = []
bad = 0
tot = 0
for s, e, f in SETS:
    for panel in ('stream', 'nvar'):
        for kid in range(10):
            for mid in range(5):
                B, T, p = lattice(s, kid, mid, panel)
                G = gram_norm(B, p)
                M = pari.matrix(4, 4, [G[r][c] for r in range(4) for c in range(4)])
                U = pari.qflllgram(M)
                Mr = U.mattranspose() * M * U
                res = pari.qfminim(Mr, None, 4, 2)
                V = res[2]
                # exact norm of PARI's minimal vectors (its reported norm is
                # a floating-point value at these sizes)
                Gm = [[int(Mr[r, t]) for t in range(4)] for r in range(4)]
                lam1_pari = None
                for j in range(int(pari.matsize(V)[1])):
                    x = [int(V[r, j]) for r in range(4)]
                    nx = sum(x[r] * Gm[r][t] * x[t] for r in range(4) for t in range(4))
                    lam1_pari = nx if lam1_pari is None else min(lam1_pari, nx)
                lam1_ours, _ = shortest_norm(G)
                bits = 2 * e - 5 + (n_bound(p) if panel == 'nvar' else 0)
                nu = zi_gauss_fixed(T, B[2], p, tmax(bits, 'zi'), keep_trace=False)[2]
                ok = lam1_pari == lam1_ours == nu
                tot += 1
                bad += not ok
        lines.append(f'{s} {panel}: {tot} checked so far, {bad} mismatches')
        print(lines[-1], flush=True)
lines.append(f'TOTAL {tot} lattices, {bad} mismatches between PARI qfminim, '
             f'our enumeration, and the Z[i]-Lagrange output (stream and varied-N product)')
open('results/pari_crosscheck.txt', 'w').write('\n'.join(lines) + '\n')
print(lines[-1])
