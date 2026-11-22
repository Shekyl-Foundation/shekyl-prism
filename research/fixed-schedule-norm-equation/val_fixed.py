# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Validation of the fixed-schedule executor (fsne.py) on fresh inputs.

Per set: n_full inputs run the whole schedule (P passes, never an early exit)
and again with an early exit on the same tape; the two must return the same
answer, every full run must have the same operation-trace hash, and every
answer must satisfy the norm equation. n_early further inputs run with the
early exit only, to measure the per-pass success rate of the fixed
schedule itself (K-draw generator, capped trial division, Jacobi-chosen
non-residue) against the variable 'fresh' policy. Resumable per set.
"""
import csv
import json
import multiprocessing as mp
import os
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))


def job(args):
    from fne import (SET_MAP, prime_of, sec_degree, tape, random_prime_ideal,
                     prime_norm_equivalent, intersect_coprime, prepare_small,
                     nrd_num, gmpy2)
    from fsne import Sched, qlapoti_fixed
    name, P, K, i0, i1, full = args
    out = HERE / 'data' / name / f"val_{'full' if full else 'early'}_{i0:05d}_{i1:05d}.csv"
    if out.exists():
        return str(out), 'skip'
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    SEC = sec_degree(name)
    sch = Sched(p.bit_length(), e, P=P, K=K)
    rows = []
    for i in range(i0, i1):
        kid = 5 * 10 ** 6 + i
        rng = tape(b'val', name, kid)
        Is, _ = prime_norm_equivalent(random_prime_ideal(SEC, p, rng), p, rng)
        while True:
            s = rng.getrandbits(e - 4) | 1
            q = (1 << (2 * e - 5)) - s * s
            if gmpy2.is_prime(q):
                break
        S = prepare_small(intersect_coprime(Is, random_prime_ideal(q, p, rng)), p)
        t0 = time.time()
        r2, i2 = qlapoti_fixed(S, p, e - 2, tape(b'valt', name, kid), sch, early_exit=True)
        row = dict(set=name, i=i, P=P, K=K, n_bits=int(S.norm).bit_length(),
                   early_first=i2['first'], early_ok=int(r2 is not None),
                   unf_lat_e=i2['unfinished']['lat'], unf_corn_e=i2['unfinished']['corn'])
        if r2 is not None:
            (g1, d1), (g2, d2), _, _ = r2
            row['eq_ok'] = int(nrd_num(g1, p) // d1 ** 2 // S.norm +
                               nrd_num(g2, p) // d2 ** 2 // S.norm == 1 << (e - 2))
        else:
            row['eq_ok'] = 0
        if full:
            r1, i1_ = qlapoti_fixed(S, p, e - 2, tape(b'valt', name, kid), sch)
            row.update(full_ok=int(r1 is not None), same=int(r1 == r2),
                       trace=i1_['trace'], full_passes=i1_['passes'],
                       unf_lat=i1_['unfinished']['lat'], unf_corn=i1_['unfinished']['corn'])
        row['sec'] = round(time.time() - t0, 3)
        rows.append(row)
    tmp = out.with_suffix('.tmp')
    with open(tmp, 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    os.replace(tmp, out)
    return str(out), len(rows)


if __name__ == '__main__':
    summ = json.load(open(HERE / 'results' / 'summary.json'))
    sets = sys.argv[1].split(',') if len(sys.argv) > 1 else ['p248', 'p324', 'p376', 'p500', 'p664']
    n_full = {'p248': 24, 'p324': 20, 'p376': 12, 'p500': 12, 'p664': 10}
    n_early = 250
    jobs = []
    for name in sets:
        sc = summ[name]['schedule']
        P = [b for b in sc['budgets'] if b['target'] == '99.9%'][0]['P']
        K = sc['K']
        for i0 in range(0, n_full[name], 4):
            jobs.append((name, P, K, i0, min(n_full[name], i0 + 4), True))
        for i0 in range(1000, 1000 + n_early, 100):
            jobs.append((name, P, K, i0, i0 + 100, False))
    print('jobs', len(jobs), flush=True)
    with mp.get_context('spawn').Pool(2) as pool:
        for r in pool.imap_unordered(job, jobs):
            print(time.strftime('%H:%M:%S'), r, flush=True)
    print('DONE', flush=True)
