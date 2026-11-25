# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""A valid ideal on which the 64-bit quotient clamp breaks the reduction.

Model ideal {T, alpha} of prime norm T at the signing size, containing
beta = a + b i + j with nrd(beta) = k T, k ~ 2^(L/2 + 20) (so beta is far
shorter than T but not reduced against the rest of the lattice). The first
swap leaves a quotient near sqrt(T/k) > 2^60: every 64-bit step clamps and
makes additive progress only; the exact cleanup takes the large quotient
in one go, but the further steps it then needs are not in the schedule.
Widths stay inside the proven bound (Lemma 3 covers the clamp); the output
is not reduced. Writes results/crafted_saturation.txt.
"""
import os
import random

import gmpy2

from q5common import SETS, params
from instrumented import Rec, reduce_instrumented
from bounds import intermediate_bounds
import gen_inputs as G

HERE = os.path.dirname(os.path.abspath(__file__))


def main():
    out = []
    for s in SETS:
        P = params(s[0]); p = P['p']; rng = random.Random(77)
        T = int(gmpy2.next_prime(rng.randrange(1 << (P['Tbits'] - 1), 1 << P['Tbits'])))
        while True:
            k = rng.randrange(1 << (P['L'] // 2 + 19), 1 << (P['L'] // 2 + 20))
            r = k * T - p
            if r % 4 == 1 and gmpy2.is_prime(r):
                break
        a, b = G.two_squares_prime(r)
        x, y = a % T, b % T
        assert (x * x + y * y + p) % T == 0
        u0, v0 = G.model_basis(T, x, y)
        _, _, nu_ex, nv_ex, st = G.exact_lagrange(u0, v0, p)
        rec = Rec()
        u, v, nu, nv, info = reduce_instrumented(u0, v0, p, P['tprov_sec'], rec)
        Ms, bnd = intermediate_bounds(P, info['M0'], P['tprov_sec'] + 1)
        viol = [kk for kk, vv in rec.mx.items() if kk in bnd and vv > bnd[kk]]
        rec_e = Rec()
        ue, ve, nue, nve, info_e = reduce_instrumented(u0, v0, p, P['tprov_sec'], rec_e, prec64=False)
        line = (f"{P['name']}: T {T.bit_length()} bits prime, k {k.bit_length()} bits; exact early-exit steps {st}; "
                f"64-bit schedule: saturated quotients {info['nsat']}, final nrd(u) = lambda_1: {nu == nu_ex}, "
                f"bits(nrd u) {nu.bit_length()} vs lambda_1 {nu_ex.bit_length()}; width-lemma violations {len(viol)}; "
                f"max intermediate {max(vv.bit_length() for vv in rec.mx.values())} bits (M0 {info['M0'].bit_length()}); "
                f"same schedule with exact quotients: lambda_1 {nue == nu_ex}")
        print(line); out.append(line)
    open(os.path.join(HERE, 'results', 'crafted_saturation.txt'), 'w').write('\n'.join(out) + '\n')


if __name__ == '__main__':
    main()
