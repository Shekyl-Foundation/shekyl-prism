# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Analyse results/raw/*.csv -> summary JSON/CSV, histograms, plots."""
import json, os
import numpy as np, pandas as pd
from scipy import stats
import matplotlib; matplotlib.use('Agg')
import matplotlib.pyplot as plt

RAW, RES, PL, HI = 'results/raw', 'results', 'plots', 'results/histograms'
os.makedirs(PL, exist_ok=True); os.makedirs(HI, exist_ok=True)
SETS = ['p248', 'p324', 'p376', 'p500', 'p664']
NAMES = ['B0', 'Hpk', 'Hsk', 'COM', 'EPH', 'REV']
rng = np.random.default_rng(11)


def desc(s):
    s = pd.Series(s).dropna()
    if len(s) == 0:
        return None
    return dict(n=int(len(s)), mean=round(float(s.mean()), 2), sd=round(float(s.std()), 2),
                min=int(s.min()), max=int(s.max()))


def eta2_kw(df, group, val):
    g = df.groupby(group)[val]
    y = df[val]; means = g.mean()
    ssb = (g.size() * (means - y.mean()) ** 2).sum(); sst = ((y - y.mean()) ** 2).sum()
    groups = [x.values for _, x in df.groupby(group)[val]]
    if len(groups) < 2 or any(len(x) < 2 for x in groups):
        return dict(eta2=None, kruskal_H=None, kruskal_p=None)
    kw = stats.kruskal(*groups)
    return dict(eta2=round(float(ssb / sst) if sst else 0, 4),
                kruskal_H=round(float(kw.statistic), 1),
                kruskal_p=float(f'{kw.pvalue:.2e}'))


def chi_uniform(values, bins):
    cnt, edges = np.histogram(values, bins=bins)
    # expected equal; drop empty edge issues
    chi = stats.chisquare(cnt)
    return dict(chi2=round(float(chi.statistic), 2), p=float(f'{chi.pvalue:.2e}'),
                bins=int(len(cnt)), n=int(cnt.sum())), cnt, edges


def hist_csv(path, set_name, construction, kind, edges, cnt):
    rows = [dict(set=set_name, construction=construction, kind=kind,
                 bin_lo=float(edges[i]), bin_hi_excl=float(edges[i + 1]), count=int(cnt[i]))
            for i in range(len(cnt))]
    pd.DataFrame(rows).to_csv(path, index=False)


def collisions(df, key='key', cols=('y_mod256', 'sign', 'y_over_q')):
    """Rate of (key, msg-i) colliding with (key, msg-j) on the H2 index.

    y_over_q is continuous; we use a rounded fingerprint plus y_mod256 and sign.
    """
    hits = tot = 0
    for _, g in df.groupby(key):
        fp = list(zip(g['y_mod256'], g['sign'], np.round(g['y_over_q'] * 2**53).astype(np.int64)))
        tot += len(fp) * (len(fp) - 1) // 2
        from collections import Counter
        for c in Counter(fp).values():
            hits += c * (c - 1) // 2
    return dict(pairs=int(tot), collisions=int(hits),
                rate=float(f'{hits / tot:.2e}') if tot else 0.0)


