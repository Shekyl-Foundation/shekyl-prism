# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Fixed-schedule norm equation ('fresh' order): every call executes the
same sequence of operations, sized from public bounds, and keeps the first
valid answer with a select.

Research model only: Python integers are not constant-time; the selects
below are written as arithmetic on 0/1 flags to show that no control flow
depends on data, and the operation trace (kinds and counts, not values)
is hashed so that its input-independence can be checked. A port needs
constant-time big integers, a constant-time gcd/inverse (e.g. safegcd),
a fixed-exponent primality test and fixed-iteration Euclid, all of which
this file only costs.

Schedule per pass (all executed every pass, P passes, never an early exit):
  1. K generator draws on the reduced small-ideal basis; first valid kept.
  2. m, the 2-D lattice [[n-x, n],[1, 0]], reduced by T_lat Gauss steps
     with a frozen-when-done select.
  3. nearest-plane close vector and M4.
  4. residue conditions (bit logic).
  5. trial division by the 101 good primes, V_i conditional divisions each.
  6. one probable-prime test on the cofactor (bits <= B_M4).
  7. sqrt(-1) mod cofactor as c^((r-1)/4) for the first of J small
     candidates c with Jacobi(c, r) = -1 (J Jacobi symbols, one powering).
  8. Cornacchia's Euclid, T_corn steps with frozen-when-done select;
     isqrt; exactness check.
  9. complex powers for the small-prime parts with fixed-width exponents
     (table of the 101 primes' two-square decompositions is public).
 10. parity swap, gamma1/gamma2, theta, end-type and odd-norm tests.
 11. success flag; first success kept by select.
"""
import hashlib
import math

import gmpy2
from gmpy2 import mpz

from fne import (GOOD_PRIMES, GEN_BITS, rand_bits, nrd_num, normalize,
                 add_el, qmul, conj, rounded_div, check_mod_condition,
                 endtype, is_odd_norm, cornacchia_prime, Ctr)

SMALL_C = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53,
           59, 61, 67, 71, 73, 79, 83, 89]
# public two-square table for the good primes, from the reference's routine
_TAB = {}
for _p in GOOD_PRIMES:
    _TAB[_p] = cornacchia_prime(1, mpz(_p), Ctr())


def sel(b, x, y):
    """b ? x : y with b in {0,1}, arithmetic only."""
    return b * x + (1 - b) * y


def sel_t(b, xs, ys):
    return tuple(sel(b, x, y) for x, y in zip(xs, ys))


class Sched:
    """Public schedule parameters for one set."""

    def __init__(self, L_bits, e, P, K, J=16, tail_bits=30, nbits_max=None,
                 m4_bits_max=None):
        self.P, self.K, self.J = P, K, J
        self.nbits = nbits_max or (L_bits + 1) // 2 + 2
        # M4 ~ 2 m / n with m <= 2^(e-2); n >= 2^(nbits-6) assumed (public bound;
        # a smaller n makes the pass fail, it does not change the schedule)
        self.m4bits = m4_bits_max or (e - 2) + 2 - (self.nbits - 8)
        # nearest-integer Gauss reduction shrinks by >= 1+sqrt(2) per step,
        # from length ~n to ~sqrt(n): (nbits/2) log_{1+sqrt2} 2 steps (+3)
        self.T_lat = math.ceil(0.3932 * self.nbits) + 3
        # Cornacchia's plain Euclid from r to sqrt(r): Lame bound
        # (m4bits/2) log_phi 2 = 0.7202 m4bits steps (+3)
        self.T_corn = math.ceil(0.7202 * self.m4bits) + 3
        self.V = {}
        for p_ in GOOD_PRIMES:
            if p_ == 2:
                self.V[p_] = 2          # M4 mod 8 != 0 is required
            else:
                self.V[p_] = max(1, math.ceil(tail_bits / math.log2(p_)) - 1)
        self.td = sum(self.V.values())
        self.cpow_bits = sum(v.bit_length() for v in self.V.values())

    def pass_cost(self):
        """Work units per pass, same unit as fne.work_units ('big mults')."""
        nb, mb = self.nbits, self.m4bits
        ctgcd = max(1.0, nb / 8)            # constant-time gcd/inverse, per call
        gen = self.K * (16 + 6 + 2 * ctgcd)
        lat = self.T_lat * 10 + 12
        cvp = 30
        td = self.td
        pt = 16.5 * mb
        jac = self.J * max(1.0, mb / 8)
        sq = 1.5 * mb
        corn = 2 * self.T_corn + 10
        cpow = min(8 * self.cpow_bits, 4 * self.td)   # ladder or V_i cond. mults
        post = 40 + 3 * ctgcd
        return gen + lat + cvp + td + pt + jac + sq + corn + cpow + post

    def describe(self):
        return dict(P=self.P, K=self.K, J=self.J, nbits=self.nbits,
                    m4bits=self.m4bits, T_lat=self.T_lat, T_corn=self.T_corn,
                    trial_divs=self.td, cpow_bits=self.cpow_bits,
                    pass_cost=round(self.pass_cost(), 1))


def _safe_inv(a, n):
    try:
        return gmpy2.invert(a, n), 1
    except ZeroDivisionError:
        return mpz(0), 0


def qlapoti_fixed(S, p, e2, rng, sch, early_exit=False):
    """Return (result or None, info). info['trace'] is the hash of the
    operation schedule; info['first'] is the 1-based pass of the kept
    answer (for checking only: a constant-time port would not expose it)."""
    n = S.norm
    n2 = n * n
    den = S.den
    two_e = mpz(1) << e2
    tr = hashlib.sha256()
    ops = []

    def op(kind, k=1):
        ops.append((kind, k))

    got = 0
    res = None
    first = 0
    unfin = dict(lat=0, corn=0)
    for ps in range(1, sch.P + 1):
        # 1. generator: K draws, first valid kept
        f = 0
        gsel = (mpz(0),) * 4
        for _ in range(sch.K):
            co = [rand_bits(rng, GEN_BITS) for _ in range(4)]
            g = tuple(sum(co[k] * S.cols[k][t] for k in range(4)) for t in range(4))
            if den == 1:
                v1 = int(gmpy2.gcd(2 * g[0], n) == 1)
            else:
                v1 = int(gmpy2.gcd(n, g[0]) == 1) if den == 2 else 1
            nn, r = divmod(nrd_num(g, p), den * den)
            v2 = int(gmpy2.gcd(n2, nn) == n)
            v = v1 * v2
            take = v * (1 - f)
            gsel = sel_t(take, g, gsel)
            f = f | v
        op('gen', sch.K)
        ok = f
        g0 = sel_t(f, gsel, (mpz(1), mpz(0), mpz(0), mpz(0)))
        alpha, dal = normalize(g0, den)
        a0norm, _ = divmod(nrd_num(alpha0 := g0, p), den * den)
        q_, _ = divmod(2 * a0norm, n)
        m = two_e - q_
        ok &= int(m > 0)
        op('arith', 1)
        a_al = sel(int(dal == 1), 2 * alpha[0], alpha[0])
        b_al = sel(int(dal == 1), 2 * alpha[1], alpha[1])
        tmp, inv_ok = _safe_inv(a_al, n)
        ok &= inv_ok
        T = (tmp * m) % n
        vt0, vt1 = -T, mpz(0)
        op('inv', 1)
        # 2. 2-D lattice reduction, T_lat steps with freeze
        x = (b_al * tmp) % n
        a = [n - x, mpz(1)]
        b = [n, mpz(0)]
        na = a[0] * a[0] + a[1] * a[1]
        nb = b[0] * b[0] + b[1] * b[1]
        sw = int(na < nb)
        a, b = [sel(sw, b[i], a[i]) for i in range(2)], [sel(sw, a[i], b[i]) for i in range(2)]
        na, nb = sel(sw, nb, na), sel(sw, na, nb)
        done = 0
        r = mpz(0)
        nt = mpz(0)
        for _ in range(sch.T_lat):
            nn_ = a[0] * b[0] + a[1] * b[1]
            r = rounded_div(nn_, nb)
            nt = na - 2 * nn_ * r + r * r * nb
            go = int(nb > nt) * (1 - done)
            t = [a[0] - r * b[0], a[1] - r * b[1]]
            na2, nb2 = nb, nt
            a = [sel(go, b[i], a[i]) for i in range(2)]
            b = [sel(go, t[i], b[i]) for i in range(2)]
            na, nb = sel(go, na2, na), sel(go, nb2, nb)
            done = done | (1 - go)
        op('lat', sch.T_lat)
        unfin['lat'] += 1 - done
        ok &= done
        fin = int(nt < na)
        a = [sel(fin, a[i] - r * b[i], a[i]) for i in range(2)]
        Lr = [[b[0], a[0]], [b[1], a[1]]]
        Ld = Lr[0][0] * Lr[1][1] - Lr[0][1] * Lr[1][0]
        ok &= int(Ld != 0)
        Ld1 = sel(int(Ld != 0), Ld, mpz(1))
        Li = [[Lr[1][1], -Lr[0][1]], [-Lr[1][0], Lr[0][0]]]
        # 3. close vector, M4
        w0 = rounded_div(Li[0][0] * vt0 + Li[0][1] * vt1, Ld1)
        w1 = rounded_div(Li[1][0] * vt0 + Li[1][1] * vt1, Ld1)
        vd0 = vt0 - (Lr[0][0] * w0 + Lr[0][1] * w1)
        vd1 = vt1 - (Lr[1][0] * w0 + Lr[1][1] * w1)
        A, B = -vd0, -vd1
        m2, rr = divmod(m - (A * a_al + B * b_al), n)
        ok &= int(rr == 0)
        M4 = 2 * (m2 - A * A - B * B) + A * A + B * B
        op('cvp', 1)
        # 4. residues
        ok &= int(M4 > 0) * int(check_mod_condition(M4, A, B))
        ok &= int(M4.bit_length() <= sch.m4bits)
        op('mod', 1)
        # 5. trial division, fixed V_i each
        nodd = sel(int(M4 > 0), M4, mpz(1))
        vals = []
        for pr in GOOD_PRIMES:
            act = 1
            vv = 0
            for _ in range(sch.V[pr]):
                qd, rd = gmpy2.t_divmod(nodd, pr)
                d = int(rd == 0) * act
                nodd = sel(d, qd, nodd)
                vv += d
                act = d
            vals.append(vv)
        op('td', sch.td)
        one = int(nodd == 1)
        # 6. probable-prime test (always run)
        isp = int(gmpy2.is_prime(sel(one, mpz(5), nodd), 25))
        op('pt', sch.m4bits)
        ok &= sel(one, 1, isp * int(nodd % 4 == 1))
        # 7. sqrt(-1) mod nodd via a non-residue
        usable = (1 - one) * int(nodd & 1) * int(nodd > 2)
        ok &= sel(one, 1, usable)
        rmod = sel(usable, nodd, mpz(5))
        cs = 0
        cf = 0
        for c in SMALL_C[:sch.J]:
            j = int(gmpy2.jacobi(c, rmod) == -1)
            take = j * (1 - cf)
            cs = sel(take, c, cs)
            cf |= j
        op('jac', sch.J)
        ok &= sel(one, 1, cf)
        cs = sel(cf, cs, 2)
        root = pow(mpz(cs), (rmod - 1) // 4, rmod)
        op('pow', 1)
        sq_ok = int((root * root + 1) % rmod == 0)
        ok &= sel(one, 1, sq_ok)
        # 8. Cornacchia Euclid, fixed steps with freeze
        r2, r1, r0, prod = root, rmod, rmod, rmod
        fz = 0
        for _ in range(sch.T_corn):
            go = int(prod >= rmod) * (1 - fz)
            r0n = r2 % sel(int(r1 != 0), r1, mpz(1))
            r0 = sel(go, r0n, r0)
            prod = sel(go, r0n * r0n, prod)
            r2n, r1n = r1, r0n
            r2, r1 = sel(go, r2n, r2), sel(go, r1n, r1)
            fz = fz | (1 - go)
        op('euclid', sch.T_corn)
        unfin['corn'] += (1 - fz) * (1 - one)
        ok &= sel(one, 1, fz)
        aa = rmod - prod
        yy, ex = gmpy2.iroot(sel(int(aa >= 0), aa, mpz(0)), 2)
        ok &= sel(one, 1, int(ex))
        xs = sel(one, mpz(1), r0)
        ys = sel(one, mpz(0), yy)
        op('isqrt', 1)
        # 9. complex powers, fixed exponent width per prime
        for pr, vv in zip(GOOD_PRIMES, vals):
            re, im = _TAB[pr]
            w = sch.V[pr].bit_length()
            rx, ix = mpz(1), mpz(0)
            for i in range(w - 1, -1, -1):
                rx, ix = rx * rx - ix * ix, 2 * rx * ix
                bit = (vv >> i) & 1
                tx, ti = rx * re - ix * im, rx * im + ix * re
                rx, ix = sel(bit, tx, rx), sel(bit, ti, ix)
            xs, ys = xs * rx - ys * ix, xs * ix + ys * rx
        op('cpow', sch.cpow_bits)
        # 10. parity swap, gammas, theta, postconditions
        swp = int((A & 1) != (xs & 1))
        a0_, b0_ = sel(swp, ys, xs), sel(swp, xs, ys)
        ok &= int((A & 1) == (a0_ & 1)) * int((B & 1) == (b0_ & 1))
        a1 = (a0_ + A) // 2
        b1 = (b0_ + B) // 2
        enc = (a1, A - a1, b1, B - b1)
        g1, d1 = normalize(*add_el((enc[0] * n, enc[2] * n, 0, 0), 1, alpha, dal))
        g2, d2 = normalize(*add_el((enc[1] * n, enc[3] * n, 0, 0), 1, alpha, dal))
        th, dth = normalize(qmul(g2, conj(g1), p), d1 * d2 * n)
        et = endtype(th)
        po = sel(int(dth == 1), int(et in (1, 2)), int(is_odd_norm(th)))
        ok &= po
        op('post', 1)
        # 11. keep the first success
        take = ok * (1 - got)
        if take:              # bookkeeping of the *value*; a port uses a cmov
            res = ((g1, d1), (g2, d2), (th, dth), int(dth == 1))
            first = ps
        got |= ok
        if early_exit and got:
            break
    for o in ops:
        tr.update(repr(o).encode())
    return res, dict(trace=tr.hexdigest(), first=first, passes=len([o for o in ops if o[0] == 'gen']),
                     n_ops=len(ops), unfinished=unfin)
