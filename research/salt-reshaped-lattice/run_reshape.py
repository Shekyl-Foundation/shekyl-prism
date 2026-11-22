# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Run the salt-reshape panels. Skips any CSV that already exists.

Panels:
  stream   real salted Fiat-Shamir challenges, I_q-side constructions
  prod     I_sec * I_q (and reshaped products), key panel
  sweep    synthetic prime degrees of many bit lengths at fixed p (P248
           and P664), to separate the q-size slope from the set change

Messages and secret keys are drawn from SHAKE with a fixed experiment
seed, so a rerun of a finished CSV can be skipped safely.
"""
import argparse
import csv
import os
import sys
import time
from multiprocessing import Pool
from pathlib import Path

from srl import (
    SETS, STREAM_NAMES, PROD_NAMES, SALT_LEN,
    shake, stream_job_data, run_stream_construction, run_prod_construction,
    prime_of, select_xy, select_xy_d0, ideal_basis, reduce_ideal, gram_norm,
    lll_gram, apply, nrd,
)

import gmpy2


def write_csv(path, rows):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    if not rows:
        return
    keys = list(rows[0].keys())
    with path.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=keys)
        w.writeheader()
        w.writerows(rows)
    print(f'wrote {path} ({len(rows)} rows)', flush=True)


def sk_of(set_name, kid):
    return shake(b'sk', set_name.encode(), kid.to_bytes(4, 'little'), n=32)


def msg_of(set_name, kid, mid):
    return shake(b'msg', set_name.encode(), kid.to_bytes(4, 'little'),
                 mid.to_bytes(4, 'little'), n=32)


def stream_job(args):
    set_name, e, f, kid, mid, names = args
    sk = sk_of(set_name, kid)
    msg = msg_of(set_name, kid, mid)
    data = stream_job_data(sk, msg, set_name, e, f)
    rows = []
    for name in names:
        out = run_stream_construction(name, data)
        rows.append(dict(
            set=set_name, construction=name, key=kid, msg=mid,
            q_bits=data['q'].bit_length(), p_bits=data['p'].bit_length(),
            salt_len=SALT_LEN[set_name], qtries=data['qtries'],
            **out,
        ))
    return rows


def prod_job(args):
    set_name, e, f, kid, mid, names = args
    sk = sk_of(set_name, kid)
    msg = msg_of(set_name, kid, mid)
    data = stream_job_data(sk, msg, set_name, e, f)
    rows = []
    for name in names:
        out = run_prod_construction(name, sk, data)
        rows.append(dict(
            set=set_name, construction=name, key=kid, msg=mid,
            q_bits=data['q'].bit_length(), p_bits=data['p'].bit_length(),
            salt_len=SALT_LEN[set_name], qtries=data['qtries'],
            **out,
        ))
    return rows


def prime_of_bits(bits, salt, ctr_start=0):
    """A bits-bit prime ≡ 7 (mod 8), like a challenge degree, from a seed."""
    ctr = ctr_start
    while True:
        raw = shake(b'sweep-q', salt, bits.to_bytes(2, 'little'),
                    ctr.to_bytes(4, 'little'), n=bits // 8 + 8)
        q = int.from_bytes(raw, 'little') & ((1 << bits) - 1)
        q |= (1 << (bits - 1)) | 7
        q -= q % 8
        q += 7
        ctr += 1
        if gmpy2.is_prime(q):
            return q, ctr


def sweep_job(args):
    """H2 and D0 only, at a synthetic bit length, fixed p."""
    set_name, e, f, bits, draw = args
    p = prime_of(e, f)
    salt = shake(b'sweep-salt', set_name.encode(), bits.to_bytes(2, 'little'),
                 draw.to_bytes(4, 'little'), n=SALT_LEN[set_name])
    q, hops = prime_of_bits(bits, salt)
    seed = shake(b'h2-seed', salt, q.to_bytes((q.bit_length() + 7) // 8, 'little'))
    rows = []
    for name, selector in (('H2', lambda: select_xy(q, p, seed)),
                           ('D0', lambda: select_xy_d0(q, p))):
        x, y, tries = selector()
        B = ideal_basis(q, p, (x, y, 1, 0))
        out = reduce_ideal(B, p, q)
        rows.append(dict(
            set=set_name, construction=name, key=0, msg=draw,
            q_bits=q.bit_length(), p_bits=p.bit_length(),
            salt_len=SALT_LEN[set_name], qtries=hops,
            r_bits=0, scale=1, prec=0, index=1, pre_after=1,
            format_change=0, tries=tries, **out,
        ))
    return rows


def run_panel(path, jobs, fn, procs, chunksize=2):
    path = Path(path)
    if path.exists():
        print(f'skip {path} (exists)', flush=True)
        return 0
    t0 = time.time()
    if not jobs:
        return 0
    with Pool(procs) as pool:
        chunks = pool.map(fn, jobs, chunksize=chunksize)
    rows = [r for ch in chunks for r in (ch if isinstance(ch, list) else [ch])]
    write_csv(path, rows)
    print(f'  {path.name}: {len(rows)} rows in {time.time() - t0:.0f}s', flush=True)
    return len(rows)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', nargs='+', default=[s[0] for s in SETS])
    ap.add_argument('--keys', type=int, default=16,
                    help='stream: distinct (sk) count')
    ap.add_argument('--msgs', type=int, default=8,
                    help='stream: messages per key')
    ap.add_argument('--keys-prod', type=int, default=16)
    ap.add_argument('--msgs-prod', type=int, default=8)
    ap.add_argument('--sweep-draws', type=int, default=24,
                    help='synthetic primes per bit-length per fixed p')
    ap.add_argument('--no-sweep', action='store_true')
    ap.add_argument('--no-prod', action='store_true')
    ap.add_argument('--procs', type=int, default=7)
    ap.add_argument('--out', default='results/raw')
    a = ap.parse_args()

    set_map = {n: (n, e, f) for n, e, f in SETS}
    wanted = [set_map[n] for n in a.sets]

    for name, e, f in wanted:
        t0 = time.time()
        sjobs = [(name, e, f, k, m, STREAM_NAMES)
                 for k in range(a.keys) for m in range(a.msgs)]
        n1 = run_panel(f'{a.out}/stream_{name}.csv', sjobs, stream_job, a.procs)
        n2 = 0
        if not a.no_prod:
            pjobs = [(name, e, f, k, m, PROD_NAMES)
                     for k in range(a.keys_prod) for m in range(a.msgs_prod)]
            n2 = run_panel(f'{a.out}/prod_{name}.csv', pjobs, prod_job, a.procs)
        print(f'{name} stream={n1} prod={n2} in {time.time() - t0:.0f}s', flush=True)

    if not a.no_sweep:
        # Bit lengths from just below P248's q to P664's q.
        bit_lengths = [400, 491, 550, 643, 747, 850, 995, 1100, 1323]
        for host in ('p248', 'p664'):
            name, e, f = set_map[host]
            jobs = [(name, e, f, b, d)
                    for b in bit_lengths for d in range(a.sweep_draws)]
            run_panel(f'{a.out}/sweep_{name}.csv', jobs, sweep_job, a.procs,
                      chunksize=1)

    print('ALL_DONE', flush=True)


if __name__ == '__main__':
    main()
