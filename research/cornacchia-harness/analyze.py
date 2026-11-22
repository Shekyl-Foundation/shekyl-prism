# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Analyse results/*.csv -> results/summary.json, results/summary_table.csv, plots/*.png"""
import json, glob, os, sys
import numpy as np, pandas as pd
from scipy import stats
import matplotlib; matplotlib.use('Agg')
import matplotlib.pyplot as plt

RES, PL = 'results', 'plots'
rng = np.random.default_rng(1)

def mi_discrete(x, y):
    x = pd.factorize(x)[0]; y = pd.factorize(y)[0]
    c = np.zeros((x.max() + 1, y.max() + 1)); np.add.at(c, (x, y), 1)
    p = c / c.sum(); px = p.sum(1, keepdims=True); py = p.sum(0, keepdims=True)
    nz = p > 0
    return float((p[nz] * np.log2(p[nz] / (px @ py)[nz])).sum())

def mi_test(x, y, nperm=300):
    obs = mi_discrete(x, y)
    null = np.array([mi_discrete(x, rng.permutation(y)) for _ in range(nperm)])
    return dict(mi_bits=obs, null_mean=float(null.mean()), excess=obs - float(null.mean()),
                p_perm=float((1 + (null >= obs).sum()) / (1 + nperm)))

def qbin(s, q=8):
    return pd.qcut(s, q, labels=False, duplicates='drop')

def desc(s):
    return dict(mean=float(s.mean()), sd=float(s.std()), min=int(s.min()), max=int(s.max()))

