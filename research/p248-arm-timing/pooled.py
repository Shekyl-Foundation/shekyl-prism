#!/usr/bin/env python3
"""Pooled G7-mode contrasts at P248 over many processes: median paired diff and a
bootstrap (over processes) CI of the mean of 5%-trimmed per-process means.
Usage: pooled.py FILE... (G7 `timing` csvs, one per process)"""
import sys, statistics as st, random, collections
random.seed(7)
runs = collections.defaultdict(list)
for f in sys.argv[1:]:
    rows = collections.defaultdict(list)
    for l in open(f):
        if l.startswith("timing,") and l.split(",")[1] == "p248":
            _, s, c, sch, i, r, rf, ref, o = l.strip().split(","); rows[(c, int(i))].append(int(o) - int(ref))
    for k, d in rows.items():
        runs[k].append(d)
def trim(d, p=0.05):
    s = sorted(d); k = int(len(s) * p); return st.mean(s[k:-k])
for k, procs in sorted(runs.items()):
    allv = [x for p in procs for x in p]
    pm = [trim(p) for p in procs]
    b = sorted(st.mean(random.choices(pm, k=len(pm))) for _ in range(2000))
    print(f"{k[0]} other={k[1]}: processes={len(procs)} pairs={len(allv)} median_diff={st.median(allv):.0f}ns "
          f"mean_trimmed5={st.mean(pm):.0f}ns CI95=[{b[50]:.0f},{b[1950]:.0f}]")
