# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Analyse the higher-dim / ablation panels into one summary table."""
import csv
import json
from pathlib import Path

import numpy as np
import pandas as pd
from scipy import stats
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

RAW = Path('results/raw')
OUT = Path('results')
HIST = OUT / 'histograms'
PLOTS = Path('plots')


def eta2_kw(df, group, val):
    if df is None or df.empty or df[group].nunique() < 2:
        return dict(eta2=None, kruskal_H=None, kruskal_p=None)
    y = df[val].to_numpy(dtype=float)
    sst = float(((y - y.mean()) ** 2).sum())
    ssb = 0.0
    for _, g in df.groupby(group):
        yy = g[val].to_numpy(dtype=float)
        ssb += len(yy) * (yy.mean() - y.mean()) ** 2
    try:
        H, p = stats.kruskal(*[g[val].to_numpy(dtype=float)
                               for _, g in df.groupby(group)])
    except Exception:
        H, p = None, None
    return dict(eta2=round(float(ssb / sst) if sst else 0, 4),
                kruskal_H=None if H is None else round(float(H), 1),
                kruskal_p=None if p is None else float(p))


def mean_sd(s):
    s = pd.to_numeric(s, errors='coerce').dropna()
    if s.empty:
        return dict(n=0, mean=None, sd=None)
    return dict(n=int(s.size), mean=round(float(s.mean()), 2),
                sd=round(float(s.std(ddof=1)), 2) if s.size > 1 else 0.0)


def slope_vs_bits(df, x='q_bits', y='iterations'):
    if df is None or len(df) < 4 or df[x].nunique() < 2:
        return dict(slope=None, intercept=None, r=None, r2=None, p=None)
    xv = df[x].to_numpy(dtype=float)
    yv = df[y].to_numpy(dtype=float)
    if np.allclose(xv, xv[0]):
        return dict(slope=None, intercept=None, r=None, r2=None, p=None)
    r = stats.linregress(xv, yv)
    return dict(slope=round(float(r.slope), 4),
                intercept=round(float(r.intercept), 2),
                r=round(float(r.rvalue), 4),
                r2=round(float(r.rvalue ** 2), 4),
                p=float(r.pvalue))


def load(name):
    p = RAW / name
    return pd.read_csv(p) if p.exists() else None


