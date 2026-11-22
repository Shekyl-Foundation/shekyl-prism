# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Runs for the secret-ideal re-randomization experiment.

Panels (per set):
  sampg         as samp, Gaussian-then-round sampler (prime/odd);
  samp          sampler only: many keys x signatures, all four sampler
                variants (accept prime/odd x coefficient shape plain/norm);
                retry counts, primality tests, N' length, key features.
  pair          same (key, msg): baseline product I ∩ I_q vs the
                re-randomized product J*K_J (variants prime-norm, odd-norm,
                odd-gauss);
                exact LLL op counts and the fixed Z[i] schedule on each;
                map-back and output-identity checks.
  dudect_fixed  one fixed key (its N at the bound B), many messages;
  dudect_rand   a fresh random key per trace.
Every schedule is sized from the public bound, never from N or N'.
Output is chunked and skipped if present (resumable).
"""
import argparse
import math
import time
from multiprocessing import Pool
from pathlib import Path

from hdl import (SETS, prime_of, shake, stream_job_data, select_xy,
                 ideal_from_classes, gram_norm, apply, nrd, in_span)
from fsr import lll_exact, zi_gauss_fixed
from run_fsr import (sk_of, msg_of, secret_class_var, n_bound, tmax,
                     write_csv)
from rr import (ACCEPT, SHAPES, keygen_reduce, sample_any, rerandomized_product,
                is_standard, map_back, canon)

SET_MAP = {n: (n, e, f) for n, e, f in SETS}
C_COEF = 1 << 8
DOWN_VARIANTS = (('prime', 'norm'), ('odd', 'norm'), ('odd', 'gauss'))


def key_material(set_name, sk, force_nb=None):
    _, e, f = SET_MAP[set_name]
    p = prime_of(e, f)
    N, xN, yN, nb = secret_class_var(sk, p, force_nb=force_nb)
    kr = keygen_reduce(N, xN, yN, p)
    feat = dict(N_bits=nb, N_top=N >> (nb - 8),
                shape=kr['nv'].bit_length() - kr['nu'].bit_length(),
                nu_rel=kr['nu'].bit_length() - nb - p.bit_length() // 2)
    return p, e, f, N, xN, yN, kr, feat


def job_samp(args):
    set_name, kid, nsig, shapes = args
    sk = shake(b'sk-rr-samp', set_name.encode(), kid.to_bytes(4, 'little'), n=32)
    p, e, f, N, xN, yN, kr, feat = key_material(set_name, sk)
    rows = []
    for m in range(nsig):
        seed = shake(b'rr-seed', sk, m.to_bytes(4, 'little'))
        row = dict(set=set_name, key=kid, sig=m, p_bits=p.bit_length(),
                   N_bound=n_bound(p), **feat)
        for acc in ACCEPT:
            for sh in shapes:
                t0 = time.perf_counter()
                r = sample_any(kr, N, p, seed, acc, sh, C_COEF, n_bound(p))
                pre = f'{acc}_{sh}'
                row[f'{pre}_time'] = round(time.perf_counter() - t0, 6)
                row[f'{pre}_tries'] = r['tries']
                row[f'{pre}_mr'] = r['mr']
                row[f'{pre}_hnf'] = r['hnf']
                row[f'{pre}_Np_bits'] = r['Np'].bit_length()
                row[f'{pre}_Np_top'] = r['Np'] >> (r['Np'].bit_length() - 8)
        rows.append(row)
    return rows


def reduce_both(B, T, p, bits_nom):
    G = gram_norm(B, p)
    t0 = time.perf_counter()
    H, d, lam, c = lll_exact(G)
    ex_time = time.perf_counter() - t0
    b1 = tuple(apply(H, B)[0])
    steps = tmax(bits_nom, 'zi')
    t0 = time.perf_counter()
    u, v, nu, nv, cz, eff = zi_gauss_fixed(T, B[2], p, steps)
    zi_time = time.perf_counter() - t0
    out = dict(ex_mul=c.mul, ex_div=c.div, ex_lovasz=c.lovasz, ex_swap=c.swap,
               ex_redi=c.redi, ex_time=round(ex_time, 5),
               zi_steps=steps, zi_mul=cz.mul, zi_trace=cz.trace.hexdigest()[:16],
               zi_time=round(zi_time, 5), zi_short=int(nu <= d[1]))
    return out, b1, tuple(u), nu, d[1]


def job_down(args):
    set_name, kid, mid, panel = args
    _, e, f = SET_MAP[set_name]
    force = None
    if panel == 'dudect_fixed':
        sk = shake(b'sk-rr-dudect-fixed', set_name.encode(), n=32)
        msg = msg_of(set_name, 10 ** 7, mid)
        force = 'B'
    elif panel == 'dudect_rand':
        sk = shake(b'sk-rr-dudect-rand', set_name.encode(), kid.to_bytes(4, 'little'), n=32)
        msg = msg_of(set_name, 10 ** 7, kid)
    else:
        sk = shake(b'sk-rr-pair', set_name.encode(), kid.to_bytes(4, 'little'), n=32)
        msg = msg_of(set_name, kid, mid)
    p0 = prime_of(e, f)
    p, e, f, N, xN, yN, kr, feat = key_material(
        set_name, sk, force_nb=n_bound(p0) if force == 'B' else None)
    data = stream_job_data(sk, msg, set_name, e, f)
    q = data['q']
    xq, yq, _ = select_xy(q, p, data['seed'])
    Bp, T = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
    bits_nom = 2 * e - 5 + n_bound(p)
    row = dict(set=set_name, panel=panel, key=kid, msg=mid, q_bits=q.bit_length(),
               N_bound=n_bound(p), **feat)
    full = panel == 'pair'
    o, b1_base, u_base, nu_base, d1_base = reduce_both(Bp, T, p, bits_nom)
    row.update({f'base_{k}': v for k, v in o.items()})
    for acc, sh in DOWN_VARIANTS:
        pre = f'{acc}_{sh}'
        salt = data['salt']
        seed = shake(b'rr-seed', sk, msg, salt)   # hedged model: H(sk, msg, salt[, rnd])
        t0 = time.perf_counter()
        r = sample_any(kr, N, p, seed, acc, sh, C_COEF, n_bound(p))
        Br = rerandomized_product(Bp, r['beta'], N, p)
        Tr = r['Np'] * q
        row[f'{pre}_samp_time'] = round(time.perf_counter() - t0, 5)
        row[f'{pre}_tries'] = r['tries']
        row[f'{pre}_mr'] = r['mr']
        row[f'{pre}_Np_bits'] = r['Np'].bit_length()
        std = is_standard(Br, Tr)
        row[f'{pre}_std'] = int(std)
        if not std:
            continue
        o, b1, u, nu, d1 = reduce_both(Br, Tr, p, bits_nom)
        row.update({f'{pre}_{k}': v for k, v in o.items()})
        # map back and compare with the baseline outputs
        ub = map_back(u, r['beta'], r['Np'], p)
        eb = map_back(b1, r['beta'], r['Np'], p)
        row[f'{pre}_zi_norm_ok'] = int(nu * N == nu_base * r['Np'])
        row[f'{pre}_zi_same'] = int(canon(ub) == canon(u_base))
        row[f'{pre}_ex_same'] = int(canon(eb) == canon(b1_base))
        row[f'{pre}_ex_b1_ratio_ok'] = int(d1 * N == d1_base * r['Np'])
        if full:
            row[f'{pre}_mapped_in'] = int(in_span(list(ub), Bp) and in_span(list(eb), Bp))
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', nargs='+', default=[s[0] for s in SETS])
    ap.add_argument('--panels', nargs='+', default=['samp', 'pair', 'dudect_fixed', 'dudect_rand'])
    ap.add_argument('--samp-keys', type=int, default=1000)
    ap.add_argument('--samp-sigs', type=int, default=20)
    ap.add_argument('--pair-keys', type=int, default=200)
    ap.add_argument('--pair-msgs', type=int, default=6)
    ap.add_argument('--dudect', type=int, default=1500)
    ap.add_argument('--procs', type=int, default=2)
    ap.add_argument('--out', default='results/raw')
    a = ap.parse_args()
    out = Path(a.out)
    with Pool(a.procs) as pool:
        for panel in a.panels:
            for set_name in a.sets:
                if panel in ('samp', 'sampg'):
                    blk = 250
                    shapes = SHAPES if panel == 'samp' else ('gauss',)
                    for k0 in range(0, a.samp_keys, blk):
                        path = out / f'{panel}_{set_name}_k{k0:04d}.csv'
                        if path.exists():
                            continue
                        t0 = time.time()
                        jobs = [(set_name, k, a.samp_sigs, shapes) for k in range(k0, min(a.samp_keys, k0 + blk))]
                        rows = [r for rs in pool.map(job_samp, jobs, chunksize=5) for r in rs]
                        write_csv(path, rows)
                        print(f'wrote {path} ({len(rows)}) {time.time() - t0:.0f}s', flush=True)
                    continue
                if panel == 'pair':
                    nk, nm, blk = a.pair_keys, a.pair_msgs, 50
                elif panel == 'dudect_fixed':
                    nk, nm, blk = 1, a.dudect, 1
                else:
                    nk, nm, blk = a.dudect, 1, 500
                for k0 in range(0, nk, blk):
                    path = out / f'{panel}_{set_name}_k{k0:04d}.csv'
                    if path.exists():
                        continue
                    t0 = time.time()
                    jobs = [(set_name, k, m, panel) for k in range(k0, min(nk, k0 + blk))
                            for m in range(nm)]
                    rows = pool.map(job_down, jobs, chunksize=4)
                    write_csv(path, rows)
                    print(f'wrote {path} ({len(rows)}) {time.time() - t0:.0f}s', flush=True)
    print('ALL_DONE', flush=True)


if __name__ == '__main__':
    main()
