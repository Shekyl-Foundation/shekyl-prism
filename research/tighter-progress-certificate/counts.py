# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Certified public step counts from the cell potential (exact integers and
fractions only).

Ingredients (REPORT.md section 2):
  * main phase: a cell certificate (cellcert check / check_potential.py)
    proving Psi_t <= rho * Psi_{t-1} for every transition with
    lam_{t-1} <= LAM and an n = 0 next step, with 1 <= g <= G;
  * charge lemma: a transition whose next logical step has n >= 1 exp64
    big sub-steps satisfies Psi_t <= rho^(1+n) Psi_{t-1}
    (needs G * 2^-(116 n + 4) <= rho^(1+n));
  * boundary lemma: 2^(2(58 n + 2)) >= rho^(-2(1+n)) / LAM for n >= 1;
  * no big quotient in the tail: LAM >= 2^-120;
  * tail: at most tau swaps once lam > LAM (certify_tail_exact);
  * head: exp64 big sub-steps at step 1 force A0 < C0 / 4^(58 n1 + 2).

  T_exact = K(R0) + tau + 2
  T_exp64 = max(K(R0), max_{k >= 1} K(R_k) + k) + tau + 2,
  R_k = min(R0, RC / 16^(58 k + 2)),
  K(R) = the largest M >= -1 with rho^(2M) G^2 R LAM >= 1 (M = -1 if none),
  R0 = sup A0^2 / Delta, RC = sup C0^2 / Delta over the class.
The counts are scale-invariant (ratios only), so they hold for O0
numerators and for the normalized Gram alike.
"""
import csv, os, sys
from fractions import Fraction as F
import gmpy2

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, 'vendor'))
from q5common import SETS, params


def read_cert(path):
    rho = lam = None; gmax = 0
    for line in open(path):
        if line.startswith('rho'):
            a, b = line.split()[1].split('/'); rho = F(int(a), int(b))
        elif line.startswith('lam'):
            lam = F(1, 1 << int(line.split()[1]))
        elif not line.startswith('#'):
            gmax = max(gmax, int(line.split()[3]))
    return rho, lam, F(gmax, 1 << 32)


def lemmas(rho, lam, G):
    """rho = rho_1 (the main certificate), lam = l_B (the top band)."""
    assert F(1, 4) < rho < 1
    # charge lemma, n = 1, and each further n multiplies the left side by 2^-116 <= rho
    assert G * F(1, 2 ** 120) <= rho ** 2 and F(1, 2 ** 116) <= rho
    # boundary lemma, n = 1, and each further n: 2^116 >= rho^-2
    assert F(2 ** 120) >= 1 / (rho ** 4 * lam) and F(2 ** 116) >= 1 / rho ** 2
    assert lam >= F(1, 2 ** 120)
    return True


def band_caps(bands):
    """bands = [(l_1, rho_1), ..., (l_B, rho_B)], l ascending, band b covers
    lam in (l_{b-1}, l_b] (band 1: [0, l_1]).  A swap with lam <= l_B has
    r <= 1/2 + l_B, so lam grows by f = (1/2 + l_B)^-2 per swap, and band
    b >= 2 holds at most 1 + max{k : l_{b-1} f^k < l_b} transitions."""
    lB = bands[-1][0]
    f = 1 / (F(1, 2) + lB) ** 2
    caps = [None]
    for b in range(1, len(bands)):
        lo, hi = bands[b - 1][0], bands[b][0]
        k = 0
        while lo * f ** (k + 1) < hi:
            k += 1
        caps.append(1 + k)
    return caps


def K(R, bands, G):
    """Largest M >= -1 such that M units can fit: j_b <= cap_b units at
    rho_b (b >= 2, the worst-first allocation) and the rest at rho_1, with
    (product)^2 G^2 R l_B >= 1.  M = -1 if even M = 0 fails."""
    rho1, lB = bands[0][1], bands[-1][0]
    caps = band_caps(bands)
    order = sorted(range(1, len(bands)), key=lambda b: -bands[b][1])
    base = G * G * R * lB
    if base < 1:
        return -1
    M = 0
    while True:
        m = M + 1; prod = F(1); left = m
        for b in order:
            j = min(caps[b], left); prod *= bands[b][1] ** j; left -= j
        prod *= rho1 ** left
        if prod * prod * base >= 1:
            M = m
        else:
            return M


def classes_o0(P):
    """(name, R0 = sup A0^2/Delta, RC = sup C0^2/Delta) in O0 numerators
    (N4 = 4 nrd).  Ratios, so identical in nrd units and in the normalized
    Gram (Delta = p)."""
    p, e, B = P['p'], P['e'], P['B']
    Tq = 1 << (2 * e - 5)
    out = []
    for name, Tmax, fixed in [('sign-generic', 1 << P['Tbits'], False), ('challenge', Tq, False),
                              ('keygen-Isec', 1 << P['B'], False),
                              ('keygen-D', int(gmpy2.next_prime(1 << (2 * P['W']))), True)]:
        # A0 <= 4 T^2, C0 < 8 T^2 + p, Delta = 4 p T^2 (U = (2T,0,0,0), V = (x,y,1,0))
        R0 = F(16 * Tmax ** 4, 4 * p * Tmax ** 2)
        f = lambda T: F((8 * T * T + p) ** 2, 4 * p * T * T)
        RC = f(Tmax) if fixed else max(f(Tmax), f(1))  # convex in T: max at an endpoint
        out.append((name, R0, RC))
    for name, Fg in [('prered-guard2', 2), ('prered-guard256', 256)]:
        # A0 <= 4 q^2 F sqrt(p) N, C0 <= 4 (1+q)^2 F sqrt(p) N, Delta = 4 p N^2 q^2
        out.append((name, F(4 * Tq * Tq * Fg * Fg), F(4 * (1 + Tq) ** 4 * Fg * Fg, Tq * Tq)))
    # unguarded: nrd(v_s) <= 2 p N: A0 <= 8 p N q^2, C0 <= 8 p N (1+q)^2
    out.append(('prered-unguarded', F(16 * p * Tq * Tq), F(16 * p * (1 + Tq) ** 4, Tq * Tq)))
    return out


def counts(bands, G, tau):
    rho, lam = bands[0][1], bands[-1][0]
    assert all(bands[b][1] >= rho for b in range(len(bands)))
    lemmas(rho, lam, G)
    rows = []
    for s in SETS:
        P = params(s[0])
        for name, R0, RC in classes_o0(P):
            k0 = K(R0, bands, G)
            Tex = k0 + tau + 2
            best = k0; kk = 1
            while F(16) ** (58 * kk + 2) < RC:          # n1 = kk is possible at all
                Rk = min(R0, RC / F(16) ** (58 * kk + 2))
                best = max(best, K(Rk, bands, G) + kk)
                kk += 1
            rows.append(dict(set=P['name'], cls=name, T_exact=Tex, T_exp64=best + tau + 2, n1_max=kk - 1))
    return rows


if __name__ == '__main__':
    rho, lam, G = read_cert(sys.argv[1]); tau = int(sys.argv[2])
    bands = [(lam, rho)]
    for a in sys.argv[3:]:
        l, r = a.split(':'); n, d = r.split('/')
        bands.append((F(1, 1 << int(l)), F(int(n), int(d))))
    print(f'rho {rho} = {float(rho):.6f}, LAM {lam}, G {float(G):.6f}, tau {tau}, caps {band_caps(bands)}')
    for r in counts(bands, G, tau):
        print(r)