def hist_and_bins(g, title, stem):
    HIST.mkdir(parents=True, exist_ok=True)
    PLOTS.mkdir(parents=True, exist_ok=True)
    vals = pd.to_numeric(g['iterations'], errors='coerce').dropna().to_numpy()
    if len(vals) == 0:
        return
    fig, ax = plt.subplots(figsize=(6, 3.5))
    ax.hist(vals, bins=min(32, max(5, len(vals) // 8)),
            color='#215f8a', edgecolor='white')
    ax.set_title(title)
    ax.set_xlabel('LLL iterations')
    ax.set_ylabel('count')
    fig.tight_layout()
    fig.savefig(PLOTS / f'{stem}.png', dpi=120)
    plt.close(fig)
    counts, edges = np.histogram(vals, bins=32)
    with (HIST / f'{stem}.csv').open('w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['bin_left', 'bin_right', 'count'])
        for c, a, b in zip(counts, edges[:-1], edges[1:]):
            w.writerow([float(a), float(b), int(c)])


def summarize(g):
    it = mean_sd(g['iterations'])
    return dict(
        n=int(len(g)),
        ok_q=float(g['ok_q'].mean()) if 'ok_q' in g else None,
        ok_in=float(g['ok_in'].mean()) if 'ok_in' in g else None,
        failed=float(g['failed'].mean()) if 'failed' in g else None,
        iterations=it,
        cv=(round(it['sd'] / it['mean'], 4)
            if it['mean'] else None),
        defect=mean_sd(g['defect']) if 'defect' in g else None,
        eta2_vs_key=eta2_kw(g, 'key', 'iterations') if g['key'].nunique() > 1 else None,
        eta2_vs_q=eta2_kw(g, 'q_bits', 'iterations') if g['q_bits'].nunique() > 1 else None,
        slope_vs_q=slope_vs_bits(g),
    )


def concat_panel(kind):
    frames = []
    for set_name in ['p248', 'p324', 'p376', 'p500', 'p664']:
        df = load(f'{kind}_{set_name}.csv')
        if df is not None:
            frames.append(df)
    return pd.concat(frames, ignore_index=True) if frames else None


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    HIST.mkdir(parents=True, exist_ok=True)
    PLOTS.mkdir(parents=True, exist_ok=True)
    summary = {}
    table_rows = []

    for kind in ('stream', 'prod'):
        alls = concat_panel(kind)
        if alls is None:
            continue
        summary[kind] = {}
        # per set, per construction
        for set_name, sdf in alls.groupby('set'):
            summary[kind][set_name] = {}
            for name, g in sdf.groupby('construction'):
                summary[kind][set_name][name] = summarize(g)
                hist_and_bins(g, f'{set_name} {name} ({kind})',
                              f'{kind}_{set_name}_{name}')
        # across-set slope and global stats per construction
        fig, ax = plt.subplots(figsize=(8, 5))
        summary[f'{kind}_across'] = {}
        # H2_4 mean for cost ratios, per set
        h2_mean = {}
        if 'H2_4' in set(alls.construction):
            h2_mean = (alls[alls.construction == 'H2_4']
                       .groupby('set')['iterations'].mean().to_dict())
        for name, g in alls.groupby('construction'):
            sl = slope_vs_bits(g)
            means = (g.groupby('set')
                      .agg(q_bits=('q_bits', 'mean'),
                           iterations=('iterations', 'mean'))
                      .reset_index())
            slm = slope_vs_bits(means)
            # cost vs H2_4 at each set, then a single P248 and P664 figure
            costs = {}
            for s, gg in g.groupby('set'):
                if s in h2_mean and h2_mean[s]:
                    costs[s] = round(float(gg['iterations'].mean() / h2_mean[s]), 3)
            ch = g['characteristic'].iloc[0] if 'characteristic' in g else ''
            dim = int(g['dim'].iloc[0]) if 'dim' in g else None
            summ = summarize(g)
            rec = dict(
                characteristic=ch, dim=dim,
                ok_q=summ['ok_q'], ok_in=summ['ok_in'], failed=summ['failed'],
                slope_rows=sl, slope_set_means=slm,
                iterations_by_set={s: mean_sd(gg['iterations'])
                                   for s, gg in g.groupby('set')},
                cv_by_set={s: (round(gg['iterations'].std() / gg['iterations'].mean(), 4)
                               if gg['iterations'].mean() else None)
                           for s, gg in g.groupby('set')},
                eta2_vs_key_by_set={s: eta2_kw(gg, 'key', 'iterations')
                                    for s, gg in g.groupby('set')},
                cost_vs_H2_4=costs,
                defect=summ['defect'],
            )
            summary[f'{kind}_across'][name] = rec
            if sl['slope'] is not None and g['q_bits'].nunique() > 1:
                ax.scatter(g['q_bits'], g['iterations'], s=6, alpha=0.2)
                xs = np.array([g['q_bits'].min(), g['q_bits'].max()], dtype=float)
                ax.plot(xs, sl['slope'] * xs + sl['intercept'],
                        label=f"{name} {sl['slope']:.2f}/bit R²={sl['r2']:.3f}")
            # one table row (panel-level)
            # report P248 and P664 means, overall slope, ok, cv at P500 as mid
            def pick(s):
                d = rec['iterations_by_set'].get(s) or {}
                return d.get('mean'), d.get('sd'), rec['cv_by_set'].get(s)
            m248, s248, c248 = pick('p248')
            m664, s664, c664 = pick('p664')
            # eta2: median across sets (q is constant within a set, so eta2 vs q
            # is meaningless within set; use key-panel eta2)
            ekeys = [v.get('eta2') for v in rec['eta2_vs_key_by_set'].values()
                     if v.get('eta2') is not None]
            table_rows.append(dict(
                panel=kind, construction=name, characteristic=ch, dim=dim,
                n=int(len(g)),
                ok_q=None if summ['ok_q'] is None else round(summ['ok_q'], 4),
                ok_in=None if summ['ok_in'] is None else round(summ['ok_in'], 4),
                failed=None if summ['failed'] is None else round(summ['failed'], 4),
                slope=sl['slope'], r2=sl['r2'],
                mean_p248=m248, sd_p248=s248, cv_p248=c248,
                mean_p664=m664, sd_p664=s664, cv_p664=c664,
                cost_p248=(rec['cost_vs_H2_4'] or {}).get('p248'),
                cost_p664=(rec['cost_vs_H2_4'] or {}).get('p664'),
                eta2_key=(None if not ekeys else round(float(np.median(ekeys)), 4)),
            ))
        ax.set_xlabel('bits of q')
        ax.set_ylabel('LLL iterations')
        ax.set_title(f'{kind}: iterations vs challenge size')
        if ax.get_legend_handles_labels()[0]:
            ax.legend(fontsize=7)
        fig.tight_layout()
        fig.savefig(PLOTS / f'{kind}_slope.png', dpi=140)
        plt.close(fig)

        # dimension scaling plot for EMB* + H2_4 + DS8
        if kind == 'stream':
            fig, ax = plt.subplots(figsize=(7, 4.5))
            dim_arms = alls[alls.construction.isin(
                ['H2_4', 'EMB6', 'EMB8', 'EMB12', 'EMB16', 'DS8', 'QARY8', 'FIX8'])]
            for name, g in dim_arms.groupby('construction'):
                sl = slope_vs_bits(g)
                means = g.groupby('dim').agg(it=('iterations', 'mean')).reset_index()
                if len(means):
                    ax.scatter([g['dim'].iloc[0]], [g['iterations'].mean()], s=40,
                               label=f"{name} slope={sl['slope']}")
            ax.set_xlabel('dimension (rank)')
            ax.set_ylabel('mean LLL iterations (all sets)')
            ax.set_title('How mean cost scales with dimension')
            ax.legend(fontsize=7)
            fig.tight_layout()
            fig.savefig(PLOTS / 'dim_cost.png', dpi=140)
            plt.close(fig)

    (OUT / 'summary_hd.json').write_text(json.dumps(summary, indent=2, default=str))
    if table_rows:
        pd.DataFrame(table_rows).to_csv(OUT / 'summary_hd_table.csv', index=False)
    print('wrote results/summary_hd.json and summary_hd_table.csv')
    print('stream arms', list((summary.get('stream_across') or {}).keys()))
    print('prod arms', list((summary.get('prod_across') or {}).keys()))


if __name__ == '__main__':
    main()
