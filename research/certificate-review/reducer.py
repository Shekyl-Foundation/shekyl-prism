# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""SHE-24: an independent implementation of the reducer contract, written
from SIGNER_SPEC (PR #45) section "Exact step and terminal state" and the
exp64 paragraph.  Coordinate form (4-vectors, numerator units) and Gram
form (A, C, h0, h1) with the same arithmetic."""


def D(p, X, Y):
    return X[0] * Y[0] + X[1] * Y[1] + p * (X[2] * Y[2] + X[3] * Y[3])


def J(X):
    return (-X[1], X[0], -X[3], X[2])


def half_up(h, A):
    """floor(h/A + 1/2) = floor((2h + A)/(2A)), A > 0."""
    return (2 * h + A) // (2 * A)


# ------------------------------------------------------------ coordinates --
def order(p, U, V):
    if D(p, V, V) < D(p, U, U):
        return V, U
    return U, V


def coord_step(p, U, V, mfun=None):
    A = D(p, U, U)
    h0, h1 = D(p, V, U), D(p, V, J(U))
    if mfun is None:
        m0, m1 = half_up(h0, A), half_up(h1, A)
    else:
        m0, m1 = mfun(h0, h1, A)
    JU = J(U)
    V = tuple(V[i] - m0 * U[i] - m1 * JU[i] for i in range(4))
    swapped = D(p, V, V) < A
    if swapped:
        U, V = V, U
    return U, V, (m0, m1), swapped


# ------------------------------------------------------------------ Gram --
def gram(p, U, V):
    return (D(p, U, U), D(p, V, V), D(p, V, U), D(p, V, J(U)))


def gram_order(G):
    A, C, h0, h1 = G
    if C < A:
        return (C, A, h0, -h1)
    return G


def gram_apply(G, m0, m1):
    A, C, h0, h1 = G
    C2 = C - 2 * (m0 * h0 + m1 * h1) + (m0 * m0 + m1 * m1) * A
    h0, h1 = h0 - m0 * A, h1 - m1 * A
    if C2 < A:                                    # strict swap
        return (C2, A, h0, -h1), True
    return (A, C2, h0, h1), False


def exact_m(h0, h1, A):
    return half_up(h0, A), half_up(h1, A)


def bl(x):
    return abs(x).bit_length()


def exp64_m(h0, h1, A, T0=60, W=64):
    """exp64 per SIGNER_SPEC: returns (m0, m1, path) with path 'window'/'big'."""
    k = max(0, A.bit_length() - W)
    n = A >> k
    e = max(bl(h0), bl(h1)) - A.bit_length()
    if e <= T0:
        out = []
        for h in (h0, h1):
            mt = (2 * (h >> k) + n) // (2 * n)          # floor shift on signed h
            X = 2 * h + A - 2 * mt * A
            mt = mt - (1 if X < 0 else 0) + (1 if X >= 2 * A else 0)
            out.append(mt)
        return out[0], out[1], 'window'
    s = e - T0
    out = []
    for h in (h0, h1):
        qtop = (abs(h) >> (k + s)) // n
        mag = max(qtop - 1, 0) << s
        out.append(mag if h >= 0 else -mag)
    return out[0], out[1], 'big'


def run(G, slots, rule='exact', trace=False):
    """Run a fixed schedule from the ordered Gram G.  Returns
    (final Gram, need, swaps, big_substeps) where need = the first slot count
    after which the state never changes again (fixed point reached), computed
    by running until a no-op slot occurs (bounded by `slots`)."""
    G = gram_order(G)
    need = 0
    swaps = big = 0
    for s in range(1, slots + 1):
        A, C, h0, h1 = G
        if rule == 'exact':
            m0, m1 = exact_m(h0, h1, A)
            path = 'window'
        else:
            m0, m1, path = exp64_m(h0, h1, A)
        if m0 == 0 and m1 == 0 and C >= A:
            return G, need, swaps, big                 # fixed point: no-op slot
        G, sw = gram_apply(G, m0, m1)
        need = s
        swaps += sw
        big += path == 'big'
    return G, None, swaps, big                          # not reached within slots
