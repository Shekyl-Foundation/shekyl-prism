# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Analysis of the norm-equation runs -> results/*.md and results/*.csv."""
import csv
import glob
import json
import math
import sys
from pathlib import Path

import numpy as np
from scipy import stats

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fne import SET_MAP, prime_of  # noqa: E402
from fsne import Sched  # noqa: E402

HERE = Path(__file__).resolve().parent
DATA = HERE / 'data'
RES = HERE / 'results'
SETS = ['p248', 'p324', 'p376', 'p500', 'p664']
TARGETS = [('99.9%', 1e-3), ('2^-20', 2.0 ** -20), ('2^-40', 2.0 ** -40),
           ('2^-64', 2.0 ** -64), ('2^-128', 2.0 ** -128)]


def load(name, panel):
    rows = []
    for f in sorted(glob.glob(str(DATA / name / f'{panel}_*.csv'))):
        with open(f) as fh:
            rows += list(csv.DictReader(fh))
    return rows


def col(rows, k, t=float):
    return np.array([t(r[k]) for r in rows])


def detectable_r(n):
    # two-sided alpha 0.05, power 0.8 (Fisher z)
    return math.tanh((1.96 + 0.84) / math.sqrt(max(n - 3, 1)))


def welch(a, b):
    t, p = stats.ttest_ind(a, b, equal_var=False)
    return float(t), float(p)


def kw_eta2(groups):
    groups = [g for g in groups if len(g) > 1]
    H, p = stats.kruskal(*groups)
    n = sum(len(g) for g in groups)
    k = len(groups)
    return float((H - k + 1) / (n - k)), float(p)


def geo_budget(p_pass, target):
    return math.ceil(math.log(target) / math.log1p(-p_pass))


def fmt(x, d=3):
    if isinstance(x, str):
        return x
    if x == 0:
        return '0'
    if abs(x) >= 1e5 or abs(x) < 1e-3:
        return f'{x:.2e}'
    return f'{x:.{d}g}'


