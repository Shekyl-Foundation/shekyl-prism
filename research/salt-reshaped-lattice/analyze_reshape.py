# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Analyse salt-reshape panels: slope, eta^2, CV, cost, histograms."""
import csv
import json
from collections import defaultdict
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
        H, p = stats.kruskal(*[g[val].to_numpy(dtype=float) for _, g in df.groupby(group)])
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
    if df is None or len(df) < 4:
        return dict(slope=None, intercept=None, r=None, r2=None, p=None)
    xv = df[x].to_numpy(dtype=float)
    yv = df[y].to_numpy(dtype=float)
    r = stats.linregress(xv, yv)
    return dict(slope=round(float(r.slope), 4),
                intercept=round(float(r.intercept), 2),
                r=round(float(r.rvalue), 4),
                r2=round(float(r.rvalue ** 2), 4),
                p=float(r.pvalue))


def load(name):
    p = RAW / name
    if not p.exists():
        return None
    return pd.read_csv(p)


def summarize_group(g):
    it = mean_sd(g['iterations'])
    d = dict(
        n=int(len(g)),
        ok_q=float(g['ok_q'].mean()) if 'ok_q' in g else None,
        ok_in=float(g['ok_in'].mean()) if 'ok_in' in g else None,
        ok_extra=float(g['ok_extra'].mean()) if 'ok_extra' in g else None,
        iterations=it,
        swaps=mean_sd(g['swaps']) if 'swaps' in g else None,
        sizered=mean_sd(g['sizered']) if 'sizered' in g else None,
        cv=round(it['sd'] / it['mean'], 4) if it['mean'] else None,
        q_bits=mean_sd(g['q_bits']),
        covol_bits=mean_sd(g['covol_bits']) if 'covol_bits' in g else None,
        r_bits=mean_sd(g['r_bits']) if 'r_bits' in g else None,
        scale=mean_sd(g['scale']) if 'scale' in g else None,
        slope_vs_q=slope_vs_bits(g, 'q_bits', 'iterations'),
        eta2_vs_q=eta2_kw(g, 'q_bits', 'iterations') if g['q_bits'].nunique() > 1 else None,
        eta2_vs_key=eta2_kw(g, 'key', 'iterations') if 'key' in g and g['key'].nunique() > 1 else None,
        pre_after=float(g['pre_after'].mean()) if 'pre_after' in g else None,
        format_change=float(g['format_change'].mean()) if 'format_change' in g else None,
    )
    return d


