# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Tables for REPORT.md from results/bench.csv and results/raw/dudect_*.csv.

dudect method (Reparaz, Balasch, Verbauwhede 2017): Welch t between the
fixed class (one input) and the random class (fresh inputs, interleaved in
a random order); also on measurements cropped below 100 percentiles of the
pooled distribution (p_k = 1 - 0.5^(10 (k+1) / 100)), and a second-order
test on centred squares. Threshold |t| = 4.5. Also the Pearson correlation
of cycles with bits(N) in the random class (the channel named in review 2).
"""
import csv
import glob
import json
import math
import os

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
PY_STEP_RATIO = {'p248': 0.299, 'p324': 0.291, 'p376': 0.298, 'p500': 0.286, 'p664': 0.279}
SETS = ['p248', 'p324', 'p376', 'p500', 'p664']


def welch(a, b):
    if len(a) < 2 or len(b) < 2:
        return float('nan')
    va, vb = a.var(ddof=1), b.var(ddof=1)
    den = math.sqrt(va / len(a) + vb / len(b))
    return float((a.mean() - b.mean()) / den) if den > 0 else 0.0


def dudect(path):
    d = np.loadtxt(path, delimiter=',', dtype=np.int64)
    cls, cyc, nb = d[:, 0], d[:, 1].astype(float), d[:, 2]
    a, b = cyc[cls == 0], cyc[cls == 1]
    out = dict(n=len(cyc), n_fixed=len(a), n_rand=len(b), t_raw=welch(a, b))
    ts = []
    for k in range(100):
        thr = np.percentile(cyc, 100 * (1 - 0.5 ** (10 * (k + 1) / 100)))
        m = cyc < thr
        ts.append(welch(cyc[m & (cls == 0)], cyc[m & (cls == 1)]))
    ts = [t for t in ts if not math.isnan(t)]
    out['t_crop_max'] = max(ts, key=abs) if ts else float('nan')
    mu = cyc.mean()
    sq = (cyc - mu) ** 2
    out['t_2nd'] = welch(sq[cls == 0], sq[cls == 1])
    out['t_max_abs'] = max(abs(out['t_raw']), abs(out['t_crop_max']), abs(out['t_2nd']))
    rb, rnb = cyc[cls == 1], nb[cls == 1]
    out['r_bitsN'] = float(np.corrcoef(rb, rnb)[0, 1]) if rnb.std() > 0 else float('nan')
    out['med_fixed'] = float(np.median(a))
    out['med_rand'] = float(np.median(b))
    out['cv_fixed'] = float(a.std() / a.mean())
    return out


def main():
    res = {'bench': {}, 'dudect': {}}
    bench = {}
    with open(os.path.join(HERE, 'results', 'bench.csv')) as fh:
        for r in csv.DictReader(fh):
            bench.setdefault(r['set'], {})[r['impl']] = r
    res['bench'] = bench
    print('## Cycles (median, rdtsc at 2.4 GHz TSC)\n')
    print('| Set | CT Z[i] (T steps) | exact LLL, GMP | exact LLL, num-bigint | exact Z[i], GMP (mean steps) | CT / LLL-GMP | CT / LLL-num | model step ratio | CT / exact Z[i]-GMP |')
    print('|---|---|---|---|---|---|---|---|---|')
    for s in SETS:
        if s not in bench:
            continue
        b = bench[s]
        ct, lg, ln, gg = (float(b[k]['median_cycles']) for k in ('ct_zi', 'lll_gmp', 'lll_num', 'gauss_gmp'))
        print(f"| {s.upper()} | {ct/1e6:.2f} M ({int(float(b['ct_zi']['mean_steps']))}) | {lg/1e6:.2f} M | {ln/1e6:.2f} M | "
              f"{gg/1e6:.2f} M ({float(b['gauss_gmp']['mean_steps']):.0f}) | {ct/lg:.3f} | {ct/ln:.3f} | {PY_STEP_RATIO[s]:.3f} | {ct/gg:.2f} |")
    print('\n## dudect (fixed vs random input), threshold |t| = 4.5\n')
    print('| Set | impl | measurements | t raw | max t cropped | t 2nd order | max abs t | r(cycles, bits N) random class | verdict |')
    print('|---|---|---|---|---|---|---|---|---|')
    for s in SETS:
        for imp in ('ct', 'ct_rep2', 'gauss', 'lll_gmp'):
            p = os.path.join(HERE, 'results', 'raw', f'dudect_{s}_{imp}.csv')
            if not os.path.exists(p):
                continue
            try:
                o = dudect(p)
            except ValueError:
                continue  # still being written
            res['dudect'][f'{s}_{imp}'] = o
            verdict = 'leak' if o['t_max_abs'] > 4.5 else 'no leak detected'
            print(f"| {s.upper()} | {imp} | {o['n']:,} | {o['t_raw']:.2f} | {o['t_crop_max']:.2f} | {o['t_2nd']:.2f} | "
                  f"{o['t_max_abs']:.2f} | {o['r_bitsN']:.3f} | {verdict} |")
    with open(os.path.join(HERE, 'results', 'summary.json'), 'w') as fh:
        json.dump(res, fh, indent=1, sort_keys=True)


if __name__ == '__main__':
    main()
