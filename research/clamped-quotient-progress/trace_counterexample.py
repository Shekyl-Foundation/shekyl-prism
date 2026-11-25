# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Step-by-step trace of the Q5 counterexample (crafted_saturation.py) under
the Rust prototype's clamped 64-bit quotient and under exact quotients.

For every step: bits of A = N4(u) and C = N4(v), the exact Z[i] quotient
q = h/A (log2 of its larger component), the m actually used, whether it
clamped, whether the step swapped, and the lagrange-tour-bound potential
Phi = A (1 + 2|eps|^2) with eps the exact rounding residual of q.
Writes results/counterexample_trace_<set>.csv and results/counterexample_summary.txt.
"""
import csv
import math
import os
import random
from fractions import Fraction

import gmpy2

from q5common import SETS, params, N4, herm
from instrumented import quotient_rust, exact_round
import gen_inputs as G

HERE = os.path.dirname(os.path.abspath(__file__))


def crafted_k1(P, seed=78):
    """k = 1: T = a^2 + b^2 + p prime at the signing size, so beta = a + b i + j
    has nrd(beta) = T = lambda_1 and the first exact quotient is about
    sqrt(T), the largest a valid ideal of this size allows."""
    p = P['p']; rng = random.Random(seed)
    hb = (P['Tbits'] - 1) // 2
    while True:
        a = rng.randrange(1 << (hb - 1), 1 << hb); b = rng.randrange(1 << (hb - 1), 1 << hb)
        T = a * a + b * b + p
        if T.bit_length() <= P['Tbits'] and gmpy2.is_prime(T):
            return T, 1, a, b


def crafted(P, seed=77, kbits=None):
    """The crafted_saturation.py ideal (same seed, same construction)."""
    p = P['p']; rng = random.Random(seed)
    T = int(gmpy2.next_prime(rng.randrange(1 << (P['Tbits'] - 1), 1 << P['Tbits'])))
    kb = P['L'] // 2 + 20 if kbits is None else kbits
    while True:
        k = rng.randrange(1 << (kb - 1), 1 << kb)
        r = k * T - p
        if r % 4 == 1 and gmpy2.is_prime(r):
            break
    a, b = G.two_squares_prime(r)
    x, y = a % T, b % T
    assert (x * x + y * y + p) % T == 0
    return T, k, x, y


def log2abs(x):
    x = abs(x)
    if x == 0:
        return float('-inf')
    return math.log2(x) if x.bit_length() < 1000 else x.bit_length() - 1 + math.log2(x / (1 << (x.bit_length() - 1))) if False else math.log2(float(x >> max(0, x.bit_length() - 60))) + max(0, x.bit_length() - 60)


def qlog(h, A):
    """log2 of max(|h0|,|h1|)/A."""
    return log2abs(max(abs(h[0]), abs(h[1]))) - log2abs(A)


def run(u0, v0, p, steps, mode):
    u = list(u0); v = list(v0)
    nu, nv = N4(u, p), N4(v, p)
    if nv < nu:
        u, v, nu, nv = v, u, nv, nu
    rows = []
    for t in range(steps):
        h = herm(v, u, p)
        me = [exact_round(h[0], nu), exact_round(h[1], nu)]
        if mode == 'clamp64':
            m0, s0 = quotient_rust(h[0], nu); m1, s1 = quotient_rust(h[1], nu)
            m = [m0, m1]; sat = s0 or s1
        else:
            m = me; sat = False
        eps2 = (Fraction(h[0], nu) - me[0]) ** 2 + (Fraction(h[1], nu) - me[1]) ** 2
        phi_bits = log2abs(nu) + math.log2(1 + 2 * float(eps2))
        iu = [-u[1], u[0], -u[3], u[2]]
        v = [v[r] - m[0] * u[r] - m[1] * iu[r] for r in range(4)]
        nv = N4(v, p)
        sw = nv < nu
        if sw:
            u, v, nu, nv = v, u, nv, nu
        rows.append(dict(step=t + 1, bitsA=nu.bit_length() if not sw else v and N4(v, p).bit_length(),
                         log2q=round(qlog(h, nu if not sw else N4(v, p)), 3),
                         m0=m[0], m1=m[1], exact_m_bits=max(abs(me[0]), abs(me[1])).bit_length(),
                         clamped=int(sat), swapped=int(sw), phi_bits=round(phi_bits, 4),
                         bitsA_after=nu.bit_length(), bitsC_after=nv.bit_length()))
        if mode == 'exact' and m == [0, 0] and not sw:
            break
    return u, v, nu, nv, rows


def main():
    out = []
    for s in SETS:
        P = params(s[0]); p = P['p']
        T, k, x, y = crafted(P)
        u0, v0 = G.model_basis(T, x, y)
        _, _, lam1, _, st = G.exact_lagrange(u0, v0, p)
        S = P['tprov_sec']
        u, v, nu, nv, rows = run(u0, v0, p, S, 'clamp64')
        ue, ve, nue, nve, rows_e = run(u0, v0, p, S, 'exact')
        with open(os.path.join(HERE, 'results', f'counterexample_trace_{P["name"]}.csv'), 'w', newline='') as f:
            w = csv.DictWriter(f, fieldnames=list(rows[0].keys()) + ['mode'])
            w.writeheader()
            for r in rows_e:
                w.writerow({**r, 'mode': 'exact'})
            for r in rows:
                w.writerow({**r, 'mode': 'clamp64'})
        nclamp = sum(r['clamped'] for r in rows)
        first_clamp = next((r['step'] for r in rows if r['clamped']), None)
        nsw = sum(r['swapped'] for r in rows)
        big = max(r['exact_m_bits'] for r in rows_e)
        # the exact quotient the clamp faces at its first clamped step
        qbits_first = next((r['log2q'] for r in rows if r['clamped']), None)
        need = None
        if qbits_first is not None:
            need = 2 ** (qbits_first - 62)
        line = (f"{P['name']}: T {T.bit_length()} b, k {k.bit_length()} b; exact: {st} steps to the fixed point, "
                f"largest exact quotient {big} bits; clamp64 ({S} steps): first clamp at step {first_clamp}, "
                f"clamped steps {nclamp}, swaps {nsw} (exact run swaps {sum(r['swapped'] for r in rows_e)}), "
                f"log2|q| at first clamp {qbits_first}, so ~{need:.3g} clamped steps would be needed; "
                f"end: bits(nrd u) {nu.bit_length()} vs lambda_1 {lam1.bit_length()}, reduced {nu == lam1}; "
                f"Phi over clamped steps: first {rows[first_clamp-1]['phi_bits'] if first_clamp else None} "
                f"last {rows[-1]['phi_bits']} bits")
        print(line); out.append(line)
    open(os.path.join(HERE, 'results', 'counterexample_summary.txt'), 'w').write('\n'.join(out) + '\n')


if __name__ == '__main__':
    main()
