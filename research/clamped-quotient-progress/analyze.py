# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Summaries of adversarial.py: results/adversarial_summary.txt,
results/adversarial_table.csv, results/quotient_tail.csv."""
import csv
import json
import math
import os
from collections import defaultdict

from q5common import SETS

HERE = os.path.dirname(os.path.abspath(__file__))
RULES = ['exact', 'clamp64', 'clamp128', 'corr64', 'exp64']


def main():
    out = []; table = []; tail_rows = []
    worst = defaultdict(lambda: [0, 0, ''])
    for s in SETS:
        name = s[0]
        d = json.load(open(os.path.join(HERE, 'results', f'adv_{name}.json')))
        agg = defaultdict(lambda: dict(n=0, red=0, flagged=0, first_max=0, T=0, grew=0, cls=''))
        for r in d['rows']:
            a = agg[(r['family'], r['rule'])]
            a['n'] += 1; a['red'] += r['reduced']; a['flagged'] += r['flags'] > 0
            a['grew'] += r['grew']; a['T'] = r['T']; a['cls'] = r['cls']
            if r['first_reduced'] != '':
                a['first_max'] = max(a['first_max'], r['first_reduced'])
            if r['rule'] in ('exact', 'exp64') and r['first_reduced'] != '':
                w = worst[(name, r['rule'], r['cls'])]
                if r['first_reduced'] > w[0]:
                    w[0] = r['first_reduced']; w[1] = r['T']; w[2] = r['family']
        out.append(f'== {name}')
        fams = sorted({k[0] for k in agg})
        for f in fams:
            cells = []
            for rule in RULES:
                a = agg[(f, rule)]
                cells.append(f"{rule} {a['red']}/{a['n']} red, {a['flagged']} flag, need<= {a['first_max']} of {a['T']}")
                table.append(dict(set=name, family=f, cls=a['cls'], rule=rule, n=a['n'], reduced=a['red'],
                                  flagged=a['flagged'], max_steps_needed=a['first_max'], schedule=a['T'], grew=a['grew']))
            out.append(f'  {f:24s} ' + '; '.join(cells))
        # failures
        for rule in RULES:
            bad = [f for f in fams if agg[(f, rule)]['red'] < agg[(f, rule)]['n']]
            out.append(f'  {rule}: families with an unreduced output: {bad if bad else "none"}')
        assert all(agg[(f, 'exact')]['red'] == agg[(f, 'exact')]['n'] for f in fams)
        assert all(agg[(f, 'exp64')]['red'] == agg[(f, 'exp64')]['n'] for f in fams)
        assert all(agg[(f, r)]['grew'] == 0 for f in fams for r in ('exact', 'exp64', 'corr64'))
        # honest quotient tail: log2|q|^2 per logical step
        tl = d['tail']; n = len(tl)
        for b in range(0, 11):
            k = sum(1 for x in tl if x >= 2 * b)
            tail_rows.append(dict(set=name, log2_X=b, steps=n, frac_q_ge_X=k / n, X2_times_frac=(4 ** b) * k / n))
    out.append('== largest steps needed vs schedule, per rule and class (all families)')
    for k, v in sorted(worst.items()):
        out.append(f'  {k[0]} {k[1]:6s} {k[2]:17s} needed {v[0]:4d} of {v[1]:4d} ({v[2]})')
    # tail fit: c = X^2 P(|q| >= X) averaged over X = 2^3..2^6
    out.append('== honest quotient tail (exact run, all logical steps of 400 signing ideals a set)')
    for s in SETS:
        rows = [r for r in tail_rows if r['set'] == s[0]]
        cs = [r['X2_times_frac'] for r in rows if 3 <= r['log2_X'] <= 6 and r['frac_q_ge_X'] > 0]
        c = sum(cs) / len(cs)
        per_step = c * 2.0 ** -120
        steps = rows[0]['steps'] / 400
        out.append(f"  {s[0]}: P(|q| >= X) ~ {c:.3f}/X^2 (X = 8..64; " +
                   ', '.join(f"2^{r['log2_X']}: {r['frac_q_ge_X']:.2e}" for r in rows if r['log2_X'] in (1, 2, 4, 6, 8, 10)) +
                   f"); {steps:.0f} logical steps a run -> P(some |q| >= 2^60) ~ 2^{math.log2(per_step * steps):.1f} a reduction")
    open(os.path.join(HERE, 'results', 'adversarial_summary.txt'), 'w').write('\n'.join(out) + '\n')
    with open(os.path.join(HERE, 'results', 'adversarial_table.csv'), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(table[0].keys())); w.writeheader(); w.writerows(table)
    with open(os.path.join(HERE, 'results', 'quotient_tail.csv'), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(tail_rows[0].keys())); w.writeheader(); w.writerows(tail_rows)
    print('\n'.join(out))


if __name__ == '__main__':
    main()
