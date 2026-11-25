# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Python model of the normalized class Gram form for the exp64 reducer.

The native signer stores an ideal basis as O0 numerators over two
(SIGNER_SPEC 2.3). For a basis U, V of an integral O0 left ideal of norm T
the numerator Gram entries A = N4(U), C = N4(V), h0 = D(V, U),
h1 = D(V, JU) are all multiples of 2T (the diagonal ones of 4T), and the
numerator Hermitian determinant is A C - h0^2 - h1^2 = 4 p T^2. Dividing
all four entries by 2T gives an integral Gram with determinant exactly p
(Rick, research/reduction-alternatives, a60fd06). This module:

  * builds that normalized Gram for every admitted input class
    (`normalize`, `gram_hnf`);
  * runs exp64 with the carried Gram on either representation, recording
    the physical decisions (mantissa, shift, big, swap) of every step and
    the logical exact steps they compose to (`run_gram`);
  * replays the reducer's arithmetic in the order of the Rust code to
    record the largest magnitude of every intermediate (`run_gram(...,
    rec=...)`), for the width bounds in bounds_ng.py.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, 'vendor'))

from q5common import N4, herm, params, SETS  # noqa: E402
from variants import q_expw, exact_round  # noqa: E402

T0 = 60
NAMES = [s[0] for s in SETS]

# SIGNER_SPEC 2.3 at PR #45 head 84a798a (abed5c1 has the same table):
# native O0 counts, exact / exp64 any-basis, both including the final size
# reduction after the last swap.
EXACT = {
    'sign-generic':    (301, 387, 447, 587, 774),
    'prered-guard256': (287, 373, 432, 573, 759),
    'keygen-D':        (223, 347, 331, 442, 686),
    'keygen-Isec':     (22, 22, 23, 22, 23),
    'challenge':       (212, 276, 319, 425, 565),
}
EXP64 = {
    'sign-generic':    (312, 401, 463, 609, 803),
    'prered-guard256': (297, 387, 448, 594, 787),
    'keygen-D':        (231, 360, 344, 459, 712),
    'keygen-Isec':     (24, 25, 26, 26, 29),
    'challenge':       (220, 287, 331, 442, 587),
}


def counts(cls, name):
    i = NAMES.index(name)
    return EXACT[cls][i], EXP64[cls][i]


def bl(x):
    return abs(x).bit_length()


def times_i(u):
    return [-u[1], u[0], -u[3], u[2]]


def num_gram(u, v, p):
    """Numerator Gram (A, C, h0, h1) of the ordered pair (u, v)."""
    h0, h1 = herm(v, u, p)
    return N4(u, p), N4(v, p), h0, h1


class NotIntegral(ValueError):
    pass


