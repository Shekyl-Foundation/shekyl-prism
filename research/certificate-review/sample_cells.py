# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Monte-Carlo cross-check of cellcheck.py (SHE-24): sampled one-step ratios
r*g(eps')/g(eps) must never exceed cellcheck's per-leaf bound, and the
sampled supremum shows how tight that bound is.  Exact rationals for the
dynamics; g is looked up on HALF-OPEN leaves (the contract's convention)."""
import random
import sys
from gmpy2 import mpq
import cellcheck as cc

HALF = mpq(1, 2)


def leaf_of(leaves, maxL, x, y):
    # half-open leaves [i/d - 1/2, (i+1)/d - 1/2)
    for L in range(maxL + 1):
        d = 1 << L
        i = int((x + HALF) * d)
        j = int((y + HALF) * d)
        if (L, i, j) in leaves:
            return (L, i, j)
    raise RuntimeError('no leaf for %s %s' % (x, y))


def per_leaf_bounds(leaves, LAM, idx):
    worst, _, _ = cc.check(leaves, None, LAM, idx, report_worst=len(leaves))
    return {k: r for (r, k) in worst}


def main():
    path, lexp, n, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
    leaves, rho, _ = cc.load(path)
    maxL = max(k[0] for k in leaves)
    idx = cc.Index(leaves)
    LAM = mpq(1, 1 << lexp)
    bound = per_leaf_bounds(leaves, LAM, idx)
    rng = random.Random(seed)
    keys = sorted(leaves)
    # bias towards the leaves with the largest bounds
    top = sorted(keys, key=lambda k: -bound[k])[:200]
    D = 1 << 40
    best = (0, None)
    viol = 0
    for s in range(n):
        key = rng.choice(top) if s % 2 else rng.choice(keys)
        x0, x1, y0, y1 = cc.box(key)
        x = x0 + (x1 - x0) * mpq(rng.randrange(D), D)
        y = y0 + (y1 - y0) * mpq(rng.randrange(D), D)
        lam = LAM * mpq(rng.choice([0, D, rng.randrange(D)]), D)
        r = lam + x * x + y * y
        if r == 0 or r >= 1:
            continue
        qx, qy = x / r, -y / r
        m0, m1 = cc.fhu(qx), cc.fhu(qy)
        if m0 * m0 + m1 * m1 < 2:
            continue
        ex, ey = qx - m0, qy - m1
        k2 = leaf_of(leaves, maxL, ex, ey)
        ratio = r * leaves[k2] / leaves[key]
        if ratio > bound[key]:
            viol += 1
            print('VIOLATION', key, x, y, lam, ratio, bound[key])
        if ratio > best[0]:
            best = (ratio, (key, float(x), float(y), float(lam)))
    print('LAM 2^-%d: %d samples, sampled max ratio %.9f at %s; bound violations %d'
          % (lexp, n, float(best[0]), best[1], viol))


if __name__ == '__main__':
    main()
