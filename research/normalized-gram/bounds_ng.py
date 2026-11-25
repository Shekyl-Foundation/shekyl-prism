# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Public width bounds for the normalized Gram reducer, the limb counts
chosen from them, and the exp64 sub-step budget in normalized units.

Notation: M0 is a public upper bound on the larger starting diagonal entry
of the normalized Gram (numerator entries divided by 2T). The lemmas used
(REPORT.md section 3):

  W1  A, C <= M0 at every step (exact steps never lengthen V; exp64 big
      sub-steps never lengthen V, Lemma U(ii) of clamped-quotient-progress;
      swaps exchange). |h_j| < sqrt(A C) <= M0 (det = p > 0).
  W2  quotient, both paths: |mw| <= |h|/A + 1 on the window path, and
      qtop A <= |h| (1 + 2^-62) / 2^s on the big path, so |mw A| <= 2 M0;
      X = 2h + A - 2 mw A and X - 2A are at most 9 M0 in magnitude.
  W3  update: |m 2^s A| <= |h| + A/2 <= 1.5 M0, |r| <= M0, |h + r| <= 2 M0,
      |m (h + r)| 2^s <= C + |h| + A/4 <= 2.25 M0 (window) and
      <= 2 C <= 2 M0 (big); the partial C - c0 lies in [C', C].
  W4  coordinates (numerator units, unchanged by normalization): products
      <= 1.5 sqrt(M_num), the pre-shift combination <= 3 sqrt(M_num), the
      partial difference <= 4 sqrt(M_num), M_num = 2 T M0.
Every intermediate is therefore below 9 M0 (Gram) or 4 sqrt(2 Tmax M0)
(coordinates); a signed limb array needs one more bit.
"""
import json
import math
import os
from fractions import Fraction

import gmpy2

import ngram as n
from q5common import params

HERE = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HERE, 'results')

# Analytic factor per observed category (W1 to W3), in units of M0.
FACTOR = {'A': 1, 'C': 1, 'h': 1, 'two_A': 2, 'X': 9, 'mA_est': 2, 'mA': Fraction(3, 2), 'mA_shift': Fraction(3, 2),
          'r': 1, 'h_plus_r': 2, 'm_hr': Fraction(9, 4), 'm_hr_shift': Fraction(9, 4), 'C_partial': 1}
COORD = {'coord_prod': Fraction(3, 2), 'coord_pre': 3, 'coord_shift': 3, 'coord': 4}
RUST_C = {'p248': 11, 'p324': 14, 'p376': 16, 'p500': 21, 'p664': 27}


def isqrt_up(x):
    r = math.isqrt(x)
    return r if r * r == x else r + 1


def class_bounds(P):
    """(Tmax exclusive, M0 normalized inclusive) per admitted class."""
    p, e, B = P['p'], P['e'], P['B']
    qmax = 1 << (2 * e - 5)
    D = int(gmpy2.next_prime(1 << (2 * P['W'])))
    out = {}
    # Generic HNF: A0 = 2T, C0 = (x^2 + y^2 + p) / (2T) < 4T + p/(2T), T >= 1.
    for cls, Tm in (('sign-generic', 1 << P['Tbits']), ('keygen-Isec', 1 << B), ('challenge', qmax)):
        out[cls] = (Tm, 4 * Tm + p // 2 + 1)
    out['keygen-D'] = (D + 1, 4 * D + p // (2 * D) + 1)
    # Guarded pre-reduced, F = 256: c_s = N4(v_s)/(2N) <= 2 F sqrt(p);
    # A0 = q c_s, C0 <= (1 + |gamma|)^2 c_s / q with |gamma| <= q.
    F = 256
    out['prered-guard256'] = ((1 << B) * qmax, 2 * F * isqrt_up(p) * (qmax + 3))
    return out


def E_of(ratio):
    k = 0
    while Fraction(4) ** (58 * (k + 1) + 2) < ratio:
        k += 1
    return k if Fraction(4) ** 60 < ratio else 0


def main():
    lines, rows = [], []
    say = lambda s: (print(s), lines.append(s))
    choice = {}
    for name in n.NAMES:
        P = params(name)
        p = P['p']
        gen = json.load(open(os.path.join(RES, f'gen_{name}.json')))
        cb = class_bounds(P)
        say(f'== {name}: p has {P["L"]} bits, Tbits = {P["Tbits"]}, coordinate limbs C = {RUST_C[name]}')
        need_bits = 0
        for cls, (Tm, M0) in cb.items():
            gram_bits = (9 * M0).bit_length() + 1
            mnum = 2 * Tm * M0
            coord_bits = (4 * isqrt_up(mnum)).bit_length() + 1
            obs = gen['widths'].get(cls, {})
            ok = True
            for k, f in FACTOR.items():
                if k in obs and obs[k] > (math.ceil(f * M0)).bit_length():
                    ok = False
                    say(f'  !! {cls} {k}: observed {obs[k]} bits above the bound')
            for k, f in COORD.items():
                if k in obs and obs[k] > (math.ceil(f * isqrt_up(mnum))).bit_length():
                    ok = False
                    say(f'  !! {cls} {k}: observed {obs[k]} bits above the coordinate bound')
            # sub-step budget: conservative (spec, reduced units, lambda >= 1) and normalized (lambda >= 2)
            tex, texp = n.counts(cls, name)
            say(f'  {cls:16s} M0 < 2^{M0.bit_length():4d}  Gram needs {gram_bits:4d} signed bits  coords need {coord_bits:4d}'
                f'  observed max Gram {max([obs.get(k, 0) for k in FACTOR] or [0]):4d} (X {obs.get("X", 0)})'
                f'  coord {max([obs.get(k, 0) for k in COORD] or [0]):4d}  {"ok" if ok else "FAIL"}'
                f'  E spec {texp - tex}  E normalized bound {E_of(Fraction(M0, 2))}')
            need_bits = max(need_bits, gram_bits)
            assert coord_bits <= 64 * RUST_C[name], (name, cls, coord_bits)
            assert ok
            assert E_of(Fraction(M0, 2)) <= texp - tex
            rows.append(dict(set=name, cls=cls, M0_bits=M0.bit_length(), gram_signed_bits=gram_bits,
                             coord_signed_bits=coord_bits, obs_gram=max([obs.get(k, 0) for k in FACTOR] or [0]),
                             obs_coord=max([obs.get(k, 0) for k in COORD] or [0]), E_spec=texp - tex,
                             E_norm=E_of(Fraction(M0, 2))))
        # chains are signing-ideal bases inside the generic O0 starting bound (max <= 2 Tmax)
        obs = gen['widths'].get('chains', {})
        M0 = 1 << (P['Tbits'] + 1)
        assert all(obs[k] <= math.ceil(f * M0).bit_length() for k, f in FACTOR.items() if k in obs)
        W = -(-need_bits // 64)
        choice[name] = W
        say(f'  -> Gram limbs W = {W} ({64 * W} bits, {64 * W - need_bits} spare); numerator Gram limbs 2C = {2 * RUST_C[name]};'
            f' coordinate limbs C = {RUST_C[name]}')
    say('')
    say('W per set: ' + ', '.join(f'{k} {v}' for k, v in choice.items()))
    open(os.path.join(RES, 'bounds.txt'), 'w').write('\n'.join(lines) + '\n')
    with open(os.path.join(RES, 'bounds.csv'), 'w') as f:
        keys = list(rows[0])
        f.write(','.join(keys) + '\n')
        for r in rows:
            f.write(','.join(str(r[k]) for k in keys) + '\n')
    json.dump(choice, open(os.path.join(RES, 'limbs.json'), 'w'))


if __name__ == '__main__':
    main()
