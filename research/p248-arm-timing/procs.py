#!/usr/bin/env python3
"""Per-process analysis of runD (many short processes, ASLR on/off).

For each contrast (G7 orig mode) the per-process mean paired difference is
computed; the between-process sd of those means is compared with the
within-process standard error. A between sd well above the within se is a
process-level (layout) offset. Split-mode processes: per-process median paired
difference (ns) per class / phase / fixture."""
import sys, glob, json, math, statistics as st, collections, re
D = sys.argv[1]
out = {}
orig = collections.defaultdict(lambda: collections.defaultdict(list))   # (mode, cls, other) -> [(mean, se, t)]
for f in sorted(glob.glob(f"{D}/orig_*.csv")):
    mode = re.search(r"orig_(aslr|noaslr)_", f)[1]
    rows = collections.defaultdict(list)
    for line in open(f):
        if line.startswith("timing,"):
            _, s, cls, sch, idx, rep, rf, ref, oth = line.strip().split(",")
            rows[(cls, int(idx))].append(int(oth) - int(ref))
    for (cls, idx), d in rows.items():
        m, sd = st.mean(d), st.stdev(d)
        orig[(mode, cls, idx)][0:0] = []
        orig[(mode, cls, idx)]["v"].append((m, sd / math.sqrt(len(d)), m * math.sqrt(len(d)) / sd))
res = {}
for (mode, cls, idx), dd in sorted(orig.items()):
    v = dd["v"]
    means = [x[0] for x in v]
    ts = [x[2] for x in v]
    se_within = st.mean(x[1] for x in v)
    res[f"orig {mode} {cls} other={idx}"] = {
        "processes": len(v), "mean_of_means_ns": round(st.mean(means), 1),
        "between_process_sd_ns": round(st.stdev(means), 1), "within_se_ns": round(se_within, 1),
        "ratio_between_over_within": round(st.stdev(means) / se_within, 2),
        "max_abs_t": round(max(abs(t) for t in ts), 2), "n_abs_t_gt_2": sum(abs(t) > 2 for t in ts),
        "n_abs_t_gt_3": sum(abs(t) > 3 for t in ts),
        "t_of_process_means": round(st.mean(means) * math.sqrt(len(v)) / st.stdev(means), 2)}
# split mode: per-process median diffs
sp = collections.defaultdict(list)
for f in sorted(glob.glob(f"{D}/split_*.csv")):
    mode = re.search(r"split_(aslr|noaslr)_", f)[1]
    cells = collections.defaultdict(dict)
    for line in open(f):
        if line.startswith("split,"):
            _, s, cls, fx, ph, rnd, pos, ns = line.strip().split(",")
            cells[(cls, ph, int(fx))][int(rnd)] = int(ns)
    for (cls, ph, fx), c in cells.items():
        if fx == 0 or ph in ("nop",):
            continue
        ref = cells[(cls, ph, 0)]
        d = [c[r] - ref[r] for r in c if r in ref]
        sp[(mode, cls, ph, fx)].append(st.median(d))
for (mode, cls, ph, fx), meds in sorted(sp.items()):
    res[f"split {mode} {cls} {ph} fixture={fx}"] = {
        "processes": len(meds), "mean_of_process_medians_ns": round(st.mean(meds), 1),
        "sd_of_process_medians_ns": round(st.stdev(meds), 1),
        "min_ns": min(meds), "max_ns": max(meds)}
json.dump(res, sys.stdout, indent=1)
