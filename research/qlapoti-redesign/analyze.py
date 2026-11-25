# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Turn results/*.csv (summarize.py) into the experiment's tables
(results/TABLES.md and results/*.json). Every figure is a model estimate
(Python model of quat_qlapoti, fsne-compatible work units).

Worst-case sizing: a fixed schedule must not depend on S, so it is sized
from the worst prepared ideal. Rates are pooled over the top u-decile
(u = n / sqrt(p)) and then scaled to the Hermite bound u = 1/sqrt(2) with a
power law fitted to the binned rates of the upper half of u."""
import json
import math
import sys
from pathlib import Path

import numpy as np
import pandas as pd

from costmodel import SetCost, REF_MEAN_WORK, PRIOR, passes_for, sieve_words
from fne import SET_MAP

HERE = Path(__file__).resolve().parent
RES = HERE / 'results'
SETS = ['p248', 'p324', 'p376', 'p500', 'p664']
EPS = {'99.9%': 1e-3, '2^-64': 2.0 ** -64}
UMAX = 1 / math.sqrt(2)
LN = math.log


def wilson(k, n, z=1.96):
    if n == 0:
        return (0.0, 0.0, 1.0)
    ph = k / n
    d = 1 + z * z / n
    c = ph + z * z / (2 * n)
    r = z * math.sqrt(ph * (1 - ph) / n + z * z / (4 * n * n))
    return ph, (c - r) / d, (c + r) / d


def rcsv(stem):
    """results/<stem>.csv, or the committed results/<stem>.csv.gz."""
    f = RES / f'{stem}.csv'
    return pd.read_csv(f if f.exists() else RES / f'{stem}.csv.gz')


def have(stem):
    return (RES / f'{stem}.csv').exists() or (RES / f'{stem}.csv.gz').exists()


def load(name):
    return rcsv(f'gen_{name}'), rcsv(f'gd_{name}')


def fit_u(g, de, succ='n6', slots='L', bins=10):
    """Binned pooled rate vs u; power-law slope over the upper half."""
    x = g[g.de == de]
    q = np.quantile(x.u, np.linspace(0, 1, bins + 1))
    pts = []
    for i in range(bins):
        m = (x.u >= q[i]) & (x.u <= q[i + 1])
        s, n = x[m][succ].sum(), x[m][slots].sum()
        pts.append((x[m].u.mean(), s / n if n else 0, s, n))
    up = [pt for pt in pts[bins // 2:] if pt[1] > 0]
    b = np.polyfit([LN(pt[0]) for pt in up], [LN(pt[1]) for pt in up], 1)[0]
    return pts, b, q[-2]


def worst_scale(g, de):
    """Factor taking a top-decile pooled rate to u = 1/sqrt(2)."""
    pts, b, q90 = fit_u(g, de)
    top = g[(g.de == de) & (g.u >= q90)]
    ut = top.u.mean()
    return min(1.0, (UMAX / ut) ** b), b, ut, top


def block_fail(top, succ, slots, M, scale):
    rg = (top[succ] / top[slots]).to_numpy() * scale
    rg = np.clip(rg, 0, 1)
    return float(np.mean((1 - rg) ** M))


def gen_validity(d):
    v = []
    mx = 0
    for s in d.gd:
        xs = [int(t) for t in str(s).split()]
        mx = max(mx, max(xs))
        v.append(len(xs) / sum(xs))
    return min(v), float(np.mean(v)), mx


def opt_one_stage(sc, top, succ, slots, lam_sel, scale, vmin, eps, Ms, Ks=None, exp=None):
    """Generator pool (D draws -> B generators) + B blocks of M slots, each
    slot running the full expensive stage."""
    best = None
    exp = exp or sc.exp_slot()
    for M in Ms:
        fb = block_fail(top, succ, slots, M, scale)
        if fb >= 1:
            continue
        B = math.ceil(LN(eps / 2) / LN(fb)) if fb > 0 else 1
        D, Wg = gen_pool(sc, B, vmin, eps / 2)
        W = Wg + B * M * (sc.cheap_slot(lam_sel) + exp)
        if best is None or W < best['W']:
            best = dict(M=M, D=D, B=B, P=B * M, W=W, fb=fb)
    return best


def binom_upper(P, a, eps):
    """Smallest Q with Pr[Binomial(P, a) > Q] <= eps (exact, log domain)."""
    from scipy.stats import binom
    le = LN(eps)
    lo, hi = int(P * a), P
    while lo < hi:
        mid = (lo + hi) // 2
        if binom.logsf(mid, P, a) <= le:
            hi = mid
        else:
            lo = mid + 1
    return lo


def binom_lower_D(B, v, eps):
    """Smallest D with Pr[Binomial(D, v) < B] <= eps (exact, log domain)."""
    from scipy.stats import binom
    le = LN(eps)
    lo, hi = B, max(B + 10, int(2 * B / v) + 50)
    while binom.logcdf(B - 1, hi, v) > le:
        hi *= 2
    while lo < hi:
        mid = (lo + hi) // 2
        if binom.logcdf(B - 1, mid, v) <= le:
            hi = mid
        else:
            lo = mid + 1
    return lo


def gen_pool(sc, B, v, eps):
    """D generator draws, oblivious compaction of the valid ones into B
    block slots (instead of a fixed K-draw loop per block)."""
    D = binom_lower_D(B, v, eps)
    return D, D * sc.gen_draw() + sc.compaction(D) + B * sc.gen_block(0)


def opt_two_stage(sc, top, succ, slots, lam_sel, scale, vmin, eps, Ms, Ks, a_hi, Ksv, exp=None):
    """Generator pool, cheap stage over P = B*M slots with a sieve below
    2^Ksv, oblivious compaction of survivors, Q expensive slots covering
    every survivor w.p. 1-eps/3."""
    best = None
    exp = exp or sc.exp_slot()
    for M in Ms:
        fb = block_fail(top, succ, slots, M, scale)
        if fb >= 1:
            continue
        B = math.ceil(LN(eps / 3) / LN(fb)) if fb > 0 else 1
        D, Wg = gen_pool(sc, B, vmin, eps / 3)
        P = B * M
        Q = binom_upper(P, a_hi, eps / 3)
        W = (Wg + P * (sc.cheap_slot(lam_sel) + sc.sieve_slot(Ksv))
             + sc.compaction(P) + Q * exp)
        if best is None or W < best['W']:
            best = dict(M=M, D=D, B=B, P=P, Q=Q, W=W, fb=fb, Ksv=Ksv)
    return best


def cap_tables(name, g, d):
    """Sub-experiment 1: completion probability of each inner loop vs cap."""
    out = {}
    draws = np.array([int(t) for s in d.gd for t in str(s).split()])
    out['gen'] = {K: float((draws <= K).mean()) for K in (1, 2, 4, 8, 13, 16, 24, 32, 48, 64)}
    out['gen_max'] = int(draws.max())
    out['gen_n'] = int(len(draws))
    vmin, vmean, _ = gen_validity(d)
    out['gen_v'] = (vmin, vmean)
    x = g[g.de == 2]
    used = x.L - x.n0
    out['skip'] = {}
    for C in (0, 1, 2, 3, 4, 6, 8):
        # fraction of slots that find a coprime lambda within C+1 tries:
        # coprime lambdas / slots
        frac = (used / x[f'sk{C}_slots'])
        out['skip'][C] = (float(used.sum() / x[f'sk{C}_slots'].sum()), float(frac.min()))
    gs = x.drop_duplicates(['sid', 'g']).gauss
    out['gauss'] = dict(max=int(gs.max()), q999=float(np.quantile(gs, 0.999)), mean=float(gs.mean()))
    out['corn'] = {}
    for de in (2, 1, 0):
        cm = g[g.de == de].corn_max
        out['corn'][de] = dict(max=int(cm.max()), mean_of_max=float(cm.mean()))
    return out


def exp_prime(sc):
    """Expensive slot without the good-prime stripping / extended
    Cornacchia (Qlapoty: z must itself be a prime = 1 mod 4)."""
    mb = sc.mb
    return (16.5 * mb + sc.J * max(1.0, mb / 8) + 1.5 * mb + 2 * sc.T_corn + 10
            + 40 + 3 * sc.ctgcd)


QVARS = {'p': 'prime only, PRISM_v2 postcondition',
         'pe': 'extended Cornacchia, PRISM_v2 postcondition',
         'pn': 'prime only, no postcondition',
         'pen': 'extended Cornacchia, no postcondition'}


def qlapoty_tables(name):
    q = rcsv(f'qy_{name}')
    S = rcsv(f'qyS_{name}')
    out = dict(odd_share=float((S.why != 'even').mean()))
    l1 = [int(t) for s_ in S.loop1.dropna() for t in str(s_).split()]
    l1 = np.array(l1)
    out['loop1'] = dict(mean=float(l1.mean()), q999=float(np.quantile(l1, 0.999)),
                        max=int(l1.max()), capped=float((l1 >= 2000).mean()),
                        cdf={K: float((l1 <= K).mean()) for K in (8, 16, 32, 64, 128, 256)})
    # per-S Loop1 validity (one 'draw' = one Loop1 iteration)
    # per-S Loop1 validity; ideals where the prototype's Loop1-lite never
    # finds an alpha0 (cap 2000 hit by every draw) are a prototype defect
    # (the paper's Loop1 fails only on 2^-112..2^-233 'bad' inputs): they are
    # counted separately and left out of the validity bound
    vS, stuck = [], 0
    for s_ in S.loop1.dropna():
        xs = [int(t) for t in str(s_).split()]
        if all(t >= 2000 for t in xs):
            stuck += 1
            continue
        vS.append(len(xs) / sum(xs))
    vmin = float(min(vS))
    out['loop1_vmin'] = vmin
    out['loop1_stuck_S'] = stuck
    out['loop1_S'] = len(vS) + stuck
    out['de'] = {}
    for de in (2, 1, 0):
        x = q[q.de == de]
        L = x.L.sum()
        R = {('frac' + k): float(x['n' + k].sum() / L) for k in '023456789'}
        R['post_reject'] = float((x.n7 + x.n9).sum() / max(1, (x.n6 + x.n7 + x.n8 + x.n9).sum()))
        scale, b, ut, top = worst_scale(x.assign(n6=x.n6), de)
        R.update(scale=scale, slope=float(b), u_top=float(ut))
        sc = SetCost(name, de)
        des = {}
        Ms = [8, 16, 32, 64, 128, 256, 512]
        for lab, eps in EPS.items():
            dd = {}
            for v in QVARS:
                ex = exp_prime(sc) if v in ('p', 'pn') else sc.exp_slot()
                for C in (0, 1, 2, 4):
                    dd[f'{v}_skip{C}'] = opt_one_stage(sc, top, f'{v}_sk{C}_succ', f'{v}_sk{C}_slots',
                                                       C + 1, scale, vmin, eps, Ms, exp=ex)
                for Ksv in (10, 12, 14):
                    C = 1
                    a_s = x[f'sv{Ksv}'] / x[f'{v}_sk{C}_slots']
                    a_hi = float(np.quantile(a_s, 0.99) * 1.2)
                    r = opt_two_stage(sc, top, f'{v}_sk{C}_succ', f'{v}_sk{C}_slots', C + 1, scale,
                                      vmin, eps, Ms, None, a_hi, Ksv, exp=ex)
                    r['a_hi'] = a_hi
                    dd[f'{v}_2st_K{Ksv}'] = r
            for k, v in dd.items():
                if v:
                    v['factor'] = v['W'] / REF_MEAN_WORK[name]
            des[lab] = dd
        R['designs'] = des
        out['de'][de] = R
    return out


def main():
    tables = {}
    md = ['# Tables (model estimates)', '',
          'Generated by analyze.py from results/*.csv. All figures are model',
          'estimates (Python model, fsne-compatible work units), not cycles.', '']
    for name in SETS:
        if not have(f'gen_{name}'):
            continue
        g, d = load(name)
        T = dict(set=name, n_ideals=int(d.shape[0]),
                 even_share=float((d.v2 > 0).mean()),
                 u_min=float(d.u.min()), u_max=float(d.u.max()), u_med=float(d.u.median()))
        T['caps'] = cap_tables(name, g, d)
        vmin = T['caps']['gen_v'][0]
        T['de'] = {}
        for de in (2, 1, 0):
            x = g[g.de == de]
            L = x.L.sum()
            cop = (x.L - x.n0).sum()
            R = dict(rate_int=float(x.n6.sum() / L), rate_cop=float(x.n6.sum() / cop),
                     post_reject=float(x.n5.sum() / max(1, x.n5.sum() + x.n6.sum())),
                     dead_gen=float(((x.n6 == 0) & (x.n5 > 0)).mean()),
                     zero_gen=float((x.n6 == 0).mean()))
            for k in '0123456':
                R['frac' + k] = float(x['n' + k].sum() / L)
            scale, b, ut, top = worst_scale(g, de)
            R.update(scale=scale, slope=float(b), u_top=float(ut))
            pts, _, _ = fit_u(g, de)
            R['bins'] = [(float(a), float(r)) for a, r, _, _ in pts]
            tk = top.n6.sum()
            tn = top.L.sum()
            R['top_rate'] = wilson(int(tk), int(tn))
            R['worst_rate_int'] = R['top_rate'][0] * scale
            sc = SetCost(name, de)
            designs = {}
            Ms = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512]
            Ks = [4, 6, 8, 10, 13, 16, 20, 24, 32]
            # survivor-rate upper bound (all S, per slot, 99th pct x 1.2)
            for lab, eps in EPS.items():
                dd = {}
                for C in (0, 1, 2, 3, 4, 6, 8):
                    dd[f'skip{C}'] = opt_one_stage(sc, top, f'sk{C}_succ', f'sk{C}_slots',
                                                   C + 1, scale, vmin, eps, Ms, Ks)
                dd['prime'] = opt_one_stage(sc, top, 'pr_succ', 'pr_slots', 0.25, scale, vmin,
                                            eps, [m for m in Ms if m <= 256], Ks)
                for Ksv in (8, 10, 12, 14):
                    for pol, sl, ls in (('skip1', 'sk1', 2), ('prime', 'pr', 0.25)):
                        slots = f'{sl}_slots'
                        # survivors per slot under this policy, upper bound over S
                        a_s = (x[f'sv{Ksv}'] / x.L) * (x.L / (x.L - x.n0).clip(lower=1))
                        a_s = a_s * ((x[f'{sl}_slots'] * 0 + 1) if pol == 'prime'
                                     else ((x.L - x.n0) / x[slots]))
                        a_hi = float(np.quantile(a_s, 0.99) * 1.2)
                        dd[f'2st_{pol}_K{Ksv}'] = opt_two_stage(
                            sc, top, f'{sl}_succ', slots, ls, scale, vmin, eps,
                            [m for m in Ms if m >= 8 and (pol != 'prime' or m <= 256)], Ks,
                            a_hi, Ksv)
                        dd[f'2st_{pol}_K{Ksv}']['a_hi'] = a_hi
                for k, v in dd.items():
                    if v:
                        v['factor'] = v['W'] / REF_MEAN_WORK[name]
                designs[lab] = dd
            R['designs'] = designs
            T['de'][de] = R
        if have(f'qy_{name}'):
            T['qlapoty'] = qlapoty_tables(name)
        tables[name] = T
        print(name, 'done', flush=True)
    (RES / 'tables.json').write_text(json.dumps(tables, indent=1, default=float))


if __name__ == '__main__':
    main()
