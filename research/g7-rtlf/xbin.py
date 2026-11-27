#!/usr/bin/env python3
"""dev vs a5a6f60 directly: the same fixture's samples from a dev run and an
a5a6f60 run on the same board and core (separate processes, minutes apart, so
any difference mixes code with time). Welch t and RTLF only.

  xbin.py OUTDIR LABEL=DEVCSV,A5CSV ...
"""
import collections, json, os, sys
import numpy as np
from scipy import stats

def load(path):
    arms = collections.defaultdict(lambda: ([], []))
    for line in open(path):
        p = line.strip().split(",")
        if p[0] == "timing" and p[1] == "p248":
            arms[(p[2], int(p[4]))][0].append(int(p[7])); arms[(p[2], int(p[4]))][1].append(int(p[8]))
    return arms

out = sys.argv[1]; os.makedirs(out + "/inputs", exist_ok=True); man = []
for spec in sys.argv[2:]:
    label, pair = spec.split("="); dev, a5 = (load(p) for p in pair.split(","))
    for (cls, idx) in sorted(dev):
        for side, k in (("ref", 0), ("other", 1)):
            if side == "ref" and idx != 1:
                continue  # one reference sample per class is enough
            x = np.array(dev[(cls, idx)][k]); y = np.array(a5[(cls, idx)][k])
            cid = "%s__%s__i%d__%s" % (label, cls, idx if side == "other" else 0, side)
            f = os.path.join(out, "inputs", cid + ".csv")
            open(f, "w").write("V1,V2\n" + "".join("1,%d\n" % v for v in x) + "".join("2,%d\n" % v for v in y))
            man.append(dict(id=cid, file=f, cond=label, cls=cls, idx=idx if side == "other" else 0, n=len(x),
                            t_welch=float(stats.ttest_ind(y, x, equal_var=False).statistic),
                            mean_diff_ns=float(y.mean() - x.mean()), median_diff_ns=float(np.median(y) - np.median(x))))
json.dump(man, open(out + "/manifest.json", "w")); print(len(man))
