# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Measurement driver for the norm-equation experiment.

Panels (per set), each written as chunked CSVs under data/<set>/ and
skipped when present (resumable):

  keys     K keys x M messages. Per key: keygen's prime-norm search
           (draws, N) and keygen's qlapoti on I_sec. Per signature: the
           reference qlapoti ('ref') and the fixed-order 'fresh' policy on
           the prepared small ideal of I_sec cap I_q, with independent tapes.
  fixkey   one key, many messages (dudect "fixed" class for key dependence).
  fixS     a few fixed small ideals, many tapes (dudect "fixed" class for
           dependence on the secret-derived ideal itself).
  rnd      fresh key and message per row (dudect "random" class).
  equiv    class invariance: the same L and an equivalent L' = L*(-conj d)/n(L)
           of a different norm; does the prepared small ideal coincide?

Keys are simulated: the secret ideal follows the reference's key generation
(random ideal of norm SEC_DEGREE, then the prime-norm reduced equivalent),
from a seeded tape. The challenge q = 2^(2e-5) - s^2 is drawn from a hash
of (key, message); the response ideal of norm q is uniform among those
reachable by alpha = x + y i + j.
"""
import argparse
import csv
import os
import time
import multiprocessing as mp
from pathlib import Path

from fne import (SET_MAP, prime_of, sec_degree, tape, random_prime_ideal,
                 prime_norm_equivalent, intersect_coprime, prepare_small,
                 qlapoti, Ctr, work_units, lll_cols, equivalent_by,
                 nrd_num, gmpy2)

OUT = Path(__file__).resolve().parent / 'data'
SIG_FIELDS = ['passes', 'gens', 'gen_draws', 'lam_steps', 'lat_builds',
              'mod_ok', 'trial_divs', 'pt', 'pt_bits', 'pt_pass', 'post',
              'resets', 'm_neg', 'success_pass', 'transformed', 'mx_gen',
              'mx_lat', 'mx_corn', 'mx_tonelli']


def keygen(name, kid, p, SEC):
    rng = tape(b'keygen', name, kid)
    I0 = random_prime_ideal(SEC, p, rng)
    Is, info = prime_norm_equivalent(I0, p, rng)
    return Is, info


def challenge(name, e, kid, mid, p):
    rng = tape(b'chal', name, kid, mid)
    while True:
        s = rng.getrandbits(e - 4) | 1
        q = (1 << (2 * e - 5)) - s * s
        if gmpy2.is_prime(q):
            return q, rng


def small_bits_info(S):
    n = S.norm
    return dict(n_bits=int(n).bit_length(), n_v2=int(gmpy2.bit_scan1(n)),
                n_mod3=int(n % 3), n_den=int(S.den))


def run_policy(S, p, e, tp, pol):
    kw = {} if pol == 'ref' else dict(policy='fresh')
    c = Ctr()
    t0 = time.perf_counter()
    qlapoti(S, p, e - 2, tp, c, **kw)
    dt = time.perf_counter() - t0
    d = c.asdict()
    w = work_units(d, int(S.norm).bit_length() + e)
    row = {f'{pol}_{k}': d[k] for k in SIG_FIELDS}
    row[f'{pol}_work'] = round(w)
    row[f'{pol}_sec'] = round(dt, 5)
    return row


def sign_row(name, e, p, Is, N, kid, mid, tag, policies=('ref', 'fresh')):
    q, rng = challenge(name, e, kid, mid, p)
    Iq = random_prime_ideal(q, p, rng)
    L = intersect_coprime(Is, Iq)
    S = prepare_small(L, p)
    row = dict(set=name, kid=kid, mid=mid, N_bits=int(N).bit_length())
    row.update(small_bits_info(S))
    for pol in policies:
        row.update(run_policy(S, p, e, tape(b'ne', tag, name, kid, mid, pol), pol))
    return row, S


def write(path, rows):
    tmp = path.with_suffix('.tmp')
    with open(tmp, 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    os.replace(tmp, path)


def job_keys(args):
    name, k0, k1, M = args
    path = OUT / name / f'keys_{k0:05d}_{k1:05d}.csv'
    if path.exists():
        return str(path), 'skip'
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    SEC = sec_degree(name)
    rows = []
    for kid in range(k0, k1):
        Is, info = keygen(name, kid, p, SEC)
        N = Is.norm
        kg = dict(kg_draws=info['draws'], kg_pt_bits=info['pt_bits'])
        Sk = prepare_small(Is, p)
        kg.update({('kgq_' + k): v for k, v in
                   run_policy(Sk, p, e, tape(b'kgq', name, kid), 'ref').items()})
        kg['kg_n_bits'] = int(Sk.norm).bit_length()
        for mid in range(M):
            row, _ = sign_row(name, e, p, Is, N, kid, mid, b'keys')
            row.update(kg)
            rows.append(row)
    write(path, rows)
    return str(path), len(rows)


def job_fixkey(args):
    name, m0, m1 = args
    path = OUT / name / f'fixkey_{m0:05d}_{m1:05d}.csv'
    if path.exists():
        return str(path), 'skip'
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    Is, _ = keygen(name, 10 ** 6, p, sec_degree(name))
    rows = []
    for mid in range(m0, m1):
        row, _ = sign_row(name, e, p, Is, Is.norm, 10 ** 6, mid, b'fixkey', ('ref',))
        rows.append(row)
    write(path, rows)
    return str(path), len(rows)


def job_rnd(args):
    name, r0, r1 = args
    path = OUT / name / f'rnd_{r0:05d}_{r1:05d}.csv'
    if path.exists():
        return str(path), 'skip'
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    SEC = sec_degree(name)
    rows = []
    for r in range(r0, r1):
        kid = 2 * 10 ** 6 + r
        Is, _ = keygen(name, kid, p, SEC)
        row, _ = sign_row(name, e, p, Is, Is.norm, kid, 0, b'rnd', ('ref',))
        rows.append(row)
    write(path, rows)
    return str(path), len(rows)


def job_fixS(args):
    name, cls, t0, t1 = args
    path = OUT / name / f'fixS_{cls:02d}_{t0:05d}_{t1:05d}.csv'
    if path.exists():
        return str(path), 'skip'
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    kid = 3 * 10 ** 6 + cls
    Is, _ = keygen(name, kid, p, sec_degree(name))
    q, rng = challenge(name, e, kid, 0, p)
    S = prepare_small(intersect_coprime(Is, random_prime_ideal(q, p, rng)), p)
    rows = []
    for t in range(t0, t1):
        row = dict(set=name, cls=cls, t=t, N_bits=int(Is.norm).bit_length())
        row.update(small_bits_info(S))
        row.update(run_policy(S, p, e, tape(b'fixS', name, cls, t, 'ref'), 'ref'))
        row.update(run_policy(S, p, e, tape(b'fixS', name, cls, t, 'fresh'), 'fresh'))
        rows.append(row)
    write(path, rows)
    return str(path), len(rows)


def job_equiv(args):
    name, j0, j1 = args
    path = OUT / name / f'equiv_{j0:05d}_{j1:05d}.csv'
    if path.exists():
        return str(path), 'skip'
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    SEC = sec_degree(name)
    rows = []
    for j in range(j0, j1):
        kid = 4 * 10 ** 6 + j
        Is, _ = keygen(name, kid, p, SEC)
        q, rng = challenge(name, e, kid, 0, p)
        L = intersect_coprime(Is, random_prime_ideal(q, p, rng))
        S = prepare_small(L, p)
        red = lll_cols(L, p)
        for box in (1, 4, 64, 2 ** 12):
            while True:
                x = [rng.randint(-box, box) for _ in range(4)]
                if any(x):
                    break
            d = tuple(sum(x[k] * red[k][t] for k in range(4)) for t in range(4))
            L2 = equivalent_by(L, d, p)
            S2 = prepare_small(L2, p)
            same = (S2.den == S.den and S2.norm == S.norm and
                    sorted(S2.cols) == sorted(S.cols))
            c1, c2 = Ctr(), Ctr()
            same_lat = same or (S2.norm == S.norm and S2.den == S.den and
                                lattice_equal(S, S2))
            qlapoti(S, p, e - 2, tape(b'eq', name, j), c1)
            qlapoti(S2, p, e - 2, tape(b'eq', name, j), c2)
            rows.append(dict(set=name, j=j, box=box,
                             L_norm_bits=int(L.norm).bit_length(),
                             L2_norm_bits=int(L2.norm).bit_length(),
                             n_bits=int(S.norm).bit_length(),
                             n2_bits=int(S2.norm).bit_length(),
                             same_norm=int(S2.norm == S.norm),
                             same_ideal=int(same_lat),
                             same_basis=int(same),
                             passes1=c1.passes, passes2=c2.passes,
                             same_trace=int(c1.asdict() == c2.asdict())))
    write(path, rows)
    return str(path), len(rows)


def job_pS(args):
    """Per-ideal success rate: R independent fixed-order searches per ideal."""
    name, j0, j1, R = args
    path = OUT / name / f'pS_{j0:05d}_{j1:05d}.csv'
    if path.exists():
        return str(path), 'skip'
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    SEC = sec_degree(name)
    rows = []
    for j in range(j0, j1):
        kid = 7 * 10 ** 6 + j
        Is, _ = keygen(name, kid, p, SEC)
        q, rng = challenge(name, e, kid, 0, p)
        S = prepare_small(intersect_coprime(Is, random_prime_ideal(q, p, rng)), p)
        tot = m4 = cv = gen_dr = gen_ct = 0
        for t in range(R):
            c = Ctr()
            qlapoti(S, p, e - 2, tape(b'pS', name, j, t), c, policy='fresh')
            tot += c.passes
            m4 += c.m4pos
            cv += c.cvp
            gen_dr += c.gen_draws
            gen_ct += c.gens
        row = dict(set=name, j=j, R=R, passes=tot, p_hat=R / tot, m4pos_share=m4 / max(cv, 1),
                   gen_valid=gen_ct / gen_dr, N_bits=int(Is.norm).bit_length())
        row.update(small_bits_info(S))
        rows.append(row)
    write(path, rows)
    return str(path), len(rows)


def lattice_equal(S, T):
    from fne import hnf
    a = hnf(S.cols, S.den)
    b = hnf(T.cols, T.den)
    return a.den == b.den and a.cols == b.cols


def plan(name, K, M, FK, RN, FS_CLS, FS_T, EQ):
    jobs = []
    for k0 in range(0, K, 20):
        jobs.append((job_keys, (name, k0, min(K, k0 + 20), M)))
    for m0 in range(0, FK, 250):
        jobs.append((job_fixkey, (name, m0, min(FK, m0 + 250))))
    for r0 in range(0, RN, 250):
        jobs.append((job_rnd, (name, r0, min(RN, r0 + 250))))
    for cls in range(FS_CLS):
        for t0 in range(0, FS_T, 250):
            jobs.append((job_fixS, (name, cls, t0, min(FS_T, t0 + 250))))
    for j0 in range(0, EQ, 25):
        jobs.append((job_equiv, (name, j0, min(EQ, j0 + 25))))
    return jobs


def _run(job):
    fn, a = job
    t0 = time.time()
    r = fn(a)
    return r, round(time.time() - t0, 1)


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', default='p248,p324,p376,p500,p664')
    ap.add_argument('--K', type=int, default=400)
    ap.add_argument('--M', type=int, default=10)
    ap.add_argument('--fixkey', type=int, default=2000)
    ap.add_argument('--rnd', type=int, default=2000)
    ap.add_argument('--fixS_cls', type=int, default=6)
    ap.add_argument('--fixS_T', type=int, default=1000)
    ap.add_argument('--equiv', type=int, default=50)
    ap.add_argument('--workers', type=int, default=2)
    ap.add_argument('--pS_only', type=int, default=0, help='number of ideals for the per-ideal panel only')
    ap.add_argument('--pS_R', type=int, default=30)
    a = ap.parse_args()
    jobs = []
    for name in a.sets.split(','):
        (OUT / name).mkdir(parents=True, exist_ok=True)
        if a.pS_only:
            jobs += [(job_pS, (name, j0, min(a.pS_only, j0 + 10), a.pS_R))
                     for j0 in range(0, a.pS_only, 10)]
        else:
            jobs += plan(name, a.K, a.M, a.fixkey, a.rnd, a.fixS_cls, a.fixS_T, a.equiv)
    print('jobs', len(jobs), flush=True)
    with mp.get_context('spawn').Pool(a.workers) as pool:
        for r, dt in pool.imap_unordered(_run, jobs):
            print(time.strftime('%H:%M:%S'), r, dt, flush=True)
    print('DONE', flush=True)
