# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Left O0-ideals of every shape the HNF must handle, at a given set (p, B)."""
import random
from math import isqrt
from sage.all import Zmod, crt, is_prime, next_prime, random_prime, ZZ
import lib


def lift_parity(x, y, N):
    """Shift the root (x, y) mod N (N odd) to x even, y odd in [0, 2N)."""
    x %= N
    y %= N
    if x % 2:
        x += N
    if y % 2 == 0:
        y += N
    return x, y


def root_mod_prime_power(rng, p, l, e):
    """(x, y) with x^2 + y^2 + p = 0 mod l^e (l odd prime)."""
    M = l ** e
    Z = Zmod(M)
    for _ in range(200):
        y = rng.randrange(M)
        t = Z(-p - y * y)
        if t == 0 or t.is_square():
            try:
                return int(t.sqrt()), y
            except Exception:
                continue
    raise RuntimeError('no root')


def root_composite(rng, p, fac):
    """fac = [(l, e)] odd primes: root mod prod l^e by CRT."""
    xs, ys, ms = [], [], []
    for l, e in fac:
        x, y = root_mod_prime_power(rng, p, l, e)
        xs.append(x); ys.append(y); ms.append(l ** e)
    return int(crt(xs, ms)), int(crt(ys, ms))


def ideal_from_root(N, x, y):
    """HNF generators of O0*N + O0*(x + y i + j)/2, x even y odd, via the oracle."""
    x, y = lift_parity(x, y, N)
    gens = [[2 * N, 0, 0, 0], [x, y, 1, 0]]
    return lib.left_ideal_gens(gens, PRIME[0])


PRIME = [None]


def families(rng, p, B, Nbits_target=None):
    """Yield (name, generators-of-ideal list, expected N or None)."""
    PRIME[0] = p
    out = []
    top = 1 << B

    def add(name, gens):
        cols = lib.sage_hnf_cols(gens)
        N = lib.ideal_norm_from_cols(cols)
        out.append((name, cols, N))

    # O0 itself, tiny norms
    add('N=1 (O0)', lib.left_ideal_gens([[2, 0, 0, 0]], p))
    add('N=2 (1+i)', lib.left_ideal_gens([[2, 2, 0, 0]], p))
    for l in (3, 5, 7):
        for _ in range(3):
            x, y = root_mod_prime_power(rng, p, l, 1)
            add('N=%d generic' % l, ideal_from_root(l, x, y))
    # prime near 2^B, generic shape
    for _ in range(3):
        l = int(random_prime(top - 1, lbound=top >> 1))
        x, y = root_mod_prime_power(rng, p, l, 1)
        add('prime ~2^B generic', ideal_from_root(l, x, y))
    # prime just below the Q4 bound
    l = int(ZZ(top).previous_prime())
    x, y = root_mod_prime_power(rng, p, l, 1)
    add('largest prime < 2^B', ideal_from_root(l, x, y))
    # composite squarefree, three prime factors
    for _ in range(2):
        ls = [int(random_prime((1 << (B // 3)) - 1, lbound=1 << (B // 3 - 1))) for _ in range(3)]
        if len(set(ls)) < 3:
            continue
        x, y = root_composite(rng, p, [(l, 1) for l in ls])
        N = ls[0] * ls[1] * ls[2]
        add('composite 3 primes', ideal_from_root(N, x, y))
    # prime powers (cyclic)
    for l, e in ((3, 5), (5, 3), (7, 2), (int(next_prime(1 << (B // 2 - 1))), 2)):
        if l ** e >= top:
            continue
        x, y = root_composite(rng, p, [(l, e)])
        add('prime power %d^%d generic' % (l, e), ideal_from_root(l ** e, x, y))
    # non-primitive: l * J (pivot shape l * generic)
    for (l, Np) in ((3, 5), (5, 7), (3, 1)):
        x, y = root_mod_prime_power(rng, p, Np, 1) if Np > 1 else (0, 1)
        Jg = ideal_from_root(Np, x, y) if Np > 1 else lib.left_ideal_gens([[2, 0, 0, 0]], p)
        add('non-primitive %d*J (N(J)=%d)' % (l, Np), [[l * t for t in v] for v in Jg])
    # l * J at full width: l^2 * N' near 2^B
    l = 1009
    Np = int(random_prime((top // (l * l)) - 1, lbound=top // (4 * l * l)))
    x, y = root_mod_prime_power(rng, p, Np, 1)
    add('non-primitive 1009*J, large N\'', [[l * t for t in v] for v in ideal_from_root(Np, x, y)])
    # l O0 (norm l^2, pivots l*(2,2,1,1)); and (l1 l2) O0 composite scalar ideal
    for l in (3, 11):
        add('scalar %d*O0' % l, lib.left_ideal_gens([[2 * l, 0, 0, 0]], p))
    l1, l2 = 10007, int(random_prime(1 << (B // 2 - 2), lbound=1 << (B // 2 - 3)))
    add('scalar (l1*l2)*O0 composite', lib.left_ideal_gens([[2 * l1 * l2, 0, 0, 0]], p))
    # even norms: (1+i)^a * beta, 2-part
    for a in (1, 2, 5, 9):
        al = [2, 0, 0, 0]
        for _ in range(a):
            al = lib.qmul_num(al, [2, 2, 0, 0], p)
        beta = [2 * rng.randrange(-30, 30) + 0, 0, 0, 0]
        beta = [rng.randrange(-30, 30) * 2, rng.randrange(-30, 30) * 2, 0, 0]
        beta[2], beta[3] = beta[1] % 2, beta[0] % 2        # parity of O0
        if not lib.in_O0(beta):
            beta = [2, 2, 0, 0]
        add('even norm (1+i)^%d*beta' % a, lib.left_ideal_gens([lib.qmul_num(al, beta, p)], p))
    # principal ideals, random alpha, norm near 2^B
    for _ in range(4):
        while True:
            bits = rng.randrange(B // 2 - 4, B // 2 + 1)
            a = rng.randrange(-(1 << bits), 1 << bits) * 2
            b = rng.randrange(-(1 << bits), 1 << bits) * 2
            c, d = rng.randrange(-2, 3), rng.randrange(-2, 3)
            c, d = c, d
            v = [a + (d % 2), b + (c % 2), c, d]
            if lib.in_O0(v) and 4 <= lib.n4(v, p) // 4 < top:
                add('principal random alpha', lib.left_ideal_gens([v], p))
                break
    # O0*alpha + O0*M, M small prime dividing nrd(alpha)
    for M in (3, 5, 13):
        for _ in range(500):
            v = [rng.randrange(-100, 100) * 2 + 0, rng.randrange(-100, 100) * 2, 0, 0]
            c, d = rng.randrange(-3, 4), rng.randrange(-3, 4)
            v = [v[0] + (d % 2), v[1] + (c % 2), c, d]
            if lib.in_O0(v) and (lib.n4(v, p) // 4) % M == 0:
                add('O0*alpha + O0*%d' % M, lib.left_ideal_gens([v, [2 * M, 0, 0, 0]], p))
                break
    # ramified: N = p (j generates norm p) -- outside the Q4 class, K widened by the caller
    add('ramified N=p (O0*j)', lib.left_ideal_gens([[0, 0, 2, 0]], p))
    return out


def shape_label(cols, N):
    d = [cols[c][c] for c in range(4)]
    if d == [2 * N, 2 * N, 1, 1]:
        return 'generic (2N,2N,1,1)'
    from math import gcd
    g = 0
    for c in range(4):
        g = gcd(g, d[c])
    return 'other d=(%s)' % ','.join('%d' % x if x < 10 ** 6 else 'big' for x in d)
