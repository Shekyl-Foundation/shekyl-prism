# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Shared test vectors for the Rust port, from the Python model in
research/fixed-schedule-reduction (fsr.py, hdl.py), unchanged.

Each vector is a secret-product lattice I_sec * I_q with HNF basis
{T, T*i, alpha, i*alpha}, alpha = x + y*i + j, T = N*q. Written per set:

  vectors/<set>_check.txt   cross-check: 'nbound' (bits(N) = B) and
                            'nvar' (bits(N) = ceil(L/2)+u, u in 0..32)
                            rows, with lambda_1 (exhaustive enumeration
                            on the exact-LLL basis, fsr.shortest_norm)
                            and the model's Z[i] output nrd(u) after the
                            set's fixed step count, exact quotients and
                            64-bit quotients.
  vectors/<set>_pool.txt    random-class pool for the dudect test (fresh
                            key per row, varied bits(N)); and one fixed
                            row (the fixed class) on the first line.

Line format (hex, space separated):
  kind nb T x y nu_exact nu_p64 lam1     (lam1 = 0 when not computed)

Usage: PY=/workspace/mamba/envs/sage/bin/python
       FSR_DIR=../fixed-schedule-reduction $PY gen_vectors.py [n_check n_pool]
"""
import os
import sys
from multiprocessing import Pool

sys.path.insert(0, os.environ.get('FSR_DIR', os.path.join(
    os.path.dirname(os.path.abspath(__file__)), '..', 'fixed-schedule-reduction')))

from hdl import SETS, shake, stream_job_data, select_xy, gram_norm, ideal_from_classes  # noqa: E402
from fsr import zi_gauss_fixed, shortest_norm  # noqa: E402
from run_fsr import n_bound, secret_class_var  # noqa: E402

# Fixed step counts of the Python model (REPORT.md section 4, secret product, sized from B)
ZI_T = {'p248': 189, 'p324': 239, 'p376': 283, 'p500': 359, 'p664': 465}
SET_MAP = {n: (e, f) for n, e, f in SETS}


def one(args):
    set_name, kind, kid, want_lam1 = args
    e, f = SET_MAP[set_name]
    sk = shake(b'rlc-sk-' + kind.encode(), set_name.encode(), kid.to_bytes(4, 'little'), n=32)
    msg = shake(b'rlc-msg', set_name.encode(), kid.to_bytes(4, 'little'), n=32)
    data = stream_job_data(sk, msg, set_name, e, f)
    p, q = data['p'], data['q']
    xq, yq, _ = select_xy(q, p, data['seed'])
    force = n_bound(p) if kind in ('nbound', 'fixed') else None
    N, xN, yN, nb = secret_class_var(sk, p, force_nb=force)
    B, T = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
    alpha = B[2]
    assert alpha[2:] == [1, 0]
    steps = ZI_T[set_name]
    _, _, nu, _, _, _ = zi_gauss_fixed(T, alpha, p, steps, keep_trace=False)
    _, _, nu64, _, _, _ = zi_gauss_fixed(T, alpha, p, steps, keep_trace=False, prec=64)
    lam1 = shortest_norm(gram_norm(B, p))[0] if want_lam1 else 0
    return ' '.join([kind, format(nb, 'x'), format(T, 'x'), format(alpha[0], 'x'),
                     format(alpha[1], 'x'), format(nu, 'x'), format(nu64, 'x'),
                     format(lam1, 'x')])


def main():
    n_check = int(sys.argv[1]) if len(sys.argv) > 1 else 100
    n_pool = int(sys.argv[2]) if len(sys.argv) > 2 else 1000
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'vectors')
    os.makedirs(out, exist_ok=True)
    with Pool(int(os.environ.get('JOBS', '2'))) as pool:
        for set_name, e, f in SETS:
            jobs = [(set_name, k, i, True) for k in ('nbound', 'nvar') for i in range(n_check)]
            rows = pool.map(one, jobs, chunksize=8)
            with open(os.path.join(out, f'{set_name}_check.txt'), 'w') as fh:
                fh.write(f'# {set_name} p={format((f << e) - 1, "x")} steps={ZI_T[set_name]}\n')
                fh.write('\n'.join(rows) + '\n')
            jobs = [(set_name, 'fixed', 0, False)] + [(set_name, 'rand', i, False) for i in range(n_pool)]
            rows = pool.map(one, jobs, chunksize=16)
            with open(os.path.join(out, f'{set_name}_pool.txt'), 'w') as fh:
                fh.write(f'# {set_name} p={format((f << e) - 1, "x")} steps={ZI_T[set_name]}\n')
                fh.write('\n'.join(rows) + '\n')
            print(set_name, 'done', flush=True)


if __name__ == '__main__':
    main()
