#!/usr/bin/env python3
"""R reference vs numpy port, and RTLF vs t cross-tabs.  agree.py WORKDIR"""
import csv, json, sys
W = sys.argv[1]
def tsv(p):
    return {r["file"]: r for r in csv.DictReader(open(p), delimiter="\t")}
R = tsv(W + "/main/rtlf_a09.tsv"); N = tsv(W + "/np_main_a09.tsv")
common = sorted(set(R) & set(N))
agree = sum(R[f]["detect"] == N[f]["detect"] for f in common)
print("R vs numpy, alpha=0.09, same files: %d, same verdict: %d" % (len(common), agree))
dis = [f for f in common if R[f]["detect"] != N[f]["detect"]]
for f in dis[:20]:
    print("  disagree", f.split("/")[-1], R[f]["detect"], R[f]["sig_deciles"], "|", N[f]["detect"], N[f]["sig_deciles"])
import numpy as np
rel = []
for f in common:
    a = np.array([float(x) for x in R[f]["threshold"].split(";")]); b = np.array([float(x) for x in N[f]["threshold"].split(";")])
    rel.append(np.median(np.abs(a - b) / np.maximum(a, 1)))
print("median over files of the median per-decile relative threshold difference: %.3f; 90th pct %.3f" % (np.median(rel), np.percentile(rel, 90)))
m = json.load(open(W + "/main/manifest.json"))
tab = {}
for r in m:
    if r["kind"] in ("orig",) and r["run"] != "D_procs":
        a = int(R[r["file"]]["detect"]); t = int(abs(r["t_welch"]) > 4.5); tab[(a, t)] = tab.get((a, t), 0) + 1
print("orig real, RTLF x Welch:", tab)