def normalize(u, v, p, T):
    """Normalized class Gram of an O0 basis of a norm-T ideal: every
    numerator entry divided by 2T. Raises NotIntegral if a division is not
    exact or the diagonal is not a multiple of 4T, or if the result does not
    have determinant p."""
    A, C, h0, h1 = num_gram(u, v, p)
    if A % (4 * T) or C % (4 * T) or h0 % (2 * T) or h1 % (2 * T):
        raise NotIntegral('nonintegral class Gram')
    g = (A // (2 * T), C // (2 * T), h0 // (2 * T), h1 // (2 * T))
    if g[0] * g[1] - g[2] ** 2 - g[3] ** 2 != p:
        raise NotIntegral('determinant is not p')
    return g


def gram_hnf(T, x, y, p):
    """The generic HNF shortcut: U = (2T,0,0,0), V = (x,y,1,0) gives
    (2T, (x^2+y^2+p)/(2T), x, y). Same checks as `normalize`."""
    n = x * x + y * y + p
    if T % 2 == 0 or n % (4 * T):
        raise NotIntegral('nonintegral class Gram')
    return (2 * T, n // (2 * T), x, y)


def o0_lift(t, x, y):
    """Rick's lift (check_native.py at a60fd06): residues mod T to the O0
    parities in [0, 2T): x even, y odd."""
    return [2 * t, 0, 0, 0], [x + (x % 2) * t, y + ((y + 1) % 2) * t, 1, 0]


def fnv(s):
    h = 0xcbf29ce484222325
    for ch in s.encode():
        h ^= ch
        h = (h * 0x100000001b3) & 0xffffffffffffffff
    return h


class Rec(dict):
    """Largest bit length per intermediate category."""

    def put(self, key, val):
        b = bl(val)
        if b > self.get(key, 0):
            self[key] = b


def quotient_rec(h0, h1, A, rec):
    """The Rust quotient_exp (a60fd06) value by value: shared truncated
    division, window correction and undershoot, both paths always."""
    bla = bl(A)
    ka = max(bla - 64, 0)
    ns = A >> ka
    e = max(bl(h0), bl(h1)) - bla
    big = e > T0
    s = e - T0 if big else 0
    rec.put('two_A', 2 * A)
    out = []
    for h in (h0, h1):
        a = abs(h)
        sg = -1 if h < 0 else 1
        num = a >> (ka + s)
        assert (num >> 64) < ns and num < (1 << 128)
        qtop = num // ns
        assert qtop < (1 << 64)
        rec.put('qtop', qtop)
        mw = sg * qtop
        mn = mw * A                       # mul_scalar(nu, mw), both paths
        rec.put('mA_est', mn)
        x = h; rec.put('X', x)
        x = x + h; rec.put('X', x)
        x = x + A; rec.put('X', x)
        x = x - mn; rec.put('X', x)
        x = x - mn; rec.put('X', x)
        t = x - 2 * A; rec.put('X', t)
        mwin = mw - (1 if x < 0 else 0) + (1 if t >= 0 else 0)
        mag = qtop - 1 if qtop >= 2 else 0
        mbig = sg * mag
        out.append(mbig if big else mwin)
    return out[0], out[1], s, int(big)


def run_gram(u, v, G, steps, rec=None, coords=True):
    """exp64 with the carried Gram from the ordered-or-not pair (u, v) and
    its Gram G = (A, C, h0, h1) in any integer scaling. Initial strict swap,
    then `steps` physical steps. Returns dict with the final pair and Gram,
    the physical trace [(m0, m1, s, big, swap)] (m_j mantissas), and the
    logical trace [(M0, M1, swap)] of composed exact steps."""
    A, C, h0, h1 = G
    u = list(u); v = list(v)
    if C < A:
        u, v, A, C, h1 = v, u, C, A, -h1
    phys = []
    logical = []
    acc0 = acc1 = 0
    for _ in range(steps):
        if rec is not None:
            m0, m1, s, big = quotient_rec(h0, h1, A, rec)
        else:
            info = {}
            q0, q1, fl = q_expw(h0, h1, A, 64, T0, info)
            s = info['s'] if fl else 0
            m0, m1, big = q0 >> s if q0 >= 0 else -((-q0) >> s), q1 >> s if q1 >= 0 else -((-q1) >> s), int(fl)
        f0, f1 = m0 << s if m0 >= 0 else -((-m0) << s), m1 << s if m1 >= 0 else -((-m1) << s)
        # carried update, in the Rust order
        if rec is not None:
            rec.put('mA', m0 * A); rec.put('mA', m1 * A)
            rec.put('mA_shift', f0 * A); rec.put('mA_shift', f1 * A)
        r0 = h0 - f0 * A
        r1 = h1 - f1 * A
        if rec is not None:
            rec.put('r', r0); rec.put('r', r1)
            rec.put('h_plus_r', h0 + r0); rec.put('h_plus_r', h1 + r1)
            rec.put('m_hr', m0 * (h0 + r0)); rec.put('m_hr', m1 * (h1 + r1))
            rec.put('m_hr_shift', f0 * (h0 + r0)); rec.put('m_hr_shift', f1 * (h1 + r1))
        c1 = C - f0 * (h0 + r0)
        C2 = c1 - f1 * (h1 + r1)
        assert f0 * (h0 + r0) >= 0 and f1 * (h1 + r1) >= 0
        if rec is not None:
            rec.put('C_partial', c1); rec.put('C', C2)
        if coords:
            iu = times_i(u)
            if rec is not None:
                for r in range(4):
                    rec.put('coord_prod', m0 * u[r]); rec.put('coord_prod', m1 * iu[r])
                    pre = m0 * u[r] + m1 * iu[r]
                    rec.put('coord_pre', pre); rec.put('coord_shift', pre << s if pre >= 0 else -((-pre) << s))
            v = [v[r] - f0 * u[r] - f1 * iu[r] for r in range(4)]
            if rec is not None:
                for r in range(4):
                    rec.put('coord', v[r])
        sw = C2 < A
        assert not (big and sw), 'a big sub-step swapped'
        if sw:
            u, v, A, C, h0, h1 = v, u, C2, A, r0, -r1
        else:
            C, h0, h1 = C2, r0, r1
        if rec is not None:
            rec.put('A', A); rec.put('C', C); rec.put('h', h0); rec.put('h', h1)
        phys.append((m0, m1, s, big, int(sw)))
        acc0 += f0; acc1 += f1
        if not big:
            logical.append((acc0, acc1, int(sw)))
            acc0 = acc1 = 0
    return dict(u=u, v=v, G=(A, C, h0, h1), phys=phys, logical=logical, nbig=sum(t[3] for t in phys),
                pending=(acc0, acc1))


def run_exact(u, v, p, steps):
    """The contract's exact oracle, recomputing the Gram from coordinates."""
    u = list(u); v = list(v)
    nu, nv = N4(u, p), N4(v, p)
    if nv < nu:
        u, v, nu, nv = v, u, nv, nu
    logical = []
    for _ in range(steps):
        h0, h1 = herm(v, u, p)
        m0, m1 = exact_round(h0, nu), exact_round(h1, nu)
        iu = times_i(u)
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        nv = N4(v, p)
        sw = nv < nu
        if sw:
            u, v, nu, nv = v, u, nv, nu
        logical.append((m0, m1, int(sw)))
    return dict(u=u, v=v, logical=logical)


def exact_steps_needed(u, v, p, cap=100000):
    """Variable-time exact run to the fixed point: number of steps after
    the initial swap that change the pair (excludes the final checking
    step)."""
    u = list(u); v = list(v)
    nu, nv = N4(u, p), N4(v, p)
    if nv < nu:
        u, v, nu, nv = v, u, nv, nu
    for k in range(cap):
        h0, h1 = herm(v, u, p)
        m0, m1 = exact_round(h0, nu), exact_round(h1, nu)
        if m0 == 0 and m1 == 0 and nu <= nv:
            return k
        iu = times_i(u)
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        nv = N4(v, p)
        if nv < nu:
            u, v, nu, nv = v, u, nv, nu
    raise RuntimeError('no fixed point')


def trim(logical):
    out = list(logical)
    while out and out[-1] == (0, 0, 0):
        out.pop()
    return out


def phys_digest(phys):
    return fnv(''.join(f'{a},{b},{s},{g},{w};' for a, b, s, g, w in phys))


def logical_digest(logical):
    return fnv(''.join(f'{a},{b},{w};' for a, b, w in trim(logical)))


def is_reduced(u, v, G):
    A, C, h0, h1 = G
    return A <= C and all(-A <= 2 * h < A for h in (h0, h1))
