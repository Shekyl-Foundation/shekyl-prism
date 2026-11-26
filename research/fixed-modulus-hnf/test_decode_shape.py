# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""The decode generator shape itself: the Gaussian pair (u, v) as the four rows u, Ju, v, Jv.

Generic ideals (prime, composite, prime-power norms near 2^B): U = (2N,0,0,0), V = (x,y,1,0), then the
same pair after exact Z[i]-Lagrange reduction (what a pre-reduced stored key holds), and after random
Z[i] shears.  Every set.  Compared with Sage and with the ideal's own HNF.
"""
import random, sys
import lib, gen_ideals
from fmhnf import fixed_modulus_hnf
from sets import classes


def J(v):
    return [-v[1], v[0], -v[3], v[2]]


def Bform(u, v, p):
    return u[0] * v[0] + u[1] * v[1] + p * (u[2] * v[2] + u[3] * v[3])


def gmul(g, w):
    return [g[0] * a + g[1] * b for a, b in zip(w, J(w))] if False else [g[0] * w[i] + g[1] * J(w)[i] for i in range(4)]


def rdiv(a, b):                    # round half up, b > 0
    return (2 * a + b) // (2 * b)


def lagrange(u, v, p):
    while True:
        if Bform(v, v, p) < Bform(u, u, p):
            u, v = v, u
        A = Bform(u, u, p)
        g = (rdiv(Bform(u, v, p), A), rdiv(Bform(J(u), v, p), A))
        w = [v[i] - gmul(g, u)[i] for i in range(4)]
        if Bform(w, w, p) >= Bform(v, v, p):
            return u, w
        v = w


def main():
    rng = random.Random(5)
    tot = 0
    for name in ('P248', 'P324', 'P376', 'P500', 'P664'):
        P, cl = classes(name)
        p, B, K = P['p'], P['B'], cl['ideal']['K']
        gen_ideals.PRIME[0] = p
        top = 1 << B
        cases = []
        from sage.all import random_prime
        for _ in range(4):
            l = int(random_prime(top - 1, lbound=top >> 1))
            cases.append((l, gen_ideals.root_mod_prime_power(rng, p, l, 1)))
        ls = [int(random_prime((1 << (B // 3)) - 1, lbound=1 << (B // 3 - 1))) for _ in range(3)]
        cases.append((ls[0] * ls[1] * ls[2], gen_ideals.root_composite(rng, p, [(l, 1) for l in ls])))
        cases.append((3 ** 5, gen_ideals.root_composite(rng, p, [(3, 5)])))
        cases.append((1, (0, 1)))
        for N, (x, y) in cases:
            x, y = gen_ideals.lift_parity(x, y, N) if N > 1 else (0, 1)
            gens = lib.left_ideal_gens([[2 * N, 0, 0, 0], [x, y, 1, 0]], p)
            cols = lib.sage_hnf_cols(gens)
            assert lib.ideal_norm_from_cols(cols) == N
            m = 2 * N
            U, V = [2 * N, 0, 0, 0], [x, y, 1, 0]
            pairs = [('HNF generators', U, V)]
            ur, vr = lagrange(U, V, p)
            pairs.append(('Lagrange-reduced', ur, vr))
            for s in range(3):
                g = (rng.randrange(-(1 << 40), 1 << 40), rng.randrange(-(1 << 40), 1 << 40))
                w = [vr[i] - gmul(g, ur)[i] for i in range(4)]
                pairs.append(('sheared v', ur, w))
            for lbl, u, v in pairs:
                G = [u, J(u), v, J(v)]
                P_, ctx = fixed_modulus_hnf(G, m, K)
                assert P_ == cols == lib.sage_hnf_cols(G) == lib.pari_hnf_cols(G), (name, N, lbl)
                tot += 1
        print('%s: decode-shape (u,Ju,v,Jv) equal to Sage/PARI/ideal HNF for %d ideals x %d pairs' % (name, len(cases), 5))
        sys.stdout.flush()
    print('total %d runs' % tot)


if __name__ == '__main__':
    main()
