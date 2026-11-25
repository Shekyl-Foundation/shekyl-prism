# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""SHE-24: independent step counts from the cell certificate, in exact
integer/rational arithmetic.  Written from the mathematics; shares no code
with research/tighter-progress-certificate.

Exact count:  T = max(K(R0), 0) + 6, where K(R) is the largest M >= 0 with
    Pi(M)^2 * G^2 * R >= 2^11
and Pi(M) is the worst product of M transition ratios: at most c_i
transitions in band i (counted below), each band's ratio applied to the
LAST transitions (largest ratios), the main ratio to the rest.  K = -1 when
no M works (no transition with lambda <= 2^-11 can exist): the run is then
all tail, at most 1 + 4 swaps plus the final size reduction = 6.

exp64:  T64 = max over k in {0} U {k >= 1 : 4^(58k+3) < R_C} of
    max(K(min(R0, R_k)), 0) + 6 + k,   R_0 := R0, R_k = R_C / 16^(58k+2).

Each class enters only through R0 = sup A0^2/Delta and R_C = sup C0^2/Delta.
"""
import csv
import sys
from fractions import Fraction as Fr
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SETS = ('p248', 'p324', 'p376', 'p500', 'p664')


def params(name):
    vals = {}
    for line in (ROOT / 'prism-vectors/data/signer' / (name + '.txt')).read_text().splitlines():
        if line.startswith(('e ', 'f ', 'secret_norm ')):
            k, v = line.split()
            vals[k] = v
    e = int(vals['e'])
    p = (int(vals['f']) << e) - 1
    D = int.from_bytes(bytes.fromhex(vals['secret_norm']), 'little')
    Nmax = 1 << ((p.bit_length() + 1) // 2 + 32)      # Q4: N < 2^B
    qmax = 1 << (2 * e - 5)
    return e, p, D, Nmax, qmax


def generic_ratios(p, Tmax):
    """Generic HNF U = (2T,0,0,0), V = (x,y,1,0), 0 <= x,y < 2T, T <= Tmax,
    O0 numerators: Delta = 4pT^2, after the strict ordering A0 <= 4T^2 and
    C0 <= max(4T^2, x^2+y^2+p) < 8T^2 + p.
    R0 = 4Tmax^2/p.  R_C = sup_T (8T^2+p)^2/(4pT^2), convex in T^2, so the
    sup is at T = 1 or T = Tmax."""
    R0 = Fr(4 * Tmax * Tmax, p)
    f = lambda T: Fr((8 * T * T + p) ** 2, 4 * p * T * T)
    return R0, max(f(1), f(Tmax))


def classes(name, F=256):
    e, p, D, Nmax, qmax = params(name)
    out = {}
    out['sign-generic'] = generic_ratios(p, Nmax * qmax)
    # pre-reduced key (u_s, v_s), 0 < nrd(u_s) <= nrd(v_s) <= F sqrt(p) N;
    # signing columns q v_s and u_s + gamma v_s, |gamma| <= q; T = N q.
    # A0 <= 4 q^2 F sqrt(p) N, Delta = 4 p N^2 q^2  ->  R0 = 4 F^2 q^2.
    # C0 <= 4 (1+q)^2 F sqrt(p) N              ->  R_C = 4 F^2 (1+q)^4 / q^2.
    for FF, nm in ((256, 'prered-guard256'), (2, 'prered-guard2')):
        out[nm] = (Fr(4 * FF * FF * qmax * qmax), Fr(4 * FF * FF * (1 + qmax) ** 4, qmax * qmax))
    out['keygen-D'] = (Fr(4 * D * D, p), Fr((8 * D * D + p) ** 2, 4 * p * D * D))
    out['keygen-Isec'] = generic_ratios(p, Nmax)
    out['challenge'] = generic_ratios(p, qmax)
    return out


def band_counts(band_exps, top=11):
    """Max number of transitions with lambda_{t-1} in (2^-a, 2^-b], given
    lambda_t / lambda_{t-1} = 1/r^2 >= gamma = (1/2 + 2^-top)^-2:
    1 + max{j : gamma^j < 2^(a-b)}."""
    gamma = Fr(1 << (top + 1), (1 << top) + 1) ** 2       # (1/2 + 2^-11)^-2
    out = []
    edges = band_exps
    for a, b in zip(edges[:-1], edges[1:]):
        j = 0
        while gamma ** (j + 1) < Fr(2) ** (a - b):
            j += 1
        out.append(j + 1)
    return out


def make_K(rho0, bands, G, top=11):
    """bands: list of (count, ratio) from lowest to highest lambda band."""
    seq = []                       # ratios of the LAST transitions, last first
    for c, rr in reversed(bands):
        seq += [rr] * c
    G2 = G * G
    cache = {}

    def Pi(M):
        prod = Fr(1)
        for i in range(M):
            prod *= seq[i] if i < len(seq) else rho0
        return prod

    def K(R):
        if R in cache:
            return cache[R]
        if G2 * R < Fr(1 << top):
            cache[R] = -1
            return -1
        # Pi(M) is decreasing; exponential then binary search
        lo, hi = 0, 1
        while Pi(hi) ** 2 * G2 * R >= (1 << top):
            lo, hi = hi, hi * 2
        while hi - lo > 1:
            mid = (lo + hi) // 2
            if Pi(mid) ** 2 * G2 * R >= (1 << top):
                lo = mid
            else:
                hi = mid
        cache[R] = lo
        return lo
    return K


def counts(K, R0, RC):
    T = max(K(R0), 0) + 6
    T64, kbest, k = T, 0, 1
    while Fr(4) ** (58 * k + 3) < RC:
        Rk = RC / Fr(16) ** (58 * k + 2)
        v = max(K(min(R0, Rk)), 0) + 6 + k
        if v > T64:
            T64, kbest = v, k
        k += 1
    return T, T64, kbest, k - 1


def load_G(path):
    g = []
    for line in open(path):
        s = line.split()
        if len(s) == 4 and s[0].isdigit():
            g.append(Fr(int(s[3]), 1 << 32))
    return max(g), min(g)


CLAIMED = {   # SHE-23 report, exact / exp64
    'sign-generic': ((280, 360, 415, 546, 719), (280, 360, 415, 546, 719)),
    'prered-guard256': ((267, 347, 402, 533, 706), (267, 347, 402, 533, 706)),
    'keygen-D': ((208, 323, 308, 411, 638), (208, 323, 308, 411, 638)),
    'keygen-Isec': ((21, 21, 21, 21, 21), (21, 22, 22, 22, 23)),
    'challenge': ((197, 257, 297, 396, 525), (197, 257, 297, 396, 525)),
    'prered-guard2': ((263, 344, 398, 529, 702), (263, 344, 398, 529, 702)),
}


def main():
    cert = sys.argv[1]
    G, gmin = load_G(cert)
    assert gmin >= 1
    edges = [20, 16, 14, 13, 12, 11]
    cnt = band_counts(edges)
    print('G = %s (%.9f); band transition caps for (2^-20,2^-16],...,(2^-12,2^-11]: %s' % (G, float(G), cnt))
    variants = {
        # SHE-23's stated ratios (reproduction of its arithmetic)
        'subject-ratios': (Fr(537, 2000), [Fr(271313, 10**6), Fr(300770, 10**6), Fr(300838, 10**6),
                                            Fr(300973, 10**6), Fr(304449, 10**6)]),
        # this review's ratios (results/cellcheck.txt), rounded up to 1e-9
        'review-ratios': (Fr(537, 2000), [Fr(268525146, 10**9), Fr(268605695, 10**9), Fr(268713095, 10**9),
                                           Fr(272906433, 10**9), Fr(273153948, 10**9)]),
    }
    rows = []
    for vname, (rho0, br) in variants.items():
        K = make_K(rho0, list(zip(cnt, br)), G)
        for cls in CLAIMED:
            ex, e64, kb, kmax = [], [], [], []
            for s in SETS:
                R0, RC = classes(s)[cls]
                T, T64, kbest, km = counts(K, R0, RC)
                ex.append(T); e64.append(T64); kb.append(kbest); kmax.append(km)
                rows.append(dict(variant=vname, set=s, cls=cls, log2_R0='%.3f' % (R0.numerator.bit_length() - R0.denominator.bit_length()),
                                 K=K(R0), T_exact=T, T_exp64=T64, head_k=kbest, k_enumerated=km))
            ce, c64 = CLAIMED[cls]
            print('%-15s %-16s exact %-26s exp64 %-26s | claimed %s / %s | match %s'
                  % (vname, cls, '/'.join(map(str, ex)), '/'.join(map(str, e64)),
                     '/'.join(map(str, ce)), '/'.join(map(str, c64)),
                     tuple(ex) == ce and tuple(e64) == c64))
    with open(Path(__file__).parent / 'results/mycounts.csv', 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)


if __name__ == '__main__':
    main()
