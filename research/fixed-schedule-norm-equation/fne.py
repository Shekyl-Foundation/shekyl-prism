# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Faithful step-counting model of PRISM's norm equation (quat_qlapoti)
and key generation's prime-norm search, plus a fixed-schedule variant.

Research model only. Python big integers are not constant-time and the
counts below are operation counts, not cycles. Nothing here proposes a
change to the signature format.

PRISM is the scheme of ePrint 2026/443 (Basso, Borin, Castryck,
Corte-Real Santos, Invernizzi, Leroux, Maino, Vercauteren, Wesolowski);
shekyl-prism implements it. The search below is a line-by-line port of
the reference at mariascrs/PRISM_v2 87fad7d (the submodule pin):

  quat_qlapoti, quat_lideal_generator_small_coprime,
  quat_dim2_lattice_short_basis, ibz_rounded_div,
  quat_dim2_lattice_qlapoti_cvp_condition, quat_qlapoti_check_mod_condition,
  ibz_cornacchia_extended (+ _prime, _prime_loop, complex power),
  ibz_sqrt_mod_p, quat_elem_is_odd_norm, get_endtype       (qlapoti.c,
  integers.c, intbig.c)
  quat_lideal_prime_norm_reduced_equivalent          (lll_applications.c)

Deliberate differences, all stated in the report:
  * lattice HNF and 4-D LLL use PARI (mathnf, qflllgram), not the
    reference's own HNF/L2; both are deterministic functions of the
    lattice, which is what matters for the search;
  * the random tape is Python's Mersenne Twister seeded from SHAKE, not
    the reference's RNG (no byte-for-byte replay is claimed);
  * primality is gmpy2.is_prime (MR, 25 rounds) in place of GMP's
    mpz_probab_prime_p(., 32); the verdicts agree w.h.p.
  * the order is the true maximal order O0 = <1, i, (i+j)/2, (1+k)/2>.
