# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Exact-integer oracles and input generators for the fixed-modulus HNF tests.

Independent of fmhnf.py: HNF from Sage (row style, coordinates reversed) and
from PARI mathnf (column style, dev's convention directly).  Quaternion
arithmetic in O0 numerators over Q(i,j), i^2 = -1, j^2 = -p.
"""
import random
from math import gcd, isqrt
from sage.all import matrix, ZZ, Zmod, factor, crt, is_prime, next_prime, random_prime
import cypari2

pari = cypari2.Pari()
pari.allocatemem(2 * 10 ** 9, silent=True)


# ---------------------------------------------------------------- oracles
def sage_hnf_cols(G, m=None):
    """Dev-convention HNF columns of span(G) (+ m Z^4 if m is not None); list c -> col[0..c]."""
    rows = [list(reversed(r)) for r in G]
    if m is not None:
        rows += [[m if i == j else 0 for i in range(4)] for j in range(4)]
    H = matrix(ZZ, rows).hermite_form()
    cols = {}
    nz = [r for r in H.rows() if any(r)]
    assert len(nz) == 4, 'rank deficient'
    for j in range(4):
        r = list(nz[j])
        c = 3 - j
        assert all(r[3 - i] == 0 for i in range(c + 1, 4))
        cols[c] = [int(r[3 - i]) for i in range(c + 1)]
    return [cols[c] for c in range(4)]


def pari_hnf_cols(G, m=None):
    gens = [list(r) for r in G]
    if m is not None:
        gens += [[m if i == j else 0 for i in range(4)] for j in range(4)]
    M = pari.matrix(4, len(gens), [gens[c][r] for r in range(4) for c in range(len(gens))])
    H = pari.mathnf(M)
    cols = []
    for c in range(4):
        cols.append([int(H[i, c]) for i in range(c + 1)])
        assert all(int(H[i, c]) == 0 for i in range(c + 1, 4))
    return cols


def det_of_cols(cols):
    d = 1
    for c in range(4):
        d *= cols[c][c]
    return d


def lattice_contains(cols, v):
    """Is v (4-vector) in the lattice of the dev-convention HNF cols?  exact back substitution."""
    v = list(v)
    for c in (3, 2, 1, 0):
        d = cols[c][c]
        if v[c] % d:
            return False
        q = v[c] // d
        for i in range(c + 1):
            v[i] -= q * cols[c][i]
    return all(x == 0 for x in v)


# ---------------------------------------------------------------- quaternions
def qmul_raw(x, y, p):
    a, b = -1, -p
    x0, x1, x2, x3 = x
    y0, y1, y2, y3 = y
    return [x0 * y0 + a * x1 * y1 + b * x2 * y2 - a * b * x3 * y3,
            x0 * y1 + x1 * y0 - b * x2 * y3 + b * x3 * y2,
            x0 * y2 + x2 * y0 + a * x1 * y3 - a * x3 * y1,
            x0 * y3 + x3 * y0 + x1 * y2 - x2 * y1]


def qmul_num(x, y, p):
    """Numerator of (x/2)(y/2): raw product / 2 (exact when x, y in O0)."""
    r = qmul_raw(x, y, p)
    assert all(t % 2 == 0 for t in r)
    return [t // 2 for t in r]


O0_BASIS = [[2, 0, 0, 0], [0, 2, 0, 0], [0, 1, 1, 0], [1, 0, 0, 1]]   # numerators of 1, i, (i+j)/2, (1+k)/2


def in_O0(v):
    return (v[0] - v[3]) % 2 == 0 and (v[1] - v[2]) % 2 == 0


def n4(v, p):
    return v[0] ** 2 + v[1] ** 2 + p * (v[2] ** 2 + v[3] ** 2)      # = 4 nrd(v/2)


def left_ideal_gens(alphas, p):
    """Generators (numerators) of O0*alpha_1 + O0*alpha_2 + ..."""
    return [qmul_num(e, a, p) for a in alphas for e in O0_BASIS]


def ideal_norm_from_cols(cols):
    d = det_of_cols(cols)
    assert d % 4 == 0
    N2 = d // 4
    N = isqrt(N2)
    assert N * N == N2, 'index is not 4 N^2'
    return N


def rand_unimodular(rng, size_bits, n=4, rounds=12):
    """Random unimodular n x n matrix with controlled entry size."""
    U = [[int(i == j) for j in range(n)] for i in range(n)]
    for _ in range(rounds * n):
        i, j = rng.sample(range(n), 2)
        t = rng.randrange(-(1 << size_bits), 1 << size_bits)
        U[i] = [a + t * b for a, b in zip(U[i], U[j])]
    return U


def scramble(rng, cols, k, size_bits, bound):
    """k >= 4 generator rows spanning exactly the lattice of the HNF columns, entries < bound."""
    basis = [[cols[c][i] if i <= c else 0 for i in range(4)] for c in range(4)]
    for _ in range(50):
        U = rand_unimodular(rng, size_bits)
        rows = [[sum(U[r][s] * basis[s][i] for s in range(4)) for i in range(4)] for r in range(4)]
        extra = []
        for _e in range(k - 4):
            co = [rng.randrange(-3, 4) for _ in range(4)]
            extra.append([sum(co[s] * rows[s][i] for s in range(4)) for i in range(4)])
        allr = rows + extra
        if all(abs(x) < bound for r in allr for x in r):
            rng.shuffle(allr)
            return allr
        size_bits = max(1, size_bits - 4)
    return basis + [[0] * 4 for _ in range(k - 4)]
