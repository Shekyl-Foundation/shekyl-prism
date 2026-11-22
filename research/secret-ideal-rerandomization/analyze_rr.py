# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Tables and plots for the re-randomization experiment."""
import json
import math
from pathlib import Path

import numpy as np
import pandas as pd
from scipy import stats
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

RAW = Path('results/raw')
OUT = Path('results')
PLOTS = Path('plots')
SETS = ['p248', 'p324', 'p376', 'p500', 'p664']
SAMP_V = ['prime_plain', 'prime_norm', 'odd_plain', 'odd_norm', 'prime_gauss', 'odd_gauss']
DOWN_V = ['prime_norm', 'odd_norm', 'odd_gauss']


def load(prefix):
    fs = sorted(RAW.glob(f'{prefix}_*.csv'))
    if not fs:
        return None
    return pd.concat([pd.read_csv(f) for f in fs], ignore_index=True)


def anova_eta(df, col, by='key'):
    g = [x[col].values for _, x in df.groupby(by)]
    g = [x for x in g if len(x) > 1]
    if len(g) < 2:
        return float('nan'), float('nan'), float('nan')
    F, p = stats.f_oneway(*g)
    allv = np.concatenate(g)
    ssb = sum(len(x) * (x.mean() - allv.mean()) ** 2 for x in g)
    sst = ((allv - allv.mean()) ** 2).sum()
    eta = ssb / sst if sst > 0 else 0.0
    null = (len(g) - 1) / (len(allv) - 1)
    return eta, null, p


def kw_p(df, col, by='key'):
    g = [x[col].values for _, x in df.groupby(by)]
    try:
        return stats.kruskal(*g).pvalue
    except ValueError:
        return float('nan')


def spearman(a, b):
    if np.std(a) == 0 or np.std(b) == 0:
        return 0.0, 1.0
    r = stats.spearmanr(a, b)
    return r.correlation, r.pvalue


def slope(x, y, ci=False):
    if np.std(x) == 0 or np.std(y) == 0:
        return (0.0, 0.0, 0.0) if ci else (0.0, 0.0)
    s = stats.linregress(x, y)
    if ci:
        return s.slope, s.rvalue, 1.96 * s.stderr
    return s.slope, s.rvalue


def welch(a, b):
    a, b = np.asarray(a, float), np.asarray(b, float)
    if a.std() == 0 and b.std() == 0:
        return 0.0 if a.mean() == b.mean() else float('inf')
    return stats.ttest_ind(a, b, equal_var=False).statistic


def k_for(rho, bits=64):
    if rho >= 1:
        return 1
    return math.ceil(bits * math.log(2) / -math.log1p(-rho))


