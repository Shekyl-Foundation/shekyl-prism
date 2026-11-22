# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Pass 2: fixed-budget short-vector search after a shortened fixed
pre-reduction.

For each sample, the pre-reduction is the fixed schedule cut Delta
tours/steps short of the per-set fixed count chosen in pass 1
(results/schedules.json). Two pre-reductions are tried:
  updown  4-D fixed tours (generic), then 4-D enumeration / sieve;
  zi      Z[i]-Lagrange fixed steps, then 4-D enumeration on {u, iu, v, iv}.
For each (pre-reduction, Delta) we record the node index at which
Schnorr-Euchner enumeration first holds a vector no longer than exact
LLL's b1 (capped), and the first sieve round that does (updown only).
"""
import argparse
import csv
import json
import time
from multiprocessing import Pool
from pathlib import Path

from hdl import (SETS, shake, stream_job_data, select_xy, ideal_basis, gram_norm,
                 secret_class, ideal_from_classes)
from fsr import (lll_exact, reduced_gram, enum_budget, fixed_tours,
                 zi_gauss_fixed, sieve_rounds, _init_state, Ctr)
from run_fsr import SET_MAP, sk_of, msg_of, write_csv, secret_class_var, n_bound

DELTAS = (0, 2, 4, 8, 12, 16, 24, 32)
CAP = 1 << 13
SIEVE_R = 6
SIEVE_DELTAS = (0, 2, 4, 8)


def lattice(set_name, kid, mid, panel):
    _, e, f = SET_MAP[set_name]
    sk = sk_of(set_name, kid)
    msg = msg_of(set_name, kid, mid)
    data = stream_job_data(sk, msg, set_name, e, f)
    p, q = data['p'], data['q']
    xq, yq, _ = select_xy(q, p, data['seed'])
    if panel == 'stream':
        return ideal_basis(q, p, (xq, yq, 1, 0)), q, p
    if panel == 'nvar':
        sk = shake(b'sk-nvar', set_name.encode(), kid.to_bytes(4, 'little'), n=32)
        data = stream_job_data(sk, msg, set_name, e, f)
        p, q = data['p'], data['q']
        xq, yq, _ = select_xy(q, p, data['seed'])
        N, xN, yN, _ = secret_class_var(sk, p)
        B, T = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
        return B, T, p
    N, xN, yN = secret_class(sk, p, p.bit_length() // 2)
    B, T = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
    return B, T, p


def job(args):
    set_name, panel, kid, mid, T_fix, S_fix = args
    B, T, p = lattice(set_name, kid, mid, panel)
    G = gram_norm(B, p)
    _, d, _, _ = lll_exact(G)
    target = d[1]
    row = dict(set=set_name, panel=panel, key=kid, msg=mid, T_bits=T.bit_length(),
               T_fix=T_fix, S_fix=S_fix)

    want = {T_fix - D: D for D in DELTAS if T_fix - D >= 1}
    snaps = {}

    def hook(t, H_, d_, l_):
        if t in want:
            snaps[t] = ([r[:] for r in H_], d_[:], [r[:] for r in l_])

    fixed_tours(G, T_fix, 'updown', on_tour=hook, keep_trace=False)
    for t, D in want.items():
        H_, d_, l_ = snaps[t]
        Gr = reduced_gram(G, H_)
        r = enum_budget(Gr, d_, l_, target, CAP)
        row[f'ud_enum_d{D}'] = -1 if r['hit'] is None else r['hit']
        row[f'ud_tree_d{D}'] = r['nodes'] if r['complete'] else -1
        if D in SIEVE_DELTAS:
            hit, _ = sieve_rounds(Gr, target, SIEVE_R)
            row[f'ud_sieve_d{D}'] = -1 if hit is None else hit

    wantz = {S_fix - D: D for D in DELTAS if S_fix - D >= 1}
    zs = {}

    def zhook(t, u, v, nu, nv):
        if t in wantz:
            zs[t] = (u[:], v[:])

    zi_gauss_fixed(T, B[2], p, S_fix, on_step=zhook, keep_trace=False)
    for t, D in wantz.items():
        u, v = zs[t]
        rows = [u, [-u[1], u[0], -u[3], u[2]], v, [-v[1], v[0], -v[3], v[2]]]
        G4 = gram_norm(rows, p)
        _, d4, l4 = _init_state(G4, Ctr())
        r = enum_budget(G4, d4, l4, target, CAP)
        row[f'zi_enum_d{D}'] = -1 if r['hit'] is None else r['hit']
        row[f'zi_tree_d{D}'] = r['nodes'] if r['complete'] else -1
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', nargs='+', default=[s[0] for s in SETS])
    ap.add_argument('--panels', nargs='+', default=['stream', 'nvar'])
    ap.add_argument('--keys', type=int, default=50)
    ap.add_argument('--msgs', type=int, default=10)
    ap.add_argument('--block', type=int, default=10)
    ap.add_argument('--procs', type=int, default=7)
    ap.add_argument('--out', default='results/raw_budget')
    a = ap.parse_args()
    sched = json.loads(Path('results/schedules.json').read_text())
    out = Path(a.out)
    with Pool(a.procs) as pool:
        for set_name in a.sets:
            for panel in a.panels:
                sc = sched[panel][set_name]
                for k0 in range(0, a.keys, a.block):
                    path = out / f'{panel}_{set_name}_k{k0:03d}.csv'
                    if path.exists():
                        print(f'skip {path}', flush=True)
                        continue
                    t0 = time.time()
                    jobs = [(set_name, panel, k, m, sc['updown'], sc['zi'])
                            for k in range(k0, min(a.keys, k0 + a.block))
                            for m in range(a.msgs)]
                    rows = pool.map(job, jobs, chunksize=1)
                    write_csv(path, rows)
                    print(f'wrote {path} ({len(rows)}) {time.time() - t0:.0f}s',
                          flush=True)
    print('ALL_DONE', flush=True)


if __name__ == '__main__':
    main()
