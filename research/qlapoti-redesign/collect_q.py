# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Qlapoty-style prototype on the same prepared small ideals as collect.py
(same keys and messages, odd n only). Per S: G alpha0 draws (Loop1-lite),
each swept over odd lambda = 1, 3, ..., 2*LAM-1 at e-2, e-1 and e, prime
and extended Cornacchia. Resumable chunks in data/<set>/qy_*.jsonl."""
import argparse
import json
import os
import random
import time
import multiprocessing as mp
from pathlib import Path

import gmpy2

from qlr import SET_MAP, prime_of, sec_degree, keygen, small_ideal, tape
from qlapoty_proto import prepare, loop2

OUT = Path(__file__).resolve().parent / 'data'


def job(args):
    name, k0, k1, M, G, LAM = args
    path = OUT / name / f'qy_{k0:04d}_{k1:04d}.jsonl'
    if path.exists():
        return str(path), 'skip', 0.0
    t0 = time.time()
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    SEC = sec_degree(name)
    sq = gmpy2.isqrt(p)
    lines = []
    lams = range(1, 2 * LAM, 2)
    for kid in range(k0, k1):
        Is = keygen(name, kid, p, SEC)
        for mid in range(M):
            S = small_ideal(name, Is, kid, mid, p, e)
            n = S.norm
            rec = dict(set=name, kid=kid, mid=mid, n_bits=int(n).bit_length(),
                       u=float(gmpy2.mpfr(n) / gmpy2.mpfr(sq)), v2=int(gmpy2.bit_scan1(n)))
            if n % 2 == 0:
                rec['why'] = 'even'
                lines.append(json.dumps(rec))
                continue
            gens = []
            for g in range(G):
                rs = random.Random(int.from_bytes(tape(b'qy', name, kid, mid, g).getrandbits(64).to_bytes(8, 'little'), 'little'))
                P = prepare(S, p, rs, e - 2)
                gr = dict(why=P.why, loop1=P.loop1)
                if P.why is None:
                    gr.update(balanced=P.balanced, gauss=P.gauss, r0bits=int(P.r0).bit_length(),
                              m1bits=int(P.m1).bit_length(), interval=P.interval)
                    for de in (2, 1, 0):
                        o = loop2(S, p, e - de, P, lams, extended=True)
                        gr[f'c{de}'] = ''.join(str(x[0]) for x in o)
                        gr[f'd{de}'] = [(i, x[1], x[2]) for i, x in enumerate(o) if x[0] >= 4]
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
    ap.add_argument('--keys', type=int, default=60)
    ap.add_argument('--msgs', type=int, default=10)
    ap.add_argument('--gens', type=int, default=4)
    ap.add_argument('--lam', type=int, default=4000)
    ap.add_argument('--chunk', type=int, default=4)
    ap.add_argument('--procs', type=int, default=3)
    a = ap.parse_args()
    jobs = []
    for name in a.sets.split(','):
        for k0 in range(0, a.keys, a.chunk):
            jobs.append((name, k0, min(a.keys, k0 + a.chunk), a.msgs, a.gens, a.lam))
    with mp.Pool(a.procs) as pool:
        for r in pool.imap_unordered(job, jobs):
            print(time.strftime('%H:%M:%S'), *r, flush=True)


if __name__ == '__main__':
    main()
