# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Independent check of the SHE-23 cell potential (SHE-24 review).

Written from the mathematics of the SHE-23 report only; no code is shared
with research/tighter-progress-certificate.  Exact rationals (gmpy2.mpq).

Claim checked, for every leaf c of the quadtree and every
lambda in [0, LAM]:

    sup_{eps in c, lambda, admissible mu}  r * g(eps') / g(c)  <=  rho,

where r = lambda + |eps|^2, q = conj(eps)/r, mu = floor(q + 1/2)
componentwise, |mu|^2 >= 2 (Lemma 1), eps' = q - mu.

Soundness choices (all conservative):
  * source cells are CLOSED boxes (the contract's residual square is
    half-open, [-1/2,1/2)^2, a subset);
  * q is enclosed by the exact projection of the map onto each axis,
    computed in closed form (unimodal x/(c + x^2)); the only irrational
    value, 1/(2 sqrt c), is bounded above by a rational;
  * successor regions are closed and are intersected with CLOSED leaves;
  * r is bounded by LAM + max |eps|^2 independently of g(eps').
"""
import sys
import time
from math import isqrt
from gmpy2 import mpq

HALF = mpq(1, 2)


def load(path):
    leaves = {}
    rho = lam = None
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            parts = line.split()
            if parts[0] == 'rho':
                n, d = parts[1].split('/')
                rho = mpq(int(n), int(d))
            elif parts[0] == 'lam':
                lam = int(parts[1])
            else:
                L, i, j, g = map(int, parts)
                key = (L, i, j)
                if key in leaves:
                    raise SystemExit('duplicate leaf %r' % (key,))
                leaves[key] = mpq(g, 1 << 32)
    return leaves, rho, lam


def check_tiling(leaves):
    """Leaves are disjoint dyadic squares inside [-1/2,1/2)^2 of total area 1.

    Two dyadic squares are nested or disjoint, so 'no leaf has an ancestor
    that is a leaf' gives disjointness; disjoint half-open squares of total
    area 1 inside the unit square tile it exactly."""
    maxL = max(L for (L, _, _) in leaves)
    area = 0
    for (L, i, j) in leaves:
        if not (0 <= i < (1 << L) and 0 <= j < (1 << L)):
            return False, 'leaf out of range %r' % ((L, i, j),)
        area += 1 << (2 * (maxL - L))
        a, b = i, j
        for LL in range(L - 1, -1, -1):
            a >>= 1
            b >>= 1
            if (LL, a, b) in leaves:
                return False, 'overlap: %r inside %r' % ((L, i, j), (LL, a, b))
    if area != 1 << (2 * maxL):
        return False, 'area %d != %d' % (area, 1 << (2 * maxL))
    return True, 'maxdepth %d' % maxL


def box(key):
    L, i, j = key
    d = 1 << L
    return (mpq(i, d) - HALF, mpq(i + 1, d) - HALF,
            mpq(j, d) - HALF, mpq(j + 1, d) - HALF)


class Index:
    """Quadtree max-g over every leaf whose closed box meets a closed box."""

    def __init__(self, leaves):
        self.leaves = leaves
        self.sub = {}          # internal node -> max g below
        for (L, i, j), g in leaves.items():
            a, b = i, j
            for LL in range(L - 1, -1, -1):
                a >>= 1
                b >>= 1
                k = (LL, a, b)
                if self.sub.get(k, 0) < g:
                    self.sub[k] = g

    def maxg(self, xl, xh, yl, yh, key=(0, 0, 0), best=mpq(0)):
        if key in self.leaves:
            return max(best, self.leaves[key])
        if self.sub[key] <= best:
            return best
        L, i, j = key
        for di in (0, 1):
            for dj in (0, 1):
                k = (L + 1, 2 * i + di, 2 * j + dj)
                bx0, bx1, by0, by1 = box(k)
                if bx0 > xh or bx1 < xl or by0 > yh or by1 < yl:
                    continue
                best = self.maxg(xl, xh, yl, yh, k, best)
        return best


def sqrt_lo(c):
    """Rational lower bound of sqrt(c), c >= 0 rational."""
    n, d = c.numerator, c.denominator
    k = 80
    return mpq(isqrt(n * d << (2 * k)), d << k)


def range_pos(a, b, cmin, cmax):
    """Range of f(x, c) = x/(c + x^2) over x in [a,b] (0 <= a <= b),
    c in [cmin, cmax].  Returns (lo, hi) or None if unbounded."""
    # upper: c = cmin (f decreasing in c for x >= 0)
    if cmin == 0:
        if a == 0:
            return None
        hi = 1 / a
    else:
        if a * a <= cmin <= b * b:
            hi = 1 / (2 * sqrt_lo(cmin))
        elif b * b < cmin:
            hi = b / (cmin + b * b)
        else:
            hi = a / (cmin + a * a)
    # lower: c = cmax, unimodal in x -> min at an endpoint
    lo = min(a / (cmax + a * a), b / (cmax + b * b))
    return lo, hi


def range_f(a, b, cmin, cmax):
    """Hull of x/(c + x^2) over [a,b] x [cmin,cmax]; None if unbounded."""
    lo, hi = None, None
    parts = []
    if b > 0:
        r = range_pos(max(a, mpq(0)), b, cmin, cmax)
        if r is None:
            return None
        parts.append(r)
    if a < 0:
        r = range_pos(max(-b, mpq(0)), -a, cmin, cmax)
        if r is None:
            return None
        parts.append((-r[1], -r[0]))
    if a <= 0 <= b:
        parts.append((mpq(0), mpq(0)))
    lo = min(p[0] for p in parts)
    hi = max(p[1] for p in parts)
    return lo, hi


def sq_range(a, b):
    if a >= 0:
        return a * a, b * b
    if b <= 0:
        return b * b, a * a
    return mpq(0), max(a * a, b * b)


def fhu(f):
    return (2 * f.numerator + f.denominator) // (2 * f.denominator)


def succ_intervals(lo, hi):
    """Integer quotients m and residual intervals [lo-m, hi-m] cut to the
    closed [-1/2, 1/2]; 'wide' when 4 or more quotients occur."""
    m_lo, m_hi = fhu(lo), fhu(hi)
    if m_hi - m_lo >= 3:
        return 'wide'
    out = []
    for m in range(m_lo, m_hi + 1):
        a, b = max(lo - m, -HALF), min(hi - m, HALF)
        if a <= b:
            out.append((m, a, b))
    return out


def check(leaves, rho, LAM, idx, report_worst=3):
    G = max(leaves.values())
    worst = []
    nwild = 0
    for key, g in leaves.items():
        x0, x1, y0, y1 = box(key)
        sx = sq_range(x0, x1)
        sy = sq_range(y0, y1)
        rhi = LAM + sx[1] + sy[1]
        qx = range_f(x0, x1, sy[0], LAM + sy[1])          # c = lam + y^2
        qy = range_f(y0, y1, sx[0], LAM + sx[1])          # c = lam + x^2
        gs = None
        if qx is None or qy is None:
            gs = G
            nwild += 1
        else:
            qy = (-qy[1], -qy[0])                          # conj
            ix = succ_intervals(*qx)
            iy = succ_intervals(*qy)
            if ix == 'wide' and iy == 'wide':
                gs = G
            else:
                gs = mpq(0)
                X = [(None, -HALF, HALF)] if ix == 'wide' else ix
                Y = [(None, -HALF, HALF)] if iy == 'wide' else iy
                for (m0, a, b) in X:
                    for (m1, c, d) in Y:
                        if m0 is not None and m1 is not None and m0 * m0 + m1 * m1 < 2:
                            continue   # Lemma 1
                        gs = idx.maxg(a, b, c, d, best=gs)
        ratio = rhi * gs / g
        worst.append((ratio, key))
    worst.sort(reverse=True)
    return worst[:report_worst], nwild, G


def main():
    path = sys.argv[1]
    leaves, rho, lam = load(path)
    ok, msg = check_tiling(leaves)
    G = max(leaves.values())
    gmin = min(leaves.values())
    print('%s: %d leaves; tiling %s (%s); min g = %s (>= 1: %s); G = max g = %.9f'
          % (path, len(leaves), ok, msg, float(gmin), gmin >= 1, float(G)))
    idx = Index(leaves)
    bands = [(20, rho)] + [(int(a), mpq(b)) for a, b in (s.split(':') for s in sys.argv[2:])]
    for lexp, target in bands:
        t0 = time.time()
        worst, nwild, _ = check(leaves, rho, mpq(1, 1 << lexp), idx)
        r = worst[0][0]
        print('LAM 2^-%d: max ratio = %.9f (exact %s) at leaf %r; wild leaves %d; '
              'target %s = %.9f: %s  [%.1fs]'
              % (lexp, float(r), r, worst[0][1], nwild, target, float(target),
                 r <= target, time.time() - t0))
        sys.stdout.flush()


if __name__ == '__main__':
    main()
