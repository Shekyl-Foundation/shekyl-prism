# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Machine-checked arithmetic for the Q5 width proof (REPORT.md, section 3).

Everything here is exact integer / Fraction arithmetic: no floats enter a
bound. For every set and every input variant it computes M0 (the largest
starting value of the form N4), M* = ceil(M0 (1 + eps)^S) (Lemma 3), and
from M* the bound on every named intermediate (Lemma 4), and writes
results/proven_bounds.csv and results/proven_summary.csv.

Run: /workspace/mamba/envs/sage/bin/python3 bounds.py
"""
import csv
import math
import os
import sys
from fractions import Fraction as F

from q5common import SETS, RUST_C, params, isqrt_ceil

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'results')

PREC = 64
DELTA = F(1, 2 ** (PREC - 1))                   # ns >= 2^63 when k > 0
EPS = 12 * DELTA / (1 - DELTA) ** 2             # Lemma 3
C_MU = 1 + 2 * DELTA                            # |m_j| <= C_MU |q_j| + C_HALF
C_HALF = F(1, 2) + 2 * DELTA


def ceil_frac(x):
    return -((-x.numerator) // x.denominator)


def sqrt_bound(c, M):
    """Smallest integer >= c * sqrt(M), c a Fraction (exact)."""
    return isqrt_ceil(ceil_frac(c * c * M))


def mstar(M0, S):
    return ceil_frac(F(M0) * (1 + EPS) ** S)


def next_prime_above(n):
    import gmpy2
    return int(gmpy2.next_prime(n))


def start_bounds(P, rep):
    """M0 for each input variant, in N4 units of the representation.
    rep 'O0': numerators over 2 (N4 = 4 nrd); 'model': Z<1,i,j,k> (N4 = nrd).
    Returns list of (variant, M0, S, note)."""
    p, e, B, W, L = P['p'], P['e'], P['B'], P['W'], P['L']
    s4 = 4 if rep == 'O0' else 1        # N4 / nrd
    out = []

    def gen(T):
        # generic Z[i]-HNF {T, alpha}: O0 numerators (2T,0,0,0), (x,y,1,0), x,y in [0,2T);
        # model (T,0,0,0), (x,y,1,0), x,y in [0,T)
        if rep == 'O0':
            return max(4 * T * T, 2 * (2 * T - 1) ** 2 + p)
        return max(T * T, 2 * (T - 1) ** 2 + p)

    def gen_centered(T):
        if rep == 'O0':
            return max(4 * T * T, 2 * T * T + p)
        return max(T * T, 2 * ((T + 1) // 2) ** 2 + p)

    Tsig = (1 << (B + 2 * e - 5)) - 1
    Tq = (1 << (2 * e - 5)) - 1
    Nmax = (1 << B) - 1
    qmax = Tq
    D = next_prime_above(1 << (2 * W))
    S_sec, S_q, S_pre, S_preu = P['tprov_sec'], P['tprov_q'], P['tprov_pre'], P['tprov_pre_ung']
    S_kg = S_sec  # keygen's ideal (norm D) is smaller than the signing bound at all five sets
    assert D.bit_length() <= B + 2 * e - 5

    out.append(('sign_generic', gen(Tsig), S_sec + 1, 'I_sec cap I_q, generic Z[i]-HNF, Q4 enforced'))
    out.append(('sign_generic_centered', gen_centered(Tsig), S_sec + 1, 'same, x,y centred mod T'))
    out.append(('Iq_only', gen(Tq), S_q + 1, 'challenge ideal alone'))
    out.append(('keygen_D', gen(D), S_kg + 1, 'key generation: ideal of norm D'))
    out.append(('keygen_prereduce_Isec', gen(Nmax), S_sec + 1, 'keygen pre-reduction of I_sec (norm N < 2^B)'))
    # shape-agnostic: every numerator |x_r| <= 2T (O0) or T (model)
    X = 2 * Tsig if rep == 'O0' else Tsig
    out.append(('sign_any_shape', (2 + 2 * p) * X * X, S_sec + 1, 'any HNF shape, all |numerators| <= 2T'))

    # pre-reduced signing: start {w1 = q v_s, w2 = u_s + gamma v_s}, |gamma_j| <= (q-1)/2,
    # N4(u_s) <= N4(v_s) <= C4 (decoder-checked)
    def pre(C4):
        g2 = 2 * ((qmax - 1) // 2) ** 2          # |gamma|^2
        w1 = qmax * qmax * C4
        # N4(w2) <= (sqrt(A)+|gamma| sqrt(C))^2 <= C4 (1 + |gamma|)^2
        w2 = ceil_frac(F(C4) * (1 + F(math.isqrt(g2) + 1)) ** 2)
        return max(w1, w2)
    # guard: nrd(v_s) <= 2 sqrt(p) N  ->  N4 <= s4 * 2 sqrt(p) N
    C4_guard = isqrt_ceil(4 * p * Nmax * Nmax * s4 * s4)
    out.append(('sign_prereduced_guarded', pre(C4_guard), S_pre + 1,
                'keygen pre-reduced, guard nrd(v_s) <= 2 sqrt(p) N'))
    # unguarded, generic shape (A_s >= p/4 nrd in O0, >= p in model):
    if rep == 'O0':
        C_gen = max(4 * Nmax * Nmax + ceil_frac(F(p, 2)), sqrt_bound(F(1), 8 * p * Nmax * Nmax))
        C_any = p * Nmax + 2 * Nmax
    else:
        C_gen = max(Nmax * Nmax + ceil_frac(F(p, 2)), sqrt_bound(F(1), 2 * p * Nmax * Nmax))
        C_any = p * Nmax + ceil_frac(F(Nmax, 2))
    out.append(('sign_prereduced_unguarded_generic', pre(C_gen), S_preu + 1,
                'pre-reduced, no guard, generic-shape I_sec'))
    out.append(('sign_prereduced_unguarded_anyshape', pre(C_any), S_preu + 1,
                'pre-reduced, no guard, any shape (A_s >= N)'))
    # Q4 not enforced: N only proven < 2^(2L+18) (SIGNER_SPEC 2.1.2)
    Tnoq4 = (1 << (2 * L + 18 + 2 * e - 5)) - 1
    out.append(('sign_generic_noQ4', gen(Tnoq4), S_sec + 1, 'Q4 NOT enforced (N < 2^(2L+18))'))
    return out


def intermediate_bounds(P, M0, S):
    """Lemma 4: bound on each named intermediate, from M* (exact)."""
    p = P['p']
    Ms = mstar(M0, S)
    b = {}
    b['init.nrd.N4'] = M0
    for pre in ('', 'cl.'):
        b[pre + 'herm.prod'] = Ms
        b[pre + 'herm.partial'] = Ms
        b[pre + 'herm.prod_j'] = Ms // p
        b[pre + 'herm.partial_j'] = Ms // p
        b[pre + 'herm.h'] = Ms
        b[pre + 'nrd.prod'] = Ms
        b[pre + 'nrd.partial'] = Ms
        b[pre + 'nrd.prod_j'] = Ms // p
        b[pre + 'nrd.partial_j'] = Ms // p
        b[pre + 'nrd.N4'] = Ms
        c_mu = (C_MU + C_HALF) if pre == '' else F(3, 2)
        b[pre + 'upd.mu_u'] = sqrt_bound(c_mu, Ms)
        b[pre + 'upd.mu_u_j'] = sqrt_bound(c_mu, Ms // p + 1)
        c_a = (C_MU + F(7072, 10000) + 2 * DELTA) if pre == '' else F(17072, 10000)
        b[pre + 'upd.a'] = sqrt_bound(c_a, Ms)
        b[pre + 'upd.a_j'] = sqrt_bound(c_a, Ms // p + 1)
        b[pre + 'upd.v'] = math.isqrt(Ms)
        b[pre + 'upd.v_j'] = math.isqrt(Ms // p)
    b['state.coord'] = math.isqrt(Ms)
    b['in.coord'] = math.isqrt(M0)
    b['cl.round_num'] = 3 * Ms
    b['cl.round_den'] = 2 * Ms
    b['cl.m'] = math.isqrt(Ms) + 1           # |q| <= sqrt(C/A), A >= 1
    b['q64.hs'] = 1 << 124
    b['q64.ns'] = (1 << 64) - 1
    b['q64.num'] = (1 << 125) + (1 << 64)
    b['q64.den'] = (1 << 65) - 2
    b['q64.m'] = 1 << 62
    b['conv.coord'] = math.isqrt(Ms)
    b['conv.gram'] = Ms
    b['conv.gram_ref2x'] = 2 * Ms
    return Ms, b


def main():
    os.makedirs(OUT, exist_ok=True)
    rows, summ = [], []
    for s in SETS:
        P = params(s[0])
        for rep in ('O0', 'model'):
            for variant, M0, S, note in start_bounds(P, rep):
                Ms, b = intermediate_bounds(P, M0, S)
                mx_name = max(b, key=lambda k: b[k])
                for k, v in sorted(b.items()):
                    rows.append(dict(set=P['name'], rep=rep, variant=variant, intermediate=k,
                                     bound=v, bound_bits=v.bit_length()))
                top = b[mx_name].bit_length()
                C = RUST_C[P['name']]
                rust_ok = (b['upd.a'] + b['state.coord'] < 2 ** (64 * C - 1)
                           and 3 * Ms < 2 ** (128 * C - 1))
                summ.append(dict(set=P['name'], rep=rep, variant=variant, steps=S,
                                 M0_bits=M0.bit_length(), Mstar_bits=Ms.bit_length(),
                                 max_intermediate=mx_name, max_bits=top,
                                 width=P['width'], spare=P['width'] - top,
                                 need_row3=None, fits=top <= P['width'],
                                 rust_proto_fits=rust_ok, note=note))
    # SIGNER_SPEC row 3 / need, for the comparison column
    for r in summ:
        P = params(r['set'])
        need = max(2 * (2 * P['W'] + 1), 2 * (2 * P['e'] - 5 + P['B'])) + P['L'] + 4
        r['need_row3'] = need
    with open(os.path.join(OUT, 'proven_bounds.csv'), 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader(); w.writerows(rows)
    with open(os.path.join(OUT, 'proven_summary.csv'), 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(summ[0].keys()))
        w.writeheader(); w.writerows(summ)
    print('eps =', float(EPS), 'log2 eps =', math.log2(float(EPS)))
    for r in summ:
        if r['rep'] == 'O0':
            print(f"{r['set']} {r['variant']:38s} S={r['steps']:4d} M0={r['M0_bits']:5d} "
                  f"M*={r['Mstar_bits']:5d} max={r['max_bits']:5d} ({r['max_intermediate']}) "
                  f"width={r['width']} spare={r['spare']:5d} need={r['need_row3']} rust={r['rust_proto_fits']}")
    return 0


if __name__ == '__main__':
    sys.exit(main())
