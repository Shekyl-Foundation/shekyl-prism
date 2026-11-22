# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Key-identity panel: does the trace depend on a SECRET ideal?

In PRISM the challenge degree q is public; what a timing adversary wants is
the secret. Model: a secret left ideal of prime norm N (N of about bits(p)/2
bits; its identity is the class of (x, y) mod N with x^2 + y^2 = -p mod N),
and a public challenge q. The signing-side lattice is the left ideal of norm
N*q, alpha = x + y i + j with (x, y) fixed mod N by the secret and mod q by
either the deterministic rule (D0) or the hash rule (H2, seed = H(pk)),
joined by CRT. One key = (N, secret class, pk); one draw = one signature
with a fresh public q. eta^2 across keys measures how much the key
determines the trace."""
import argparse, csv, os, secrets, sys
from multiprocessing import Pool
import gmpy2
from lp4d import *
from run_4d import t1_target, PK_BYTES, write


def crt(a, m, b, n):
    return int((a + m * ((b - a) * int(gmpy2.invert(m, n)) % n)) % (m * n))


def alpha_mod_q(q, p, seed, mode):
    if mode == 'D0':
        y = 0
        while True:
            x = alpha_for(q, p, y)
            if x is not None:
                return min(x, q - x), y
            y += 1
    ctr = 0
    while True:
        y, sgn = xof_ints(seed, q, b'H2' + ctr.to_bytes(4, 'little'), 2, q.bit_length() + 64)
        y %= q; ctr += 1
        x = alpha_for(q, p, y)
        if x is not None:
            return (q - x if sgn & 1 else x), y


def key_panel(args):
    name, e, f, kid, draws = args
    p = prime_of(e, f)
    nb = p.bit_length() // 2
    while True:
        N = secrets.randbits(nb) | (1 << (nb - 1))
        if gmpy2.is_prime(N):
            break
    while True:  # secret class of (x, y) mod N
        yN = secrets.randbelow(N)
        xN = alpha_for(N, p, yN)
        if xN is not None:
            break
    seed = seed_from_pk(secrets.token_bytes(PK_BYTES[name]))
    rows = []
    for dr in range(draws):
        q = t1_target(e)
        r = dict(set=name, key=kid, draw=dr, N_bits=N.bit_length())
        for mode in ('D0', 'H2'):
            xq, yq = alpha_mod_q(q, p, seed, mode)
            x, y = crt(xN, N, xq, q), crt(yN, N, yq, q)
            T = N * q
            B = ideal_basis(T, p, (x, y, 1, 0))
            H, tr = lll_gram(gram_norm(B, p))
            beta = apply(H, B)[0]
            ok = nrd(beta, p) % T == 0
            r[f'{mode}_iterations'] = tr['iterations']; r[f'{mode}_swaps'] = tr['swaps']; r[f'{mode}_ok'] = int(ok)
        rows.append(r)
    return rows


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--keys', type=int, default=20)
    ap.add_argument('--draws', type=int, default=100)
    ap.add_argument('--out', default='results/raw')
    a = ap.parse_args()
    with Pool(7) as pool:
        for name, e, f in SETS:
            rows = pool.map(key_panel, [(name, e, f, k, a.draws) for k in range(a.keys)])
            write(f'{a.out}/keys_{name}.csv', [r for rr in rows for r in rr])
            print(name, 'done', flush=True)
