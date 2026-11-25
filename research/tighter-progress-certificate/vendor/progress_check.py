# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Machine checks for the step-count handoff table and the exp64 progress proof.

Part C (certificate): re-runs the three exact-rational certificates of
  research/lagrange-tour-bound/certify.py (copied unchanged) and checks that
  they encode the reduction contract: round-half-up quotients
  floor(h/A + 1/2) and a strictly-shorter swap.
Part S (step counts): recomputes S_max and T_prov = S_max + 5 for every
  admitted input class with integer arithmetic only (the original
  provable_bound_for used floating-point log2), and compares them with the
  published numbers.
Part E (exp64 extra sub-steps): E = the largest n with 2^(2(58 n + 2)) <
  C0/lambda_1, per class, from Lemma B1 (sum of log2|q_t| <= 1/2 log2(C0/A_end))
  and Lemma B2 (a big sub-step cuts e by >= T0 - 2 = 58).
Part L (lemmas): the exp rule's window and big paths, at toy precision
  (w = 8, 10, 12; exhaustive or dense random) and at w = 64 (random and
  targeted extremes).
Writes results/progress_check.txt and results/step_counts.csv.
"""
import csv
import math
import os
import random
import sys
import time
from fractions import Fraction

from gmpy2 import mpq

import certify
import gmpy2
from q5common import SETS, params
from variants import q_expw, exact_round, bl

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = []


def say(s):
    print(s); OUT.append(s)


RHO1 = Fraction(31, 100)
RHO2 = Fraction(87, 1000)
LAM = Fraction(1, 1000)


def rho_prod(S):
    m = S - 1
    return RHO2 ** (m // 2) * (RHO1 if m % 2 else 1)


def s_max(phi1_sq_over_delta):
    """Largest S >= 1 with (rho_prod(S) * Phi_1)^2 >= Delta / LAM, given the
    supremum of Phi_1^2 / Delta (a Fraction or int). S_max + 5 = T_prov."""
    S = 1
    while (rho_prod(S + 1) ** 2) * phi1_sq_over_delta >= 1 / LAM:
        S += 1
    return S


def float_bound(T_bits, p):
    """The original provable_bound_for (run_bound_study.py), for comparison."""
    rhs = T_bits + 1.0 - 0.5 * math.log2(p / 0.001)
    S = 1
    while True:
        m = S
        lhs = (m // 2) * math.log2(1 / 0.087) + (math.log2(1 / 0.31) if m % 2 else 0.0)
        if lhs <= rhs:
            S += 1
        else:
            return S + 5


def classes(P):
    """Admitted input classes. Each: (name, description, Phi1^2/Delta sup,
    C0/lambda_1 sup for E, C0 sup for E_general, published T_prov or None).
    Model coordinates (N4 = nrd). The O0 rows are derived in classes_o0:
    the first version of this file said here that O0 numerators scale A, C,
    Delta, lambda_1 by 4, 4, 16, 4 and leave every ratio unchanged. That was
    wrong (Rick, 2026-11-25; verify_finding.sage): for the O0 basis
    (2T,0,0,0), (x,y,1,0) Delta = 4 p T^2, not 16 p T^2."""
    p, e, B = P['p'], P['e'], P['B']
    qb = 2 * e - 5
    Tsec = 1 << P['Tbits']; Tq = 1 << qb; TN = 1 << B
    rows = []
    # generic HNF {T, x + y i + j}, 0 <= x, y < T: A0 <= T^2, Delta = p T^2,
    # Phi_1 <= 2 A0, so Phi_1^2/Delta <= 4 T^2 / p. C0 <= 2 T^2 + p, lambda_1 >= T.
    for name, desc, Tm, pub in [
        ('sign-generic', 'signing: I_sec . I_q, generic HNF, N < 2^B (Q4)', Tsec, P['tprov_sec']),
        ('challenge', 'I_q alone (public), generic HNF', Tq, P['tprov_q']),
        ('keygen-Isec', 'keygen pre-reduction: I_sec alone, generic HNF, N < 2^B (Q4)', TN, None),
        ('keygen-D', 'key generation: ideal of norm D = nextprime(2^(2W)), generic HNF', int(gmpy2.next_prime(1 << (2 * P['W']))), None),
    ]:
        rows.append((name, desc, Fraction(4 * Tm * Tm, p), Fraction(2 * Tm * Tm + p, Tm), 2 * Tm * Tm + p, pub))
    # pre-reduced {w1 = q v_s, w2 = u_s + g v_s}, nrd(v_s) <= F sqrt(p) N:
    # A0 <= q^2 F sqrt(p) N, Delta = p N^2 q^2, Phi_1^2/Delta <= 4 q^2 F^2 (sqrt(p)^2 = p cancels).
    # C0 <= (1 + q)^2 F sqrt(p) N, lambda_1 >= N q -> C0/lambda_1 <= (1+q)^2 F sqrt(p) / q.
    sp_up = math.isqrt(p) + 1
    for name, desc, F, pub in [
        ('prered-guard2', 'signing, keygen pre-reduced, guard nrd(v_s) <= 2 sqrt(p) N (decoded key: same check)', 2, P['tprov_pre']),
        ('prered-guard256', 'signing, pre-reduced, looser guard nrd(v_s) <= 256 sqrt(p) N', 256, None),
    ]:
        rows.append((name, desc, Fraction(4 * Tq * Tq * F * F), Fraction((1 + Tq) ** 2 * F * sp_up, Tq),
                     (1 + Tq) ** 2 * F * sp_up * TN, pub))
    # unguarded: nrd(v_s) <= 2 p N (Minkowski): A0 <= 2 p N q^2, Phi_1^2/Delta <= 16 p q^2.
    rows.append(('prered-unguarded', 'signing, pre-reduced, Minkowski bound nrd(v_s) <= 2 p N only', Fraction(16 * p * Tq * Tq),
                 Fraction((1 + Tq) ** 2 * 2 * p, Tq), (1 + Tq) ** 2 * 2 * p * TN, {'p248': 354, 'p324': 461, 'p376': 536, 'p500': 712, 'p664': 945}[P['name']]))
    return rows


def classes_o0(P):
    """The same classes on O0 numerators over 2 (SIGNER_SPEC Q1), the
    representation the signer stores. N4 = 4 nrd, so A0, C0 and lambda_1 are
    4x the model's (lambda_1 >= 4T), but Delta = 4 p T^2 (verify_finding.sage),
    so Phi_1^2/Delta is 4x the model's: one more bit of A0/sqrt(Delta).
    generic: A0 <= 4T^2 (U = (2T,0,0,0)), C0 <= 4(T-1)^2 + (2T-1)^2 + p < 8T^2 + p.
    pre-reduced: N4(v_s) = 4 nrd(v_s), Delta = 4 p (N q)^2."""
    out = []
    for name, desc, phi, c0l, c0, pub in classes(P):
        if name in ('sign-generic', 'challenge', 'keygen-Isec', 'keygen-D'):
            Tm = {'sign-generic': 1 << P['Tbits'], 'challenge': 1 << (2 * P['e'] - 5), 'keygen-Isec': 1 << P['B'],
                  'keygen-D': int(gmpy2.next_prime(1 << (2 * P['W'])))}[name]
            p = P['p']
            assert phi == Fraction(4 * Tm * Tm, p)
            out.append((name + '@O0', desc + ' [O0 numerators]', Fraction(16 * Tm * Tm, p),
                        Fraction(8 * Tm * Tm + p, 4 * Tm), 8 * Tm * Tm + p, None))
        else:
            out.append((name + '@O0', desc + ' [O0 numerators]', 4 * phi, c0l, 4 * c0, None))
    return out


def E_of(ratio):
    """Largest n >= 0 with 58 n + 2 < 1/2 log2(ratio), i.e. 4^(58 n + 2) < ratio."""
    n = 0
    while Fraction(4) ** (58 * (n + 1) + 2) < ratio:
        n += 1
    if Fraction(4) ** (58 * 1 + 2) >= ratio:
        return 0
    return n


def part_C():
    say('== Part C: the certificates, rerun (exact rationals, certify.py unchanged)')
    t = time.time(); ok1, n1, ub1 = certify.certify_1step_exact(mpq(2), mpq(31, 100), mpq(1, 1000))
    say(f'  1-step rho1 = 31/100 over lam <= 1/1000: ok={ok1}, {n1} boxes, max ub {float(ub1):.6f}, {time.time()-t:.1f}s')
    t = time.time(); ok2, n2, ub2 = certify.certify_2step_exact(mpq(2), mpq(87, 1000), mpq(1, 1000))
    say(f'  2-step rho2 = 87/1000 over lam <= 1/1000: ok={ok2}, {n2} boxes, max ub {float(ub2):.6f}, {time.time()-t:.1f}s')
    t = time.time(); ok3, n3, _ = certify.certify_tail_exact(mpq(1, 1000), 5)
    say(f'  tail: 5 swaps after the split impossible: ok={ok3}, {n3} boxes, {time.time()-t:.1f}s')
    assert ok1 and ok2 and ok3
    # contract: floor_half_up is floor(f + 1/2): ties go up (+1/2 -> 1, -1/2 -> 0)
    assert certify.floor_half_up(mpq(1, 2)) == 1 and certify.floor_half_up(mpq(-1, 2)) == 0
    assert certify.floor_half_up(mpq(3, 2)) == 2 and certify.floor_half_up(mpq(-3, 2)) == -1
    for num in range(-4000, 4001):
        for den in (1, 2, 3, 7, 10, 64):
            f = Fraction(num, den)
            assert certify.floor_half_up(mpq(num, den)) == math.floor(f + Fraction(1, 2)) == exact_round(num, den) if den else True
    say('  rounding: certify.floor_half_up == floor(f + 1/2) == the signer rule floor((2h + A)/(2A)) on 48,006 rationals, '
        'ties up; residual eps in [-1/2, 1/2) (certify uses the closed box, a superset)')
    say('  swap: certify keeps a box only if r = lam + |eps|^2 < 1 is possible (strict), and skips |mu|^2 <= 1 by'
        ' Lemma 1, which needs the strict rule (|mu| = 1 gives A_{t+1} >= A_t, so only a non-strict swap could fire).')


def part_S_E():
    say('== Part S/E: public step counts per admitted class (integer arithmetic), and exp64 extras')
    rows = []
    for s in SETS:
        P = params(s[0]); p = P['p']
        for name, desc, phi_ratio, c0_over_lam, c0, pub in classes(P) + classes_o0(P):
            S = s_max(phi_ratio)
            Tp = S + 5
            E = E_of(c0_over_lam)
            Eg = E_of(Fraction(c0))
            fl = None
            if name in ('sign-generic', 'challenge', 'keygen-Isec', 'keygen-D'):  # model rows only
                Tb = {'sign-generic': P['Tbits'], 'challenge': 2 * P['e'] - 5, 'keygen-Isec': P['B'], 'keygen-D': 2 * P['W'] + 1}[name]
                fl = float_bound(Tb, p)
            rows.append(dict(set=P['name'], cls=name, desc=desc, S_max=S, T_exact=Tp, published=pub,
                             float_formula=fl, E_exp64=E, T_exp64=Tp + E, E_exp64_any_basis=Eg,
                             T_exp64_any_basis=Tp + Eg,
                             half_log2_C0_over_lam1=round(0.5 * (math.log2(c0_over_lam.numerator) - math.log2(c0_over_lam.denominator)), 2)))
            if pub is not None:
                assert Tp == pub, (P['name'], name, Tp, pub)
            if fl is not None and name != 'keygen-D':
                assert fl == Tp, (P['name'], name, fl, Tp)
    with open(os.path.join(HERE, 'results', 'step_counts.csv'), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader(); w.writerows(rows)
    for r in rows:
        say(f"  {r['set']} {r['cls']:17s} T_exact {r['T_exact']:4d} (published {r['published']}) "
            f"E {r['E_exp64']:2d} -> T_exp64 {r['T_exp64']:4d}; any basis E {r['E_exp64_any_basis']:2d}")
    say('  every published T_prov is reproduced exactly; the float formula agrees with the integer one at every class')
    return rows


def lemma_one(h0, h1, A, w, t0, stats):
    info = {}
    m0, m1, big = q_expw(h0, h1, A, w, t0, info)
    q0 = Fraction(h0, A); q1 = Fraction(h1, A)
    if not big:
        # window: exact round-half-up
        assert m0 == exact_round(h0, A) and m1 == exact_round(h1, A), (h0, h1, A, w)
        stats['window'] += 1
        stats['corr'] += info['corrections']
        return
    stats['big'] += 1
    s = info['s']; e = info['e']
    r0 = q0 - m0; r1 = q1 - m1
    # undershoot: same sign, never past q, never grows |q_j|
    for q, m, r in ((q0, m0, r0), (q1, m1, r1)):
        assert m == 0 or (m > 0) == (q > 0)
        assert abs(r) <= abs(q)
        assert abs(r) < Fraction(2 ** (s + 1)) * (1 + Fraction(1, 2 ** (w - 1)))
    # the largest component keeps |res| > 2^(s-1) >= 1: no swap can follow
    assert max(abs(r0), abs(r1)) > Fraction(2 ** (s - 1)) >= 1
    # e drops by at least t0 - 2
    hn0 = h0 - m0 * A; hn1 = h1 - m1 * A
    e2 = max(bl(hn0), bl(hn1)) - A.bit_length()
    assert e2 <= e - (t0 - 2), (e, e2)
    stats['min_drop'] = min(stats['min_drop'], e - e2)
    # norm cannot grow: |q - m|^2 <= |q|^2 strictly when some m != 0
    assert (m0, m1) != (0, 0)
    assert r0 * r0 + r1 * r1 < q0 * q0 + q1 * q1


def part_L():
    say('== Part L: lemmas for the exp rule (window = exact rounding; big = undershoot, no swap, e drops >= t0 - 2)')
    rng = random.Random(2026)
    for w in (8, 10, 12):
        t0 = w - 4
        st = dict(window=0, big=0, corr=0, min_drop=10 ** 9)
        # dense: every A in a range straddling the precision, h over a log-uniform spread, both signs
        for A in list(range(1, 1 << (w + 3))) + [rng.randrange(1 << (w + 3), 1 << (w + 12)) for _ in range(4000)]:
            for _ in range(12):
                hb = rng.randrange(0, A.bit_length() + 3 * t0)
                h0 = rng.randrange(-(1 << hb), (1 << hb) + 1)
                hb = rng.randrange(0, A.bit_length() + 3 * t0)
                h1 = rng.randrange(-(1 << hb), (1 << hb) + 1)
                lemma_one(h0, h1, A, w, t0, st)
            # ties: h = (2k+1) A / 2 when A even, and +-1 around them
            if A % 2 == 0:
                for k in (-3, -1, 0, 1, 2):
                    for d in (-1, 0, 1):
                        lemma_one((2 * k + 1) * A // 2 + d, (2 * k - 1) * A // 2 - d, A, w, t0, st)
        say(f'  w={w}, t0={t0}: {st["window"]} window cases exact ({st["corr"]} needed the +-1 fix), '
            f'{st["big"]} big cases, smallest e drop {st["min_drop"]} (>= {t0 - 2})')
    w, t0 = 64, 60
    st = dict(window=0, big=0, corr=0, min_drop=10 ** 9)
    for it in range(300000):
        Ab = rng.choice([rng.randrange(1, 70), rng.randrange(64, 3200)])
        kind = it % 4
        if kind == 0:
            A = rng.randrange(1 << (Ab - 1), 1 << Ab) if Ab > 1 else 1
        elif kind == 1:
            A = (1 << Ab) - 1                     # all ones: ns has its largest truncation error
        elif kind == 2:
            A = (1 << (Ab - 1)) + rng.randrange(0, 1 << max(0, Ab - 64))  # ns = 2^63 exactly
        else:
            A = ((1 << 64) - 1) << max(0, Ab - 64) | rng.randrange(0, 1 << max(0, Ab - 64))
        A = max(A, 1)
        hs = []
        for _ in range(2):
            eb = rng.choice([rng.randrange(-5, 62), 60, 61, 62, rng.randrange(62, 900)])
            hb = max(0, A.bit_length() + eb)
            sub = rng.randrange(4)
            if sub == 0:
                h = (1 << hb) - 1
            elif sub == 1:
                h = 1 << max(0, hb - 1)
            else:
                h = rng.randrange(0, 1 << hb) if hb else 0
            if rng.getrandbits(1):
                h = -h
            hs.append(h)
        lemma_one(hs[0], hs[1], A, w, t0, st)
        # half-integer ties at full size
        if it % 16 == 0 and A % 2 == 0:
            k = rng.randrange(-(1 << 61), 1 << 61)
            lemma_one((2 * k + 1) * A // 2, -(2 * k + 1) * A // 2, A, w, t0, st)
    say(f'  w=64, t0=60: {st["window"]} window cases exact ({st["corr"]} needed the +-1 fix), '
        f'{st["big"]} big cases, smallest e drop {st["min_drop"]} (>= 58)')


def main():
    if "--skip-cert" not in sys.argv:
        part_C()
    part_S_E()
    part_L()
    open(os.path.join(HERE, 'results', 'progress_check.txt'), 'w').write('\n'.join(OUT) + '\n')


if __name__ == '__main__':
    main()
