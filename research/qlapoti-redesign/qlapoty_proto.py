# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Qlapoty-style norm-equation prototype on PRISM's prepared small ideal.

Research model, not a faithful port. It follows the structure of SQIsign
3.0's IdealToIsogenyNormEquation (spec 2026-09-01, Algorithms 4.141-4.149,
from [DLS26] = Qlapoty, ePrint 2026/1700; see also sqisign-rs
quat/qlapoty.rs) and applies it to the reference's prepared small ideal S
(PRISM_v2's shortest equivalent), restricted to odd n(S):

  * inert generator beta = x + y i + (i+j)/2 in S (S = O0<n, beta>);
  * 'Loop1-lite': alpha0 = (k0 + k1 i) * beta with |k|^2 drawn from
    Qlapoty's interval [ceil((n/2)/l1), floor((5n/2)/l1)] (l1 = squared
    first minimum of the 2-D lattice of beta), k0, k1 of opposite parity
    and coprime, accepted when the 2-D lattice of alpha0 is balanced
    (first minimum^2 >= n/2) and gcd(Tr(alpha0), Tr(i alpha0), n) = 1;
    no DivideMinima branch (the interval is reported instead);
  * Loop2: lambda over a public list (odd integers, or odd primes), the
    congruence lambda * T(s, t) = 2^e - 2 lambda^2 r0 (mod n) solved by
    Babai rounding in the reduced 2-D lattice (covolume n; Qlapoty uses
    4n with built-in parity, measured separately below as an a-posteriori
    parity condition), z = 2k - s^2 - t^2 must be a positive sum of two
    squares with the right parities.

The output is checked algebraically: nrd(b1) + nrd(b2) = 2^e n, b_k in S.
It also records whether PRISM_v2's postcondition on theta (end type /
odd norm) would accept the solution, because the reference's chain
relies on it.

