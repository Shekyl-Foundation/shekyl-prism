#!/usr/bin/env python3
"""Summaries for the P248 ARM timing runs.

  analyze.py orig FILE...    G7 `timing` rows: adjacent reference/other pairs.
  analyze.py split FILE...   phase-split randomized rounds.

Statistic as in docs/benchmarks/stage2c/README.md: Student's t of the paired
differences (other - reference), t = mean * sqrt(n) / sample_sd. Added here:
d_z = mean / sample_sd (paired effect size); the same t on 1%-winsorized data
(timer interrupts and the host's other work make heavy right tails); a blocked
t, where the paired differences are averaged over blocks of consecutive rounds
and t is taken over block means (honest under drift and autocorrelation); and
the two halves of the run.
"""
import json, math, sys, collections, statistics as st

def tstat(d):
    n = len(d)
    m = sum(d) / n
    sd = st.stdev(d)
    return m, sd, (m * math.sqrt(n) / sd if sd else float("nan"))

def winsor(d, p=0.01):
    s = sorted(d)
    lo, hi = s[int(len(s) * p)], s[min(len(s) - 1, int(len(s) * (1 - p)))]
    return [min(max(x, lo), hi) for x in d]

def blocked(d, nb=20):
    k = len(d) // nb
    if k < 2:
        return float("nan")
    means = [sum(d[i * k:(i + 1) * k]) / k for i in range(nb)]
    return tstat(means)[2]

def sign_test(d):
    """Two-sided exact sign test on the nonzero paired differences."""
    from scipy import stats
    pos = sum(x > 0 for x in d)
    neg = sum(x < 0 for x in d)
    return pos / max(pos + neg, 1), float(stats.binomtest(pos, pos + neg, 0.5).pvalue) if pos + neg else float("nan")

def summarize(d):
    n = len(d)
    frac_pos, sign_p = sign_test(d)
    m, sd, t = tstat(d)
    w = winsor(d)
    h1, h2 = d[: n // 2], d[n // 2:]
    return {
        "n": n, "mean_ns": round(m, 1), "sd_ns": round(sd, 1), "t": round(t, 2),
        "d_z": round(m / sd, 4) if sd else None,
        "median_diff_ns": st.median(d), "frac_pos": round(frac_pos, 4), "sign_p": round(sign_p, 4),
        "t_winsor1": round(tstat(w)[2], 2), "mean_winsor1_ns": round(tstat(w)[0], 1),
        "t_winsor5": round(tstat(winsor(d, 0.05))[2], 2), "mean_winsor5_ns": round(tstat(winsor(d, 0.05))[0], 1),
        "t_blocked20": round(blocked(d), 2),
        "t_half1": round(tstat(h1)[2], 2), "t_half2": round(tstat(h2)[2], 2),
    }

def read_orig(files):
    pairs = collections.defaultdict(list)
    for f in files:
        for line in open(f):
            if not line.startswith("timing,"):
                continue
            _, s, cls, sched, idx, rep, rf, ref, oth = line.strip().split(",")
            pairs[(s, cls, sched, int(idx))].append((int(rep), int(ref), int(oth)))
    return pairs

def read_split(files):
    cells = collections.defaultdict(dict)  # (set, cls, phase, fixture) -> round -> ns
    pos = collections.defaultdict(dict)
    for fi, f in enumerate(files):
        for line in open(f):
            if not line.startswith("split,"):
                continue
            _, s, cls, fx, ph, rnd, p, ns = line.strip().split(",")
            cells[(s, cls, ph, int(fx))][int(rnd) + 1_000_000 * fi] = int(ns)  # files are separate runs
    return cells

def rm_anova(table):
    """Repeated-measures one-way ANOVA. `table` is rounds x levels (numpy). Returns (F, p)."""
    import numpy as np
    from scipy import stats
    n, k = table.shape
    grand = table.mean()
    ss_lvl = n * ((table.mean(axis=0) - grand) ** 2).sum()
    ss_rnd = k * ((table.mean(axis=1) - grand) ** 2).sum()
    ss_err = ((table - grand) ** 2).sum() - ss_lvl - ss_rnd
    df1, df2 = k - 1, (k - 1) * (n - 1)
    f = (ss_lvl / df1) / (ss_err / df2)
    return float(f), float(stats.f.sf(f, df1, df2))

def main():
    mode, files = sys.argv[1], sys.argv[2:]
    out = {}
    if mode == "orig":
        for key, rows in sorted(read_orig(files).items()):
            rows.sort()
            d = [o - r for _, r, o in rows]
            out["%s %s %s other=%d" % key] = summarize(d) | {"ref_mean_ns": round(sum(r for _, r, _ in rows) / len(rows))}
    else:
        cells = read_split(files)
        sets = sorted({k[:2] for k in cells})
        for s, cls in sets:
            fixtures = sorted({k[3] for k in cells if k[:2] == (s, cls)})
            ref_idx = 0
            null_idx = max(fixtures)  # second copy of the reference row
            for ph in ["adm", "red", "full", "inv", "herm", "nop"]:
                import numpy as np
                rounds_all = sorted(set.intersection(*(set(cells[(s, cls, ph, fx)]) for fx in fixtures)))
                tab = np.array([[cells[(s, cls, ph, fx)][r] for fx in fixtures] for r in rounds_all], float)
                real = [i for i, fx in enumerate(fixtures) if fx != null_idx]
                f_real, p_real = rm_anova(tab[:, real])
                f_all, p_all = rm_anova(tab)
                from scipy import stats as _st
                fr = _st.friedmanchisquare(*[tab[:, i] for i in real])
                out["%s %s %s ANOVA" % (s, cls, ph)] = {
                    "rounds": len(rounds_all),
                    "levels_real": len(real), "friedman_p_real": float(fr.pvalue), "F_real_fixtures": round(f_real, 3), "p_real": p_real,
                    "levels_with_null_copy": len(fixtures), "F_with_copy": round(f_all, 3), "p_with_copy": p_all,
                    "fixture_means_ns": [round(float(tab[:, i].mean()), 1) for i in range(len(fixtures))]}
                ref = cells[(s, cls, ph, ref_idx)]
                for fx in fixtures:
                    if fx == ref_idx:
                        continue
                    oth = cells[(s, cls, ph, fx)]
                    rounds = sorted(set(ref) & set(oth))
                    d = [oth[r] - ref[r] for r in rounds]
                    label = "NULL(ref copy)" if fx == null_idx else "fixture %d" % fx
                    out["%s %s %s %s" % (s, cls, ph, label)] = summarize(d) | {
                        "ref_median_ns": st.median(ref[r] for r in rounds)}
    json.dump(out, sys.stdout, indent=1)
    print()

main()
