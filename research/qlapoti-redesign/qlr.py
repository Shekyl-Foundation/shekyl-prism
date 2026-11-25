# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""qlapoti redesign experiment: per-lambda sweep logger.

Research model only (Python + gmpy2, not constant time; counts are model
work units, not cycles). Builds on fne.py / fsne.py, copied unchanged from
research/fixed-schedule-norm-equation (research branch @ fe0b1b9 lineage),
which port quat_qlapoti of mariascrs/PRISM_v2 @ 87fad7d.

A *sweep* fixes one prepared small ideal S and one generator alpha0 (the
reference's generator search), builds and reduces the 2-D lattice once, and
then evaluates a list of multipliers lambda, recording for every lambda
the stage at which it stopped. Any fixed-schedule variant that uses the
same generator/lambda structure (caps, reuse length, lambda sequence,
two-stage filtering) can then be evaluated offline from the logs.

Outcome codes per lambda:
  0 gcd(lambda, n) != 1          (reference skips; fixed schedule masks)
  1 m < 0
  2 M4 <= 0                      (the 'area' condition)
  3 residue (mod 8) condition fails
  4 odd part not prime / -1 not QR / Cornacchia fails
  5 postcondition (end type / odd norm) fails
  6 success
"""
import gmpy2
from gmpy2 import mpz

from fne import (SET_MAP, prime_of, sec_degree, tape, random_prime_ideal,
                 prime_norm_equivalent, intersect_coprime, prepare_small,
                 Ctr, nrd_num, normalize, add_el, qmul, conj, rounded_div,
                 check_mod_condition, cornacchia_extended, endtype,
                 is_odd_norm, generator, dim2_short_basis, GOOD_PRIMES)

OK_CODES = ('coprime', 'mneg', 'area', 'mod', 'prime', 'post', 'ok')


def keygen(name, kid, p, SEC):
    rng = tape(b'keygen', name, kid)
    I0 = random_prime_ideal(SEC, p, rng)
    Is, info = prime_norm_equivalent(I0, p, rng)
    return Is


def challenge(name, e, kid, mid):
    rng = tape(b'chal', name, kid, mid)
    while True:
        s = rng.getrandbits(e - 4) | 1
        q = (1 << (2 * e - 5)) - s * s
        if gmpy2.is_prime(q):
            return q, rng


def small_ideal(name, Is, kid, mid, p, e):
    """Prepared small ideal of I_sec cap I_q for message mid (as run_fne)."""
    q, rng = challenge(name, e, kid, mid)
    Iq = random_prime_ideal(q, p, rng)
    L = intersect_coprime(Is, Iq)
    return prepare_small(L, p)


def sweep(S, p, e_t, rng, lams, K_cap=None):
    """One generator, many lambdas. Returns dict with generator info and a
    list of per-lambda (code, m4bits, corn_steps)."""
    n = S.norm
    two_e = mpz(1) << e_t
    psqrt = gmpy2.isqrt(p)
    c = Ctr()
    got, draws = generator(S, p, rng, c, K_cap)
    out = dict(draws=draws, gauss=0, suitable=0, lam=[])
    if got is None:
        return out
    g0, d0 = got
    a0norm = nrd_num(g0, p) // (d0 * d0)
    alpha, dal = normalize(g0, d0)
    a_al, b_al = alpha[0], alpha[1]
    if dal == 1:
        a_al, b_al = 2 * a_al, 2 * b_al
    try:
        tmp1 = gmpy2.invert(a_al, n)
    except ZeroDivisionError:
        return out
    x = (b_al * tmp1) % n
    Lm = [[n - x, n], [mpz(1), mpz(0)]]
    s0 = c.lat_red_steps
    L_red = dim2_short_basis(Lm, c)
    out['gauss'] = c.lat_red_steps - s0
    L_det = L_red[0][0] * L_red[1][1] - L_red[0][1] * L_red[1][0]
    L_inv = [[L_red[1][1], -L_red[0][1]], [-L_red[1][0], L_red[0][0]]]
    out['suitable'] = int(gmpy2.gcd(a_al, b_al) == 1 and
                          L_red[0][1] ** 2 + L_red[1][1] ** 2 < psqrt)
    out['lat_short2'] = int(L_red[0][0] ** 2 + L_red[1][0] ** 2).bit_length()
    out['lat_long2'] = int(L_red[0][1] ** 2 + L_red[1][1] ** 2).bit_length()
    out['r0bits'] = int(2 * a0norm // n).bit_length()
    res = []
    for lam in lams:
        lam = mpz(lam)
        if gmpy2.gcd(n, lam) != 1:
            res.append((0, 0, 0, -1))
            continue
        al, dl = normalize(tuple(lam * t for t in g0), d0)
        q_ = (2 * a0norm * lam * lam) // n
        m = two_e - q_
        if m < 0:
            res.append((1, 0, 0, -1))
            continue
        aa, bb = al[0], al[1]
        if dl == 1:
            aa, bb = 2 * aa, 2 * bb
        tmp = gmpy2.invert(aa, n)
        T = (tmp * m) % n
        vt0, vt1 = -T, mpz(0)
        w0 = rounded_div(L_inv[0][0] * vt0 + L_inv[0][1] * vt1, L_det)
        w1 = rounded_div(L_inv[1][0] * vt0 + L_inv[1][1] * vt1, L_det)
        vd0 = vt0 - (L_red[0][0] * w0 + L_red[0][1] * w1)
        vd1 = vt1 - (L_red[1][0] * w0 + L_red[1][1] * w1)
        A, B = -vd0, -vd1
        m2, rr = divmod(m - (A * aa + B * bb), n)
        assert rr == 0
        M4 = 2 * (m2 - A * A - B * B) + A * A + B * B
        if M4 <= 0:
            res.append((2, 0, 0, -1))
            continue
        mb = int(M4).bit_length()
        if not check_mod_condition(M4, A, B):
            res.append((3, mb, 0, -1))
            continue
        sb = sieve_bucket(M4)
        cc = Ctr()
        sol = cornacchia_extended(M4, cc)
        enc = None
        if sol is not None:
            x0, y0 = sol
            if (A & 1) != (x0 & 1):
                x0, y0 = y0, x0
            if (A & 1) == (x0 & 1) and (B & 1) == (y0 & 1):
                a1 = (x0 + A) // 2
                b1 = (y0 + B) // 2
                enc = (a1, A - a1, b1, B - b1)
        if enc is None:
            res.append((4, mb, cc.mx_corn, sb))
            continue
        g1, dg1 = normalize(*add_el((enc[0] * n, enc[2] * n, 0, 0), 1, al, dl))
        g2, dg2 = normalize(*add_el((enc[1] * n, enc[3] * n, 0, 0), 1, al, dl))
        n1 = nrd_num(g1, p) // (dg1 * dg1)
        n2 = nrd_num(g2, p) // (dg2 * dg2)
        assert n1 % n == 0 and n2 % n == 0 and n1 // n + n2 // n == two_e
        th, dth = normalize(qmul(g2, conj(g1), p), dg1 * dg2 * n)
        if dth == 1:
            ok = endtype(th) in (1, 2)
        else:
            ok = is_odd_norm(th)
        res.append((6 if ok else 5, mb, cc.mx_corn, sb))
    out['lam'] = res
    return out


# ------------------------------------------------------------ sieve buckets
def _primes_upto(B):
    s = bytearray([1]) * (B + 1)
    s[0:2] = b'\x00\x00'
    for i in range(2, int(B ** 0.5) + 1):
        if s[i]:
            s[i * i::i] = bytearray(len(s[i * i::i]))
    return [i for i in range(B + 1) if s[i]]


_GOOD = set(GOOD_PRIMES)
# products of the primes outside the reference's good list, by octave
SIEVE_OCT = []
for _k in range(1, 17):
    _lo, _hi = 1 << _k, 1 << (_k + 1)
    _pr = 1
    for _q in _primes_upto(_hi - 1):
        if _q >= _lo and _q not in _GOOD:
            _pr *= _q
    SIEVE_OCT.append((_k, mpz(_pr)))


def residual_after_good(M4):
    r = mpz(M4)
    for pr in GOOD_PRIMES:
        while r % pr == 0:
            r //= pr
    return r


def sieve_bucket(M4):
    """Smallest octave k (prime factor in [2^k, 2^(k+1)), k <= 16) of the
    residual after the reference strips the good primes; 0 if none below
    2^17 (or residual == 1). Good primes above the list (> 1237 and
    1 mod 4) are included: the reference fails on them too."""
    r = residual_after_good(M4)
    if r == 1:
        return 0
    for k, P in SIEVE_OCT:
        g = gmpy2.gcd(r, P)
        if g != 1 and g != r:
            return k
    return 0