out = {}; table = []
sizes = sorted(int(os.path.basename(f).split('_')[1].split('.')[0]) for f in glob.glob(f'{RES}/primes_*.csv'))
for b in sizes:
    P = pd.read_csv(f'{RES}/primes_{b}.csv'); F = pd.read_csv(f'{RES}/fixed_{b}.csv')
    o = dict(n_primes=len(P))
    for v in ['A', 'L', 'B']:
        o[f'{v}_steps'] = desc(P[f'{v}_steps']); o[f'{v}_qsumbits'] = desc(P[f'{v}_qsumbits'])
        table.append(dict(bits=b, variant=v, **{k: round(x, 2) for k, x in o[f'{v}_steps'].items()}))
    o['B0_steps'] = desc(P['B0_steps'])
    table.append(dict(bits=b, variant='B0', **{k: round(x, 2) for k, x in o['B0_steps'].items()}))
    o['B0_minus_L'] = sorted(map(int, (P['B0_steps'] - P['L_steps']).unique()))
    d = P['B_steps'] - P['L_steps']; o['B_minus_L'] = desc(d)
    o['B_Sbits'] = desc(P['B_Sbits'])
    o['sqrt_attempts'] = desc(P['sqrt_attempts'])
    o['A_square_ok_rate'] = float(P['A_ok'].mean())
    # correlation of trace with input
    o['corr'] = {}
    for a_, b_ in [('B_steps', 'L_steps'), ('B_steps', 'A_steps'), ('A_steps', 'L_steps'),
                   ('B_qsumbits', 'L_qsumbits'), ('B_steps', 'm_hw'), ('A_steps', 'm_hw'),
                   ('B_steps', 'u_bits'), ('A_steps', 'u_bits')]:
        r, p = stats.pearsonr(P[a_], P[b_]); rs, ps = stats.spearmanr(P[a_], P[b_])
        o['corr'][f'{a_}~{b_}'] = dict(pearson=float(r), p=float(p), spearman=float(rs))
    # mutual information: trace vs input features, permutation null
    o['mi'] = {}
    feats = {'m_hw': qbin(P['m_hw']), 'm_mod8': P['m_mod8'], 'm_mod3': P['m_mod3'],
             'm_mod5': P['m_mod5'], 'm_mod7': P['m_mod7'], 'u_bits': P['u_bits'].clip(lower=b - 6),
             'u_hw': qbin(P['u_hw']), 'L_steps(=lattice of M,u)': qbin(P['L_steps'])}
    for tr in ['A_steps', 'B_steps']:
        for fn, fv in feats.items():
            o['mi'][f'{tr}|{fn}'] = mi_test(fv.values, qbin(P[tr]).values)
    # fixed-m panel
    g = F.groupby('m_id')
    o['fixed'] = dict(n_m=int(F.m_id.nunique()), draws=int(g.size().iloc[0]),
                      A_deterministic=bool((g['A_steps'].nunique() == 1).all()),
                      A_same_with_fresh_sqrt=bool((F.A_steps == F.A_steps_recheck).all()),
                      B0_spread_per_m=int(g['B0_steps'].nunique().max()),
                      B_sd_within_mean=float(g['B_steps'].std().mean()),
                      B_sd_of_means=float(g['B_steps'].mean().std()),
                      icc=float(g['B_steps'].mean().var() / F['B_steps'].var()))
    kw = stats.kruskal(*[x['B_steps'].values for _, x in g])
    o['fixed']['kruskal_H'] = float(kw.statistic); o['fixed']['kruskal_p'] = float(kw.pvalue)
    ct = pd.crosstab(F['m_id'], qbin(F['B_steps'], 6))
    chi = stats.chi2_contingency(ct)
    o['fixed']['chi2'] = float(chi.statistic); o['fixed']['chi2_dof'] = int(chi.dof); o['fixed']['chi2_p'] = float(chi.pvalue)
    per = g.agg(L=('L_steps', 'first'), A=('A_steps', 'first'), Bm=('B_steps', 'mean'))
    lr = stats.linregress(per['L'], per['Bm'])
    o['fixed']['meanB_vs_L'] = dict(slope=float(lr.slope), intercept=float(lr.intercept), r=float(lr.rvalue), p=float(lr.pvalue))
    lo, hi = per['L'].idxmin(), per['L'].idxmax()
    ks = stats.ks_2samp(F[F.m_id == lo]['B_steps'], F[F.m_id == hi]['B_steps'])
    o['fixed']['ks_minL_vs_maxL'] = dict(m_lo_L=int(per.loc[lo, 'L']), m_hi_L=int(per.loc[hi, 'L']),
                                         D=float(ks.statistic), p=float(ks.pvalue))
    # residual (B - L) independence across m
    F['BmL'] = F['B_steps'] - F['L_steps']
    kw2 = stats.kruskal(*[x['BmL'].values for _, x in F.groupby('m_id')])
    o['fixed']['kruskal_BminusL_H'] = float(kw2.statistic); o['fixed']['kruskal_BminusL_p'] = float(kw2.pvalue)
    o['fixed']['BminusL'] = desc(F['BmL'])
    # random stream
    R = pd.read_csv(f'{RES}/random_{b}_summary.csv').iloc[0].to_dict()
    o['random_stream'] = {k: int(v) for k, v in R.items()}
    out[b] = o

    # ---- plots
    fig, ax = plt.subplots(1, 3, figsize=(15, 4))
    bins = np.arange(min(P.L_steps.min(), P.A_steps.min()) - 1, P.A_steps.max() + 2)
    for v, c in [('A', 'C0'), ('L', 'C2'), ('B', 'C1')]:
        ax[0].hist(P[f'{v}_steps'], bins=bins, alpha=.5, label=v, color=c)
    ax[0].set_title(f'{b}-bit primes: step counts'); ax[0].legend(); ax[0].set_xlabel('steps')
    ax[1].scatter(P.L_steps + rng.uniform(-.3, .3, len(P)), P.B_steps, s=2, alpha=.3)
    ax[1].set_xlabel('L steps (unrandomized Lagrange, = f(M,u))'); ax[1].set_ylabel('B steps (Alg. 10)')
    ax[1].set_title(f'r = {o["corr"]["B_steps~L_steps"]["pearson"]:.3f}')
    order = per.sort_values('L').index
    ax[2].boxplot([F[F.m_id == k]['B_steps'] for k in order], showfliers=False)
    ax[2].plot(range(1, len(order) + 1), per.loc[order, 'L'], 'r.', label='L steps of M')
    ax[2].set_title(f'fixed M, {o["fixed"]["draws"]} draws each (sorted by L)'); ax[2].legend()
    ax[2].set_xlabel('M index'); ax[2].set_ylabel('B steps')
    fig.tight_layout(); fig.savefig(f'{PL}/steps_{b}.png', dpi=90); plt.close(fig)
    fig, ax = plt.subplots(figsize=(5, 3.5))
    ax.hist(F['BmL'], bins=np.arange(F.BmL.min() - .5, F.BmL.max() + 1.5), color='C1')
    ax.set_title(f'{b}-bit: B steps - L steps (fixed panel)'); fig.tight_layout()
    fig.savefig(f'{PL}/b_minus_l_{b}.png', dpi=90); plt.close(fig)

pd.DataFrame(table).to_csv(f'{RES}/summary_table.csv', index=False)
json.dump(out, open(f'{RES}/summary.json', 'w'), indent=1)
print(json.dumps(out, indent=1))
