# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Run the 4-D embeddings on PRISM target streams at all five repo sets.

Streams per set (p = f*2^e - 1 from prism-field):
  T1  challenge degrees q = 2^(2e-5) - s^2, s odd with e-4 free bits, q prime
      (q = 7 mod 8: no x^2 + y^2 = q, so classical A runs RepresentInteger
      style: M = q - p(z^2 + w^2) for (z, w) from a public PRNG keyed by q,
      retried until M is a prime = 1 mod 4, then Cornacchia on M)
  T2  primes N = 1 mod 4 with bit length uniform in [bits(p) - 16, bits(p)]
      (norm-equation cofactor scale); A is Cornacchia on N directly
Each target is run through D0 and H1, H2, H3, H3s with (a) the fixed seed,
(b) one fresh simulated public key, (c) BLAKE3 instead of SHAKE-256 (fixed
seed, H1 and H2). Panels: K targets of each stream x D simulated keys.
"""
import argparse, csv, hashlib, os, secrets, sys, time
from multiprocessing import Pool
import gmpy2

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'cornacchia-harness'))
from cornacchia_variants import cornacchia_classical
from lp4d import *

PK_BYTES = {'p248': 64, 'p324': 82, 'p376': 96, 'p500': 128, 'p664': 168}
SMALL_Q = 3329


def t1_target(e):
    while True:
        s = 2 * secrets.randbits(e - 4) + 1
        q = (1 << (2 * e - 5)) - s * s
        if gmpy2.is_prime(q):
            return q


def t2_target(pbits):
    b = pbits - secrets.randbelow(17)
    while True:
        m = secrets.randbits(b) | (1 << (b - 1))
        m = m - (m % 4) + 1
        if m.bit_length() == b and gmpy2.is_prime(m):
            return m


def classical_A(T, p):
    """Returns (cornacchia_steps, retries, sqrt_attempts)."""
    if T % 4 == 1:
        r = sqrt_mod(T - 1, T); u = min(r, T - r)
        sol, tr = cornacchia_classical(T, u)
        assert sol
        return tr['steps'], 0
    # RepresentInteger style for q = 3 mod 4
    bound = int(gmpy2.isqrt(T // (2 * p)))
    ctr = 0
    while True:
        z, w = [x % bound for x in xof_ints(b'A-classical', T, b'zw' + ctr.to_bytes(4, 'little'), 2, bound.bit_length() + 64)]
        ctr += 1
        M = T - p * (z * z + w * w)
        if M % 4 == 1 and gmpy2.is_prime(M):
            r = sqrt_mod(M - 1, M); u = min(r, M - r)
            sol, tr = cornacchia_classical(M, u)
            assert sol
            return tr['steps'], ctr


def run_variant(kind, T, p, seed, xk='shake', B0=None):
    if kind == 'D0':
        B, info = embed_D0(T, p)
    elif kind == 'H1':
        B, info = embed_H1(T, p, seed, kind=xk, B0=B0)
    elif kind == 'H2':
        B, info = embed_H2(T, p, seed, kind=xk)
    elif kind == 'H3':
        B, info = embed_H3(T, seed, kind=xk)
    elif kind == 'H3s':
        B, info = embed_H3(T, seed, kind=xk, mod=SMALL_Q)
    G = gram_norm(B, p) if kind in ('D0', 'H1', 'H2') else gram_eucl(B)
    H, tr = lll_gram(G)
    R = apply(H, B)
    if kind in ('D0', 'H1', 'H2'):
        n0 = nrd(R[0], p)
        tr['ok'] = int(n0 % T == 0 and n0 > 0)
        tr['ratio_bits'] = (n0 // T).bit_length()
    else:
        tr['ok'] = -1; tr['ratio_bits'] = -1
    tr['alpha_tries'] = info.get('alpha_tries', 0)
    tr['basis_maxbits'] = max(abs(x).bit_length() for row in B for x in row)
    return tr


VARS = ['D0', 'H1', 'H2', 'H3', 'H3s']


def stream_row(args):
    name, e, f, stream = args
    p = prime_of(e, f)
    T = t1_target(e) if stream == 'T1' else t2_target(p.bit_length())
    row = dict(set=name, stream=stream, T_bits=T.bit_length(), T_hw=gmpy2.popcount(T),
               T_mod3=T % 3, T_mod5=T % 5, T_mod8=T % 8, T_mod16=T % 16)
    t = time.perf_counter(); a_steps, a_retries = classical_A(T, p)
    row.update(A_steps=a_steps, A_retries=a_retries)
    B0, _ = embed_D0(T, p)
    pk_seed = seed_from_pk(secrets.token_bytes(PK_BYTES[name]))
    for v in VARS:
        for tag, seed, xk in [('fix', FIXED_SEED, 'shake'), ('pk', pk_seed, 'shake'), ('b3', FIXED_SEED, 'blake3')]:
            if v == 'D0' and tag != 'fix':
                continue
            if tag == 'b3' and v not in ('H1', 'H2'):
                continue
            tr = run_variant(v, T, p, seed, xk, B0=B0)
            col = v if v == 'D0' else f'{v}_{tag}'
            for k_ in ('iterations', 'swaps', 'sizered', 'lovasz', 'ok', 'ratio_bits', 'alpha_tries', 'basis_maxbits'):
                row[f'{col}_{k_}'] = tr[k_]
    # determinism check: recomputing H1 with the same public seed gives the same trace
    row['H1_pk_recompute_equal'] = int(run_variant('H1', T, p, pk_seed, B0=B0)['iterations'] == row['H1_pk_iterations'])
    return row


def tours_row(args):
    name, e, f = args
    p = prime_of(e, f)
    T = t1_target(e)
    B, _ = embed_D0(T, p)
    G, tours = fixed_schedule(gram_norm(B, p))
    _, tr = lll_gram(gram_norm(B, p))
    return dict(set=name, T_bits=T.bit_length(), tours_needed=tours, D0_iterations=tr['iterations'])


def panel_target(args):
    name, e, f, stream, k, draws = args
    p = prime_of(e, f)
    T = t1_target(e) if stream == 'T1' else t2_target(p.bit_length())
    a_steps, a_ret = classical_A(T, p)
    B0, _ = embed_D0(T, p)
    _, d0 = lll_gram(gram_norm(B0, p))
    rows = []
    for dr in range(draws):
        seed = seed_from_pk(secrets.token_bytes(PK_BYTES[name]))
        r = dict(set=name, stream=stream, t_id=k, draw=dr, T_bits=T.bit_length(),
                 A_steps=a_steps, A_retries=a_ret, D0_iterations=d0['iterations'])
        for v in ['H1', 'H2', 'H3', 'H3s']:
            tr = run_variant(v, T, p, seed, B0=B0)
            r[f'{v}_iterations'] = tr['iterations']; r[f'{v}_swaps'] = tr['swaps']
        rows.append(r)
    return rows


def write(path, rows):
    with open(path, 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', nargs='+', default=[s[0] for s in SETS])
    ap.add_argument('--n', type=int, default=400)
    ap.add_argument('--tours', type=int, default=200)
    ap.add_argument('--panel', type=int, default=20)
    ap.add_argument('--draws', type=int, default=120)
    ap.add_argument('--procs', type=int, default=7)
    ap.add_argument('--out', default='results/raw')
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    with Pool(a.procs) as pool:
        for name, e, f in [s for s in SETS if s[0] in a.sets]:
            t0 = time.time()
            for st in ('T1', 'T2'):
                rows = pool.map(stream_row, [(name, e, f, st)] * a.n, chunksize=4)
                write(f'{a.out}/stream_{name}_{st}.csv', rows)
                prow = pool.map(panel_target, [(name, e, f, st, k, a.draws) for k in range(a.panel)])
                write(f'{a.out}/panel_{name}_{st}.csv', [r for rr in prow for r in rr])
            write(f'{a.out}/tours_{name}.csv', pool.map(tours_row, [(name, e, f)] * a.tours, chunksize=4))
            print(f'{name} done in {time.time() - t0:.0f}s', flush=True)
