# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Exhaustive small-K checks of the two scalar lemmas of REPORT.md.

Lemma 3  pair elimination: for all 0 <= a, b < 2^K the shifted-subtraction
         schedule reaches (g, 0) with g = gcd(a, b) in at most 2K-1 active
         steps, with a, b decreasing (we also record the exact worst case).
Lemma 4  Barrett: for every m in [2, 2^K) and x in [0, 2^(2K)): the
         quotient estimate is floor(x/m) or one less.
"""
import sys
from math import gcd


def pair_scalar(a, b):
    """Active steps until one is zero; returns (steps, g)."""
    steps = 0
    while a and b:
        if a < b:
            a, b = b, a
        t = a.bit_length() - b.bit_length()
        d = a - (b << t)
        assert abs(d) < (1 << (a.bit_length() - 1))          # Lemma 3 key inequality
        a = abs(d)
        steps += 1
    return steps, a + b


def lemma3(Kmax):
    out = []
    for K in range(1, Kmax + 1):
        worst = 0
        arg = None
        for a in range(0, 1 << K):
            for b in range(0, 1 << K):
                s, g = pair_scalar(a, b)
                assert g == gcd(a, b)
                assert s <= 2 * K - 1
                if s > worst:
                    worst, arg = s, (a, b)
        out.append((K, worst, 2 * K - 1, arg))
        print('lemma3 K=%2d worst active steps %2d  (proved bound 2K-1 = %2d) at %s' % (K, worst, 2 * K - 1, arg))
        sys.stdout.flush()
    return out


def lemma4(Kmax):
    for K in range(1, Kmax + 1):
        X = 1 << (2 * K)
        bad = 0
        n = 0
        for m in range(2, 1 << K):
            R = X // m
            for x in range(X):
                qh = (x * R) >> (2 * K)
                q = x // m
                assert qh in (q, q - 1), (K, m, x)
                r = x - qh * m
                assert 0 <= r < 2 * m
                n += 1
        print('lemma4 K=%d: %d (m,x) pairs, quotient error in {0,1}, remainder in [0,2m)' % (K, n))
        sys.stdout.flush()


if __name__ == '__main__':
    lemma3(int(sys.argv[1]) if len(sys.argv) > 1 else 10)
    lemma4(int(sys.argv[2]) if len(sys.argv) > 2 else 7)