def samp_tables():
    df = load('samp')
    if df is None:
        return None
    dg = load('sampg')
    if dg is not None:
        keep = ['set', 'key', 'sig'] + [c for c in dg.columns if 'gauss' in c]
        df = df.merge(dg[keep], on=['set', 'key', 'sig'], how='left')
    rows = []
    for s in SETS:
        d = df[df.set == s]
        if d.empty:
            continue
        for v in SAMP_V:
            if f'{v}_tries' not in d or d[f'{v}_tries'].isna().any():
                continue
            tries = d[f'{v}_tries']
            rho = len(tries) / tries.sum()
            nb = d[f'{v}_Np_bits']
            eta, null, p_an = anova_eta(d, f'{v}_tries')
            eta_nb, null_nb, p_nb = anova_eta(d, f'{v}_Np_bits')
            r_t_N, p_t_N = spearman(d.N_bits, tries)
            r_t_S, p_t_S = spearman(d['shape'], tries)
            r_b_N, p_b_N = spearman(d.N_bits, nb)
            r_b_S, p_b_S = spearman(d['shape'], nb)
            rows.append(dict(
                set=s, variant=v, n=len(d), keys=d.key.nunique(),
                tries_mean=round(tries.mean(), 2), tries_p99=int(tries.quantile(.99)),
                tries_max=int(tries.max()), mr_mean=round(d[f'{v}_mr'].mean(), 2),
                hnf_mean=round(d[f'{v}_hnf'].mean(), 3),
                accept_rate=round(rho, 5), K_2pow64=k_for(rho),
                time_ms=round(1e3 * d[f'{v}_time'].mean(), 3),
                Np_bits_mean=round(nb.mean(), 2), Np_bits_sd=round(nb.std(), 2),
                Np_bits_min=int(nb.min()), Np_bits_max=int(nb.max()),
                N_bound=int(d.N_bound.iloc[0]),
                tries_key_eta2=round(eta, 4), tries_key_eta2_null=round(null, 4),
                tries_key_anova_p=p_an, tries_key_kw_p=kw_p(d, f'{v}_tries'),
                Npbits_key_eta2=round(eta_nb, 4), Npbits_key_anova_p=p_nb,
                rho_tries_Nbits=round(r_t_N, 4), p_tries_Nbits=p_t_N,
                rho_tries_shape=round(r_t_S, 4), p_tries_shape=p_t_S,
                rho_Npbits_Nbits=round(r_b_N, 4), p_Npbits_Nbits=p_b_N,
                rho_Npbits_shape=round(r_b_S, 4), p_Npbits_shape=p_b_S))
    t = pd.DataFrame(rows)
    t.to_csv(OUT / 'sampler_table.csv', index=False)
    # plot: N' length histogram per set (odd_norm)
    fig, ax = plt.subplots(figsize=(7, 4))
    for s in SETS:
        d = df[df.set == s]
        if d.empty:
            continue
        col = 'odd_gauss_Np_bits' if 'odd_gauss_Np_bits' in d else 'odd_norm_Np_bits'
        x = d[col] - (d.p_bits + 1) // 2
        ax.hist(x, bins=range(int(x.min()), int(x.max()) + 2), histtype='step', label=s)
    ax.set_xlabel("bits(N') - ceil(L/2)  (odd/gauss sampler)")
    ax.set_ylabel('signatures')
    ax.axvline(32, ls='--', c='k', lw=.8)
    ax.legend()
    fig.tight_layout()
    fig.savefig(PLOTS / 'np_bits_hist.png', dpi=110)
    plt.close(fig)
    return t


def pair_tables():
    df = load('pair')
    if df is None:
        return None
    rows = []
    for s in SETS:
        d = df[df.set == s]
        if d.empty:
            continue
        for v in ['base'] + DOWN_V:
            col = f'{v}_ex_lovasz'
            if v != 'base':
                d2 = d[d[f'{v}_std'] == 1]
            else:
                d2 = d
            sl, r, ci = slope(d2.N_bits, d2[col], ci=True)
            rs, ps = spearman(d2.N_bits, d2[col])
            rsh, psh = spearman(d2['shape'], d2[col])
            eta, null, p_an = anova_eta(d2, col)
            row = dict(set=s, arm=v, n=len(d2), keys=d2.key.nunique(),
                       ex_lovasz_mean=round(d2[col].mean(), 1),
                       ex_mul_mean=round(d2[f'{v}_ex_mul'].mean(), 1),
                       slope_steps_per_Nbit=round(sl, 3), slope_ci95=round(ci, 3),
                       r_Nbits=round(r, 4),
                       spearman_Nbits=round(rs, 4), p_Nbits=ps,
                       spearman_shape=round(rsh, 4), p_shape=psh,
                       key_eta2=round(eta, 4), key_eta2_null=round(null, 4),
                       key_anova_p=p_an,
                       zi_traces=d2[f'{v}_zi_trace'].nunique(),
                       zi_short=round(d2[f'{v}_zi_short'].mean(), 4),
                       ex_time_ms=round(1e3 * d2[f'{v}_ex_time'].mean(), 2),
                       zi_time_ms=round(1e3 * d2[f'{v}_zi_time'].mean(), 3))
            if v != 'base':
                sn, rn = slope(d2[f'{v}_Np_bits'], d2[col])
                row.update(dict(
                    std_rate=round(d[f'{v}_std'].mean(), 4),
                    slope_steps_per_Npbit=round(sn, 3), r_Npbits=round(rn, 4),
                    cost_vs_base=round(d2[f'{v}_ex_mul'].mean() / d2['base_ex_mul'].mean(), 3),
                    samp_time_ms=round(1e3 * d2[f'{v}_samp_time'].mean(), 3),
                    zi_same=round(d2[f'{v}_zi_same'].mean(), 4),
                    zi_norm_ok=round(d2[f'{v}_zi_norm_ok'].mean(), 4),
                    ex_same=round(d2[f'{v}_ex_same'].mean(), 4),
                    ex_b1_ratio_ok=round(d2[f'{v}_ex_b1_ratio_ok'].mean(), 4),
                    mapped_in=round(d2[f'{v}_mapped_in'].mean(), 4)))
            rows.append(row)
    t = pd.DataFrame(rows)
    t.to_csv(OUT / 'pair_table.csv', index=False)
    # plot: steps vs bits(N), base vs re-randomized, per set
    fig, axs = plt.subplots(1, len(SETS), figsize=(16, 3.6), sharey=False)
    for ax, s in zip(axs, SETS):
        d = df[df.set == s]
        if d.empty:
            continue
        ax.scatter(d.N_bits, d.base_ex_lovasz, s=3, alpha=.4, label='baseline I∩I_q')
        dd = d[d.odd_gauss_std == 1]
        ax.scatter(dd.N_bits, dd.odd_gauss_ex_lovasz, s=3, alpha=.4, label='re-randomized (odd/gauss)')
        ax.set_title(s)
        ax.set_xlabel('bits(N), long-term key')
    axs[0].set_ylabel('exact LLL Lovász tests')
    axs[0].legend(fontsize=7, markerscale=3)
    fig.tight_layout()
    fig.savefig(PLOTS / 'steps_vs_Nbits.png', dpi=110)
    plt.close(fig)
    return t


