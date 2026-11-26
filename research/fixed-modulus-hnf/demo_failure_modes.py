# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Where the claim fails if its hypotheses are dropped (REPORT.md section 6).

A. Textbook modular HNF (DKT / Cohen 2.4.8: modulus divided by every pivot) with R = 2N:
   wrong; with R = 4N^2 (a multiple of the determinant): right, at about twice the width.
B. The fixed-modulus algorithm on generators whose lattice does NOT contain 2N Z^4:
   it returns HNF(span G + 2N Z^4), a proper superlattice.  The product of its pivots then
   differs from |det|: the self-check (pivot product == 4N^2 == index of G) catches it.
C. K too small (m >= 2^K): the Barrett lemma no longer applies; the model's range checks fire.
"""
import random
from math import gcd
import lib
from fmhnf import fixed_modulus_hnf


def textbook_modhnf(G, R0):
    """Exact-integer textbook DKT: all arithmetic mod R, R := R / pivot after each column."""
    R = R0
    rows = [[x % R for x in r] for r in G]
    P = [None] * 4
    d = [0] * 4
    for c in (3, 2, 1, 0):
        acc = [0] * 4
        acc[c] = R
        rest = []
        for r in rows:
            a, b = acc, r
            while b[c] != 0:
                q = a[c] // b[c]
                a, b = b, [(x - q * y) % R for x, y in zip(a, b)]
                # keep integers: pivot entries are exact below
                a, b = a, b
            rest.append(b)
            acc = a
        P[c] = acc
        d[c] = acc[c]
        rows = rest
        if R % d[c]:
            return None
        R //= d[c]
        rows = [[x % R for x in r] for r in rows]
    return d


def main():
    print('A. textbook modular HNF')
    # left ideals l*O0 (norm l^2, numerator lattice l*O0num, pivots l*(2,2,1,1), index 4 l^4 = 4N^2)
    for l in (1, 3, 5):
        N = l * l
        G = [[l * t for t in v] for v in lib.O0_BASIS]
        true = lib.sage_hnf_cols(G)
        print('   ideal %d*O0 (N=%d): true pivots %s | textbook R=2N -> %s | textbook R=4N^2 -> %s | fixed-modulus 2N -> %s' % (
            l, N, [true[c][c] for c in range(4)], textbook_modhnf(G, 2 * N), textbook_modhnf(G, 4 * N * N),
            [fixed_modulus_hnf(G, 2 * N, 2 * N.bit_length() + 2)[0][c][c] for c in range(4)]))
    # a realistic ideal: generic HNF (2N,2N,1,1)
    N = 7
    cols = [[2 * N], [0, 2 * N], [2 * 3, 5, 1], [2 * N - 5, 6, 0, 1]]
    G = [[cols[c][i] if i <= c else 0 for i in range(4)] for c in range(4)]
    print('   generic N=7: true %s | textbook R=2N -> %s | R=4N^2 -> %s | fixed-modulus -> %s' % (
        [cols[c][c] for c in range(4)], textbook_modhnf(G, 2 * N), textbook_modhnf(G, 4 * N * N),
        [fixed_modulus_hnf(G, 2 * N, 5)[0][c][c] for c in range(4)]))

    print('B. generators whose span does not contain 2N Z^4 (not an ideal lattice)')
    N = 5
    m = 2 * N
    G = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 4 * N * N]]     # cyclic quotient Z/4N^2
    P, _ = fixed_modulus_hnf(G, m, 5)
    true = lib.sage_hnf_cols(G)
    print('   span G: pivots %s (index %d = 4N^2).  fixed-modulus output: pivots %s (index %d)' % (
        [true[c][c] for c in range(4)], lib.det_of_cols(true),
        [P[c][c] for c in range(4)], lib.det_of_cols(P)))
    assert P == lib.sage_hnf_cols(G, m)
    print('   = HNF(span G + 2N Z^4); pivot product %d != 4N^2 = %d -> self-check fails, input rejected' % (
        lib.det_of_cols(P), 4 * N * N))

    print('C. m >= 2^K')
    try:
        fixed_modulus_hnf([[1, 2, 3, 4]] * 4, 2 ** 5 + 6, 5)
        print('   no failure raised')
    except AssertionError as e:
        print('   model range check fired: %s' % (e,))


if __name__ == '__main__':
    main()
