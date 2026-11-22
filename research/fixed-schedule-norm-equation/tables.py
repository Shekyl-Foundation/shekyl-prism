# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""results/summary.json (+ validation, C cross-check, exponent what-if)
-> results/TABLES.md."""
import csv
import glob
import json
import math
import statistics as st
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
RES = HERE / 'results'
S = json.load(open(RES / 'summary.json'))
SETS = [s for s in ['p248', 'p324', 'p376', 'p500', 'p664'] if s in S]
L = []


def f(x, d=3):
    if x is None:
        return '—'
    if isinstance(x, (list, tuple)):
        return '/'.join(f(y, d) for y in x)
    if isinstance(x, int):
        return f'{x:,}'
    if x == 0:
        return '0'
    if abs(x) >= 1e6:
        return f'{x:.2e}'
    if abs(x) >= 100:
        return f'{x:,.0f}'
    if abs(x) < 1e-3:
        return f'{x:.1e}'
    return f'{x:.{d}g}'


def rho(t):
    r, p = t
    return f'{r:+.3f} (p={p:.2g})'


L.append('## T1. Reference norm equation in signing: does the work track the secret?\n')
L.append('Spearman ρ of the weighted work with bits(N) (the secret ideal\'s norm), '
         'with bits(n) and v₂(n) of the small ideal the search actually runs on; '
         'η² of the work across keys (Kruskal–Wallis); smallest |ρ| detectable at '
         'α = 0.05 with power 0.8.\n')
L.append('| Set | sigs (keys) | mean passes | p99.9 passes | ρ(work, bits N) | ρ(work, bits n) | ρ(work, v₂ n) | key η² (p) | detectable ρ |')
L.append('|---|---|---|---|---|---|---|---|---|')
for s in SETS:
    o = S[s]
    r = o['ref']
    L.append(f"| {s} | {o['n_sig']:,} ({o['n_keys']}) | {f(r['pass_mean'])} | {f(r['pass_p999'])} | "
             f"{rho(r['rho_work_N_bits'])} | {rho(r['rho_work_n_bits'])} | {rho(r['rho_work_n_v2'])} | "
             f"{r['kw_key_work'][0]:+.4f} ({r['kw_key_work'][1]:.2g}) | {o['det_r']:.3f} |")
L.append('')
L.append('## T2. dudect-style Welch t (log work): fixed vs random\n')
L.append('"Fixed key": one key, many messages, against a fresh key per signature. '
         '"Fixed small ideal": one prepared ideal, many random tapes, against the '
         'random class; this asks whether the count depends on the secret-derived '
         'ideal itself. |t| > 4.5 is the usual dudect threshold.\n')
L.append('| Set | fixed key: n vs n | t (log work) | detectable d | fixed small ideal: t per class | max |t| |')
L.append('|---|---|---|---|---|---|')
for s in SETS:
    d = S[s]['dudect']
    t = d['fixkey_vs_rnd_logwork'][0]
    na, nb = d['fixkey_vs_rnd_work'][2], d['fixkey_vs_rnd_work'][3]
    ts = d['fixS_t_logwork']
    L.append(f"| {s} | {na:,} vs {nb:,} | {t:+.2f} | {d['detectable_d_fixkey']:.3f} σ | "
             f"{', '.join(f'{x:+.1f}' for x in ts)} | {max(abs(x) for x in ts):.1f} |")
L.append('')
L.append('Per fixed small ideal: per-pass success rate of the fixed-order ("fresh") search.\n')
L.append('| Set | class: bits n, v₂ n, success per pass (95% lower bound) |')
L.append('|---|---|')
for s in SETS:
    cl = S[s]['dudect']['fixS_classes']
    L.append(f"| {s} | " + '; '.join(f"{c['n_bits']}b, v₂={c['n_v2']}: {c['fresh_p']:.5f} ({c['fresh_p_lo']:.5f})" for c in cl) + ' |')
L.append('')
L.append('## T3. Key generation\n')
L.append('| Set | keys | prime-norm draws: mean / max / CV | ρ(draws, bits N) | keygen norm-eq passes (mean) | ρ(work, bits N) | detectable ρ |')
L.append('|---|---|---|---|---|---|---|')
for s in SETS:
    k = S[s]['keygen']
    L.append(f"| {s} | {S[s]['n_keys']} | {f(k['draws_mean'])} / {k['draws_max']:,} / {k['draws_cv']:.2f} | "
             f"{rho(k['rho_draws_N'])} | {f(k['kgq_pass_mean'])} | {rho(k['rho_kgq_work_N'])} | {k['det_r']:.3f} |")
L.append('')
L.append('## T4. Class invariance: does an equivalent representative reach the same search?\n')
L.append('L and L′ = L·(−β̄)/n(L) for β in L from boxes of half-width 1, 4, 64, 4096 '
         '(L′ has a different norm). Same small-ideal norm, same small ideal, same '
         'operation trace on the same tape.\n')
L.append("| Set | pairs | bits n(L′) | same norm n | same small ideal | same trace |")
L.append('|---|---|---|---|---|---|')
for s in SETS:
    q = S[s].get('equiv')
    if q:
        L.append(f"| {s} | {q['n']} | {q['L2_bits'][0]}–{q['L2_bits'][1]} | {q['same_norm']:.0%} | {q['same_ideal']:.0%} | {q['same_trace']:.0%} |")
L.append('')
L.append('## T5. The uncapped inner loops of the reference (maxima over all signatures)\n')
L.append('| Set | generator draws per generator (mean / max) | coprime-λ skips per call (mean / max; v₂(n)=0 vs >0) | 2-D Gauss steps max (bound) | Cornacchia Euclid steps max (bound) | Tonelli 2-adic e max |')
L.append('|---|---|---|---|---|---|')
for s in SETS:
    i = S[s]['inner']
    sc = S[s]['schedule']
    lv = i['lam_steps_by_v2']
    L.append(f"| {s} | {i['gen_draws_per_gen']:.2f} / {i['mx_gen']} | {f(i['lam_steps_mean'])} / {i['lam_steps_max']:,}; "
             f"{f(lv['0'])} vs {f(lv['>0'])} | {i['mx_lat']} ({sc['T_lat']}) | {i['mx_corn']} ({sc['T_corn']}) | {i['mx_tonelli']} |")
L.append('')
# C cross-check
cc = {}
for lvl, s in ((1, 'p248'), (3, 'p376'), (5, 'p500')):
    fn = HERE / 'c_crosscheck' / f'qlstat_lvl{lvl}.txt'
    if fn.exists():
        r = [list(map(int, x.split()[1:])) for x in open(fn)]
        P = [x[0] for x in r]
        cc[s] = (len(r), st.mean(P), st.median(P), sum(x[4] for x in r) / sum(P))
diag = {}
for s in SETS:
    fn = HERE / 'data' / s / 'diag_exponent.csv'
    if fn.exists():
        diag[s] = list(csv.DictReader(open(fn)))
if cc:
    L.append('## T6. Model against the C reference (pass counts)\n')
    L.append('C: the pinned reference with a counting patch, 400 calls on its benchmark inputs. '
             'Model: this port, the keys panel (signing inputs). Share of passes with M4 > 0 '
             '(model: exponent what-if run, reference flow, e−2).\n')
    L.append('| Set | C mean / median passes | model mean / median passes | C share M4>0 | model share M4>0 |')
    L.append('|---|---|---|---|---|')
    for s, (n, m, md, pos) in cc.items():
        if s not in S:
            continue
        r = S[s]['ref']
        mp_ = '—'
        if s in diag:
            d = [x for x in diag[s] if x['policy'] == 'ref' and x['e2_minus_e'] == '-2']
            mp_ = f"{sum(int(x['m4pos']) for x in d) / sum(int(x['cvp']) for x in d):.3f}"
        L.append(f"| {s} | {m:,.0f} / {md:,.0f} | {f(r['pass_mean'])} / {f(r['pass_med'])} | {pos:.3f} | {mp_} |")
    L.append('')
L.append('## T7. Failure probability against the pass budget (fixed-order search)\n')
L.append('Passes of the fixed-order search are independent given the ideal, so '
         'P(fail after P passes) = (1 − p)^P. p is measured on all signing inputs '
         '(pooled), and the budget uses the worst 95% lower bound over the pooled '
         'rate, the fixed small-ideal classes and the bits(n)/v₂(n) groups. KS: '
         'distance of the empirical pass counts from the geometric law.\n')
L.append('| Set | p pooled | p worst (lower bound) | KS D (p) | empirical vs geometric survival at the p99 / p99.9 passes |')
L.append('|---|---|---|---|---|')
for s in SETS:
    b = S[s]['budget']
    sv = b['survival']
    pts = [x for x in sv if x[1] > 0][-2:]
    L.append(f"| {s} | {b['p_pass']:.5f} | {b['p_worst']:.5f} | {b['ks'][0]:.3f} ({b['ks'][1]:.2f}) | "
             + '; '.join(f"P={x[0]:,}: {x[1]:.4f} vs {x[2]:.4f}" for x in pts) + ' |')
L.append('')
L.append('Budget P (passes) for a target failure probability, per set, at the worst rate '
         'times the K-draw generator factor:\n')
hdr = [b['target'] for b in S[SETS[0]]['schedule']['budgets']]
L.append('| Set | K | p_eff | ' + ' | '.join(hdr) + ' | reference budget 400,000: log₂ fail (tail fit) |')
L.append('|---|---|---|' + '---|' * len(hdr) + '---|')
for s in SETS:
    sc = S[s]['schedule']
    L.append(f"| {s} | {sc['K']} | {sc['p_eff']:.5f} | " + ' | '.join(f"{b['P']:,}" for b in sc['budgets'])
             + f" | {S[s]['budget']['ref_fail_400k_log2']:.0f} |")
L.append('')
L.append('## T8. Cost of the fixed schedule against the reference (work units)\n')
L.append('Reference: mean and p99.9 of the weighted work per call. Fixed: P × the '
         'per-pass cost, every pass executing every step. Two-stage: P cheap passes, '
         'then the probable-prime test, square root and Cornacchia on the first Q '
         'residue-passing candidates (oblivious compaction, costed at 6 selects × '
         'log₂ P per pass).\n')
L.append('| Set | ref mean | ref p99.9 | fixed per pass | fixed 99.9% (× ref mean) | fixed 2^-64 (× mean) | two-stage 99.9% (×) | two-stage 2^-64 (×) |')
L.append('|---|---|---|---|---|---|---|---|')
for s in SETS:
    o = S[s]
    sc = o['schedule']
    bd = {b['target']: b for b in sc['budgets']}
    tw = {b['target']: b for b in o['two_stage']['budgets'] if b}
    L.append(f"| {s} | {f(o['ref']['work_mean'])} | {f(o['ref']['work_p999'])} | {f(sc['pass_cost'])} | "
             f"{f(bd['99.9%']['work'])} (×{bd['99.9%']['x_ref_mean']:.0f}) | {f(bd['2^-64']['work'])} (×{bd['2^-64']['x_ref_mean']:.0f}) | "
             f"×{tw['99.9%']['x_ref_mean']:.0f} (Q={tw['99.9%']['Q']:,}) | ×{tw['2^-64']['x_ref_mean']:.0f} (Q={tw['2^-64']['Q']:,}) |")
L.append('')
# validation
vrows = {}
for s in SETS:
    full = [x for g in glob.glob(str(HERE / 'data' / s / 'val_full_*.csv')) for x in csv.DictReader(open(g))]
    early = [x for g in glob.glob(str(HERE / 'data' / s / 'val_early_*.csv')) for x in csv.DictReader(open(g))]
    if full or early:
        vrows[s] = (full, early)
if vrows:
    L.append('## T9. Executor validation (fsne.py)\n')
    L.append('Full: the whole schedule at the 99.9% budget, compared with an early-exit run '
             'on the same tape. Early: early-exit runs only, for the per-pass success '
             'rate of the fixed schedule itself.\n')
    L.append('| Set | P, K | full runs | success | same answer as early exit | distinct traces | norm equation holds | unfinished Gauss/Euclid | early runs: p per pass (vs p_eff) |')
    L.append('|---|---|---|---|---|---|---|---|---|')
    for s, (full, early) in vrows.items():
        sc = S[s]['schedule']
        P = full[0]['P'] if full else (early[0]['P'] if early else '')
        K = full[0]['K'] if full else (early[0]['K'] if early else '')
        allr = full + early
        firsts = [int(x['early_first']) for x in early if int(x['early_ok'])]
        pe = len(firsts) / sum(firsts) if firsts else float('nan')
        unf = sum(int(x['unf_lat_e']) for x in allr), sum(int(x['unf_corn_e']) for x in allr)
        L.append(f"| {s} | {int(P):,}, {K} | {len(full)} | {sum(int(x['full_ok']) for x in full)}/{len(full)} | "
                 f"{sum(int(x['same']) for x in full)}/{len(full)} | {len(set(x['trace'] for x in full))} | "
                 f"{sum(int(x['eq_ok']) for x in allr)}/{sum(int(x['early_ok']) for x in allr)} | {unf[0]}/{unf[1]} | "
                 f"{pe:.5f} (n={len(firsts)}; {sc['p_eff']:.5f}) |")
    L.append('')
if diag:
    L.append('## T10. What-if for O29: the target power of two\n')
    L.append('Same inputs, the search aimed at 2^(e−2) (the reference), 2^(e−1) and 2^e. '
             'Whether the dimension-two chain can use the larger powers is the open '
             'question of O29 and is not addressed here.\n')
    L.append('| Set | inputs | e−2: ref passes, fresh p/pass, M4>0 | e−1 | e |')
    L.append('|---|---|---|---|---|')
    for s, d in diag.items():
        cells = []
        for k in ('-2', '-1', '0'):
            r = [x for x in d if x['policy'] == 'ref' and x['e2_minus_e'] == k]
            fr = [x for x in d if x['policy'] == 'fresh' and x['e2_minus_e'] == k]
            cells.append(f"{st.mean(int(x['passes']) for x in r):,.0f}, "
                         f"{sum(int(x['success']) for x in fr) / sum(int(x['passes']) for x in fr):.5f}, "
                         f"{sum(int(x['m4pos']) for x in r) / max(1, sum(int(x['cvp']) for x in r)):.2f}")
        L.append(f"| {s} | {len(d) // 6} | " + ' | '.join(cells) + ' |')
    L.append('')
open(RES / 'TABLES.md', 'w').write('# Tables\n\nGenerated by `tables.py` from `results/summary.json` and the data files.\n\n' + '\n'.join(L) + '\n')
print('\n'.join(L))