"""
import hashlib
import random

import gmpy2
from gmpy2 import mpz
import cypari2

pari = cypari2.Pari()
try:
    pari.allocatemem(96 * 1024 * 1024, silent=True)
except Exception:
    pass

SETS = [('p248', 248, 5), ('p324', 324, 3), ('p376', 376, 65),
        ('p500', 500, 27), ('p664', 664, 17)]
WIDTH = {'p248': 256, 'p324': 384, 'p376': 384, 'p500': 512, 'p664': 768}
SET_MAP = {n: (n, e, f) for n, e, f in SETS}

# QUAT_small_primes_where_neg_q_square (101 entries; quaternion_data.c)
GOOD_PRIMES = [2, 5, 13, 17, 29, 37, 41, 53, 61, 73, 89, 97, 101, 109, 113,
               137, 149, 157, 173, 181, 193, 197, 229, 233, 241, 257, 269,
               277, 281, 293, 313, 317, 337, 349, 353, 373, 389, 397, 401,
               409, 421, 433, 449, 457, 461, 509, 521, 541, 557, 569, 577,
               593, 601, 613, 617, 641, 653, 661, 673, 677, 701, 709, 733,
               757, 761, 769, 773, 797, 809, 821, 829, 853, 857, 877, 881,
               929, 937, 941, 953, 977, 997, 1009, 1013, 1021, 1033, 1049,
               1061, 1069, 1093, 1097, 1109, 1117, 1129, 1153, 1181, 1193,
               1201, 1213, 1217, 1229, 1237]
assert len(GOOD_PRIMES) == 101

GEN_BITS = 17          # QUAT_qlapoti_gen_bound_bits
ITER_BOUND = 20000     # QUAT_qlapoti_iteration_bound (x20 passes)
EQUIV_BOX = 64         # QUAT_equiv_bound_coeff
PRIM_ROUNDS = 25


def prime_of(e, f):
    p = f * (1 << e) - 1
    assert gmpy2.is_prime(p) and p % 4 == 3
    return p


def sec_degree(name):
    return int(gmpy2.next_prime(mpz(1) << (2 * WIDTH[name])))


def shake(tag, *parts, n=32):
    h = hashlib.shake_256(b'FSNormEq/v0/' + tag)
    for x in parts:
        if isinstance(x, str):
            x = x.encode()
        if isinstance(x, int):
            x = x.to_bytes((x.bit_length() + 8) // 8, 'little')
        h.update(len(x).to_bytes(4, 'little'))
        h.update(x)
    return h.digest(n)


def tape(*parts):
    return random.Random(int.from_bytes(shake(b'tape', *parts, n=32), 'little'))


# ------------------------------------------------------------ counters
class Ctr:
    FIELDS = ('passes', 'gens', 'gen_draws', 'lam_steps', 'lat_builds',
              'lat_red_steps', 'cvp', 'm_neg', 'mod_ok', 'trial_divs',
              'pt', 'pt_bits', 'pt_pass', 'legendre', 'sqrt_modp',
              'tonelli_steps', 'corn_steps', 'cpow', 'post', 'resets',
              'bigops', 'success_pass', 'transformed')

    def __init__(self):
        for f in self.FIELDS:
            setattr(self, f, 0)
        self.success_pass = -1

    def asdict(self):
        return {f: getattr(self, f) for f in self.FIELDS}


def work_units(c, small_bits):
    """Weighted work model in 'big multiplications'. A modular
    exponentiation of b bits counts 1.5 b; a GMP probable-prime test with
    32 requested rounds is BPSW + 8 MR, about 11 exponentiations; a trial
    division by a one-word prime counts 1; a complex power counts 64
    squarings of 4 mults; every other bigint mul/div/gcd/mod counts 1."""
    return (c['bigops'] + c['trial_divs'] + 16.5 * c['pt_bits']
            + 1.5 * small_bits * (c['sqrt_modp'] + c['legendre'])
            + 2 * c['corn_steps'] + 2 * c['tonelli_steps']
            + 320 * c['cpow'])


# ------------------------------------------------------------ algebra
def qmul(u, v, p):
    a1, b1, c1, d1 = u
    a2, b2, c2, d2 = v
    return (a1 * a2 - b1 * b2 - p * c1 * c2 - p * d1 * d2,
            a1 * b2 + b1 * a2 + p * c1 * d2 - p * d1 * c2,
            a1 * c2 - b1 * d2 + c1 * a2 + d1 * b2,
            a1 * d2 + b1 * c2 - c1 * b2 + d1 * a2)


def nrd_num(v, p):
    return v[0] * v[0] + v[1] * v[1] + p * (v[2] * v[2] + v[3] * v[3])


def conj(v):
    return (v[0], -v[1], -v[2], -v[3])


def normalize(v, d):
    g = gmpy2.gcd(gmpy2.gcd(gmpy2.gcd(v[0], v[1]), gmpy2.gcd(v[2], v[3])), d)
    if g != 1 and g != 0:
        v = tuple(x // g for x in v)
        d = d // g
    if d < 0:
        v = tuple(-x for x in v)
        d = -d
    return v, d


def add_el(a, da, b, db):
    return tuple(x * db + y * da for x, y in zip(a, b)), da * db


O0 = [(2, 0, 0, 0), (0, 2, 0, 0), (0, 1, 1, 0), (1, 0, 0, 1)]  # denom 2


class Lat:
    """Lattice: 4 column vectors (numerators) over a common denom."""
    __slots__ = ('cols', 'den', 'norm')

    def __init__(self, cols, den, norm=None):
        self.cols, self.den, self.norm = cols, den, norm


def hnf(gens, den, norm=None):
    M = pari.matrix(4, len(gens), [int(g[r]) for r in range(4) for g in gens])
    H = pari.mathnf(M)
    cols = [tuple(mpz(int(H[r, c])) for r in range(4)) for c in range(4)]
    g = mpz(den)
    for c in cols:
        for x in c:
            g = gmpy2.gcd(g, x)
    if g != 1:
        cols = [tuple(x // g for x in c) for c in cols]
        den = den // g
    return Lat(cols, mpz(den), norm)


def left_ideal(alpha, N, p):
    """O0*alpha + O0*N, alpha integral (denominator 1)."""
    gens = [qmul(b, alpha, p) for b in O0] + [tuple(N * x for x in b) for b in O0]
    return hnf(gens, 2, mpz(N))


def right_mul(L, beta, dbeta, p, new_norm):
    gens = [qmul(c, beta, p) for c in L.cols]
    return hnf(gens, L.den * dbeta, new_norm)


def intersect_coprime(I, J):
    """I cap J = n(J) I + n(I) J for coprime norms (both left O0-ideals)."""
    D = I.den * J.den // gmpy2.gcd(I.den, J.den)
    gi, gj = D // I.den, D // J.den
    gens = [tuple(J.norm * gi * x for x in c) for c in I.cols]
    gens += [tuple(I.norm * gj * x for x in c) for c in J.cols]
    return hnf(gens, D, I.norm * J.norm)


def lll_cols(L, p):
    w = (1, 1, p, p)
    G = [[int(sum(L.cols[r][t] * L.cols[s][t] * w[t] for t in range(4)))
          for s in range(4)] for r in range(4)]
    U = pari.qflllgram(pari.matrix(4, 4, [G[r][s] for r in range(4) for s in range(4)]))
    out = []
    for c in range(4):
        u = [mpz(int(U[r, c])) for r in range(4)]
        out.append(tuple(sum(u[k] * L.cols[k][t] for k in range(4)) for t in range(4)))
    out.sort(key=lambda v: nrd_num(v, p))
    return out


def lat_index_ok(L, p):
    """[O0 : L] == norm^2 (determinant check)."""
    M = pari.matrix(4, 4, [int(L.cols[c][r]) for r in range(4) for c in range(4)])
    det = abs(int(pari.matdet(M)))
    # det(O0 numerators over 2) = 4 / 16 ; index = det(L)/den^4 / (1/4)
    return det * 4 == int(L.norm) ** 2 * int(L.den) ** 4


def random_prime_ideal(N, p, rng):
    """A random left O0-ideal of prime norm N, via alpha = x + y i + j,
    x^2 + y^2 + p = 0 mod N (model of the reference's random trace-zero
    element and square root)."""
    while True:
        y = rng.randrange(N)
        a = (-p - y * y) % N
        if a and gmpy2.legendre(a, N) == 1:
            x = int(pari.lift(pari.Mod(a, N).sqrt()))
            return left_ideal((mpz(x), mpz(y), mpz(1), mpz(0)), N, p)


# ------------------------------------------------------------ keygen search
def prime_norm_equivalent(I, p, rng, box=EQUIV_BOX):
    """quat_lideal_prime_norm_reduced_equivalent: uniform box draws on a
    reduced basis until the quotient norm is a probable prime."""
    red = lll_cols(I, p)
    adj = I.den * I.den * I.norm
    cap = (2 * box + 1) ** 4
    draws = 0
    pt_bits = 0
    while draws < cap:
        draws += 1
        x = [rng.randint(-box, box) for _ in range(4)]
        g = tuple(sum(x[k] * red[k][t] for k in range(4)) for t in range(4))
        t, r = divmod(nrd_num(g, p), adj)
        assert r == 0
        pt_bits += int(t).bit_length()
        if t > 1 and gmpy2.is_prime(t, PRIM_ROUNDS):
            # I * conj(g) / n(I)  (denominator I.den * n(I))
            J = right_mul(I, conj(g), I.den * I.norm, p, t)
            return J, dict(draws=draws, pt_bits=pt_bits, N=int(t))
    return None, dict(draws=draws, pt_bits=pt_bits, N=0)


# ------------------------------------------------------------ helpers (ports)
def rand_bits(rng, m):
    """ibz_rand_interval_bits: uniform on [-2^m, 2^m], then minus m."""
    return rng.randint(-(1 << m), 1 << m) - m


def rounded_div(a, b):
    q = gmpy2.t_div(a, b)
    r = abs(a - q * b)
    s = (1 - 2 * (a * b < 0)) * (2 * r > abs(b))
    return q + s


def shortest_equivalent(L, p):
    """quat_lideal_shortest_equivalent: small = L * (-conj(beta)) / n(L)."""
    red = lll_cols(L, p)
    beta = red[0]
    nb = nrd_num(beta, p)
    d2 = L.den * L.den * L.norm
    n_small, r = divmod(nb, d2)
    assert r == 0
    nb_el = (-beta[0], beta[1], beta[2], beta[3])
    S = right_mul(L, nb_el, L.den * L.norm, p, n_small)
    return S


def dim2_short_basis(L, c):
    """quat_dim2_lattice_short_basis with norm_q = 1 (Cohen 3.1.14).
    L[r][c]: columns a=(L00,L10), b=(L01,L11)."""
    a = [L[0][0], L[1][0]]
    b = [L[0][1], L[1][1]]
    na = a[0] * a[0] + a[1] * a[1]
    nb = b[0] * b[0] + b[1] * b[1]
    c.bigops += 4
    if na < nb:
        a, b, na, nb = b, a, nb, na
    r = 0
    nt = 0
    while True:
        c.lat_red_steps += 1
        nn = a[0] * b[0] + a[1] * b[1]
        r = rounded_div(nn, nb)
        nt = na - 2 * nn * r + r * r * nb
        c.bigops += 8
        if nb > nt:
            na, nb = nb, nt
            t = [a[0] - r * b[0], a[1] - r * b[1]]
            a, b = b, t
            c.bigops += 2
        else:
            break
    if nt < na:
        a = [a[0] - r * b[0], a[1] - r * b[1]]
        c.bigops += 2
    return [[b[0], a[0]], [b[1], a[1]]]


def check_mod_condition(m, A, B):
    a = A & 1
    b = B & 1
    M8 = m & 7
    M = M8 & 3
    if M8 == 0:
        return False
    if a == b and a == 0:
        return M == 0
    if a == b and a == 1:
        return M == 2
    return M == 1


def sqrt_mod_p(a, p, c):
    """ibz_sqrt_mod_p port (returns None if not a QR)."""
    amod = a % p
    c.legendre += 1
    if gmpy2.legendre(amod, p) != 1:
        return None
    c.sqrt_modp += 1
    if p % 4 == 3:
        return pow(amod, (p + 1) // 4, p)
    if p % 8 == 5:
        t = pow(amod, (p - 1) // 4, p)
        if t == 1:
            return pow(amod, (p + 3) // 8, p)
        t = pow(4 * amod, (p - 5) // 8, p)
        return (2 * amod * t) % p
    q = p - 1
    e = 0
    while not (q >> e) & 1:
        e += 1
    q >>= e
    qnr = 2
    while gmpy2.legendre(qnr, p) != -1:
        qnr += 1
        c.tonelli_steps += 1
    z = pow(qnr, q, p)
    y = pow(amod, q, p)
    x = pow(amod, (q + 1) // 2, p)
    ex = mpz(1) << (e - 2) if e >= 2 else mpz(0)
    for _ in range(e):
        c.tonelli_steps += 1
        b = pow(y, ex, p)
        if b == p - 1:
            x = x * z % p
            y = y * z * z % p
        z = z * z % p
        ex >>= 1
    return x


def cornacchia_prime(n, p, c):
    """ibz_cornacchia_prime(x, y, n, p): x^2 + n y^2 = p, here n = 1."""
    if p == 2:
        return (1, 1) if n == 1 else None
    if p == n:
        return (0, 1)
    if gmpy2.gcd(p, n) != 1:
        return None
    r2 = sqrt_mod_p(-n, p, c)
    if r2 is None:
        return None
    prod = p
    r1 = p
    r0 = p
    while prod >= p:
        c.corn_steps += 1
        r0 = r2 % r1
        prod = r0 * r0
        r2 = r1
        r1 = r0
    a = p - prod
    a, rem = divmod(a, n)
    if rem:
        return None
    y, exact = gmpy2.iroot(a, 2)
    if not exact:
        return None
    return (r0, y)


def cplx_pow_mul(x, y, re, im, e, c):
    c.cpow += 1
    rx, ix = mpz(1), mpz(0)
    for i in range(64):
        rx, ix = rx * rx - ix * ix, 2 * rx * ix
        if (e >> (63 - i)) & 1:
            rx, ix = rx * re - ix * im, rx * im + ix * re
    return x * rx - y * ix, x * ix + y * rx


def cornacchia_extended(n, c):
    """ibz_cornacchia_extended with q = 1, bad_primes_prod = 1."""
    if n <= 0:
        return None
    vals = []
    nodd = mpz(n)
    for pr in GOOD_PRIMES:
        v = 0
        while True:
            c.trial_divs += 1
            g, r = gmpy2.t_divmod(nodd, pr)
            if r:
                break
            nodd = g
            v += 1
        vals.append(v)
    if nodd == 1:
        x, y = mpz(1), mpz(0)
    else:
        c.pt += 1
        c.pt_bits += int(nodd).bit_length()
        if not gmpy2.is_prime(nodd, PRIM_ROUNDS):
            return None
        c.pt_pass += 1
        c.legendre += 1
        if gmpy2.legendre(-1, nodd) != 1:
            return None
        s = cornacchia_prime(1, nodd, c)
        if s is None:
            return None
        x, y = s
    for pr, v in zip(GOOD_PRIMES, vals):
        if v == 0:
            continue
        s = cornacchia_prime(1, pr, c)
        if s is None:
            return None
        x, y = cplx_pow_mul(x, y, mpz(s[0]), mpz(s[1]), v, c)
    return x, y


def is_odd_norm(v):
    if (v[0] & 1) == (v[2] & 1):
        return False
    found = False
    for i in range(4):
        if found:
            if (v[i] & 3) == 2:
                return False
        elif (v[i] & 3) == 2:
            found = True
    return found


def endtype(v):
    t1, t2, t3, t4 = v[0] & 3, v[1] & 3, v[3] & 3, v[2] & 3
    if t1 == 2:
        if t2 == 2:
            if t3 == 1 and t4 == 0:
                return 1
            if t3 == 3 and t4 == 2:
                return 1
            if t3 == 0:
                return 2 if t4 in (1, 3) else 0
            return 0
        if t2 == 0:
            if t3 == 1 and t4 == 2:
                return 1
            if t3 == 3 and t4 == 2:
                return 1
            return 0
        return 0
    if t1 == 0:
        if t2 == 2 and t3 == 2:
            return 2 if t4 in (1, 3) else 0
        return 0
    return 0


# ------------------------------------------------------------ the search
def generator(S, p, rng, c, max_draws=None):
    """quat_lideal_generator_small_coprime (uncapped unless max_draws)."""
    n = S.norm
    n2 = n * n
    den = S.den
    draws = 0
    while True:
        if max_draws is not None and draws >= max_draws:
            return None, draws
        draws += 1
        c.gen_draws += 1
        co = [rand_bits(rng, GEN_BITS) for _ in range(4)]
        g = tuple(sum(co[k] * S.cols[k][t] for k in range(4)) for t in range(4))
        c.bigops += 16
        if den == 1:
            ok = gmpy2.gcd(2 * g[0], n) == 1
        else:
            ok = True
            if den == 2:
                ok = gmpy2.gcd(n, g[0]) == 1
        c.bigops += 1
        if ok:
            nn, r = divmod(nrd_num(g, p), den * den)
            assert r == 0
            ok = gmpy2.gcd(n2, nn) == n
            c.bigops += 6
        if ok:
            return (g, den), draws


def prepare_small(L, p):
    S = shortest_equivalent(L, p)
    red = lll_cols(S, p)
    return Lat(red, S.den, S.norm)


def qlapoti(S, p, e2, rng, c, max_counter=ITER_BOUND, total=None,
            policy='ref', K_gen=None, M_reuse=None, verify=False):
    """Port of quat_qlapoti's loop on the prepared small ideal S.

    policy 'ref'   : the reference control flow;
    policy 'fresh' : a new generator every pass (no lambda reuse);
    policy 'reuse' : each generator is kept for exactly M_reuse passes
                     (lambda = 1..M), a non-coprime lambda or an
                     unsuitable lattice makes that pass invalid instead of
                     skipping; no data-dependent keep/reset.
    With K_gen set, the generator search makes at most K_gen draws (a
    pass whose draws all fail is invalid).
    Returns (mu1, mu2, theta, transformed) or None; c.success_pass is the
    1-based pass index of the first success."""
    n = S.norm
    two_e = mpz(1) << e2
    psqrt = gmpy2.isqrt(p)
    if total is None:
        total = max_counter * 20
    keep = False
    lam = mpz(1)
    alpha0 = None
    a0norm = None
    L_red = L_inv = L_det = None
    for counter in range(total):
        c.passes += 1
        valid = True
        if policy == 'ref':
            if counter % max_counter == 0:
                keep = False
                lam = mpz(1)
            new_gen = not keep
        elif policy == 'fresh':
            new_gen = True
        else:
            new_gen = (counter % M_reuse == 0)
        if new_gen:
            c.gens += 1
            got, _ = generator(S, p, rng, c, K_gen)
            if got is None:
                valid = False
                alpha0 = None
                if policy == 'ref':
                    keep = False
                continue
            alpha0 = got
            a0norm, r = divmod(nrd_num(alpha0[0], p), alpha0[1] ** 2)
            lam = mpz(1)
            c.bigops += 6
        else:
            if alpha0 is None:
                continue
            lam += 1
            if policy == 'ref':
                c.bigops += 1
                while gmpy2.gcd(n, lam) != 1:
                    c.lam_steps += 1
                    lam += 1
                    c.bigops += 1
            else:
                if gmpy2.gcd(n, lam) != 1:
                    valid = False
        alpha, dal = normalize(tuple(lam * x for x in alpha0[0]), alpha0[1])
        # m = 2^e - floor(2 n(alpha0) lam^2 / n)
        num = 2 * a0norm * lam * lam
        q_, r_ = divmod(num, n)
        assert r_ == 0
        m = two_e - q_
        c.bigops += 4
        if m < 0:
            c.m_neg += 1
            continue
        a_al, b_al = alpha[0], alpha[1]
        if dal == 1:
            a_al, b_al = 2 * a_al, 2 * b_al
        tmp = gmpy2.invert(a_al, n)
        T = (tmp * m) % n
        v_t = (-T, mpz(0))
        c.bigops += 3
        if new_gen:
            c.lat_builds += 1
            x = (b_al * tmp) % n
            Lm = [[n - x, n], [mpz(1), mpz(0)]]
            L_red = dim2_short_basis(Lm, c)
            L_det = L_red[0][0] * L_red[1][1] - L_red[0][1] * L_red[1][0]
            L_inv = [[L_red[1][1], -L_red[0][1]], [-L_red[1][0], L_red[0][0]]]
            suitable = (gmpy2.gcd(a_al, b_al) == 1 and
                        L_red[0][1] ** 2 + L_red[1][1] ** 2 < psqrt)
            c.bigops += 8
            if policy == 'ref':
                keep = suitable
            elif policy == 'reuse':
                S_suitable = suitable
        if policy == 'reuse' and not new_gen and not S_suitable:
            valid = False
        # close vector
        w0 = L_inv[0][0] * v_t[0] + L_inv[0][1] * v_t[1]
        w1 = L_inv[1][0] * v_t[0] + L_inv[1][1] * v_t[1]
        w0 = rounded_div(w0, L_det)
        w1 = rounded_div(w1, L_det)
        vc0 = L_red[0][0] * w0 + L_red[0][1] * w1
        vc1 = L_red[1][0] * w0 + L_red[1][1] * w1
        vd = (v_t[0] - vc0, v_t[1] - vc1)
        c.bigops += 14
        # cvp condition
        c.cvp += 1
        A, B = -vd[0], -vd[1]
        t_ = A * a_al + B * b_al
        m2, rr = divmod(m - t_, n)
        assert rr == 0
        M4 = 2 * (m2 - A * A - B * B) + A * A + B * B
        c.bigops += 8
        enc = None
        if check_mod_condition(M4, A, B):
            c.mod_ok += 1
            sol = cornacchia_extended(M4, c)
            if sol is not None:
                a0_, b0_ = sol
                if (A & 1) != (a0_ & 1):
                    a0_, b0_ = b0_, a0_
                if (A & 1) == (a0_ & 1) and (B & 1) == (b0_ & 1):
                    a1 = (a0_ + A) // 2
                    b1 = (b0_ + B) // 2
                    enc = (a1, A - a1, b1, B - b1)
        if enc is None or not valid:
            continue
        g1, dg1 = add_el((enc[0] * n, enc[2] * n, 0, 0), 1, alpha, dal)
        g2, dg2 = add_el((enc[1] * n, enc[3] * n, 0, 0), 1, alpha, dal)
        g1, dg1 = normalize(g1, dg1)
        g2, dg2 = normalize(g2, dg2)
        if verify:
            n1 = nrd_num(g1, p) // (dg1 * dg1)
            n2 = nrd_num(g2, p) // (dg2 * dg2)
            assert nrd_num(g1, p) % (dg1 * dg1) == 0
            assert n1 % n == 0 and n2 % n == 0
            assert n1 // n + n2 // n == two_e, 'norm equation identity'
        # theta = mu2 * conj(mu1) / n
        th = qmul(g2, conj(g1), p)
        th, dth = normalize(th, dg1 * dg2 * n)
        c.post += 1
        c.bigops += 20
        ok = False
        transformed = False
        if dth == 1:
            et = endtype(th)
            if et in (1, 2):
                ok = True
                transformed = True
        else:
            ok = is_odd_norm(th)
        if not ok:
            c.resets += 1
            if policy == 'ref':
                keep = False
                lam = mpz(1)
            continue
        c.success_pass = counter + 1
        c.transformed = int(transformed)
        return (g1, dg1), (g2, dg2), (th, dth), transformed
    return None
