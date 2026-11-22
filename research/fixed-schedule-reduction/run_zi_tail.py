# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Tail run for the Z[i]-Lagrange fixed schedule only: many more samples
(messages 40..40+msgs-1, disjoint from pass 1) to pin down the upper tail
of the step count needed to reach the Z[i]-reduced fixed point, for the
challenge ideal (stream) and for the secret product with bits(N) at the
bound B (nbound). Pass 1 checks on every row that this fixed point gives
lambda_1."""
import argparse
import time
from multiprocessing import Pool
from pathlib import Path

from hdl import (SETS, stream_job_data, select_xy, ideal_basis, secret_class,
                 ideal_from_classes)
from fsr import zi_gauss_fixed, herm, Ctr
from run_fsr import SET_MAP, sk_of, msg_of, write_csv, tmax, secret_class_var, n_bound

MSG0 = 40


def job(args):
    set_name, kid, mid = args
    _, e, f = SET_MAP[set_name]
    sk = sk_of(set_name, kid)
    msg = msg_of(set_name, kid, mid)
    data = stream_job_data(sk, msg, set_name, e, f)
    p, q = data['p'], data['q']
    xq, yq, _ = select_xy(q, p, data['seed'])
    row = dict(set=set_name, key=kid, msg=mid)
    lat = {'stream': (ideal_basis(q, p, (xq, yq, 1, 0)), q, 2 * e - 5)}
    # product with the secret norm at its bound B (the worst length a
    # schedule sized from B must cover); schedule length from B
    N, xN, yN, _ = secret_class_var(sk, p, force_nb=n_bound(p))
    Bp, Tp = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
    lat['nbound'] = (Bp, Tp, 2 * e - 5 + n_bound(p))
    for panel, (B, T, bits) in lat.items():
        st = dict(red=-1)

        def zhook(t, u, v, nu, nv):
            if st['red'] < 0:
                h0, h1 = herm(v, u, p, Ctr())
                if ((2 * h0 + nu) // (2 * nu) == 0 and
                        (2 * h1 + nu) // (2 * nu) == 0 and nu <= nv):
                    st['red'] = t

        zi_gauss_fixed(T, B[2], p, tmax(bits, 'zi'), on_step=zhook,
                       keep_trace=False)
        row[f'{panel}_zi_red'] = st['red']
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', nargs='+', default=[s[0] for s in SETS])
    ap.add_argument('--keys', type=int, default=100)
    ap.add_argument('--msgs', type=int, default=200)
    ap.add_argument('--block', type=int, default=10)
    ap.add_argument('--procs', type=int, default=7)
    ap.add_argument('--out', default='results/raw_zi_tail')
    a = ap.parse_args()
    out = Path(a.out)
    with Pool(a.procs) as pool:
        for set_name in a.sets:
            for k0 in range(0, a.keys, a.block):
                path = out / f'tail_{set_name}_k{k0:03d}.csv'
                if path.exists():
                    print(f'skip {path}', flush=True)
                    continue
                t0 = time.time()
                jobs = [(set_name, k, m)
                        for k in range(k0, min(a.keys, k0 + a.block))
                        for m in range(MSG0, MSG0 + a.msgs)]
                rows = pool.map(job, jobs, chunksize=4)
                write_csv(path, rows)
                print(f'wrote {path} ({len(rows)}) {time.time() - t0:.0f}s',
                      flush=True)
    print('ALL_DONE', flush=True)


if __name__ == '__main__':
    main()
