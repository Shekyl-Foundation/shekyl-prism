# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Analyse results/raw/*.csv -> results/summary_4d.json, results/summary_4d_table.csv,
results/histograms/*.csv (raw bin counts for chi-square recomputation), plots/*.png."""
import json, os, glob
import numpy as np, pandas as pd
from scipy import stats
import matplotlib; matplotlib.use('Agg')
import matplotlib.pyplot as plt

RAW, RES, PL, HI = 'results/raw', 'results', 'plots', 'results/histograms'
os.makedirs(PL, exist_ok=True); os.makedirs(HI, exist_ok=True)
rng = np.random.default_rng(7)
SETS = ['p248', 'p324', 'p376', 'p500', 'p664']
HV = ['H1', 'H2', 'H3', 'H3s']


def desc(s):
    s = pd.Series(s)
    return dict(mean=round(float(s.mean()), 2), sd=round(float(s.std()), 2), min=int(s.min()), max=int(s.max()))


def mi_discrete(x, y):
    x = pd.factorize(np.asarray(x))[0]; y = pd.factorize(np.asarray(y))[0]
    c = np.zeros((x.max() + 1, y.max() + 1)); np.add.at(c, (x, y), 1)
    p = c / c.sum(); px = p.sum(1, keepdims=True); py = p.sum(0, keepdims=True); nz = p > 0
    return float((p[nz] * np.log2(p[nz] / (px @ py)[nz])).sum())


def mi_test(x, y, nperm=300):
    obs = mi_discrete(x, y)
    null = np.array([mi_discrete(x, rng.permutation(np.asarray(y))) for _ in range(nperm)])
    return dict(mi_bits=round(obs, 4), excess=round(obs - float(null.mean()), 4),
                p_perm=round(float((1 + (null >= obs).sum()) / (1 + nperm)), 4))


def qbin(s, q=8):
    # quantile bins; ties broken by random jitter (not by row order, which is
    # sorted by target id in the panels and would fake a target effect)
    s = pd.Series(np.asarray(s, dtype=float))
    j = s + rng.uniform(0, 1e-6, len(s))
    return pd.qcut(j.rank(method='first'), q, labels=False)


def corr(a, b):
    if np.std(a) == 0 or np.std(b) == 0:
        return dict(pearson=None, spearman=None, p=None)
    r, p = stats.pearsonr(a, b); rs, _ = stats.spearmanr(a, b)
    return dict(pearson=round(float(r), 3), spearman=round(float(rs), 3), p=float(f'{p:.2e}'))


def entropy_bits(s):
    _, c = np.unique(s, return_counts=True); p = c / c.sum()
    return round(float(-(p * np.log2(p)).sum()), 2)


out, table = {}, []
for st_name in SETS:
    for stream in ['T1', 'T2']:
        f = f'{RAW}/stream_{st_name}_{stream}.csv'
        if not (os.path.exists(f) and os.path.exists(f'{RAW}/panel_{st_name}_{stream}.csv')):
            continue
        S = pd.read_csv(f); P = pd.read_csv(f'{RAW}/panel_{st_name}_{stream}.csv')
        o = dict(n=len(S), T_bits=desc(S.T_bits), A_steps=desc(S.A_steps), A_retries=desc(S.A_retries))
        cols = ['D0_iterations'] + [f'{v}_{t}_iterations' for v in HV for t in ('fix', 'pk')] + \
               [f'{v}_b3_iterations' for v in ('H1', 'H2')]
        o['iterations'] = {c: desc(S[c]) for c in cols}
        o['swaps'] = {c.replace('iterations', 'swaps'): desc(S[c.replace('iterations', 'swaps')]) for c in cols}
        for c in cols:
            table.append(dict(set=st_name, stream=stream, variant=c.replace('_iterations', ''), **o['iterations'][c]))
        table.append(dict(set=st_name, stream=stream, variant='A (Cornacchia steps)', **o['A_steps']))
        o['correct'] = {v: float(S[f'{v}_ok'].mean()) for v in ['D0', 'H1_fix', 'H1_pk', 'H2_fix', 'H2_pk', 'H1_b3', 'H2_b3']}
        o['ratio_bits_nrd_over_T'] = {v: desc(S[f'{v}_ratio_bits']) for v in ['D0', 'H1_pk', 'H2_pk']}
        o['H1_same_output_norm_as_D0'] = float((S['H1_pk_ratio_bits'] == S['D0_ratio_bits']).mean())
        o['public_seed_recompute_equal'] = float(S['H1_pk_recompute_equal'].mean())
        o['trace_entropy_bits_fixed_seed'] = {c: entropy_bits(S[c]) for c in ['A_steps', 'D0_iterations'] + [f'{v}_fix_iterations' for v in HV]}
        o['corr'] = {}
        for c in cols:
            o['corr'][c] = {'~A': corr(S[c], S.A_steps), '~D0': corr(S[c], S.D0_iterations),
                            '~T_bits': corr(S[c], S.T_bits), '~T_hw': corr(S[c], S.T_hw)}
        o['corr']['A_steps'] = {'~T_bits': corr(S.A_steps, S.T_bits), '~T_hw': corr(S.A_steps, S.T_hw)}
        feats = {'T_hw': qbin(S.T_hw), 'T_mod3': S.T_mod3, 'T_mod5': S.T_mod5, 'T_mod16': S.T_mod16,
                 'A_steps': qbin(S.A_steps), 'D0_iterations': qbin(S.D0_iterations)}
        if stream == 'T2':
            feats['T_bits'] = S.T_bits
        o['mi'] = {}
        for c in ['A_steps', 'D0_iterations', 'H1_pk_iterations', 'H2_pk_iterations', 'H3_pk_iterations',
                  'H3s_pk_iterations', 'H1_fix_iterations', 'H2_fix_iterations']:
            for fn, fv in feats.items():
                if fn.split('_')[0] == c.split('_')[0]:
                    continue
                o['mi'][f'{c}|{fn}'] = mi_test(fv.values, qbin(S[c]).values)
        o['ks_fix_vs_pk'] = {v: round(float(stats.ks_2samp(S[f'{v}_fix_iterations'], S[f'{v}_pk_iterations']).pvalue), 4) for v in HV}
        o['ks_shake_vs_blake3'] = {v: round(float(stats.ks_2samp(S[f'{v}_fix_iterations'], S[f'{v}_b3_iterations']).pvalue), 4) for v in ('H1', 'H2')}
        # ---------------- panel: K targets x D public keys
        pn = {}
        g = P.groupby('t_id')
        per = g.agg(D0=('D0_iterations', 'first'), A=('A_steps', 'first'), bits=('T_bits', 'first'))
        for v in HV:
            y = P[f'{v}_iterations']
            means = g[f'{v}_iterations'].mean()
            ss_b = (g.size() * (means - y.mean()) ** 2).sum(); ss_t = ((y - y.mean()) ** 2).sum()
            kw = stats.kruskal(*[x[f'{v}_iterations'].values for _, x in g])
            ct = pd.crosstab(P['t_id'], qbin(y, 6)); chi = stats.chi2_contingency(ct)
            d = dict(eta2=round(float(ss_b / ss_t), 4), kruskal_H=round(float(kw.statistic), 1),
                     kruskal_p=float(f'{kw.pvalue:.2e}'), chi2=round(float(chi.statistic), 1),
                     chi2_dof=int(chi.dof), chi2_p=float(f'{chi.pvalue:.2e}'),
                     sd_within=round(float(g[f'{v}_iterations'].std().mean()), 2),
                     sd_of_means=round(float(means.std()), 2))
            for ref in ('D0', 'A'):
                if per[ref].std() > 0:
                    lr = stats.linregress(per[ref], means)
                    d[f'meanH_vs_{ref}'] = dict(slope=round(float(lr.slope), 3), r=round(float(lr.rvalue), 3), p=float(f'{lr.pvalue:.2e}'))
            if per['bits'].std() > 0:
                lr = stats.linregress(per['bits'], means)
                d['meanH_vs_T_bits'] = dict(slope=round(float(lr.slope), 3), r=round(float(lr.rvalue), 3), p=float(f'{lr.pvalue:.2e}'))
                # eta^2 left after removing a linear dependence on the target's bit length
                fit = lr.intercept + lr.slope * P['T_bits']
                res = y - fit
                mres = res.groupby(P['t_id']).mean()
                ssb = (g.size() * (mres - res.mean()) ** 2).sum(); sst = ((res - res.mean()) ** 2).sum()
                d['eta2_after_bits'] = round(float(ssb / sst), 4)
                kwr = stats.kruskal(*[res[P.t_id == t].values for t in per.index])
                d['kruskal_p_after_bits'] = float(f'{kwr.pvalue:.2e}')
            lo, hi = per['D0'].idxmin(), per['D0'].idxmax()
            ks = stats.ks_2samp(P[P.t_id == lo][f'{v}_iterations'], P[P.t_id == hi][f'{v}_iterations'])
            d['ks_minD0_vs_maxD0'] = dict(D=round(float(ks.statistic), 3), p=float(f'{ks.pvalue:.2e}'))
            pn[v] = d
        pn['targets'] = int(P.t_id.nunique()); pn['draws'] = int(g.size().iloc[0])
        o['panel'] = pn
        out[f'{st_name}_{stream}'] = o

        # ---------------- histograms (+ raw bin counts)
        hv = ['A_steps', 'D0_iterations', 'H1_pk_iterations', 'H2_pk_iterations', 'H3_pk_iterations', 'H3s_pk_iterations']
        rows = []
        fig, ax = plt.subplots(2, 3, figsize=(14, 6.5))
        for a_, c in zip(ax.flat, hv):
            x = S[c].values; w = max(1, int(np.ceil((x.max() - x.min() + 1) / 40)))
            edges = np.arange(x.min(), x.max() + w + 1, w)
            cnt, _ = np.histogram(x, bins=edges)
            rows += [dict(set=st_name, stream=stream, variant=c, bin_lo=int(edges[i]), bin_hi_excl=int(edges[i + 1]), count=int(cnt[i])) for i in range(len(cnt))]
            a_.bar(edges[:-1], cnt, width=w, align='edge', color='C1' if c.startswith('H') else 'C0')
            a_.set_title(f'{c} (n={len(x)})', fontsize=9)
        fig.suptitle(f'{st_name} {stream}: trace histograms (H* with seed = H(simulated pk))')
        fig.tight_layout(); fig.savefig(f'{PL}/hist_{st_name}_{stream}.png', dpi=80); plt.close(fig)
        pd.DataFrame(rows).to_csv(f'{HI}/hist_{st_name}_{stream}.csv', index=False)
        prow = []
        for v in HV:
            x = P[f'{v}_iterations'].values; w = max(1, int(np.ceil((x.max() - x.min() + 1) / 30)))
            edges = np.arange(x.min(), x.max() + w + 1, w)
            for tid, grp in P.groupby('t_id'):
                cnt, _ = np.histogram(grp[f'{v}_iterations'], bins=edges)
                prow += [dict(set=st_name, stream=stream, variant=v, t_id=int(tid), bin_lo=int(edges[i]), bin_hi_excl=int(edges[i + 1]), count=int(cnt[i])) for i in range(len(cnt))]
        pd.DataFrame(prow).to_csv(f'{HI}/panel_hist_{st_name}_{stream}.csv', index=False)
        fig, ax = plt.subplots(1, 4, figsize=(16, 3.6))
        order = per.sort_values('D0').index
        for a_, v in zip(ax, HV):
            a_.boxplot([P[P.t_id == t][f'{v}_iterations'] for t in order], showfliers=False)
            a_.set_title(f'{v}: eta2={pn[v]["eta2"]}', fontsize=9); a_.set_xlabel('target (sorted by D0 iterations)')
        fig.suptitle(f'{st_name} {stream}: fixed targets x {pn["draws"]} simulated public keys')
        fig.tight_layout(); fig.savefig(f'{PL}/panel_{st_name}_{stream}.png', dpi=80); plt.close(fig)

# fixed-schedule control
tours = {}
for st_name in SETS:
    f = f'{RAW}/tours_{st_name}.csv'
    if os.path.exists(f):
        Tt = pd.read_csv(f)
        tours[st_name] = dict(n=len(Tt), tours_needed=desc(Tt.tours_needed),
                              p99=int(np.percentile(Tt.tours_needed, 99)),
                              corr_with_D0=corr(Tt.tours_needed, Tt.D0_iterations),
                              ops_fixed_at_max=12 * int(Tt.tours_needed.max()),
                              trace_variance_at_fixed_T=0)
out['fixed_schedule'] = tours
# cross-set scaling (T1 streams): mean iterations vs log2(T)
sc = {}
xs = [out[f'{s}_T1']['T_bits']['mean'] for s in SETS if f'{s}_T1' in out]
for c in ['A_steps', 'D0_iterations', 'H1_pk_iterations', 'H2_pk_iterations', 'H3_pk_iterations', 'H3s_pk_iterations']:
    ys = [(out[f'{s}_T1']['A_steps'] if c == 'A_steps' else out[f'{s}_T1']['iterations'][c])['mean'] for s in SETS if f'{s}_T1' in out]
    if len(xs) > 2:
        lr = stats.linregress(xs, ys)
        sc[c] = dict(per_bit=round(float(lr.slope), 4), intercept=round(float(lr.intercept), 1), r=round(float(lr.rvalue), 4), means=ys)
out['scaling_T1_vs_log2T'] = dict(log2T=xs, fits=sc)
if sc:
    fig, ax = plt.subplots(figsize=(6, 4))
    for c, v in sc.items():
        ax.plot(xs, v['means'], 'o-', label=f'{c} ({v["per_bit"]}/bit)')
    ax.set_xlabel('log2 T (challenge degree q)'); ax.set_ylabel('mean trace length'); ax.legend(fontsize=7)
    ax.set_title('Cross-set invariant: trace length vs target size'); fig.tight_layout()
    fig.savefig(f'{PL}/scaling_T1.png', dpi=90); plt.close(fig)
pd.DataFrame(table).to_csv(f'{RES}/summary_4d_table.csv', index=False)
json.dump(out, open(f'{RES}/summary_4d.json', 'w'), indent=1)
print('ok')

# key-identity panel (run_secret.py): does the trace depend on the secret ideal?
kp = {}
for st_name in SETS:
    f = f'{RAW}/keys_{st_name}.csv'
    if not os.path.exists(f):
        continue
    K = pd.read_csv(f); g = K.groupby('key')
    d = dict(keys=int(K.key.nunique()), draws=int(g.size().iloc[0]), N_bits=int(K.N_bits.iloc[0]))
    for v in ('D0', 'H2'):
        y = K[f'{v}_iterations']; means = g[f'{v}_iterations'].mean()
        ssb = (g.size() * (means - y.mean()) ** 2).sum(); sst = ((y - y.mean()) ** 2).sum()
        kw = stats.kruskal(*[x[f'{v}_iterations'].values for _, x in g])
        ct = pd.crosstab(K['key'], qbin(y, 6)); chi = stats.chi2_contingency(ct)
        d[v] = dict(mean=round(float(y.mean()), 1), sd=round(float(y.std()), 1), eta2=round(float(ssb / sst), 4),
                    kruskal_p=float(f'{kw.pvalue:.2e}'), chi2=round(float(chi.statistic), 1), chi2_dof=int(chi.dof),
                    chi2_p=float(f'{chi.pvalue:.2e}'), correct=float(K[f'{v}_ok'].mean()))
    kp[st_name] = d
out['key_panel'] = kp
json.dump(out, open(f'{RES}/summary_4d.json', 'w'), indent=1)
print('key panel', json.dumps(kp))