def dudect_tables():
    fx, rd = load('dudect_fixed'), load('dudect_rand')
    if fx is None or rd is None:
        return None
    rows = []
    for s in SETS:
        a, b = fx[fx.set == s], rd[rd.set == s]
        if a.empty or b.empty:
            continue
        row = dict(set=s, n_fixed=len(a), n_rand=len(b),
                   fixed_N_bits=int(a.N_bits.iloc[0]),
                   t_base_ex_mul=round(welch(a.base_ex_mul, b.base_ex_mul), 2),
                   t_base_ex_lovasz=round(welch(a.base_ex_lovasz, b.base_ex_lovasz), 2),
                   t_base_zi_mul=round(welch(a.base_zi_mul, b.base_zi_mul), 2))
        for v in DOWN_V:
            aa, bb = a[a[f'{v}_std'] == 1], b[b[f'{v}_std'] == 1]
            row[f't_{v}_ex_mul'] = round(welch(aa[f'{v}_ex_mul'], bb[f'{v}_ex_mul']), 2)
            row[f't_{v}_ex_lovasz'] = round(welch(aa[f'{v}_ex_lovasz'], bb[f'{v}_ex_lovasz']), 2)
            row[f't_{v}_zi_mul'] = round(welch(aa[f'{v}_zi_mul'], bb[f'{v}_zi_mul']), 2)
            row[f't_{v}_tries'] = round(welch(a[f'{v}_tries'], b[f'{v}_tries']), 2)
            row[f't_{v}_Np_bits'] = round(welch(a[f'{v}_Np_bits'], b[f'{v}_Np_bits']), 2)
        rows.append(row)
    t = pd.DataFrame(rows)
    t.to_csv(OUT / 'dudect_table.csv', index=False)
    return t


def main():
    PLOTS.mkdir(exist_ok=True)
    summary = {}
    for name, fn in (('sampler', samp_tables), ('pair', pair_tables), ('dudect', dudect_tables)):
        t = fn()
        if t is not None:
            summary[name] = json.loads(t.to_json(orient='records'))
            with pd.option_context('display.width', 250, 'display.max_columns', 60):
                print(f'== {name}')
                print(t.to_string(index=False))
    (OUT / 'summary.json').write_text(json.dumps(summary, indent=1))


if __name__ == '__main__':
    main()