def main():
    RES.mkdir(exist_ok=True)
    out = {}
    md = []
    for name in SETS:
        keys = load(name, 'keys')
        fk = load(name, 'fixkey')
        rn = load(name, 'rnd')
        fs = load(name, 'fixS')
        eq = load(name, 'equiv')
        if not keys or not fk or not rn:
            continue
        _, e, f = SET_MAP[name]
        p = prime_of(e, f)
        L = p.bit_length()
        o = dict(set=name, L=L, n_sig=len(keys), n_keys=len(set(r['kid'] for r in keys)))
        Nb = col(keys, 'N_bits')
        nb = col(keys, 'n_bits')
        o['N_bits_range'] = [int(Nb.min()), int(Nb.max())]
        o['N_bits_mean'] = float(Nb.mean())
        o['n_bits_range'] = [int(nb.min()), int(nb.max())]
        o['det_r'] = detectable_r(len(keys))
        for pol in ('ref', 'fresh'):
            P = col(keys, f'{pol}_passes')
            W = col(keys, f'{pol}_work')
            fails = int((col(keys, f'{pol}_success_pass') < 0).sum())
            d = dict(fails=fails, pass_mean=float(P.mean()), pass_med=float(np.median(P)),
                     pass_p999=float(np.quantile(P, 0.999)), pass_max=float(P.max()),
                     pass_cv=float(P.std() / P.mean()), work_mean=float(W.mean()),
                     work_p999=float(np.quantile(W, 0.999)), work_max=float(W.max()),
                     work_cv=float(W.std() / W.mean()))
            for cov in ('N_bits', 'n_bits', 'n_v2'):
                x = col(keys, cov)
                for met, y in (('passes', P), ('work', W)):
                    r, pv = stats.spearmanr(x, y)
                    d[f'rho_{met}_{cov}'] = (float(r), float(pv))
            grp = {}
            for r_, w_ in zip(keys, W):
                grp.setdefault(r_['kid'], []).append(w_)
            d['kw_key_work'] = kw_eta2(list(grp.values()))
            # per-key mean work vs N bits (one point per key)
            kid_N = {r_['kid']: int(r_['N_bits']) for r_ in keys}
            km = [(kid_N[k], np.mean(v)) for k, v in grp.items()]
            r, pv = stats.spearmanr([a for a, _ in km], [b for _, b in km])
            d['rho_keymean_work_N'] = (float(r), float(pv))
            o[pol] = d
        # inner-loop counts of the reference (the uncapped loops)
        o['inner'] = dict(
            gen_draws_per_gen=float(col(keys, 'ref_gen_draws').sum() / col(keys, 'ref_gens').sum()),
            mx_gen=int(col(keys, 'ref_mx_gen').max()),
            lam_steps_mean=float(col(keys, 'ref_lam_steps').mean()),
            lam_steps_max=int(col(keys, 'ref_lam_steps').max()),
            mx_lat=int(max(col(keys, 'ref_mx_lat').max(), col(keys, 'fresh_mx_lat').max())),
            mx_corn=int(max(col(keys, 'ref_mx_corn').max(), col(keys, 'fresh_mx_corn').max())),
            mx_tonelli=int(col(keys, 'ref_mx_tonelli').max()),
            frac_v2_pos=float((col(keys, 'n_v2') > 0).mean()))
        lam = col(keys, 'ref_lam_steps')
        v2 = col(keys, 'n_v2')
        o['inner']['lam_steps_by_v2'] = {
            '0': float(lam[v2 == 0].mean()) if (v2 == 0).any() else None,
            '>0': float(lam[v2 > 0].mean()) if (v2 > 0).any() else None}
        # dudect
        dd = {}
        for met in ('passes', 'work'):
            a = col(fk, f'ref_{met}')
            b = col(rn, f'ref_{met}')
            dd[f'fixkey_vs_rnd_{met}'] = welch(a, b) + (len(a), len(b))
            dd[f'fixkey_vs_rnd_log{met}'] = welch(np.log(a), np.log(b))
        ts = []
        cls_info = []
        for c in sorted(set(r['cls'] for r in fs)):
            sub = [r for r in fs if r['cls'] == c]
            a = col(sub, 'ref_work')
            b = col(rn, 'ref_work')
            t, pv = welch(np.log(a), np.log(b))
            ts.append(t)
            Pf = col(sub, 'fresh_passes')
            cls_info.append(dict(cls=int(c), n_bits=int(sub[0]['n_bits']), n_v2=int(sub[0]['n_v2']),
                                 n=len(sub), t_logwork=t, ref_pass_mean=float(col(sub, 'ref_passes').mean()),
                                 fresh_p=float(len(sub) / Pf.sum()),
                                 fresh_p_lo=float(stats.gamma.ppf(0.025, len(sub), scale=1 / Pf.sum()))))
        dd['fixS_t_logwork'] = ts
        dd['fixS_classes'] = cls_info
        # standardized effect detectable with n per class (|t| > 4.5)
        na, nb_ = len(fk), len(rn)
        dd['detectable_d_fixkey'] = (4.5 + 0.84) * math.sqrt(1 / na + 1 / nb_)
        o['dudect'] = dd
        # keygen
        kg = {}
        seen = {}
        for r_ in keys:
            seen[r_['kid']] = r_
        kr = list(seen.values())
        kd = col(kr, 'kg_draws')
        kN = col(kr, 'N_bits')
        kg['draws_mean'] = float(kd.mean())
        kg['draws_max'] = int(kd.max())
        kg['draws_cv'] = float(kd.std() / kd.mean())
        kg['rho_draws_N'] = tuple(map(float, stats.spearmanr(kN, kd)))
        kq = col(kr, 'kgq_ref_work')
        kg['rho_kgq_work_N'] = tuple(map(float, stats.spearmanr(kN, kq)))
        kg['kgq_pass_mean'] = float(col(kr, 'kgq_ref_passes').mean())
        kg['det_r'] = detectable_r(len(kr))
        # theory: P(prime) for a draw of size t ~ 2/ln t on odd... report draws*ln-scale
        o['keygen'] = kg
        # equivalence
        if eq:
            o['equiv'] = dict(n=len(eq), same_norm=float(col(eq, 'same_norm').mean()),
                              same_ideal=float(col(eq, 'same_ideal').mean()),
                              same_trace=float(col(eq, 'same_trace').mean()),
                              L2_bits=[int(col(eq, 'L2_norm_bits').min()), int(col(eq, 'L2_norm_bits').max())])
        # budget: fresh per-pass success (geometric), pooled and worst class
        Pf = col(keys, 'fresh_passes')
        p_pool = len(Pf) / Pf.sum()
        p_lo = float(stats.gamma.ppf(0.025, len(Pf), scale=1 / Pf.sum()))
        # groups by n_v2 and by n_bits
        grp_p = {}
        grp_lo = []
        masks = [('v2=0', v2 == 0), ('v2=1', v2 == 1), ('v2>=2', v2 >= 2)]
        masks += [(f'nbits={b_}', nb == b_) for b_ in sorted(set(nb.astype(int)))]
        for key, msk in masks:
            if msk.sum() > 100:
                grp_p[key] = float(msk.sum() / Pf[msk].sum())
                grp_lo.append(float(stats.gamma.ppf(0.025, msk.sum(), scale=1 / Pf[msk].sum())))
        cls_lo = [c['fresh_p_lo'] for c in cls_info] or [p_lo]
        # worst case: the smallest lower 95% bound over the pooled rate, the
        # fixed small-ideal classes and the covariate groups
        p_worst = min([p_lo] + cls_lo + grp_lo)
        gen_v = float(col(keys, 'fresh_gens').sum() / col(keys, 'fresh_gen_draws').sum())
        # empirical survival vs geometric
        grid = sorted(set([1, 2, 5] + [int(x) for x in np.quantile(Pf, [.5, .9, .99, .999])] + [int(Pf.max())]))
        surv = [(g, float((Pf > g).mean()), float((1 - p_pool) ** g)) for g in grid]
        ks = stats.kstest(Pf, lambda x: 1 - (1 - p_pool) ** np.floor(x))
        Pr = col(keys, 'ref_passes')
        # reference tail rate: geometric fit on passes beyond the median
        med = np.median(Pr)
        tail = Pr[Pr > med] - med
        p_tail = len(tail) / tail.sum()
        o['budget'] = dict(p_pass=p_pool, p_pass_lo=p_lo, p_worst=p_worst, groups=grp_p,
                           gen_valid=gen_v, survival=surv, ks=(float(ks.statistic), float(ks.pvalue)),
                           ref_tail_rate=float(p_tail), ref_median=float(med),
                           ref_fail_400k_log2=float((400000 - med) * math.log2(1 - p_tail)))
        # schedule choice: K minimizing cost per success
        best = None
        for K in range(1, 33):
            sch = Sched(L, e, P=1, K=K)
            mult = 1 - (1 - gen_v) ** K
            cps = sch.pass_cost() / mult
            if best is None or cps < best[1]:
                best = (K, cps, mult)
        K, _, mult = best
        sch = Sched(L, e, P=1, K=K)
        pe = p_worst * mult
        bud = []
        for lab, tgt in TARGETS:
            Pn = geo_budget(pe, tgt)
            bud.append(dict(target=lab, P=Pn, work=Pn * sch.pass_cost(),
                            x_ref_mean=Pn * sch.pass_cost() / o['ref']['work_mean'],
                            x_ref_p999=Pn * sch.pass_cost() / o['ref']['work_p999']))
        o['schedule'] = dict(sch.describe(), K=K, gen_mult=mult, p_eff=pe, budgets=bud,
                             fits=dict(T_lat_ok=o['inner']['mx_lat'] <= sch.T_lat,
                                       T_corn_ok=o['inner']['mx_corn'] <= sch.T_corn))
        # two-stage alternative: cheap stage on all P passes, PT/Cornacchia on the first Q
        # residue-passing ones (oblivious compaction), post on all Q
        r1 = float(col(keys, 'fresh_mod_ok').sum() / Pf.sum())
        r23 = pe / r1
        cheap = sch.pass_cost() - 16.5 * sch.m4bits - sch.J * max(1, sch.m4bits / 8) - 1.5 * sch.m4bits \
            - (2 * sch.T_corn + 10) - min(8 * sch.cpow_bits, 4 * sch.td) - sch.td
        expensive = sch.pass_cost() - cheap
        two = []
        for lab, tgt in TARGETS:
            bestc = None
            Pn1 = geo_budget(pe, tgt)
            for scale in np.linspace(1.0, 2.0, 21):
                Pn = int(Pn1 * scale)
                # failure = E[(1-r23)^min(Q, X)], X ~ Bin(Pn, r1); choose least Q
                X = stats.binom(Pn, r1)
                lo, hi = 1, Pn
                def fail(Q):
                    ks_ = np.arange(0, Q)
                    pm = X.pmf(ks_)
                    return float((pm * (1 - r23) ** ks_).sum() + X.sf(Q - 1) * (1 - r23) ** Q)
                if fail(Pn) > tgt:
                    continue
                while lo < hi:
                    mid = (lo + hi) // 2
                    if fail(mid) <= tgt:
                        hi = mid
                    else:
                        lo = mid + 1
                Q = lo
                comp = Pn * math.ceil(math.log2(max(Pn, 2))) * 6   # selects, 6-word state
                cost = Pn * cheap + Q * expensive + comp
                if bestc is None or cost < bestc['work']:
                    bestc = dict(target=lab, P=Pn, Q=Q, work=cost,
                                 x_ref_mean=cost / o['ref']['work_mean'])
            two.append(bestc)
        o['two_stage'] = dict(r1=r1, r23=r23, cheap=cheap, expensive=expensive, budgets=two)
        out[name] = o
    with open(RES / 'summary.json', 'w') as fh:
        json.dump(out, fh, indent=1, default=float)
    return out


if __name__ == '__main__':
    o = main()
    for k, v in o.items():
        print(k, json.dumps({kk: v[kk] for kk in ('n_sig', 'ref', 'dudect', 'keygen', 'equiv', 'budget', 'schedule', 'two_stage') if kk in v}, default=float)[:3000])
        print()