def hist_and_bins(g, title, stem):
    HIST.mkdir(parents=True, exist_ok=True)
    PLOTS.mkdir(parents=True, exist_ok=True)
    vals = g['iterations'].to_numpy(dtype=float)
    fig, ax = plt.subplots(figsize=(6, 3.5))
    ax.hist(vals, bins=32, color='#215f8a', edgecolor='white')
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


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    HIST.mkdir(parents=True, exist_ok=True)
    PLOTS.mkdir(parents=True, exist_ok=True)

    summary = {}
    # ---- stream (all sets together for slope, and per set)
    frames = []
    for set_name in ['p248', 'p324', 'p376', 'p500', 'p664']:
        df = load(f'stream_{set_name}.csv')
        if df is None:
            continue
        frames.append(df)
        summary.setdefault('stream', {})[set_name] = {}
        for name, g in df.groupby('construction'):
            summary['stream'][set_name][name] = summarize_group(g)
            hist_and_bins(g, f'{set_name} {name} (stream)', f'stream_{set_name}_{name}')
    if frames:
        alls = pd.concat(frames, ignore_index=True)
        summary['stream_slope'] = {}
        fig, ax = plt.subplots(figsize=(7, 4.5))
        for name, g in alls.groupby('construction'):
            sl = slope_vs_bits(g, 'q_bits', 'iterations')
            # also vs set-mean q bits: use one mean per set
            means = g.groupby('set').agg(q_bits=('q_bits', 'mean'),
                                         iterations=('iterations', 'mean'),
                                         sd=('iterations', 'std')).reset_index()
            sl_set = slope_vs_bits(means, 'q_bits', 'iterations')
            h2mean = None
            summary['stream_slope'][name] = dict(
                across_rows=sl, across_set_means=sl_set,
                per_set={s: summarize_group(gg)['iterations']
                         for s, gg in g.groupby('set')},
            )
            ax.scatter(g['q_bits'], g['iterations'], s=8, alpha=0.25, label=None)
            if sl['slope'] is not None:
                xs = np.array([g['q_bits'].min(), g['q_bits'].max()], dtype=float)
                ax.plot(xs, sl['slope'] * xs + sl['intercept'],
                        label=f"{name} {sl['slope']:.2f} steps/bit  R²={sl['r2']:.3f}")
        ax.set_xlabel('bits of q')
        ax.set_ylabel('LLL iterations')
        ax.set_title('Stream: iterations vs challenge size')
        ax.legend(fontsize=8)
        fig.tight_layout()
        fig.savefig(PLOTS / 'stream_slope.png', dpi=140)
        plt.close(fig)

        # cost vs H2
        summary['cost_vs_H2'] = {}
        for set_name, df in alls.groupby('set'):
            h2 = df[df.construction == 'H2']['iterations'].mean()
            summary['cost_vs_H2'][set_name] = {
                name: round(float(g['iterations'].mean() / h2), 3) if h2 else None
                for name, g in df.groupby('construction')
            }

    # ---- product
    pframes = []
    for set_name in ['p248', 'p324', 'p376', 'p500', 'p664']:
        df = load(f'prod_{set_name}.csv')
        if df is None:
            continue
        pframes.append(df)
        summary.setdefault('prod', {})[set_name] = {}
        for name, g in df.groupby('construction'):
            summary['prod'][set_name][name] = summarize_group(g)
            hist_and_bins(g, f'{set_name} {name} (product)', f'prod_{set_name}_{name}')
    if pframes:
        allp = pd.concat(pframes, ignore_index=True)
        summary['prod_slope'] = {}
        fig, ax = plt.subplots(figsize=(7, 4.5))
        for name, g in allp.groupby('construction'):
            sl = slope_vs_bits(g, 'q_bits', 'iterations')
            means = g.groupby('set').agg(q_bits=('q_bits', 'mean'),
                                         iterations=('iterations', 'mean')).reset_index()
            sl_set = slope_vs_bits(means, 'q_bits', 'iterations')
            summary['prod_slope'][name] = dict(across_rows=sl, across_set_means=sl_set,
                                               per_set={s: summarize_group(gg)['iterations']
                                                        for s, gg in g.groupby('set')})
            ax.scatter(g['q_bits'], g['iterations'], s=8, alpha=0.25)
            if sl['slope'] is not None:
                xs = np.array([g['q_bits'].min(), g['q_bits'].max()], dtype=float)
                ax.plot(xs, sl['slope'] * xs + sl['intercept'],
                        label=f"{name} {sl['slope']:.2f} steps/bit  R²={sl['r2']:.3f}")
        ax.set_xlabel('bits of q')
        ax.set_ylabel('LLL iterations')
        ax.set_title('Product I_sec·I_q: iterations vs challenge size')
        ax.legend(fontsize=8)
        fig.tight_layout()
        fig.savefig(PLOTS / 'prod_slope.png', dpi=140)
        plt.close(fig)

    # ---- sweep
    summary['sweep'] = {}
    fig, ax = plt.subplots(figsize=(7, 4.5))
    for host in ('p248', 'p664'):
        df = load(f'sweep_{host}.csv')
        if df is None:
            continue
        for name, g in df.groupby('construction'):
            sl = slope_vs_bits(g, 'q_bits', 'iterations')
            summary['sweep'][f'{host}:{name}'] = dict(
                slope=sl, iterations_by_bits={
                    int(b): mean_sd(gg['iterations'])
                    for b, gg in g.groupby('q_bits')
                },
                ok_q=float(g['ok_q'].mean()) if 'ok_q' in g else None,
            )
            ax.scatter(g['q_bits'], g['iterations'], s=10, alpha=0.3,
                       label=None)
            if sl['slope'] is not None:
                xs = np.array([g['q_bits'].min(), g['q_bits'].max()], dtype=float)
                ax.plot(xs, sl['slope'] * xs + sl['intercept'],
                        label=f"{host} {name} {sl['slope']:.2f} /bit  R²={sl['r2']:.3f}")
    ax.set_xlabel('bits of synthetic q (p fixed)')
    ax.set_ylabel('LLL iterations')
    ax.set_title('Sweep at fixed p: q-size slope')
    if ax.get_legend_handles_labels()[0]:
        ax.legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(PLOTS / 'sweep_slope.png', dpi=140)
    plt.close(fig)

    (OUT / 'summary_reshape.json').write_text(json.dumps(summary, indent=2, default=str))

    # flat table for the report
    rows = []
    for panel in ('stream', 'prod'):
        block = summary.get(panel) or {}
        for set_name, cons in block.items():
            for name, d in cons.items():
                sl = (summary.get(f'{panel}_slope') or {}).get(name, {})
                rows.append(dict(
                    panel=panel, set=set_name, construction=name,
                    n=d.get('n'), ok_q=d.get('ok_q'), ok_in=d.get('ok_in'),
                    mean=d.get('iterations', {}).get('mean'),
                    sd=d.get('iterations', {}).get('sd'),
                    cv=d.get('cv'),
                    slope_rows=(sl.get('across_rows') or {}).get('slope'),
                    r2_rows=(sl.get('across_rows') or {}).get('r2'),
                    eta2_key=(d.get('eta2_vs_key') or {}).get('eta2'),
                    eta2_key_p=(d.get('eta2_vs_key') or {}).get('kruskal_p'),
                    pre_after=d.get('pre_after'),
                    format_change=d.get('format_change'),
                    r_bits=(d.get('r_bits') or {}).get('mean'),
                    scale=(d.get('scale') or {}).get('mean'),
                ))
    if rows:
        pd.DataFrame(rows).to_csv(OUT / 'summary_reshape_table.csv', index=False)
    print('wrote results/summary_reshape.json and summary_reshape_table.csv')
    print('sets in stream:', list((summary.get('stream') or {}).keys()))
    print('sets in prod:', list((summary.get('prod') or {}).keys()))


if __name__ == '__main__':
    main()