Outcome codes per lambda (as qlr.py, adapted):
  0 gcd(lambda, n) != 1; 2 z <= 0; 3 parity (s = t mod 2, or k even);
  4 z not prime (prime-only Cornacchia) [extended: odd part not prime];
  6 success (with PRISM_v2's postcondition) ; 7 success, postcondition
  would reject; 8 / 9 the same, found only by the extended Cornacchia
  (good primes stripped first, as the reference) when extended=True.
"""
import gmpy2
from gmpy2 import mpz
import cypari2

from qlr import sieve_bucket
from fne import (nrd_num, qmul, conj, normalize, cornacchia_prime,
                 cornacchia_extended, endtype, is_odd_norm, Ctr, rounded_div,
                 dim2_short_basis, rand_bits)

pari = cypari2.Pari()


def o0_coords(v, den):
    """(a0 + a1 i + a2 j + a3 k)/den -> coordinates on 1, i, (i+j)/2, (1+k)/2."""
    c = 2 * v[2] // den
    d = 2 * v[3] // den
    assert 2 * v[2] % den == 0 and 2 * v[3] % den == 0
    b2 = 2 * v[1] - c * den     # 2*den*b
    a2 = 2 * v[0] - d * den
    assert b2 % (2 * den) == 0 and a2 % (2 * den) == 0
    return (a2 // (2 * den), b2 // (2 * den), c, d)


def inert_beta(S):
    """x, y with x + y i + (i+j)/2 in S (odd n), as numerators over 2:
    (2x, 2y + 1, 1, 0). None if S is not inert."""
    n = int(S.norm)
    H = [o0_coords(col, S.den) for col in S.cols]
    M = pari.matrix(2, 4, [int(H[c][2]) for c in range(4)] + [int(H[c][3]) for c in range(4)])
    try:
        w = pari.matsolvemod(M, n, pari('[1,0]~'))
    except Exception:
        return None
    if w == 0:
        return None
    w = [int(w[i]) for i in range(4)]
    co = [sum(w[c] * int(H[c][r]) for c in range(4)) for r in range(4)]
    # element = co0 + co1 i + co2 (i+j)/2 + co3 (1+k)/2 with co2 = 1, co3 = 0 mod n
    # subtract n*O0 parts to make co2 = 1, co3 = 0 exactly, reduce co0, co1 mod n
    q2 = (co[2] - 1) // n
    q3 = co[3] // n
    assert co[2] - 1 == q2 * n and co[3] == q3 * n
    x = co[0] % n
    y = co[1] % n
    beta = (mpz(2 * x), mpz(2 * y + 1), mpz(1), mpz(0))   # numerators over 2
    return beta


def lin_form(alpha, dal, p):
    """T(s, t) = Tr((s + i t) * conj(alpha)) = cs * s + ct * t (integers)."""
    ca = conj(alpha)
    one = qmul((dal, 0, 0, 0), ca, p)
    ii = qmul((0, dal, 0, 0), ca, p)
    # Tr(x) = 2 * real part; elements over dal*dal
    cs = 2 * one[0]
    ct = 2 * ii[0]
    D = dal * dal
    assert cs % D == 0 and ct % D == 0
    return cs // D, ct // D


def lattice_mod(cs, ct, n):
    """Reduced basis of {(s,t) : cs s + ct t = 0 mod n}, columns."""
    g = gmpy2.gcd(gmpy2.gcd(cs, ct), n)
    if g != 1:
        return None
    if gmpy2.gcd(cs, n) == 1:
        x = (-ct * gmpy2.invert(cs, n)) % n          # s = x t
        L = [[n, x], [mpz(0), mpz(1)]]               # columns (n,0), (x,1)
    elif gmpy2.gcd(ct, n) == 1:
        x = (-cs * gmpy2.invert(ct, n)) % n          # t = x s
        L = [[mpz(1), mpz(0)], [x, n]]               # columns (1,x), (0,n)
    else:
        K = pari.matkermod(pari.matrix(1, 2, [int(cs), int(ct)]), int(n))
        G = pari.concat(K, pari.matid(2) * int(n))
        H = pari.mathnf(G)
        L = [[mpz(int(H[0, 0])), mpz(int(H[0, 1]))], [mpz(int(H[1, 0])), mpz(int(H[1, 1]))]]
    c = Ctr()
    # dim2_short_basis takes L[r][c] and returns columns [[b0,a0],[b1,a1]]
    R = dim2_short_basis(L, c)
    return R, c.lat_red_steps


def solve_target(cs, ct, n, tgt):
    """A particular (s, t) with cs s + ct t = tgt mod n."""
    if gmpy2.gcd(cs, n) == 1:
        return (tgt * gmpy2.invert(cs, n)) % n, mpz(0)
    if gmpy2.gcd(ct, n) == 1:
        return mpz(0), (tgt * gmpy2.invert(ct, n)) % n
    w = pari.matsolvemod(pari.matrix(1, 2, [int(cs), int(ct)]), int(n), int(tgt))
    return mpz(int(w[0])), mpz(int(w[1]))


class Prep:
    pass


def prepare(S, p, rng, e_t, max_loop1=2000):
    """Loop1-lite. Returns Prep or None (with .why)."""
    n = S.norm
    P = Prep()
    P.why = None
    P.loop1 = 0
    if n % 2 == 0:
        P.why = 'even'
        return P
    beta = inert_beta(S)
    if beta is None:
        P.why = 'not_inert'
        return P
    cs, ct = lin_form(beta, 2, p)
    lb = lattice_mod(cs, ct, n)
    if lb is None:
        P.why = 'beta_gcd'
        return P
    R, _ = lb
    l1 = min(R[0][0] ** 2 + R[1][0] ** 2, R[0][1] ** 2 + R[1][1] ** 2)
    lo = -(-(n // 2) // l1)
    hi = (5 * n // 2) // l1
    P.interval = (int(lo), int(hi))
    if hi < 2:
        hi = 2
    if lo > hi:
        lo = 1
    while P.loop1 < max_loop1:
        P.loop1 += 1
        if hi >= 5:
            k0 = rng.randint(0, int(gmpy2.isqrt(hi)))
            rem_lo = max(0, int(lo) - k0 * k0)
            rem_hi = int(hi) - k0 * k0
            if rem_hi < 0:
                continue
            k1lo = int(gmpy2.isqrt(rem_lo))
            if k1lo * k1lo < rem_lo:
                k1lo += 1
            k1hi = int(gmpy2.isqrt(rem_hi))
            if k1lo > k1hi:
                continue
            k1 = rng.randint(k1lo, k1hi)
        else:
            k0, k1 = 1, 0
        if (k0 - k1) % 2 == 0 or gmpy2.gcd(k0, k1) != 1:
            if hi >= 5:
                continue
        # alpha0 = (k0 + k1 i) * beta, numerators over 2
        a0 = qmul((mpz(2 * k0), mpz(2 * k1), mpz(0), mpz(0)), beta, p)
        a0 = tuple(x // 2 for x in a0)    # (k)*beta over 2
        cs, ct = lin_form(a0, 2, p)
        lb = lattice_mod(cs, ct, n)
        if lb is None:
            continue
        R, steps = lb
        m1 = min(R[0][0] ** 2 + R[1][0] ** 2, R[0][1] ** 2 + R[1][1] ** 2)
        if 2 * m1 < n:          # not balanced
            if P.loop1 < max_loop1:
                continue
        P.alpha0, P.dal = a0, 2
        P.cs, P.ct, P.R, P.gauss = cs, ct, R, steps
        P.r0 = nrd_num(a0, p) // 4 // n
        assert nrd_num(a0, p) % (4 * n) == 0
        P.balanced = int(2 * m1 >= n)
        P.m1 = m1
        return P
    P.why = 'loop1_cap'
    return P


def loop2(S, p, e_t, P, lams, extended=False):
    """Evaluate the lambdas; returns list of (code, zbits)."""
    n = S.norm
    two_e = mpz(1) << e_t
    R = P.R
    det = R[0][0] * R[1][1] - R[0][1] * R[1][0]
    inv = [[R[1][1], -R[0][1]], [-R[1][0], R[0][0]]]
    out = []
    for lam in lams:
        lam = mpz(lam)
        if gmpy2.gcd(lam, n) != 1:
            out.append((0, 0, -1))
            continue
        rhs = two_e - 2 * lam * lam * P.r0
        tgt = (rhs * gmpy2.invert(lam, n)) % n
        s0, t0 = solve_target(P.cs, P.ct, n, tgt)
        w0 = rounded_div(inv[0][0] * s0 + inv[0][1] * t0, det)
        w1 = rounded_div(inv[1][0] * s0 + inv[1][1] * t0, det)
        s = s0 - (R[0][0] * w0 + R[0][1] * w1)
        t = t0 - (R[1][0] * w0 + R[1][1] * w1)
        num = rhs - lam * (P.cs * s + P.ct * t)
        k, r = divmod(num, n)
        assert r == 0
        z = 2 * k - s * s - t * t
        if z <= 0:
            out.append((2, 0, -1))
            continue
        zb = int(z).bit_length()
        if (s - t) % 2 == 0 or k % 2 == 0:
            out.append((3, zb, -1))
            continue
        assert z % 4 == 1
        sb = sieve_bucket(z)
        sol = cornacchia_prime(1, z, Ctr()) if gmpy2.is_prime(z, 25) else None
        ext = False
        if sol is None and extended:
            sol = cornacchia_extended(z, Ctr())
            ext = sol is not None
        if sol is not None:
            sc, tc = sol
            if (sc - s) % 2:
                sc, tc = tc, sc
            if (sc - s) % 2 or (tc - t) % 2:
                sol = None
        if sol is None:
            out.append((4, zb, sb))
            continue
        a1, b1 = (sc + s) // 2, (tc + t) // 2
        a2, b2 = s - a1, t - b1
        # beta_k = n * conj(w_k) ... with w1 + w2 = sigma = s + i t appearing
        # as Tr(sigma conj(alpha)); beta_k = n w_k + lam alpha0
        b1q = tuple(x + y for x, y in zip((2 * n * a1, 2 * n * b1, 0, 0),
                                          tuple(lam * x for x in P.alpha0)))
        b2q = tuple(x + y for x, y in zip((2 * n * a2, 2 * n * b2, 0, 0),
                                          tuple(lam * x for x in P.alpha0)))
        N1 = nrd_num(b1q, p)
        N2 = nrd_num(b2q, p)
        assert N1 % (4 * n) == 0 and N2 % (4 * n) == 0
        assert (N1 + N2) // 4 == two_e * n, 'norm equation'
        g1, d1 = normalize(b1q, 2)
        g2, d2 = normalize(b2q, 2)
        th, dth = normalize(qmul(g2, conj(g1), p), d1 * d2 * n)
        ok = (endtype(th) in (1, 2)) if dth == 1 else is_odd_norm(th)
        out.append(((6 if ok else 7) + (2 if ext else 0), zb, sb))
    return out
