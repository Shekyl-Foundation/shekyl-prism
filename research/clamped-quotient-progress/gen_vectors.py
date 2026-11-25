# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Vectors for rust-xcheck: per set, model bases {T, x + y i + j} with the
Python model's outputs. Line: kind T x y steps_exp nu_exp u0 u1 u2 u3
nbig nu_clamp64_cleanup nu_exact (hex; signed values with a leading '-').
steps_exp = T_exp64 of the class (any-basis E for bases that are not ideals);
the clamp64 reference is the prototype schedule (T_exact + exact cleanup)."""
import csv
import os
import random

import gen_inputs as G
from q5common import SETS, params
from trace_counterexample import crafted, crafted_k1
from variants import run
from adversarial import chains

HERE = os.path.dirname(os.path.abspath(__file__))


def hx(v):
    return ('-' if v < 0 else '') + format(abs(v), 'x')


def main():
    counts = {}
    with open(os.path.join(HERE, 'results', 'step_counts.csv')) as f:
        for r in csv.DictReader(f):
            counts[(r['set'], r['cls'])] = r
    os.makedirs(os.path.join(HERE, 'rust-xcheck', 'vectors'), exist_ok=True)
    for s in SETS:
        P = params(s[0]); p, e, B = P['p'], P['e'], P['B']
        c = counts[(P['name'], 'sign-generic')]
        Tex, Texp, Tany = int(c['T_exact']), int(c['T_exp64']), int(c['T_exp64_any_basis'])
        rng = random.Random(4242 + e)
        rows = []
        q, _ = G.sample_q(e, p, rng)
        xq, yq = G.root_pair(q, p, rng)
        for i in range(60):
            N = G.sample_N(p, B, rng, at_bound=(i % 2 == 0), avoid=(q,))
            xN, yN = G.root_pair(N, p, rng)
            x, y = G.crt_pair(N, xN, yN, q, xq, yq)
            rows.append(('honest', N * q, x, y, Texp))
        for sd in range(3):
            T, k, x, y = crafted(P, seed=77 + sd); rows.append(('crafted_q5', T, x, y, Texp))
            T, k, x, y = crafted_k1(P, seed=78 + 10 * sd); rows.append(('crafted_k1', T, x, y, Texp))
        for mode in ('conj_alt', 'i_rot'):
            T, g, h, k = G.make_rolletschek(1 << P['Tbits'], mode=mode); rows.append(('rolletschek', T, g, h, Tany))
        for fam, T, g, h in chains(P, rng):
            rows.append((fam, T, g, h, Tany))
        lines = []
        for kind, T, x, y, st in rows:
            u0, v0 = G.model_basis(T, x, y)
            r = run(u0, v0, p, st, 'exp64')
            rc = run(u0, v0, p, Tex, 'clamp64', cleanup=True)
            re_ = run(u0, v0, p, Tex, 'exact')
            lines.append(' '.join([kind, hx(T), hx(x), hx(y), str(st), hx(r['nu'])] + [hx(t) for t in r['u']] +
                                  [str(r['flags']), hx(rc['nu']), hx(re_['nu'])]))
        open(os.path.join(HERE, 'rust-xcheck', 'vectors', f"{P['name']}.txt"), 'w').write('\n'.join(lines) + '\n')
        print(P['name'], len(lines))


if __name__ == '__main__':
    main()
