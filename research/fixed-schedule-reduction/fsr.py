# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Fixed-schedule reduction and fixed-budget short-vector search on the
4-D PRISM ideal lattices.

Research model only. Counts are operation counts in Python, not cycles,
and Python big integers are not constant-time. This is algorithm-level
evidence for whether a reduction with an input-independent schedule can
match exact LLL's output on PRISM's lattices, and at what cost.

PRISM is the scheme of ePrint 2026/443 (Basso, Borin, Castryck,
Corte-Real Santos, Invernizzi, Leroux, Maino, Vercauteren, Wesolowski);
shekyl-prism implements it. The lattice set-up (salted challenge q, H2
ideal selection, secret product I_sec * I_q) is reused unchanged from
research/higher-dim-lattice/hdl.py (copied here as hdl.py).

Three reductions are compared on the same Gram matrix:

  exact   Cohen Alg. 2.6.7 integral LLL, delta = 99/100 (the baseline);
          runs until the basis is LLL-reduced, so its length depends on
          the input.
  fixed   the same integral (d, lambda) arithmetic, but a schedule fixed by
          the parameter set: T tours, each tour visits k in a fixed order,
          always size-reduces b_k against every b_l (l < k) with the
          rounded coefficient (possibly 0), always evaluates the Lovasz
          test, and always executes the swap arithmetic, keeping the
          result with an arithmetic select. No data-dependent branch.
  enum    fixed pre-reduction (T' tours) followed by Schnorr-Euchner
          enumeration with a fixed node budget (dummy visits pad to the
          budget once the tree is exhausted).
  sieve   fixed pre-reduction followed by a fixed number of rounds of
          all-pairs pairwise reduction on a fixed-size list.
"""
import hashlib

import gmpy2
from gmpy2 import mpfr

from hdl import (
    pari, SETS, SALT_LEN, shake, stream_job_data, select_xy, ideal_basis,
    gram_norm, secret_class, ideal_from_classes, nrd, apply,
)

gmpy2.get_context().precision = 192


# ------------------------------------------------------------ counters
class Ctr:
    __slots__ = ('mul', 'div', 'sel', 'lovasz', 'redi', 'swap', 'trace')

    def __init__(self, keep_trace=False):
        self.mul = self.div = self.sel = 0
        self.lovasz = self.redi = self.swap = 0
        self.trace = hashlib.sha256() if keep_trace else None

    def op(self, *t):
        if self.trace is not None:
            self.trace.update(repr(t).encode())

    def as_dict(self, prefix):
        return {f'{prefix}_mul': self.mul, f'{prefix}_div': self.div,
                f'{prefix}_sel': self.sel, f'{prefix}_lovasz': self.lovasz,
                f'{prefix}_redi': self.redi, f'{prefix}_swap': self.swap}


def _init_state(G, c):
    """Integral Gram-Schmidt data (Cohen): d[0..n], lam[k][j] (j<k). Fixed
    cost for fixed n."""
    n = len(G)
    H = [[int(r == s) for s in range(n)] for r in range(n)]
    d = [0] * (n + 1)
    d[0] = 1
    lam = [[0] * n for _ in range(n)]
    for k in range(n):
        for j in range(k + 1):
            u = G[k][j]
            for i in range(j):
                u = (d[i + 1] * u - lam[k][i] * lam[j][i]) // d[i]
                c.mul += 2
                c.div += 1
            if j < k:
                lam[k][j] = u
            else:
                d[k + 1] = u
    c.op('init', n)
    return H, d, lam


# ---------------------------------------------------- exact (baseline)
def lll_exact(G, delta=(99, 100)):
    """Cohen Alg. 2.6.7 with op counters (same arithmetic as hdl.lll_gram)."""
    n = len(G)
    c = Ctr()
    H, d, lam = _init_state(G, c)
    dn, dd = delta

    def redi(k, l):
        c.redi += 1
        if 2 * abs(lam[k][l]) > d[l + 1]:
            q = (2 * lam[k][l] + d[l + 1]) // (2 * d[l + 1])
            c.div += 1
            H[k] = [a - q * b for a, b in zip(H[k], H[l])]
            lam[k][l] -= q * d[l + 1]
            for i in range(l):
                lam[k][i] -= q * lam[l][i]
            c.mul += n + 1 + l

    def swapi(k):
        c.swap += 1
        H[k], H[k - 1] = H[k - 1], H[k]
        for j in range(k - 1):
            lam[k][j], lam[k - 1][j] = lam[k - 1][j], lam[k][j]
        lm = lam[k][k - 1]
        Bv = (d[k - 1] * d[k + 1] + lm * lm) // d[k]
        c.mul += 2
        c.div += 1
        for i in range(k + 1, n):
            t = lam[i][k]
            lam[i][k] = (d[k + 1] * lam[i][k - 1] - lm * t) // d[k]
            lam[i][k - 1] = (Bv * t + lm * lam[i][k]) // d[k + 1]
            c.mul += 4
            c.div += 2
        d[k] = Bv

    k = 1
    while k < n:
        redi(k, k - 1)
        c.lovasz += 1
        c.mul += 4
        if dd * d[k + 1] * d[k - 1] < dn * d[k] * d[k] - dd * lam[k][k - 1] ** 2:
            swapi(k)
            k = max(1, k - 1)
        else:
            for l in range(k - 2, -1, -1):
                redi(k, l)
            k += 1
    return H, d, lam, c


# ------------------------------------------------------- fixed schedule
ORDERS = {
    'sweep': (1, 2, 3),       # bubble-sort pass, bottom-up
    'oddeven': (1, 3, 2),     # odd-even transposition (sorting network)
    'updown': (1, 2, 3, 2),   # cocktail pass; 4 positions per tour
}


def _redi_fixed(H, d, lam, k, l, c, n):
    """Always computes the rounded coefficient and always applies it."""
    c.redi += 1
    q = (2 * lam[k][l] + d[l + 1]) // (2 * d[l + 1])
    c.div += 1
    H[k] = [a - q * b for a, b in zip(H[k], H[l])]
    lam[k][l] -= q * d[l + 1]
    for i in range(l):
        lam[k][i] -= q * lam[l][i]
    c.mul += n + 1 + l
    c.op('redi', k, l)


def _cswap_fixed(H, d, lam, k, bit, c, n):
    """Executes the swap arithmetic unconditionally, then keeps old or new
    with an arithmetic select on bit in {0,1}."""
    c.swap += 1
    nb = 1 - bit
    lm = lam[k][k - 1]
    Bv = (d[k - 1] * d[k + 1] + lm * lm) // d[k]
    c.mul += 2
    c.div += 1
    newH_k, newH_k1 = H[k - 1], H[k]
    new_lam = {}
    for j in range(k - 1):
        new_lam[(k, j)] = lam[k - 1][j]
        new_lam[(k - 1, j)] = lam[k][j]
    for i in range(k + 1, n):
        t = lam[i][k]
        a = (d[k + 1] * lam[i][k - 1] - lm * t) // d[k]
        b = (Bv * t + lm * a) // d[k + 1]
        new_lam[(i, k)] = a
        new_lam[(i, k - 1)] = b
        c.mul += 4
        c.div += 2
    # arithmetic select (word-level cmov in a real implementation)
    H[k] = [bit * x + nb * y for x, y in zip(newH_k, H[k])]
    H[k - 1] = [bit * x + nb * y for x, y in zip(newH_k1, H[k - 1])]
    for (r, s), v in new_lam.items():
        lam[r][s] = bit * v + nb * lam[r][s]
    d[k] = bit * Bv + nb * d[k]
    c.sel += 2 * n + len(new_lam) + 1
    c.op('cswap', k)


def _lovasz_bit(d, lam, k, delta, c):
    dn, dd = delta
    c.lovasz += 1
    c.mul += 4
    c.op('lovasz', k)
    return int(dd * d[k + 1] * d[k - 1] < dn * d[k] * d[k] - dd * lam[k][k - 1] ** 2)


def is_lll_reduced(d, lam, delta=(99, 100)):
    n = len(d) - 1
    dn, dd = delta
    for k in range(1, n):
        for l in range(k):
            if 2 * abs(lam[k][l]) > d[l + 1]:
                return False
        if dd * d[k + 1] * d[k - 1] < dn * d[k] * d[k] - dd * lam[k][k - 1] ** 2:
            return False
    return True


def fixed_tours(G, T, order='sweep', delta=(99, 100), on_tour=None,
                keep_trace=True):
    """T tours of the fixed schedule. on_tour(t, H, d, lam) is a measurement
    hook (outside the algorithm) called after every tour t = 1..T."""
    n = len(G)
    c = Ctr(keep_trace=keep_trace)
    H, d, lam = _init_state(G, c)
    ks = ORDERS[order]
    eff_swaps = 0
    for t in range(1, T + 1):
        for k in ks:
            for l in range(k - 1, -1, -1):
                _redi_fixed(H, d, lam, k, l, c, n)
            bit = _lovasz_bit(d, lam, k, delta, c)
            eff_swaps += bit
            _cswap_fixed(H, d, lam, k, bit, c, n)
        if on_tour is not None:
            on_tour(t, H, d, lam)
    return H, d, lam, c, eff_swaps


def per_tour_cost(order, n=4):
    """Op counts of one tour (input independent by construction)."""
    G = [[int(r == s) * 7 + 1 for s in range(n)] for r in range(n)]
    _, _, _, c0, _ = fixed_tours(G, 0, order, keep_trace=False)
    _, _, _, c1, _ = fixed_tours(G, 1, order, keep_trace=False)
    return {k: getattr(c1, k) - getattr(c0, k)
            for k in ('mul', 'div', 'sel', 'lovasz', 'redi', 'swap')}, \
        {k: getattr(c0, k) for k in ('mul', 'div')}


# ------------------------------------------------- enumeration (B1)
def gso_float(d, lam):
    n = len(d) - 1
    bstar = [mpfr(d[j + 1]) / mpfr(d[j]) for j in range(n)]
    mu = [[mpfr(lam[k][j]) / mpfr(d[j + 1]) if j < k else mpfr(0)
           for j in range(n)] for k in range(n)]
    return bstar, mu


def reduced_gram(G, H):
    n = len(G)
    HG = [[sum(H[r][t] * G[t][s] for t in range(n)) for s in range(n)]
          for r in range(n)]
    return [[sum(HG[r][t] * H[s][t] for t in range(n)) for s in range(n)]
            for r in range(n)]


def qform(Gr, x):
    n = len(x)
    return sum(x[r] * Gr[r][s] * x[s] for r in range(n) for s in range(n)
               if x[r] and x[s])


class _Stop(Exception):
    pass


def enum_budget(Gr, d, lam, target, budget, shrink=True):
    """Schnorr-Euchner enumeration (zig-zag DFS, sign symmetry removed) on
    the basis with Gram Gr and integral GS data (d, lam). Radius^2 starts
    at d[1] = ||b1||^2 and shrinks to every shorter vector found.

    Returns dict(hit=first node index at which a nonzero vector of exact
    norm <= target is known (0 if b1 already qualifies, None if not within
    budget), best=shortest exact norm found, nodes=nodes visited (== full
    tree size if < budget), complete=bool).

    A fixed-budget implementation visits exactly `budget` nodes, padding
    with dummy visits after the tree is exhausted; the cost is `budget`."""
    n = len(d) - 1
    bstar, mu = gso_float(d, lam)
    st = dict(nodes=0, best=d[1], hit=0 if d[1] <= target else None,
              R2=mpfr(d[1]) * (1 + mpfr(2) ** -60))
    x = [0] * n

    def rec(k, partial, upper_zero):
        cen = -sum((x[j] * mu[j][k] for j in range(k + 1, n)), mpfr(0))
        if upper_zero:
            seq = _nonneg(cen)
        else:
            seq = _zigzag(cen)
        for xv in seq:
            st['nodes'] += 1
            if st['nodes'] > budget:
                raise _Stop
            nk = partial + (xv - cen) ** 2 * bstar[k]
            if nk > st['R2']:
                if upper_zero:
                    if xv > cen:
                        return
                    continue
                return
            x[k] = xv
            if k == 0:
                if any(x):
                    ex = qform(Gr, x)
                    if 0 < ex < st['best']:
                        st['best'] = ex
                        if shrink:
                            st['R2'] = mpfr(ex) * (1 + mpfr(2) ** -60)
                    if 0 < ex <= target and st['hit'] is None:
                        st['hit'] = st['nodes']
            else:
                rec(k - 1, nk, upper_zero and xv == 0)
            x[k] = 0

    complete = True
    try:
        rec(n - 1, mpfr(0), True)
    except _Stop:
        complete = False
        st['nodes'] = budget
    return dict(hit=st['hit'], best=st['best'], nodes=st['nodes'],
                complete=complete)


def _zigzag(c):
    x0 = int(gmpy2.rint(c))
    yield x0
    step = 1
    up = c >= x0
    while True:
        if up:
            yield x0 + step
            yield x0 - step
        else:
            yield x0 - step
            yield x0 + step
        step += 1


def _nonneg(c):
    # upper coordinates all zero: by sign symmetry only x >= 0 is needed.
    # zig-zag around max(c, 0) restricted to x >= 0 (here c == 0).
    x = 0
    while True:
        yield x
        x += 1


def shortest_norm(G):
    """lambda_1^2 of the lattice with Gram G, via exact LLL + unbounded
    enumeration (dim 4: tiny tree)."""
    H, d, lam, _ = lll_exact(G)
    Gr = reduced_gram(G, H)
    r = enum_budget(Gr, d, lam, 0, 10 ** 7)
    assert r['complete']
    return r['best'], d[1]


# ------------------------------------------------------ sieve (B2)
def sieve_list(n):
    """Fixed list: b_i, and b_i +- b_j (i < j): 4 + 12 = 16 vectors."""
    L = []
    for i in range(n):
        L.append([int(t == i) for t in range(n)])
    for i in range(n):
        for j in range(i + 1, n):
            L.append([int(t == i) + int(t == j) for t in range(n)])
            L.append([int(t == i) - int(t == j) for t in range(n)])
    return L


def sieve_rounds(Gr, target, R):
    """R rounds of all-ordered-pairs reduction v_i -= round(<v_i,v_j>/<v_j,v_j>) v_j
    on the fixed list (always applied; 0-norm partners give coefficient 0
    by select). Returns first round (0 = before any) with a nonzero norm
    <= target, or None, and the best norm per round."""
    n = len(Gr)
    L = sieve_list(n)

    def ip(a, b):
        return sum(a[r] * Gr[r][s] * b[s] for r in range(n) for s in range(n))

    norms = [ip(v, v) for v in L]
    best = min(x for x in norms if x > 0)
    hist = [best]
    hit = 0 if best <= target else None
    m = len(L)
    for r in range(1, R + 1):
        for i in range(m):
            for j in range(m):
                if i == j:
                    continue
                nj = norms[j]
                nz = int(nj != 0)
                num = ip(L[i], L[j])
                den = nj + (1 - nz)
                qq = nz * ((2 * num + den) // (2 * den))
                L[i] = [a - qq * b for a, b in zip(L[i], L[j])]
                norms[i] = ip(L[i], L[i])
        nzn = [x for x in norms if x > 0]
        best = min(nzn) if nzn else best
        hist.append(best)
        if hit is None and best <= target:
            hit = r
    return hit, hist


SIEVE_PAIR_MUL = None  # filled by analysis: per pair, 2 inner products of 16+4 mults + n


# ------------------------------------- Gaussian-integer Lagrange (A-Zi)
# The PRISM ideals here are left ideals of an order containing Z<1,i,j,k>,
# so left multiplication by i maps the lattice to itself and is an
# isometry of the reduced norm. The 4-D lattice is therefore a rank-2
# lattice over the Gaussian integers Z[i], with Hermitian form
# h(u, v) = Z[i]-part of u * conj(v), and h(u, u) = nrd(u). Its HNF basis
# is exactly {T, T*i, alpha, i*alpha} with alpha = x + y*i + j. Lagrange-
# Gauss over the Euclidean ring Z[i] (Napias 1996 treats LLL over
# Euclidean rings) reduces the 2-D Z[i]-basis {T, alpha}.

def herm(v, u, p, c):
    """Z[i]-part of v * conj(u): (h0, h1) with h0 = <v,u>_nrd."""
    v0, v1, v2, v3 = v
    u0, u1, u2, u3 = u
    h0 = v0 * u0 + v1 * u1 + p * (v2 * u2 + v3 * u3)
    h1 = v1 * u0 - v0 * u1 + p * (v3 * u2 - v2 * u3)
    c.mul += 10
    return h0, h1


def _round_q(h, nu, prec):
    """Gaussian-rounding coefficient round(h / nu). prec=None: exact.
    prec=P: computed from the top P bits of nu (and h shifted by the same
    amount), i.e. a bounded-precision quotient as a fixed-width port would
    compute it."""
    if prec is None:
        return (2 * h + nu) // (2 * nu)
    k = max(0, nu.bit_length() - prec)
    hs = h >> k
    ns = nu >> k
    return (2 * hs + ns) // (2 * ns)


def zi_gauss_fixed(T, alpha, p, steps, on_step=None, keep_trace=True, prec=None):
    """Fixed number of Z[i]-Lagrange steps. Each step: Gaussian-rounded
    coefficient mu = round(h(v,u)/nrd(u)) (always applied, possibly 0),
    v <- v - mu*u, then a branchless conditional swap that makes u the
    shorter vector (strict, so a reduced basis is a fixed point)."""
    c = Ctr(keep_trace=keep_trace)
    u = [T, 0, 0, 0]
    v = list(alpha)
    nu = nrd(u, p)
    nv = nrd(v, p)
    c.mul += 10
    # initial conditional swap
    bit = int(nv < nu)
    nb = 1 - bit
    u, v = ([bit * a + nb * b for a, b in zip(v, u)],
            [bit * a + nb * b for a, b in zip(u, v)])
    nu, nv = bit * nv + nb * nu, bit * nu + nb * nv
    c.sel += 10
    c.op('init')
    eff = 0
    for t in range(1, steps + 1):
        h0, h1 = herm(v, u, p, c)
        m0 = _round_q(h0, nu, prec)
        m1 = _round_q(h1, nu, prec)
        c.div += 2
        # mu*u = m0*u + m1*(i*u);  i*(a+bi+cj+dk) = -b + a i - d j + c k
        iu = (-u[1], u[0], -u[3], u[2])
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        c.mul += 8
        nv = nrd(v, p)
        c.mul += 5
        c.op('red', t)
        bit = int(nv < nu)
        nb = 1 - bit
        eff += bit
        u, v = ([bit * a + nb * b for a, b in zip(v, u)],
                [bit * a + nb * b for a, b in zip(u, v)])
        nu, nv = bit * nv + nb * nu, bit * nu + nb * nv
        c.sel += 10
        c.lovasz += 1
        c.swap += 1
        c.op('cswap', t)
        if on_step is not None:
            on_step(t, u, v, nu, nv)
    return u, v, nu, nv, c, eff


ZI_STEP_COST = dict(mul=23, div=2, sel=10)
