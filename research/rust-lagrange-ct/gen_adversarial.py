# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Adversarial vectors from research/lagrange-tour-bound/run_bound_study.py
(unchanged generators): Rolletschek chains (2+sqrt3 decay; conj_alt =
alternating 1+-i quotients, i_rot) at the full public size 2^(B+2e-5), the
corner cases, and the finite-precision half-integer ping-pong input.
Writes vectors/<set>_adv.txt in the gen_vectors.py format, with nu_p64 = 0
(not compared) and lam1 from exhaustive enumeration.

Usage: LTB_DIR=../lagrange-tour-bound FSR_DIR=../fixed-schedule-reduction $PY gen_adversarial.py
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.environ.get('FSR_DIR', os.path.join(HERE, '..', 'fixed-schedule-reduction')))
sys.path.insert(0, os.environ.get('LTB_DIR', os.path.join(HERE, '..', 'lagrange-tour-bound')))

from run_bound_study import make_rolletschek, set_params, zi_gauss_trace  # noqa: E402
from fsr import shortest_norm  # noqa: E402
from hdl import SETS, gram_norm  # noqa: E402


def basis(T, x, y):
    return [[T, 0, 0, 0], [0, T, 0, 0], [x, y, 1, 0], [-y, x, 0, 1]]


def main():
    os.makedirs(os.path.join(HERE, 'vectors'), exist_ok=True)
    for name, e, f in SETS:
        p, L, B, q_bits, T_bits = set_params(name, e, f)
        rows = []
        for mode in ('conj_alt', 'i_rot'):
            T, g, h, k = make_rolletschek(1 << T_bits, mode=mode)
            rows.append((f'rolletschek_{mode}', T, g, h))
        T_full = (1 << (T_bits - 1)) + 13579
        rows += [('zero_xy', T_full, 0, 0), ('half_tie', T_full, T_full // 2, T_full // 2),
                 ('skew_large_q', T_full, 1, 0), ('equal_norms', T_full, T_full - 1, 0)]
        U_pp = 1 << (L // 2 - 2)
        rows.append(('pingpong', U_pp, (U_pp >> 1) + 1, 0))
        out = [f'# {name} adversarial (lagrange-tour-bound generators)']
        for kind, T, x, y in rows:
            steps, _, nu, _, _, _ = zi_gauss_trace([T, 0, 0, 0], [x, y, 1, 0], p)
            lam1 = shortest_norm(gram_norm(basis(T, x, y), p))[0]
            out.append(' '.join([kind, format(steps, 'x'), format(T, 'x'), format(x, 'x'),
                                 format(y, 'x'), format(nu, 'x'), '0', format(lam1, 'x')]))
            print(name, kind, 'bits', T.bit_length(), 'exact steps', steps, 'nu==lam1', nu == lam1)
        open(os.path.join(HERE, 'vectors', f'{name}_adv.txt'), 'w').write('\n'.join(out) + '\n')


if __name__ == '__main__':
    main()
