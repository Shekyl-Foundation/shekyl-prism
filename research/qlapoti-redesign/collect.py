# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Data collection: per set, K keys x M messages prepared small ideals S;
per S, G generators each swept over lambda = 1..LAM at three target
exponents (e-2, e-1, e) with the same generator (paired), plus GD
independent generator searches (draw counts only). Resumable: one JSONL
chunk per (set, key range), skipped when present.

Usage: python collect.py --sets p248,p324,p376,p500,p664 --keys 40 \
          --msgs 10 --gens 8 --lam 4000 --procs 3
"""
import argparse
import json
import os
import time
import multiprocessing as mp
from pathlib import Path

import gmpy2

from qlr import (SET_MAP, prime_of, sec_degree, tape, keygen, small_ideal,
                 sweep, generator, Ctr)

OUT = Path(__file__).resolve().parent / 'data'


def small_factors(n, B=50):
    return [q for q in (2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47)
            if n % q == 0]


def job(args):
    name, k0, k1, M, G, LAM, GD = args
    path = OUT / name / f'sweep_{k0:04d}_{k1:04d}.jsonl'
    if path.exists():
        return str(path), 'skip', 0.0
    t0 = time.time()
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    SEC = sec_degree(name)
    sq = gmpy2.isqrt(p)
    lines = []
    for kid in range(k0, k1):
        Is = keygen(name, kid, p, SEC)
        for mid in range(M):
            S = small_ideal(name, Is, kid, mid, p, e)
            n = S.norm
            rec = dict(set=name, kid=kid, mid=mid, n_bits=int(n).bit_length(),
                       u=float(gmpy2.mpfr(n) / gmpy2.mpfr(sq)),
                       v2=int(gmpy2.bit_scan1(n)), den=int(S.den),
                       sf=small_factors(n))
            # generator-search draw counts (uncapped reference search)
            rg = tape(b'gd', name, kid, mid)
            rec['gd'] = [generator(S, p, rg, Ctr())[1] for _ in range(GD)]
            gens = []
            for g in range(G):
                gr = {}
                for de in (2, 1, 0):
                    rng = tape(b'sw', name, kid, mid, g)
                    o = sweep(S, p, e - de, rng, range(1, LAM + 1))
                    if de == 2:
                        gr.update(draws=o['draws'], gauss=o['gauss'],
                                  suitable=o['suitable'],
                                  lat_short2=o.get('lat_short2', 0),
                                  lat_long2=o.get('lat_long2', 0),
                                  r0bits=o.get('r0bits', 0))
                    codes = ''.join(str(x[0]) for x in o['lam'])
                    det = [(i, x[1], x[2], x[3]) for i, x in enumerate(o['lam'])
                           if x[0] >= 4]
                    m4b = [x[1] for x in o['lam'] if x[0] >= 3]
                    gr[f'c{de}'] = codes
                    gr[f'd{de}'] = det
                    gr[f'mb{de}'] = (min(m4b), max(m4b)) if m4b else (0, 0)
                gens.append(gr)
            rec['gens'] = gens
            lines.append(json.dumps(rec, separators=(',', ':')))
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix('.tmp')
    tmp.write_text('\n'.join(lines) + '\n')
    os.replace(tmp, path)
    return str(path), 'done', time.time() - t0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', default='p248,p324,p376,p500,p664')
    ap.add_argument('--keys', type=int, default=40)
    ap.add_argument('--msgs', type=int, default=10)
    ap.add_argument('--gens', type=int, default=8)
    ap.add_argument('--lam', type=int, default=4000)
    ap.add_argument('--gd', type=int, default=64)
    ap.add_argument('--chunk', type=int, default=2)
    ap.add_argument('--procs', type=int, default=3)
    a = ap.parse_args()
    jobs = []
    for name in a.sets.split(','):
        for k0 in range(0, a.keys, a.chunk):
            jobs.append((name, k0, min(a.keys, k0 + a.chunk), a.msgs, a.gens,
                         a.lam, a.gd))
    with mp.Pool(a.procs) as pool:
        for r in pool.imap_unordered(job, jobs):
            print(time.strftime('%H:%M:%S'), *r, flush=True)


if __name__ == '__main__':
    main()
