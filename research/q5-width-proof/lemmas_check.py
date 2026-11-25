# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Machine checks of the lemmas of REPORT.md section 3.

  L1  update identity        (sympy, symbolic)
  L2  Bessel / Cauchy-Schwarz (sympy, symbolic sum-of-squares identity)
  L2' determinant invariance (sympy)
  L3a rounding sign lemma    m (m - 2r) <= 0 for half-up rounding (exhaustive grid)
  L3  quasi-monotone step    N4(v') <= N4(v) + eps M, with the exact port of
                             the Rust quotient at toy precisions PREC = 6, 8, 10
                             (eps = 12 d / (1-d)^2, d = 2^-(PREC-1)) and at 64,
                             over random and saturation-regime inputs
  L5  O0 numerators: N4 = 0 mod 4; generic ideals {2T, (x,y,1,0)} are left
      O0-modules; q = 3 mod 4 so I_q cap Z[i] = q Z[i]; D mod 4 per set
Writes results/lemmas_check.txt. Exit status 1 on any failure.
"""
import os
import random
import sys
from fractions import Fraction as F

import sympy as sp

from q5common import SETS, params, N4, herm, times_i, zi_mul

HERE = os.path.dirname(os.path.abspath(__file__))
LOG = []
FAIL = []


def log(*a):
    s = ' '.join(str(x) for x in a)
    print(s)
    LOG.append(s)


def check(name, ok):
    log(('PASS ' if ok else 'FAIL ') + name)
    if not ok:
        FAIL.append(name)


def symbolic():
    p, m0, m1 = sp.symbols('p m0 m1')
    u = sp.symbols('u0:4'); v = sp.symbols('v0:4')
    n4 = lambda x: x[0]**2 + x[1]**2 + p*(x[2]**2 + x[3]**2)
    def hm(x, y):
        return (x[0]*y[0] + x[1]*y[1] + p*(x[2]*y[2] + x[3]*y[3]),
                x[1]*y[0] - x[0]*y[1] + p*(x[3]*y[2] - x[2]*y[3]))
    iu = (-u[1], u[0], -u[3], u[2])
    vp = [v[r] - m0*u[r] - m1*iu[r] for r in range(4)]
    h0, h1 = hm(v, u)
    A, C = n4(u), n4(v)
    check('L1 N4(v - m0 u - m1 iu) = N4(v) - 2(m0 h0 + m1 h1) + (m0^2+m1^2) N4(u)',
          sp.expand(n4(vp) - (C - 2*(m0*h0 + m1*h1) + (m0**2 + m1**2)*A)) == 0)
    check('L1b N4(iu) = N4(u), herm(iu,u) = (0, N4(u))',
          sp.expand(n4(iu) - A) == 0 and sp.expand(hm(iu, u)[0]) == 0 and sp.expand(hm(iu, u)[1] - A) == 0)
    w = [A*v[r] - h0*u[r] - h1*iu[r] for r in range(4)]
    check('L2 N4(A v - h0 u - h1 iu) = A (A C - h0^2 - h1^2)  [so h0^2+h1^2 <= A C]',
          sp.expand(n4(w) - A*(A*C - h0**2 - h1**2)) == 0)
    hp0, hp1 = hm(vp, u)
    check('L2b herm(v - mu u, u) = herm(v,u) - mu N4(u)',
          sp.expand(hp0 - (h0 - m0*A)) == 0 and sp.expand(hp1 - (h1 - m1*A)) == 0)
    Cp = n4(vp)
    check("L2' Delta = A C - |h|^2 invariant under the update",
          sp.expand((A*Cp - hp0**2 - hp1**2) - (A*C - h0**2 - h1**2)) == 0)
    # two-term Cauchy-Schwarz for the partial sums (Lagrange identity)
    a, b, c, d = sp.symbols('a b c d')
    check('L2c (ac+bd)^2 + (ad-bc)^2 = (a^2+b^2)(c^2+d^2)',
          sp.expand((a*c + b*d)**2 + (a*d - b*c)**2 - (a**2 + b**2)*(c**2 + d**2)) == 0)


def rounding_sign():
    bad = 0; n = 0
    for den in range(1, 41):
        for num in range(-200, 201):
            r = F(num, den)
            m = (2 * num + den) // (2 * den)    # floor(r + 1/2)
            n += 1
            if m * (m - 2 * r) > 0 or abs(m - r) > F(1, 2) or abs(m) > abs(r) + F(1, 2):
                bad += 1
    check(f'L3a half-up rounding: m(m-2r) <= 0, |m-r| <= 1/2 on {n} rationals', bad == 0)


def quotient_toy(h, nu, P):
    """zi.rs quotient() with precision P: hs clamp 2^(2P-4), m clamp 2^(P-2),
    'fits' = quotient < 2^P. P = 64 is exactly the Rust code."""
    k = max(0, nu.bit_length() - P)
    hs = h >> k; ns = nu >> k
    lim = 1 << (2 * P - 4)
    if not (-lim <= hs < lim):
        hs = lim if hs >= 0 else -lim
    num = 2 * hs + ns; den = 2 * ns
    a = abs(num)
    if num < 0:
        a += den - 1
    fits = (a >> P) < den
    qa = a // den
    if not fits or qa >= (1 << (P - 2)):
        qa = 1 << (P - 2)
    return -qa if num < 0 else qa


def quasi_monotone(P, trials, rng, p):
    d = F(1, 2 ** (P - 1))
    eps = 12 * d / (1 - d) ** 2
    worst = F(0); nsat = 0; nviol = 0
    for t in range(trials):
        regime = t % 4
        sz = rng.choice([P + 4, 2 * P, 4 * P, 8 * P])
        u = [rng.randrange(-(1 << sz), 1 << sz) for _ in range(4)]
        if regime == 1:            # tiny u, huge v: forces the saturation paths
            u = [rng.randrange(-3, 4) for _ in range(4)]
            if all(x == 0 for x in u):
                u[0] = 1
        if regime == 2:            # near-tie quotients
            v = [x * rng.randrange(1, 1 << P) // 2 + rng.randrange(-2, 3) for x in u]
        else:
            v = [rng.randrange(-(1 << (2 * sz)), 1 << (2 * sz)) for _ in range(4)]
        if regime == 3:            # v nearly parallel to u
            c0, c1 = rng.randrange(-(1 << 40), 1 << 40), rng.randrange(-(1 << 40), 1 << 40)
            v = [a + b for a, b in zip(zi_mul(c0, c1, u), [rng.randrange(-9, 10) for _ in range(4)])]
        if t % 5 == 4:             # targeted half-integer ties: h0/A just off n + 1/2
            import math
            while True:
                a0 = rng.randrange(1 << (P + 6), 1 << (P + 30)); a1 = rng.randrange(1, 1 << (P + 6))
                if math.gcd(a0, a1) == 1:
                    break
            u = [a0, a1, 0, 0]
            A_ = a0 * a0 + a1 * a1
            n_ = rng.randrange(-40, 41)
            H = (A_ * (2 * n_ + 1)) // 2 - rng.randrange(0, (A_ >> (P - 2)) + 2) * (1 if n_ >= 0 else -1)
            s_, t_, g = sp.gcdex(a0, a1)
            assert g == 1 and s_ * a0 + t_ * a1 == 1
            v0, v1 = int(s_) * H, int(t_) * H
            k_ = (v1 * a0 - v0 * a1) // A_       # minimise |v| along (a1, -a0)
            v = [v0 + k_ * a1, v1 - k_ * a0, 0, 0]
        A, C = N4(u, p), N4(v, p)
        if A == 0:
            continue
        h0, h1 = herm(v, u, p)
        m0, m1 = quotient_toy(h0, A, P), quotient_toy(h1, A, P)
        if abs(m0) == 1 << (P - 2) or abs(m1) == 1 << (P - 2):
            nsat += 1
        vp = [v[r] - m0 * u[r] - m1 * times_i(u)[r] for r in range(4)]
        Cp = N4(vp, p)
        M = max(A, C)
        ex = F(Cp - C, M)
        if ex > worst:
            worst = ex
        if Cp > C + eps * M:
            nviol += 1
        # product bound used by Lemma 4: |m_j| <= (1+2d)|q_j| + 1/2 + 2d
        for hj, mj in ((h0, m0), (h1, m1)):
            if abs(mj) > (1 + 2 * d) * abs(F(hj, A)) + F(1, 2) + 2 * d:
                nviol += 1
    check(f'L3 PREC={P}: N4(v\') <= N4(v) + eps M, eps=2^{float(sp.log(eps, 2)):.2f}, '
          f'{trials} trials ({nsat} clamped), worst excess/M = {float(worst):.3e}, bound {float(eps):.3e}', nviol == 0)


def o0_checks(rng):
    # quaternion product on numerator vectors (x/2)(y/2) = xy/4 -> numerator xy/2
    def qmul(x, y, p):
        a0, a1, a2, a3 = x; b0, b1, b2, b3 = y
        r = [a0*b0 - a1*b1 - p*a2*b2 - p*a3*b3,
             a0*b1 + a1*b0 + p*a2*b3 - p*a3*b2,
             a0*b2 - a1*b3 + a2*b0 + a3*b1,
             a0*b3 + a1*b2 - a2*b1 + a3*b0]
        assert all(c % 2 == 0 for c in r)
        return [c // 2 for c in r]

    def member(x, u, v):
        # x = c u + d v over Z[i]; u = (2T,0,0,0), v = (x', y', 1, 0)
        dr, di = x[2], x[3]
        rem = [x[r] - zi_mul(dr, di, v)[r] for r in range(4)]
        return rem[2] == 0 and rem[3] == 0 and rem[0] % u[0] == 0 and rem[1] % u[0] == 0

    import gmpy2
    ok_mod4 = True; ok_ideal = True; nid = 0
    for name, e, f, *_ in SETS:
        P = params(name); p = P['p']
        for _ in range(20):
            x = [rng.randrange(-(1 << 300), 1 << 300) for _ in range(4)]
            x[3] = x[0] + 2 * rng.randrange(-(1 << 200), 1 << 200)   # x0 = x3 mod 2
            x[2] = x[1] + 2 * rng.randrange(-(1 << 200), 1 << 200)   # x1 = x2 mod 2
            ok_mod4 &= N4(x, p) % 4 == 0
        # generic O0 ideals of prime norm T: v = (x', y', 1, 0), x' even, y' odd
        for _ in range(10):
            T = int(gmpy2.next_prime(rng.randrange(1 << 60, 1 << 61)))
            while True:
                y = rng.randrange(T)
                r = (-(y * y + p)) % T
                if gmpy2.legendre(r, T) == 1:
                    break
            x = int(gmpy2.iroot(0, 1)[0])  # placeholder
            # sqrt mod T
            from sympy.ntheory import sqrt_mod
            x = int(sqrt_mod(r, T))
            inv2 = (T + 1) // 2
            c0 = (x * inv2) % T; c1 = ((y - 1) * inv2) % T
            u = [2 * T, 0, 0, 0]; v = [2 * c0, 2 * c1 + 1, 1, 0]
            omega = [0, 1, 1, 0]; one_k = [1, 0, 0, 1]
            closed = all(member(qmul(g, w, p), u, v) for g in (omega, one_k) for w in (u, v))
            ok_ideal &= closed and N4(v, p) % (4 * T) == 0 and not member([2, 0, 0, 0], u, v)
            nid += 1
    check('L5a O0 numerators (x0=x3, x1=x2 mod 2): N4 = 4 nrd is divisible by 4', ok_mod4)
    check(f'L5b {nid} generic O0 ideals {{2T,(x\',y\',1,0)}} are left O0-modules of norm T', ok_ideal)
    qs = []
    for name, e, f, *_ in SETS:
        for s in (1, 3, 5, 7, 12345, 99999):
            if s % 2:
                qs.append(((1 << (2 * e - 5)) - s * s) % 4)
    check('L5c q = 2^(2e-5) - s^2 with s odd is 3 mod 4 (inert in Z[i]) at every set', set(qs) == {3})
    for name, e, f, W, *_ in SETS:
        D = int(gmpy2.next_prime(1 << (2 * W)))
        log(f'     {name}: D = nextprime(2^{2*W}) = 2^{2*W} + {D - (1 << (2*W))}, D mod 4 = {D % 4}')


def main():
    rng = random.Random(0x5155)
    symbolic()
    rounding_sign()
    p = params('p248')['p']
    for P_, n in ((6, 60000), (8, 60000), (10, 40000), (64, 20000)):
        quasi_monotone(P_, n, rng, p if P_ == 64 else 103)
    o0_checks(rng)
    open(os.path.join(HERE, 'results', 'lemmas_check.txt'), 'w').write('\n'.join(LOG) + '\n')
    log('FAILURES:', FAIL if FAIL else 'none')
    return 1 if FAIL else 0


if __name__ == '__main__':
    sys.exit(main())
