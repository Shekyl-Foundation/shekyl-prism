# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Analyse pass 1 (fixed schedules), the Z[i] tail run, and pass 2
(fixed-budget search). `--schedules` only writes results/schedules.json
(the per-set fixed counts pass 2 needs)."""
import argparse
import glob
import json
import math
from pathlib import Path

import numpy as np
import pandas as pd
from scipy import stats
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

from fsr import per_tour_cost

SETS = ['p248', 'p324', 'p376', 'p500', 'p664']
OUT = Path('results')
PLOTS = Path('plots')
ZI_STEP = dict(mul=23, div=2, sel=10, steps=1)
ZI_INIT = dict(mul=10, div=0, sel=10)
FOURD = ('sweep', 'updown')
NODE_MUL = 5      # float multiplies per enumeration node (center + partial norm)
SIEVE_PAIR_MUL = 44
SIEVE_PAIRS = 16 * 15


def load(pattern):
    fs = sorted(glob.glob(pattern))
    return pd.concat([pd.read_csv(f) for f in fs], ignore_index=True) if fs else None


def q999(x):
    """Smallest T with >= 99.9% of rows succeeding by T (-1 = never)."""
    if len(x) == 0:
        return None
    x = np.where(np.asarray(x) < 0, np.inf, np.asarray(x, dtype=float))
    v = float(np.quantile(x, 0.999, method='higher'))
    return None if math.isinf(v) else int(v)


def tmaxobs(x):
    x = np.asarray(x)
    if len(x) == 0:
        return None
    return None if (x < 0).any() else int(x.max())


def eta2_kw(df, group, val):
    y = pd.to_numeric(df[val], errors='coerce').to_numpy(dtype=float)
    if np.allclose(y, y[0]):
        return dict(eta2=0.0, kw_p=None, constant=True)
    sst = float(((y - y.mean()) ** 2).sum())
    ssb = sum(len(g) * (g[val].mean() - y.mean()) ** 2 for _, g in df.groupby(group))
    H, p = stats.kruskal(*[g[val].to_numpy(dtype=float) for _, g in df.groupby(group)])
    return dict(eta2=round(ssb / sst, 4), kw_p=float(p), constant=False)


def fixed_cost(variant, T):
    if variant == 'zi':
        return dict(mul=ZI_INIT['mul'] + T * ZI_STEP['mul'],
                    div=ZI_INIT['div'] + T * ZI_STEP['div'],
                    sel=ZI_INIT['sel'] + T * ZI_STEP['sel'], steps=T)
    pt, init = per_tour_cost(variant)
    return dict(mul=init['mul'] + T * pt['mul'], div=init['div'] + T * pt['div'],
                sel=T * pt['sel'], steps=T * pt['lovasz'])


PANEL_VARIANTS = {
    'stream': ('sweep', 'updown', 'zi'),
    'prod': ('sweep', 'updown', 'zi'),
    'nvar': ('updown', 'zi', 'zi_p64', 'zi_p32', 'zi_p16', 'zi_p8'),
}
VLABEL = {'sweep': '4-D fixed tours (sweep)', 'updown': '4-D fixed tours (up-down)',
          'zi': 'Z[i]-Lagrange fixed steps', 'zi_p64': 'Z[i] fixed, 64-bit quotient',
          'zi_p32': 'Z[i] fixed, 32-bit quotient', 'zi_p16': 'Z[i] fixed, 16-bit quotient',
          'zi_p8': 'Z[i] fixed, 8-bit quotient'}


def calib_values(p1, tail, panel, s, v):
    """Every observed 'first tour/step at the reduced fixed point' that the
    fixed count for (panel, set, variant) must cover. For nvar this is the
    nvar rows plus the rows with bits(N) at the bound B (nbound panel and,
    for zi, the tail run), so the count is sized from B."""
    col = f'{v}_red'
    panels = [panel] + (['nbound'] if panel == 'nvar' else [])
    g = p1[p1.panel.isin(panels) & (p1.set == s)]
    if col not in g:
        return np.array([])
    z = list(g[col].dropna())
    if v == 'zi' and tail is not None:
        tcol = {'stream': 'stream_zi_red', 'nvar': 'nbound_zi_red'}.get(panel)
        if tcol and tcol in tail:
            z += list(tail[tail.set == s][tcol].dropna())
    return np.array(z)


def schedules(p1, tail):
    sch = {}
    for panel, vs in PANEL_VARIANTS.items():
        sch[panel] = {}
        for s in SETS:
            if p1[(p1.panel == panel) & (p1.set == s)].empty:
                continue
            sch[panel][s] = {v: (tmaxobs(calib_values(p1, tail, panel, s, v))
                                 or q999(calib_values(p1, tail, panel, s, v)))
                             for v in vs}
    return sch


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--schedules', action='store_true')
    a = ap.parse_args()
    p1 = load('results/raw/*.csv')
    tail = load('results/raw_zi_tail/*.csv')
    sch = schedules(p1, tail)
    (OUT / 'schedules.json').write_text(json.dumps(sch, indent=1))
    if a.schedules:
        print(json.dumps(sch, indent=1))
        return
    PLOTS.mkdir(exist_ok=True)
    summ = dict(schedules=sch, sets={})
    rows = []
    for panel in ('stream', 'prod', 'nvar'):
        for s in SETS:
            g = p1[(p1.panel == panel) & (p1.set == s)]
            if g.empty:
                continue
            exm = g.ex_mul + g.ex_div
            ex = dict(n=len(g), steps_mean=g.ex_lovasz.mean(),
                      steps_cv=g.ex_lovasz.std() / g.ex_lovasz.mean(),
                      steps_max=int(g.ex_lovasz.max()),
                      mul_mean=exm.mean(), mul_cv=exm.std() / exm.mean(),
                      mul_p999=float(np.quantile(exm, 0.999)), mul_max=int(exm.max()),
                      is_lam1=g.ex_is_lam1.mean(), maxbits=int(g.ex_maxbits.max()),
                      eta2_key=eta2_kw(g, 'key', 'ex_lovasz'))
            rec = dict(exact=ex)
            bits = int(g.bits_nom.iloc[0])
            rows.append(dict(panel=panel, set=s, method='exact LLL (baseline)',
                             n=len(g), T=None, success_red=1.0,
                             success_b1=1.0, success_lam1=round(ex['is_lam1'], 4),
                             steps=round(ex['steps_mean'], 1),
                             steps_cv=round(ex['steps_cv'], 4),
                             mul=round(ex['mul_mean']), mul_cv=round(ex['mul_cv'], 4),
                             cost_vs_exact_mean=1.0,
                             cost_vs_exact_p999=round(ex['mul_mean'] / ex['mul_p999'], 3),
                             steps_per_bit=round(ex['steps_mean'] / bits, 3),
                             traces=None, maxbits=ex['maxbits'],
                             eta2_key=ex['eta2_key']['eta2'],
                             kw_p_key=ex['eta2_key']['kw_p']))
            for v in PANEL_VARIANTS[panel]:
                col = f'{v}_red'
                gg = g[g[col].notna()] if col in g else g.iloc[0:0]
                if gg.empty or sch[panel][s].get(v) is None:
                    continue
                zz = calib_values(p1, tail, panel, s, v)
                T = sch[panel][s][v]
                T999 = q999(zz)
                fc = fixed_cost('zi' if v.startswith('zi') else v, T)
                fm = fc['mul'] + fc['div']
                ok_red = float((gg[col].between(0, T)).mean())
                ok_b1 = float((gg[f'{v}_b1'].between(0, T)).mean())
                ok_l1 = float((gg[f'{v}_lam1'].between(0, T)).mean())
                ntr = int(gg[f'{v}_trace'].nunique())
                e_early = eta2_kw(gg, 'key', col)
                e_eff = eta2_kw(gg, 'key', f'{v}_eff')
                rec[v] = dict(n=len(zz), T_fixed=T, T999=T999,
                              first_mean=float(np.mean(zz)), first_sd=float(np.std(zz)),
                              ok_red=ok_red, ok_b1=ok_b1, ok_lam1=ok_l1,
                              traces=ntr, cost=fc, maxbits=int(gg[f'{v}_maxbits'].max()),
                              eta2_key_early_exit=e_early, eta2_key_eff_swaps=e_eff)
                rows.append(dict(panel=panel, set=s, method=VLABEL[v],
                                 n=len(gg), n_calib=len(zz), T=T, success_red=round(ok_red, 4),
                                 success_b1=round(ok_b1, 4), success_lam1=round(ok_l1, 4),
                                 steps=fc['steps'], steps_cv=0.0, mul=fm, mul_cv=0.0,
                                 cost_vs_exact_mean=round(fm / ex['mul_mean'], 3),
                                 cost_vs_exact_p999=round(fm / ex['mul_p999'], 3),
                                 steps_per_bit=round(fc['steps'] / bits, 3),
                                 traces=ntr, maxbits=rec[v]['maxbits'],
                                 eta2_key=0.0, kw_p_key=None,
                                 T999=T999))
            summ['sets'][f'{panel}/{s}'] = rec
    tab = pd.DataFrame(rows)
    tab.to_csv(OUT / 'summary_table.csv', index=False)

    # target panel: fixed-size target dependence
    tg = p1[p1.panel == 'target']
    tdep = []
    for s in SETS:
        g = tg[tg.set == s]
        if g.empty:
            continue
        r = dict(set=s, n=len(g), targets=g.key.nunique())
        r['exact_eta2'] = eta2_kw(g, 'key', 'ex_lovasz')
        for v in ('sweep', 'updown', 'zi'):
            r[f'{v}_early_exit_eta2'] = eta2_kw(g, 'key', f'{v}_red')
            r[f'{v}_traces'] = int(g[f'{v}_trace'].nunique())
        tdep.append(r)
    summ['target'] = tdep

    # across-set slopes (steps and mul-equivalents per bit)
    slopes = {}
    for panel in ('stream', 'prod', 'nvar'):
        sub = tab[tab.panel == panel]
        bits = {s: int(p1[(p1.panel == panel) & (p1.set == s)].bits_nom.iloc[0])
                for s in sub.set.unique()}
        for m, gg in sub.groupby('method'):
            x = np.array([bits[s] for s in gg['set']])
            if len(set(x)) < 2:
                continue
            for col in ('steps', 'mul'):
                r = stats.linregress(x, gg[col].to_numpy(dtype=float))
                slopes[f'{panel}/{m}/{col}'] = dict(slope=round(r.slope, 3),
                                                    r2=round(r.rvalue ** 2, 4))
    summ['slopes'] = slopes

    # fixed-width cost model: count x (W/64)^2 limb products
    fw = []
    for panel in ('stream', 'prod', 'nvar'):
        for s in SETS:
            rec = summ['sets'].get(f'{panel}/{s}')
            if not rec:
                continue
            e = rec['exact']
            W = e['maxbits']
            base = e['mul_p999'] * (math.ceil(W / 64)) ** 2
            r = dict(panel=panel, set=s, exact_W=W,
                     exact_p999_limbmul=int(base))
            for v in ('updown', 'zi'):
                if v not in rec:
                    continue
                Wv = rec[v]['maxbits']
                cv = (rec[v]['cost']['mul'] + rec[v]['cost']['div']) * math.ceil(Wv / 64) ** 2
                r[f'{v}_W'] = Wv
                r[f'{v}_limbmul'] = int(cv)
                r[f'{v}_ratio_vs_exact_p999'] = round(cv / base, 3)
            fw.append(r)
    pd.DataFrame(fw).to_csv(OUT / 'fixed_width_model.csv', index=False)
    summ['fixed_width'] = fw

    # ---- headline: does the count track bits(N) of the secret ideal?
    nleak = []
    for s in SETS:
        g = p1[(p1.panel == 'nvar') & (p1.set == s)]
        if g.empty:
            continue
        r = dict(set=s, n=len(g), keys=int(g.key.nunique()),
                 N_bits_min=int(g.N_bits.min()), N_bits_max=int(g.N_bits.max()),
                 N_bound=int(g.N_bound.iloc[0]))
        lr = stats.linregress(g.N_bits, g.ex_lovasz)
        r.update(exact_steps_per_bitN=round(lr.slope, 3), exact_r=round(lr.rvalue, 4),
                 exact_p=float(lr.pvalue))
        lr2 = stats.linregress(g.N_bits, g.ex_mul + g.ex_div)
        r['exact_mul_per_bitN'] = round(lr2.slope, 2)
        e = eta2_kw(g, 'N_bits', 'ex_lovasz')
        r.update(exact_eta2_Nbits=e['eta2'], exact_kw_p_Nbits=e['kw_p'])
        # minimum detectable |r| at alpha=0.05 two-sided, power 0.8: ~2.8/sqrt(n)
        r['min_detectable_r'] = round(2.8 / math.sqrt(len(g)), 4)
        for v in PANEL_VARIANTS['nvar']:
            if f'{v}_trace' not in g:
                continue
            r[f'{v}_traces'] = int(g[f'{v}_trace'].nunique())
            T = sch['nvar'][s].get(v)
            r[f'{v}_T'] = T
            r[f'{v}_success_lam1'] = None if T is None else round(float(g[f'{v}_lam1'].between(0, T).mean()), 4)
            nb = p1[(p1.panel == 'nbound') & (p1.set == s)]
            r[f'{v}_success_lam1_at_B'] = None if (T is None or nb.empty) else round(float(nb[f'{v}_lam1'].between(0, T).mean()), 4)
            # what an early-exit (data-dependent stop) would leak
            lr3 = stats.linregress(g.N_bits, g[f'{v}_red'].clip(lower=0))
            r[f'{v}_earlyexit_r'] = round(lr3.rvalue, 4)
        nleak.append(r)
    pd.DataFrame(nleak).to_csv(OUT / 'n_leak_table.csv', index=False)
    summ['n_leak'] = nleak

    # ---- dudect-style fixed-vs-random key Welch t-test
    dd = []
    for s in SETS:
        fx = p1[(p1.panel == 'dudect_fixed') & (p1.set == s)]
        rd = p1[(p1.panel == 'dudect_rand') & (p1.set == s)]
        if fx.empty or rd.empty:
            continue
        r = dict(set=s, n_fixed=len(fx), n_rand=len(rd),
                 fixed_key_N_bits=int(fx.N_bits.iloc[0]))
        for name, col in (('exact_steps', 'ex_lovasz'), ('exact_mul', None)):
            a1 = fx[col] if col else fx.ex_mul + fx.ex_div
            a2 = rd[col] if col else rd.ex_mul + rd.ex_div
            t = stats.ttest_ind(a1, a2, equal_var=False)
            se = math.sqrt(a1.var() / len(a1) + a2.var() / len(a2))
            r[f'{name}_mean_fixed'] = round(float(a1.mean()), 1)
            r[f'{name}_mean_rand'] = round(float(a2.mean()), 1)
            r[f'{name}_t'] = round(float(t.statistic), 2)
            # dudect flags |t| > 4.5; smallest mean gap that would cross it
            r[f'{name}_mde_at_t4.5'] = round(4.5 * se, 2)
        both = pd.concat([fx, rd])
        r['zi_traces'] = int(both['zi_trace'].nunique())
        r['zi_t'] = 0.0 if r['zi_traces'] == 1 else None
        dd.append(r)
    pd.DataFrame(dd).to_csv(OUT / 'dudect_table.csv', index=False)
    summ['dudect'] = dd

    # pass 2
    p2 = load('results/raw_budget/*.csv')
    bud_rows = []
    if p2 is not None:
        budgets = [0] + [1 << k for k in range(0, 14)]
        for (panel, s), g in p2.groupby(['panel', 'set']):
            if f'{panel}/{s}' not in summ['sets']:
                continue
            rec = summ['sets'][f'{panel}/{s}']
            ex_mean = rec['exact']['mul_mean']
            for pre in ('ud', 'zi'):
                for D in (0, 2, 4, 8, 12, 16, 24, 32):
                    col = f'{pre}_enum_d{D}'
                    if col not in g:
                        continue
                    h = g[col].to_numpy()
                    succ = {N: float(((h >= 0) & (h <= N)).mean()) for N in budgets}
                    N999 = next((N for N in budgets if succ[N] >= 0.999), None)
                    Tpre = int(g['T_fix' if pre == 'ud' else 'S_fix'].iloc[0]) - D
                    pc = fixed_cost('updown' if pre == 'ud' else 'zi', Tpre)
                    pm = pc['mul'] + pc['div']
                    total = None if N999 is None else pm + N999 * NODE_MUL
                    bud_rows.append(dict(panel=panel, set=s, pre=pre, delta=D,
                                         pre_T=Tpre, n=len(h),
                                         succ_N0=round(succ[0], 4),
                                         succ_N64=round(succ[64], 4),
                                         succ_N1024=round(succ[1024], 4),
                                         succ_N8192=round(succ[8192], 4),
                                         N999=N999, pre_mul=pm, total_mul=total,
                                         total_vs_exact_mean=None if total is None
                                         else round(total / ex_mean, 3)))
                    if pre == 'ud' and f'ud_sieve_d{D}' in g:
                        hs = g[f'ud_sieve_d{D}'].to_numpy()
                        for R in range(0, 7):
                            pass
                        sr = {R: float(((hs >= 0) & (hs <= R)).mean()) for R in range(0, 7)}
                        R999 = next((R for R in range(0, 7) if sr[R] >= 0.999), None)
                        tot = None if R999 is None else pm + R999 * SIEVE_PAIRS * SIEVE_PAIR_MUL
                        bud_rows.append(dict(panel=panel, set=s, pre='ud+sieve', delta=D,
                                             pre_T=Tpre, n=len(hs),
                                             succ_N0=round(sr[0], 4),
                                             succ_N64=round(sr[1], 4),
                                             succ_N1024=round(sr[2], 4),
                                             succ_N8192=round(sr[6], 4),
                                             N999=R999, pre_mul=pm, total_mul=tot,
                                             total_vs_exact_mean=None if tot is None
                                             else round(tot / ex_mean, 3)))
        pd.DataFrame(bud_rows).to_csv(OUT / 'budget_table.csv', index=False)
    summ['budget'] = bud_rows
    (OUT / 'summary.json').write_text(json.dumps(summ, indent=1, default=str))

    # ---------------- plots
    # 1. cost histogram: exact vs fixed lines, per set (stream & prod)
    # N-leak plot
    g = p1[p1.panel == 'nvar']
    if not g.empty:
        fig, axs = plt.subplots(1, 5, figsize=(18, 3.4))
        for ax, s in zip(axs, SETS):
            gg = g[g.set == s]
            if gg.empty:
                continue
            m = gg.groupby('N_bits').ex_lovasz.mean()
            ax.scatter(gg.N_bits, gg.ex_lovasz, s=3, alpha=0.15, color='#215f8a')
            ax.plot(m.index, m.values, color='#215f8a', label='exact LLL steps')
            for v, colr in (('updown', '#c0392b'), ('zi', '#27ae60')):
                T = sch['nvar'][s].get(v)
                if T is not None:
                    st = fixed_cost(v, T)['steps']
                    ax.axhline(st, color=colr, lw=2,
                               label=f'{VLABEL[v]} (fixed, from B)')
            ax.axvline(int(gg.N_bound.iloc[0]), color='gray', ls=':', label='bound B')
            ax.set_title(f'{s.upper()}: steps vs bits(N)')
            ax.set_xlabel('bits of secret norm N')
        axs[0].set_ylabel('Lovasz tests / Z[i] steps')
        axs[0].legend(fontsize=6)
        fig.tight_layout()
        fig.savefig(PLOTS / 'steps_vs_Nbits.png', dpi=110)
        plt.close(fig)

    for panel in ('stream', 'nvar'):
        fig, axs = plt.subplots(1, 5, figsize=(18, 3.4))
        for ax, s in zip(axs, SETS):
            g = p1[(p1.panel == panel) & (p1.set == s)]
            if g.empty:
                continue
            ax.hist(g.ex_mul + g.ex_div, bins=40, color='#215f8a', alpha=0.8,
                    label='exact LLL')
            rec = summ['sets'][f'{panel}/{s}']
            for v, colr in (('updown', '#c0392b'), ('zi', '#27ae60'), ('sweep', '#8e44ad')):
                if v not in rec:
                    continue
                c = rec[v]['cost']
                ax.axvline(c['mul'] + c['div'], color=colr, lw=2,
                           label={'updown': '4-D up-down (fixed)', 'zi': 'Z[i] (fixed)',
                                  'sweep': '4-D sweep (fixed)'}[v])
            ax.set_title(f'{s.upper()} {panel}')
            ax.set_xlabel('multiplies + divides')
        axs[0].legend(fontsize=7)
        fig.tight_layout()
        fig.savefig(PLOTS / f'cost_hist_{panel}.png', dpi=110)
        plt.close(fig)

    # 2. success vs fixed count (normalised by bits)
    fig, axs = plt.subplots(1, 2, figsize=(12, 4))
    for ax, panel in zip(axs, ('stream', 'nvar')):
        for s, colr in zip(SETS, plt.cm.viridis(np.linspace(0, 0.9, 5))):
            g = p1[(p1.panel == panel) & (p1.set == s)]
            if g.empty:
                continue
            bits = g.bits_nom.iloc[0]
            for v, ls in (('updown', '-'), ('zi', '--')):
                x = np.sort(np.where(g[f'{v}_red'] < 0, np.inf, g[f'{v}_red']))
                work = fixed_cost(v, 1)
                per = (work['mul'] + work['div'])
                ax.plot(x * per / bits, np.arange(1, len(x) + 1) / len(x), ls,
                        color=colr, label=f'{s} {v}')
            exm = np.sort((g.ex_mul + g.ex_div) / bits)
            ax.plot(exm, np.arange(1, len(exm) + 1) / len(exm), ':', color=colr,
                    label=f'{s} exact')
        ax.set_xlabel('multiplies+divides per bit of the norm')
        ax.set_ylabel('fraction done')
        ax.set_title(f'{panel}: done-by-cost CDF (dashed Z[i], solid 4-D up-down, dotted exact)')
        ax.set_ylim(0, 1.01)
    axs[1].legend(fontsize=6, ncol=3)
    fig.tight_layout()
    fig.savefig(PLOTS / 'done_cdf.png', dpi=110)
    plt.close(fig)

    # 3. cost vs bits across sets
    fig, axs = plt.subplots(1, 2, figsize=(11, 4))
    for ax, panel in zip(axs, ('stream', 'nvar')):
        sub = tab[tab.panel == panel]
        for m, gg in sub.groupby('method'):
            bits = [int(p1[(p1.panel == panel) & (p1.set == s)].bits_nom.iloc[0]) for s in gg.set]
            ax.plot(bits, gg['mul'], 'o-', label=m)
        ax.set_xlabel('bits of the ideal norm')
        ax.set_ylabel('multiplies+divides (exact: mean; fixed: fixed)')
        ax.set_title(panel)
        ax.legend(fontsize=7)
    fig.tight_layout()
    fig.savefig(PLOTS / 'cost_vs_bits.png', dpi=110)
    plt.close(fig)

    # 4. enumeration success vs budget
    if p2 is not None:
        fig, axs = plt.subplots(1, 2, figsize=(12, 4))
        for ax, pre in zip(axs, ('ud', 'zi')):
            g = p2[(p2.panel == 'stream') & (p2.set == 'p500')]
            budgets = [0] + [1 << k for k in range(0, 14)]
            for D in (0, 4, 8, 12, 16, 24, 32):
                col = f'{pre}_enum_d{D}'
                if col not in g:
                    continue
                h = g[col].to_numpy()
                ax.plot([max(N, 0.5) for N in budgets],
                        [((h >= 0) & (h <= N)).mean() for N in budgets], 'o-',
                        label=f'cut {D}')
            ax.set_xscale('log', base=2)
            ax.set_xlabel('enumeration node budget')
            ax.set_ylabel('success (b1 <= exact LLL b1)')
            ax.set_title(f'P500 stream, pre-reduction {"4-D up-down" if pre == "ud" else "Z[i]"} cut short')
            ax.legend(fontsize=7)
        fig.tight_layout()
        fig.savefig(PLOTS / 'enum_budget_p500.png', dpi=110)
        plt.close(fig)
    print(tab.to_string())
    if bud_rows:
        print(pd.DataFrame(bud_rows).to_string())


if __name__ == '__main__':
    main()
