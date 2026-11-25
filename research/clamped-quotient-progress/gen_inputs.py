# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Input generators for the empirical width check.

sample_prism_ideal, tonelli_shanks and make_rolletschek are copied
unchanged in substance from research/lagrange-tour-bound/run_bound_study.py,
and inter_prereduced_zi from research/keygen-precompute/kpc.py (op counter
removed), so that this folder runs on its own.
"""
import math

import gmpy2

from q5common import N4, herm, zi_mul


def tonelli_shanks(n, p):
    n %= p
    if n == 0:
        return 0
    if p % 4 == 3:
        return int(gmpy2.powmod(n, (p + 1) // 4, p))
    Q, S = p - 1, 0
    while Q % 2 == 0:
        Q //= 2; S += 1
    z = 2
    while gmpy2.powmod(z, (p - 1) // 2, p) != p - 1:
        z += 1
    M, c, t, R = S, gmpy2.powmod(z, Q, p), gmpy2.powmod(n, Q, p), gmpy2.powmod(n, (Q + 1) // 2, p)
    while t != 1:
        i, t2 = 1, (t * t) % p
        while t2 != 1:
            t2 = (t2 * t2) % p; i += 1
        b = gmpy2.powmod(c, 1 << (M - i - 1), p)
        c = (b * b) % p; M = i; t = (t * c) % p; R = (R * b) % p
    return int(R)


def sample_q(e, p, rng, smallest=False):
    """q = 2^(2e-5) - s^2, s odd, q prime, -p a square mod q."""
    q_base = 1 << (2 * e - 5)
    s = 1 if smallest else rng.randrange(1, 1 << 20) | 1
    while True:
        q = q_base - s * s
        if q > 2 and gmpy2.is_prime(q) and gmpy2.legendre(-p % q, q) == 1:
            return q, s
        s = s + 2 if smallest else rng.randrange(1, 1 << 20) | 1


def sample_N(p, B, rng, at_bound=False, largest=False, avoid=()):
    if largest:
        N = (1 << B) - 1
        while True:
            if gmpy2.is_prime(N) and gmpy2.legendre(-p % N, N) == 1 and N not in avoid:
                return N
            N -= 2
    lo = (1 << (B - 1)) if at_bound else (1 << max(20, B - 32))
    while True:
        N = rng.randrange(lo, 1 << B) | 1
        if gmpy2.is_prime(N) and N not in avoid and gmpy2.legendre(-p % N, N) == 1:
            return N


def root_pair(M, p, rng):
    """(x, y) with M | x^2 + y^2 + p, M an odd prime."""
    while True:
        y = rng.randrange(M)
        r = (-(y * y + p)) % M
        if r != 0 and gmpy2.legendre(r, M) == 1:
            break
    x = tonelli_shanks(r, M)
    if rng.getrandbits(1):
        x = M - x
    return x, y


def crt_pair(N, xN, yN, q, xq, yq):
    inv = int(gmpy2.invert(N, q))
    return (xN + N * (((xq - xN) * inv) % q), yN + N * (((yq - yN) * inv) % q))


def model_basis(T, x, y):
    return [T, 0, 0, 0], [x, y, 1, 0]


def o0_basis(T, x, y):
    """Model ideal {T, x + y i + j} -> the O0 ideal O0 I (same odd norm T):
    Z[i]-basis numerators (2T,0,0,0), (2c0, 2c1+1, 1, 0), c = (x + (y-1) i)/2 mod T."""
    inv2 = (T + 1) // 2
    c0 = (x * inv2) % T
    c1 = ((y - 1) * inv2) % T
    return [2 * T, 0, 0, 0], [2 * c0, 2 * c1 + 1, 1, 0]


def make_rolletschek(target, mode="conj_alt"):
    wp0, wp1, wc0, wc1, k = 0, 0, 1, 0, 0
    while True:
        if mode == "conj_alt":
            m0, m1 = (1, 1) if (k % 2 == 0) else (1, -1)
            wn0 = (m0 * wc0 - m1 * wc1) + wp0
            wn1 = (m0 * wc1 + m1 * wc0) + wp1
        else:
            wn0 = (wc0 - wc1) - wp1
            wn1 = (wc0 + wc1) + wp0
        if wn0 * wn0 + wn1 * wn1 >= target:
            break
        wp0, wp1, wc0, wc1 = wc0, wc1, wn0, wn1
        k += 1
    T = wc0 * wc0 + wc1 * wc1
    g = (wp0 * wc0 + wp1 * wc1) % T
    h = (wp1 * wc0 - wp0 * wc1) % T
    return T, g, h, k


def exact_lagrange(u, v, p, max_steps=100000):
    """Variable-time exact Z[i]-Lagrange to the reduced fixed point (keygen
    pre-reduction, and the lambda_1 oracle)."""
    u = list(u); v = list(v)
    nu, nv = N4(u, p), N4(v, p)
    if nv < nu:
        u, v, nu, nv = v, u, nv, nu
    for steps in range(1, max_steps):
        h0, h1 = herm(v, u, p)
        m0 = (2 * h0 + nu) // (2 * nu)
        m1 = (2 * h1 + nu) // (2 * nu)
        if m0 == 0 and m1 == 0 and nu <= nv:
            return u, v, nu, nv, steps
        iu = [-u[1], u[0], -u[3], u[2]]
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        nv = N4(v, p)
        if nv < nu:
            u, v, nu, nv = v, u, nv, nu
    raise RuntimeError('no convergence')


def _cmod(x, q):
    x %= q
    return x - q if x > q // 2 else x


def inter_prereduced_zi(u_sec, v_sec, xq, yq, q):
    """Z[i]-basis {w1, w2} of I_sec cap I_q from a reduced basis of I_sec
    (keygen-precompute EXP_RED_O0). Also returns the largest setup value."""
    big = 0
    def t(*xs):
        nonlocal big
        for x in xs:
            big = max(big, abs(x))
    a = [u_sec[2] * xq, u_sec[3] * yq, u_sec[2] * yq, u_sec[3] * xq,
         v_sec[2] * xq, v_sec[3] * yq, v_sec[2] * yq, v_sec[3] * xq]
    t(*a)
    pre = [u_sec[0] - (a[0] - a[1]), u_sec[1] - (a[2] + a[3]),
           v_sec[0] - (a[4] - a[5]), v_sec[1] - (a[6] + a[7])]
    t(*pre)
    ru0, ru1, rv0, rv1 = [x % q for x in pre]
    nrm_rv = (rv0 * rv0 + rv1 * rv1); t(nrm_rv); nrm_rv %= q
    if nrm_rv != 0:
        inv = int(gmpy2.invert(nrm_rv, q))
        t0 = -(ru0 * rv0 + ru1 * rv1); t1 = -(ru1 * rv0 - ru0 * rv1); t(t0, t1)
        t0 %= q; t1 %= q
        g0 = _cmod(t0 * inv, q); g1 = _cmod(t1 * inv, q); t(t0 * inv, t1 * inv)
        w1 = [q * x for x in v_sec]
        gv = zi_mul(g0, g1, v_sec)
        w2 = [u_sec[r] + gv[r] for r in range(4)]
        t(*w1, *gv, *w2)
        return w1, w2, big
    nrm_ru = (ru0 * ru0 + ru1 * ru1) % q
    inv = int(gmpy2.invert(nrm_ru, q))
    t0 = (-(rv0 * ru0 + rv1 * ru1)) % q; t1 = (-(rv1 * ru0 - rv0 * ru1)) % q
    g0 = _cmod(t0 * inv, q); g1 = _cmod(t1 * inv, q)
    w1 = [q * x for x in u_sec]
    gu = zi_mul(g0, g1, u_sec)
    w2 = [v_sec[r] + gu[r] for r in range(4)]
    t(*w1, *gu, *w2)
    return w1, w2, big


def two_squares_prime(r):
    """r prime, r = 1 mod 4 -> (a, b) with a^2 + b^2 = r (Cornacchia)."""
    x = tonelli_shanks(r - 1, r)
    a, b = r, x
    lim = math.isqrt(r)
    while b > lim:
        a, b = b, a % b
    c2 = r - b * b
    c = math.isqrt(c2)
    assert c * c == c2
    return b, c


def crafted_skew_key(N, p, k_target):
    """A generic-shape model ideal of norm N containing beta = a + b i + j with
    nrd(beta) = k N, k the first value >= k_target with k N - p a prime = 1 mod 4
    (so lambda_1^2 <= k N). The most skewed keys (k ~ p/N) fail the keygen
    guard nrd(v_s) <= 2 sqrt(p) N; k ~ sqrt(p)/2 sits at it."""
    k = max(k_target, (p // N) + 1)
    while True:
        r = k * N - p
        if r > 0 and r % 4 == 1 and gmpy2.is_prime(r):
            a, b = two_squares_prime(r)
            x, y = a % N, b % N
            assert (x * x + y * y + p) % N == 0
            return x, y, k
        k += 1
