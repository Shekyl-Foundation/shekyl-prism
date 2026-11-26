# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Fixed-modulus, fixed-schedule HNF of a lattice in Z^4 containing m*Z^4.

Executable model of the algorithm in REPORT.md section 2.

Convention (matches dev 470d537 shekyl-prism/src/ideal/mod.rs): the HNF is
four COLUMNS; column c has zeros at coordinates > c, a positive pivot d_c at
coordinate c, and entries at coordinates l < c in [0, d_l).  Coordinates are
the O0 numerators (1, i, j, k) = indices 0..3.  Pivots are therefore found
in the order c = 3, 2, 1, 0.  (This is the mirror image of the row-style HNF
of Sage's `hermite_form`; the tests reverse the coordinates to compare.)

Control flow depends only on the public (K, k).  Every secret-dependent
choice is a select.  Python ints stand for prism-int values; the Ctx counts
every primitive with its DECLARED (public) width and asserts that the actual
magnitude is below the declared bound, so the proved width table is checked
on every run.  `trace_hash` hashes the executed primitive sequence: it must
be identical for every input of the same (K, k).
"""
import hashlib
from collections import Counter


class Ctx:
    def __init__(self):
        self.count = Counter()
        self.limbmul = 0            # schoolbook limb products of `mul`
        self.maxbits = {}           # op name -> max actual magnitude bits seen
        self.declared = {}          # op name -> max declared bits
        self.h = hashlib.sha256()
        self.nops = 0

    # ---- bookkeeping -------------------------------------------------
    def _rec(self, name, w, *vals):
        self.count[name] += 1
        self.nops += 1
        self.h.update(('%s:%d;' % (name, w)).encode())
        self.declared[name] = max(self.declared.get(name, 0), w)
        for v in vals:
            b = abs(v).bit_length()
            assert b <= w, (name, 'declared', w, 'actual', b)
            if b > self.maxbits.get(name, 0):
                self.maxbits[name] = b

    # ---- primitives (each is one prism-int operation) ------------------
    def add(self, a, b, w):
        r = a + b
        self._rec('add', w, a, b, r)
        return r

    def sub(self, a, b, w):
        r = a - b
        self._rec('sub', w, a, b, r)
        return r

    def neg(self, a, w):
        self._rec('neg', w, a)
        return -a

    def abs(self, a, w):
        self._rec('abs', w, a)
        return abs(a)

    def lt(self, a, b, w):                  # secret bit
        self._rec('cmp', w, a, b)
        return 1 if a < b else 0

    def is_zero(self, a, w):
        self._rec('cmp', w, a)
        return 1 if a == 0 else 0

    def is_neg(self, a, w):
        self._rec('signbit', w, a)
        return 1 if a < 0 else 0

    def bitop(self, a, b, kind):           # secret-bit logic
        self._rec('bit', 1)
        return {'and': a & b, 'or': a | b, 'not': 1 - a}[kind]

    def sel(self, c, a, b, w):              # c ? a : b
        self._rec('sel', w, a, b)
        return a if c else b

    def bitlen(self, a, w):                 # secret magnitude bit length
        self._rec('bitlen', w, a)
        return abs(a).bit_length()

    def shl_sec(self, a, t, w):             # secret left shift, result < 2^w
        r = a << t
        self._rec('shl_secret', w, a, r)
        return r

    def shl_pub(self, a, n, w):             # public shift
        r = a << n
        self._rec('shl_public', w, a, r)
        return r

    def shr_pub(self, a, n, w):
        r = a >> n
        self._rec('shr_public', w, a)
        return r

    def mul(self, a, b, wa, wb):
        r = a * b
        self._rec('mul', wa + wb, a * 0 + r)   # result width declared = wa+wb
        assert abs(a).bit_length() <= wa and abs(b).bit_length() <= wb, ('mul', wa, wb)
        self.limbmul += (-(-wa // 64)) * (-(-wb // 64))
        return r

    def trace_hash(self):
        return self.h.hexdigest()


# --------------------------------------------------------------------------
class Fixed:
    """Public schedule parameters: K (m < 2^K) and k generator rows."""

    def __init__(self, K, k):
        self.K = K
        self.k = k
        self.F = 2 * K - 1          # pair-elimination steps (Lemma 3)
        self.w2K = 2 * K            # |y| < 2^(2K) fed to Red


def recip(ctx, m, K):
    """R = floor(2^(2K) / m) by restoring division: 2K+1 fixed iterations."""
    rem = 0
    R = 0
    for i in range(2 * K, -1, -1):
        rem = ctx.shl_pub(rem, 1, K + 2)
        if i == 2 * K:                       # public: the single set bit of 2^(2K)
            rem = ctx.add(rem, 1, K + 2)
        lt = ctx.lt(rem, m, K + 2)
        ge = ctx.bitop(lt, 0, 'not')
        rem = ctx.sel(ge, ctx.sub(rem, m, K + 2), rem, K + 2)
        R = ctx.shl_pub(R, 1, 2 * K)
        R = ctx.add(R, ge, 2 * K)
    return R


def red(ctx, y, m, R, K, tight=True):
    """y mod m in [0, m), m < 2^K (Lemma 4: Barrett, one conditional subtraction).

    tight=True : |y| < 2^K * m   (every call inside the schedule), product 3K bits.
    tight=False: |y| < 2^(2K)    (the entry reduction of arbitrary generators).
    """
    if tight:
        assert abs(y) < (m << K), 'red: |y| >= 2^K m (model check)'
        wq, wprod = K, 3 * K
    else:
        wq, wprod = 2 * K - 1, 4 * K - 1
    a = ctx.abs(y, 2 * K)
    prod = ctx.mul(a, R, 2 * K, 2 * K)
    ctx.maxbits['red_prod_%s' % ('t' if tight else 'u')] = max(
        ctx.maxbits.get('red_prod_%s' % ('t' if tight else 'u'), 0), prod.bit_length())
    assert prod.bit_length() <= wprod
    qh = ctx.shr_pub(prod, 2 * K, wprod)             # floor(a R / 2^2K)
    assert qh.bit_length() <= wq
    qm = ctx.mul(qh, m, wq, K)                       # <= a < 2^2K
    r = ctx.sub(a, qm, 2 * K)                        # in [0, 2m)
    ge = ctx.bitop(ctx.lt(r, m, K + 1), 0, 'not')
    r = ctx.sel(ge, ctx.sub(r, m, K + 1), r, K + 1)
    nz = ctx.bitop(ctx.is_zero(r, K), 0, 'not')
    flip = ctx.bitop(ctx.is_neg(y, 2 * K), nz, 'and')
    return ctx.sel(flip, ctx.sub(m, r, K), r, K)


def pair_step(ctx, X, Y, c, m, R, K):
    """One shifted-subtraction step on rows X, Y (lists over coordinates 0..c)."""
    n = c + 1
    lt = ctx.lt(X[c], Y[c], K)
    X2 = [ctx.sel(lt, Y[j], X[j], K) for j in range(n)]
    Y2 = [ctx.sel(lt, X[j], Y[j], K) for j in range(n)]
    bx = ctx.bitlen(X2[c], K)
    by = ctx.bitlen(Y2[c], K)
    t = ctx.sub(bx, by, 16)
    yz = ctx.is_zero(Y2[c], K)
    D = []
    for j in range(n):
        ys = ctx.shl_sec(Y2[j], t, 2 * K)
        D.append(ctx.sub(X2[j], ys, 2 * K))
    neg = ctx.is_neg(D[c], 2 * K)
    E = [ctx.sel(neg, ctx.neg(d, 2 * K), d, 2 * K) for d in D]
    Enew = [ctx.sel(yz, X2[j], E[j] if j == c else red(ctx, E[j], m, R, K), K)
            for j in range(n)]
    # Note: red() runs for every tail j<c on every step (also when yz): uniform.
    return Enew, Y2


def eliminate(ctx, S, c, m, R, K, F):
    """Fold the k rows S (lists over 0..c) with the permanent row m*e_c into one pivot row."""
    acc = [0] * c + [m]
    newS = []
    for Yrow in S:
        X, Y = acc, Yrow
        act = 0
        for _ in range(F):
            act += 1 if (X[c] != 0 and Y[c] != 0) else 0     # model statistic, not in the trace
            X, Y = pair_step(ctx, X, Y, c, m, R, K)
        ctx.max_active = max(getattr(ctx, 'max_active', 0), act)
        z = ctx.is_zero(X[c], K)               # normalise: pivot row first
        X, Y = ([ctx.sel(z, Y[j], X[j], K) for j in range(c + 1)],
                [ctx.sel(z, X[j], Y[j], K) for j in range(c + 1)])
        assert Y[c] == 0 and X[c] > 0, 'schedule too short (model check)'
        acc = X
        newS.append(Y[:c])
    return acc, newS


def fixed_modulus_hnf(G, m, K, ctx=None):
    """G: k rows (4 ints each), m: modulus with 2 <= m < 2^K.

    Returns (cols, ctx), cols[c] = column c as a length-(c+1) list, the HNF of
    span(G) + m*Z^4 in dev's column convention.
    """
    ctx = ctx or Ctx()
    k = len(G)
    fx = Fixed(K, k)
    F = fx.F
    R = recip(ctx, m, K)
    # entry reduction (Lemma 1: adding multiples of m*e_j)
    S = [[red(ctx, x, m, R, K, tight=False) for x in row] for row in G]
    P = [None] * 4
    d = [0] * 4
    for c in (3, 2, 1, 0):
        S_c = [row[:c + 1] for row in S]
        P[c], S = eliminate(ctx, S_c, c, m, R, K, F)
        d[c] = P[c][c]
    # canonical form: reduce entry l of column c modulo d_l, l = c-1 .. 0
    for c in (3, 2, 1):
        for l in range(c - 1, -1, -1):
            v = P[c][l]
            q = 0
            for t in range(K - 1, -1, -1):
                dt = ctx.shl_pub(d[l], t, 2 * K - 1)
                ge = ctx.bitop(ctx.lt(v, dt, 2 * K), 0, 'not')
                v = ctx.sel(ge, ctx.sub(v, dt, 2 * K), v, 2 * K)
                q = ctx.add(q, ctx.shl_pub(ge, t, K), K)    # set bit t of q
            P[c][l] = v
            for lp in range(l):
                prod = ctx.mul(q, P[l][lp], K, K)
                y = ctx.sub(P[c][lp], prod, 2 * K)
                P[c][lp] = red(ctx, y, m, R, K)
    return P, ctx


def to_matrix(P):
    """4x4 matrix M[i][c] (rows = coordinates, columns = vectors), upper triangular."""
    M = [[0] * 4 for _ in range(4)]
    for c in range(4):
        for i in range(c + 1):
            M[i][c] = P[c][i]
    return M
