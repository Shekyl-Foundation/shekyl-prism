# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Exact (gmpy2.mpq) cell abstraction of the main-phase step map.

State: the residual eps = (x, y) of the current step, in the closed square
[-1/2, 1/2]^2 (a superset of the contract's [-1/2, 1/2)^2), and
lam = Delta / A^2 in [0, LAM].  If the step swaps, r = lam + |eps|^2 < 1,
the next exact quotient is q = conj(eps) / r = (x, -y) / r, the next
quotient is mu = (floor(q0 + 1/2), floor(q1 + 1/2)) (the contract), and the
next residual is eps' = q - mu.  Lemma 1 (strict swap): if the next step also
swaps, |mu|^2 >= 2.

Cells are dyadic squares (a quadtree).  For a cell c, succ(c) lists every
leaf that can contain eps' for some (eps, lam) in c x [0, LAM] and some
admissible mu (interval enclosure, exact rationals), and rhi(c) is an exact
rational upper bound of r over c x [0, LAM].  A cell is 'wild' when the
enclosure admits more than MAXMU quotients (cells at the origin); its
successors are then 'any leaf'.
"""
from gmpy2 import mpq

HALF = mpq(1, 2)
ZERO = mpq(0)
MAXMU = 49


def fhu(f):
    """floor(f + 1/2), exact."""
    return int((2 * f.numerator + f.denominator) // (2 * f.denominator))


def iv_sq(lo, hi):
    if lo >= 0:
        return lo * lo, hi * hi
    if hi <= 0:
        return hi * hi, lo * lo
    return ZERO, max(lo * lo, hi * hi)


def iv_div(alo, ahi, blo, bhi):
    c = (alo / blo, alo / bhi, ahi / blo, ahi / bhi)
    return min(c), max(c)


class Tree:
    def __init__(self, L0):
        self.leaves = set()
        self.internal = set()
        for lvl in range(L0):
            n = 1 << lvl
            for i in range(n):
                for j in range(n):
                    self.internal.add((lvl, i, j))
        n = 1 << L0
        for i in range(n):
            for j in range(n):
                self.leaves.add((L0, i, j))

    @staticmethod
    def box(k):
        L, i, j = k
        d = 1 << L
        return (mpq(i, d) - HALF, mpq(i + 1, d) - HALF, mpq(j, d) - HALF, mpq(j + 1, d) - HALF)

    def query(self, xl, xh, yl, yh, out, k=(0, 0, 0)):
        """Every leaf whose closed box meets [xl, xh] x [yl, yh]."""
        if k in self.leaves:
            out.add(k)
            return
        L, i, j = k
        d = 1 << (L + 1)
        for di in (0, 1):
            bxl = mpq(2 * i + di, d) - HALF
            bxh = mpq(2 * i + di + 1, d) - HALF
            if bxl > xh or bxh < xl:
                continue
            for dj in (0, 1):
                byl = mpq(2 * j + dj, d) - HALF
                byh = mpq(2 * j + dj + 1, d) - HALF
                if byl > yh or byh < yl:
                    continue
                self.query(xl, xh, yl, yh, out, (L + 1, 2 * i + di, 2 * j + dj))

    def split(self, k):
        self.leaves.remove(k)
        self.internal.add(k)
        L, i, j = k
        kids = [(L + 1, 2 * i + di, 2 * j + dj) for di in (0, 1) for dj in (0, 1)]
        self.leaves.update(kids)
        return kids


def image(k, LAM):
    """(rhi, rlo, list of (m0, m1, eps' box)) or (rhi, rlo, None) if wild."""
    xl, xh, yl, yh = Tree.box(k)
    x2l, x2h = iv_sq(xl, xh)
    y2l, y2h = iv_sq(yl, yh)
    rl = x2l + y2l
    rh = x2h + y2h + LAM
    if rl <= 0:
        return rh, rl, None
    rhc = min(rh, mpq(1))
    q0l, q0h = iv_div(xl, xh, rl, rhc)
    q1l, q1h = iv_div(-yh, -yl, rl, rhc)
    m0l, m0h = fhu(q0l), fhu(q0h)
    m1l, m1h = fhu(q1l), fhu(q1h)
    if (m0h - m0l + 1) * (m1h - m1l + 1) > MAXMU:
        return rh, rl, None
    outs = []
    for m0 in range(m0l, m0h + 1):
        for m1 in range(m1l, m1h + 1):
            if m0 * m0 + m1 * m1 <= 1:
                continue
            a, b = max(-HALF, q0l - m0), min(HALF, q0h - m0)
            c, d = max(-HALF, q1l - m1), min(HALF, q1h - m1)
            if a > b or c > d:
                continue
            outs.append((m0, m1, (a, b, c, d)))
    return rh, rl, outs


def succ(tree, k, LAM):
    rh, rl, outs = image(k, LAM)
    if outs is None:
        return rh, None
    s = set()
    for _, _, bx in outs:
        tree.query(*bx, s)
    return rh, s
