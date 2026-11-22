# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Run the higher-dim / ablation panels. Skips any CSV that already exists."""
import argparse
import csv
import time
from multiprocessing import Pool
from pathlib import Path

from hdl import (
    SETS, STREAM_ARMS, PROD_ARMS, ARM_META, SALT_LEN,
    shake, stream_job_data, run_stream_arm, run_prod_arm,
)


def write_csv(path, rows):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    if not rows:
        return
    keys = list(rows[0].keys())
    with path.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=keys, extrasaction='ignore')
        w.writeheader()
        w.writerows(rows)
    print(f'wrote {path} ({len(rows)} rows)', flush=True)


def sk_of(set_name, kid):
    return shake(b'sk', set_name.encode(), kid.to_bytes(4, 'little'), n=32)


def msg_of(set_name, kid, mid):
    return shake(b'msg', set_name.encode(), kid.to_bytes(4, 'little'),
                 mid.to_bytes(4, 'little'), n=32)


STREAM_NAMES = [a[0] for a in STREAM_ARMS]
PROD_NAMES = [a[0] for a in PROD_ARMS]


def _row(set_name, name, kid, mid, data, out, tries):
    ch, dim, _sol = ARM_META[name]
    r = dict(
        set=set_name, construction=name, characteristic=ch, dim=dim,
        key=kid, msg=mid,
        q_bits=data['q'].bit_length(), p_bits=data['p'].bit_length(),
        salt_len=SALT_LEN[set_name], qtries=data['qtries'], tries=tries,
    )
    r.update(out)
    return r


def stream_job(args):
    set_name, e, f, kid, mid, names = args
    sk = sk_of(set_name, kid)
    msg = msg_of(set_name, kid, mid)
    data = stream_job_data(sk, msg, set_name, e, f)
    return [_row(set_name, name, kid, mid, data, *run_stream_arm(name, data))
            for name in names]


def prod_job(args):
    set_name, e, f, kid, mid, names = args
    sk = sk_of(set_name, kid)
    msg = msg_of(set_name, kid, mid)
    data = stream_job_data(sk, msg, set_name, e, f)
    return [_row(set_name, name, kid, mid, data, *run_prod_arm(name, sk, data))
            for name in names]


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
    rows = [r for ch in chunks for r in ch]
    write_csv(path, rows)
    print(f'  {path.name}: {len(rows)} rows in {time.time() - t0:.0f}s', flush=True)
    return len(rows)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', nargs='+', default=[s[0] for s in SETS])
    ap.add_argument('--keys', type=int, default=20)
    ap.add_argument('--msgs', type=int, default=8)
    ap.add_argument('--keys-prod', type=int, default=20)
    ap.add_argument('--msgs-prod', type=int, default=8)
    ap.add_argument('--no-prod', action='store_true')
    ap.add_argument('--no-emb16', action='store_true',
                    help='drop EMB16 from the stream (slow)')
    ap.add_argument('--procs', type=int, default=7)
    ap.add_argument('--out', default='results/raw')
    a = ap.parse_args()

    stream_names = [n for n in STREAM_NAMES
                    if not (a.no_emb16 and n == 'EMB16')]
    prod_names = list(PROD_NAMES)
    set_map = {n: (n, e, f) for n, e, f in SETS}

    for name, e, f in [set_map[n] for n in a.sets]:
        t0 = time.time()
        sjobs = [(name, e, f, k, m, stream_names)
                 for k in range(a.keys) for m in range(a.msgs)]
        n1 = run_panel(f'{a.out}/stream_{name}.csv', sjobs, stream_job, a.procs)
        n2 = 0
        if not a.no_prod:
            pjobs = [(name, e, f, k, m, prod_names)
                     for k in range(a.keys_prod) for m in range(a.msgs_prod)]
            n2 = run_panel(f'{a.out}/prod_{name}.csv', pjobs, prod_job, a.procs)
        print(f'{name} stream={n1} prod={n2} in {time.time() - t0:.0f}s', flush=True)
    print('ALL_DONE', flush=True)


if __name__ == '__main__':
    main()