out, table = {}, []
for set_name in SETS:
    sf = f'{RAW}/stream_{set_name}.csv'
    if not os.path.exists(sf):
        continue
    S = pd.read_csv(sf)
    P = pd.read_csv(f'{RAW}/prod_{set_name}.csv') if os.path.exists(f'{RAW}/prod_{set_name}.csv') else pd.DataFrame()
    F = pd.read_csv(f'{RAW}/fixedq_{set_name}.csv') if os.path.exists(f'{RAW}/fixedq_{set_name}.csv') else None
    o = dict(n_stream=len(S), n_keys=int(S.key.nunique()), n_msgs=int(S.msg.nunique()))
    per = {}
    for name, g in S.groupby('construction'):
        d = dict(
            n=len(g),
            determinism=dict(seed=float(g.det_seed.mean()), q=float(g.det_q.mean()),
                             index=float(g.det_index.mean()),
                             map=float(pd.to_numeric(g.det_map, errors='coerce').mean())
                             if name == 'REV' else None),
            verifier_q_ok=float(g.verifier_q_ok.mean()),
            precomputable_before=float(g.pre_before.mean()),
            precomputable_after=float(g.pre_after.mean()),
            attacker_has_seed_before=float(g.attacker_has_seed_before.mean()),
            attacker_has_seed_after=float(g.attacker_has_seed_after.mean()),
            qtries=desc(g.qtries), h2_tries=desc(g.h2_tries),
            sign_iterations=desc(g.sign_iterations) if 'sign_iterations' in g else None,
            D0_iterations=desc(g.D0_iterations) if 'D0_iterations' in g else None,
            sign_ok=float(g.sign_ok.mean()) if 'sign_ok' in g else None,
            q_bits=desc(g.q_bits),
        )
        if d['sign_iterations'] and d['D0_iterations'] and d['D0_iterations']['mean']:
            d['cost_vs_D0'] = round(d['sign_iterations']['mean'] / d['D0_iterations']['mean'], 3)
            d['trace_cv'] = round(d['sign_iterations']['sd'] / d['sign_iterations']['mean'], 4)
        # uniformity of the ideal index
        uni_y, c1, e1 = chi_uniform(g.y_over_q, bins=np.linspace(0, 1, 33))
        ks = stats.kstest(g.y_over_q, 'uniform')
        uni_b, c2, e2 = chi_uniform(g.y_mod256, bins=np.arange(0, 257))
        d['uniform_y_over_q'] = dict(chi2_32=uni_y, ks_D=round(float(ks.statistic), 4),
                                     ks_p=float(f'{ks.pvalue:.2e}'))
        d['uniform_y_mod256'] = uni_b
        d['sign_balance'] = float(g.sign.mean())
        d['collisions_same_key'] = collisions(g)
        # eta^2 of the q-ideal trace vs key (q varies with the message)
        if 'sign_iterations' in g:
            d['eta2_trace_vs_key'] = eta2_kw(g, 'key', 'sign_iterations')
            d['eta2_trace_vs_msg'] = eta2_kw(g, 'msg', 'sign_iterations')
            d['corr_trace_vs_qbits'] = (None if g.q_bits.std() == 0 else
                dict(pearson=round(float(stats.pearsonr(g.sign_iterations, g.q_bits).statistic), 3),
                     p=float(f'{stats.pearsonr(g.sign_iterations, g.q_bits).pvalue:.2e}')))
        # REV map trace
        if name == 'REV' and g['map_iterations'].notna().any() and (g['map_iterations'] != '').any():
            mi = pd.to_numeric(g['map_iterations'], errors='coerce')
            d['map_iterations'] = desc(mi)
            d['eta2_map_vs_key'] = eta2_kw(g.assign(map_iterations=mi), 'key', 'map_iterations')
            d['map_cv'] = round(float(mi.std() / mi.mean()), 4)
        # product panel
        pg = P[P.construction == name] if len(P) else pd.DataFrame()
        if len(pg):
            d['prod'] = dict(n=len(pg), iterations=desc(pg.prod_iterations),
                             ok=float(pg.prod_ok.mean()),
                             eta2_vs_key=eta2_kw(pg, 'key', 'prod_iterations'),
                             eta2_vs_msg=eta2_kw(pg, 'msg', 'prod_iterations'),
                             cv=round(float(pg.prod_iterations.std() / pg.prod_iterations.mean()), 4))
        # fixed-q panel
        if F is not None:
            fg = F[F.construction == name]
            if len(fg):
                d['fixedq'] = dict(n=len(fg), n_q=int(fg.qid.nunique()), draws=int(fg.groupby('qid').size().iloc[0]),
                                   iterations=desc(fg.sign_iterations),
                                   eta2_vs_q=eta2_kw(fg, 'qid', 'sign_iterations'),
                                   eta2_vs_key=eta2_kw(fg, 'key', 'sign_iterations'),
                                   D0_eta2_vs_q=eta2_kw(fg, 'qid', 'D0_iterations'))
                uni, c, e = chi_uniform(fg.y_over_q, bins=np.linspace(0, 1, 33))
                d['fixedq']['uniform_y_over_q'] = uni
        per[name] = d
        table.append(dict(set=set_name, construction=name,
                          det=d['determinism']['index'],
                          pre_before=d['precomputable_before'],
                          pre_after=d['precomputable_after'],
                          verifier_q=d['verifier_q_ok'],
                          sign_mean=d['sign_iterations']['mean'] if d['sign_iterations'] else None,
                          sign_sd=d['sign_iterations']['sd'] if d['sign_iterations'] else None,
                          cost_vs_D0=d.get('cost_vs_D0'),
                          ks_p=d['uniform_y_over_q']['ks_p'],
                          chi2_y_p=d['uniform_y_over_q']['chi2_32']['p'],
                          eta2_key_qideal=d.get('eta2_trace_vs_key', {}).get('eta2'),
                          eta2_key_prod=(d.get('prod') or {}).get('eta2_vs_key', {}).get('eta2'),
                          eta2_q_fixed=(d.get('fixedq') or {}).get('eta2_vs_q', {}).get('eta2'),
                          collisions=d['collisions_same_key']['collisions']))
        # histograms
        hist_csv(f'{HI}/y_{set_name}_{name}.csv', set_name, name, 'y_over_q', e1, c1)
        if 'sign_iterations' in g:
            it = g.sign_iterations.values
            w = max(1, int(np.ceil((it.max() - it.min() + 1) / 40)))
            edges = np.arange(it.min(), it.max() + w + 1, w)
            cnt, ed = np.histogram(it, bins=edges)
            hist_csv(f'{HI}/trace_{set_name}_{name}.csv', set_name, name, 'sign_iterations', ed, cnt)
    o['constructions'] = per
    out[set_name] = o

    # plots: y/q histograms, traces, fixed-q boxplots, prod boxplots
    fig, ax = plt.subplots(2, 3, figsize=(14, 6.5))
    for a, name in zip(ax.flat, NAMES):
        g = S[S.construction == name]
        a.hist(g.y_over_q, bins=32, range=(0, 1), color='C1', alpha=0.85)
        a.set_title(f'{name} y/q', fontsize=9)
    fig.suptitle(f'{set_name}: H2 ideal-index coordinate y/q')
    fig.tight_layout(); fig.savefig(f'{PL}/y_{set_name}.png', dpi=80); plt.close(fig)

    if 'sign_iterations' in S:
        fig, ax = plt.subplots(2, 3, figsize=(14, 6.5))
        for a, name in zip(ax.flat, NAMES):
            g = S[S.construction == name]
            a.hist(g.sign_iterations, bins=40, color='C0', alpha=0.85)
            a.set_title(f'{name} LLL steps (mean {g.sign_iterations.mean():.0f})', fontsize=8)
        fig.suptitle(f'{set_name}: q-ideal LLL iteration counts')
        fig.tight_layout(); fig.savefig(f'{PL}/trace_{set_name}.png', dpi=80); plt.close(fig)

    if F is not None:
        fig, ax = plt.subplots(2, 3, figsize=(16, 7))
        for a, name in zip(ax.flat, NAMES):
            fg = F[F.construction == name]
            order = sorted(fg.qid.unique())
            a.boxplot([fg[fg.qid == q]['sign_iterations'] for q in order], showfliers=False)
            et = per[name].get('fixedq', {}).get('eta2_vs_q', {}).get('eta2')
            a.set_title(f'{name} η²_q={et}', fontsize=8)
            a.set_xlabel('fixed q')
        fig.suptitle(f'{set_name}: traces at 24 fixed challenge degrees')
        fig.tight_layout(); fig.savefig(f'{PL}/fixedq_{set_name}.png', dpi=80); plt.close(fig)

    fig, ax = plt.subplots(2, 3, figsize=(16, 7))
    for a, name in zip(ax.flat, NAMES):
        if not len(P):
            continue
        pg = P[P.construction == name]
        if not len(pg):
            continue
        order = sorted(pg.key.unique())
        a.boxplot([pg[pg.key == k]['prod_iterations'] for k in order], showfliers=False)
        et = per[name].get('prod', {}).get('eta2_vs_key', {}).get('eta2')
        a.set_title(f'{name} η²_key={et}', fontsize=8)
        a.set_xlabel('secret key')
    fig.suptitle(f'{set_name}: I_sec·I_q traces across keys')
    fig.tight_layout(); fig.savefig(f'{PL}/prod_{set_name}.png', dpi=80); plt.close(fig)

pd.DataFrame(table).to_csv(f'{RES}/summary_seeding_table.csv', index=False)
json.dump(out, open(f'{RES}/summary_seeding.json', 'w'), indent=1)
print('wrote', f'{RES}/summary_seeding.json', 'sets', list(out))
