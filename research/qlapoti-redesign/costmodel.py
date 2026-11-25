# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Work-unit cost model for fixed-schedule norm-equation designs.

Units are the same as fne.work_units / fsne.Sched.pass_cost ('big
multiplications'; a one-word trial division counts 1; a probable-prime
test of b bits counts 16.5 b). These are model estimates, not cycles.
Constant-time primitives (gcd/inverse, primality test, Euclid) are costed
at their fixed-schedule length, as in fsne.py."""
import math

from fne import SET_MAP, GOOD_PRIMES, prime_of

# reference mean work per call at the pinned exponent e-2 (fne model,
# research/fixed-schedule-norm-equation results T8): the denominators of
# every padding factor in this experiment
REF_MEAN_WORK = {'p248': 313621, 'p324': 509650, 'p376': 734188,
                 'p500': 1.26e6, 'p664': 2.12e6}
# prior fixed-schedule ('fresh') factors at e-2, for comparison
PRIOR = {'p248': (49, 317), 'p324': (39, 249), 'p376': (391, 2513),
         'p500': (132, 847), 'p664': (114, 730)}


def _primes_upto(B):
    s = bytearray([1]) * (B + 1)
    s[0:2] = b'\x00\x00'
    for i in range(2, int(B ** 0.5) + 1):
        if s[i]:
            s[i * i::i] = bytearray(len(s[i * i::i]))
    return [i for i in range(B + 1) if s[i]]


_GOOD = set(GOOD_PRIMES)


def sieve_words(K):
    """64-bit word moduli needed to sieve by every non-good prime < 2^K
    (primes grouped into products that fit one word)."""
    ps = [q for q in _primes_upto((1 << K) - 1) if q not in _GOOD]
    words, acc = 0, 1
    for q in ps:
        if acc * q >= (1 << 64):
            words += 1
            acc = 1
        acc *= q
    return words + (acc > 1)


class SetCost:
    def __init__(self, name, de=2, nmin_drop=34, J=16, tail_bits=30):
        _, e, f = SET_MAP[name]
        p = prime_of(e, f)
        self.name, self.e, self.f, self.de = name, e, f, de
        self.L = p.bit_length()
        self.e_t = e - de
        self.nb = (self.L + 1) // 2 + 2
        # public lower bound on n used to size M4: n >= 2^(nb - nmin_drop);
        # a smaller n makes every pass fail (counted in the failure budget)
        self.nmin_drop = nmin_drop
        self.mb = self.e_t + 2 - (self.nb - nmin_drop)
        self.ctgcd = max(1.0, self.nb / 8)
        self.T_lat = math.ceil(0.3932 * self.nb) + 3
        self.T_corn = math.ceil(0.7202 * self.mb) + 3
        self.J = J
        V = {}
        for q in GOOD_PRIMES:
            V[q] = 2 if q == 2 else max(1, math.ceil(tail_bits / math.log2(q)) - 1)
        self.td = sum(V.values())
        self.cpow_bits = sum(v.bit_length() for v in V.values())

    # --- components
    def gen_draw(self):
        return 16 + 6 + 2 * self.ctgcd

    def gen_block(self, K):
        """K generator draws, inverse of a, 2-D lattice + T_lat Gauss steps."""
        return K * self.gen_draw() + self.ctgcd + self.T_lat * 10 + 12

    def cheap_slot(self, lam_sel=1.0):
        """lambda selection (lam_sel gcd-equivalents), inverse of lambda
        mod n, m, target, nearest plane, M4, residue bits."""
        return lam_sel * self.ctgcd + self.ctgcd + 12 + 30 + 2

    def exp_slot(self):
        mb = self.mb
        return (self.td + 16.5 * mb + self.J * max(1.0, mb / 8) + 1.5 * mb
                + 2 * self.T_corn + 10 + min(8 * self.cpow_bits, 4 * self.td)
                + 40 + 3 * self.ctgcd)

    def sieve_slot(self, K):
        return sieve_words(K) if K else 0

    @staticmethod
    def compaction(P):
        return 2.0 * P * max(1, math.ceil(math.log2(max(P, 2))))


def passes_for(rho, eps):
    """Smallest P with (1 - rho)^P <= eps."""
    if rho <= 0:
        return math.inf
    return math.ceil(math.log(eps) / math.log1p(-rho))


def binom_P_for(Q, a, eps):
    """Smallest P with Pr[Binomial(P, a) < Q] <= eps."""
    from scipy.stats import binom
    lo = Q
    hi = max(Q + 10, int(Q / a * 2 + 50))
    while binom.cdf(Q - 1, hi, a) > eps:
        hi *= 2
    while lo < hi:
        mid = (lo + hi) // 2
        if binom.cdf(Q - 1, mid, a) <= eps:
            hi = mid
        else:
            lo = mid + 1
    return lo


def one_stage(sc, rho, eps, M=1, K=13, v=0.96, lam_sel=1.0):
    """P slots, a generator block every M slots; every slot runs the full
    expensive stage. rho: per-slot success given a valid generator; v:
    per-draw generator validity (worst case)."""
    gv = 1 - (1 - v) ** K
    P = passes_for(rho * gv, eps)
    B = math.ceil(P / M)
    W = B * sc.gen_block(K) + P * (sc.cheap_slot(lam_sel) + sc.exp_slot())
    return dict(P=P, Q=P, B=B, W=W)


def two_stage(sc, a, s, eps, Ksv, M=64, K=13, v=0.96, lam_sel=1.0):
    """P cheap slots (+ sieve below 2^Ksv), oblivious compaction of the
    survivors into Q expensive slots. a: survivor rate per slot (worst);
    s: success rate per survivor (worst)."""
    gv = 1 - (1 - v) ** K
    Q = passes_for(s, eps / 2)
    P = binom_P_for(Q, a * gv, eps / 2)
    B = math.ceil(P / M)
    W = (B * sc.gen_block(K) + P * (sc.cheap_slot(lam_sel) + sc.sieve_slot(Ksv))
         + sc.compaction(P) + Q * sc.exp_slot())
    return dict(P=P, Q=Q, B=B, W=W)
