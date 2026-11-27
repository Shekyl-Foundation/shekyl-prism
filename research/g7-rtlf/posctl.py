#!/usr/bin/env python3
"""Synthetic positive control: a known delay added to one side of a true null.

  posctl.py OUTDIR LABEL=SPLITCSV...

The null is the split-mode same-data arm (the reference row timed twice in the
same shuffled round; fixture index = class size). For phases `full` and `adm`
of the guarded class, the rounds are cut into consecutive chunks of CHUNK
rounds (512, the G7 repeat count) and, separately, used whole. DELAY ns is
added to every sample of the null copy (class Y). Each (chunk, delay) is one
RTLF input plus its paired and Welch t. The detection rate at delay 0 is the
false-positive rate; the smallest delay detected in >= 90% of chunks is the
floor reported.
"""
import collections, json, math, os, sys
import numpy as np
from scipy import stats

CHUNK = 512
DELAYS = {"full": [0, 100, 200, 400, 800, 1600, 3200, 6400],
          "adm": [0, 25, 50, 100, 200, 400, 800, 1600]}
WHOLE = {"full": [0, 25, 50, 100, 200, 400, 800], "adm": [0, 10, 25, 50, 100, 200]}

def main():
    out = sys.argv[1]; os.makedirs(out + "/inputs", exist_ok=True)
    man = []
    for spec in sys.argv[2:]:
        label, path = spec.split("=", 1)
        cells = collections.defaultdict(dict)
        for line in open(path):
            p = line.strip().split(",")
            if p[0] == "split" and p[2] == "prered-guard256":
                cells[(p[4], int(p[3]))][int(p[5])] = int(p[7])
        nullfx = max(k[1] for k in cells)
        for ph in DELAYS:
            ref, cp = cells[(ph, 0)], cells[(ph, nullfx)]
            rounds = sorted(set(ref) & set(cp))
            x_all = np.array([ref[r] for r in rounds]); y_all = np.array([cp[r] for r in rounds])
            jobs = [("c%02d" % c, slice(c * CHUNK, (c + 1) * CHUNK), DELAYS[ph]) for c in range(len(rounds) // CHUNK)]
            jobs.append(("whole", slice(0, len(rounds)), WHOLE[ph]))
            for tag, sl, delays in jobs:
                x, y0 = x_all[sl], y_all[sl]
                for d in delays:
                    y = y0 + d
                    cid = "%s__%s__%s__d%05d" % (label, ph, tag, d)
                    f = os.path.join(out, "inputs", cid + ".csv")
                    with open(f, "w") as fh:
                        fh.write("V1,V2\n" + "".join("1,%d\n" % v for v in x) + "".join("2,%d\n" % v for v in y))
                    dd = (y - x).astype(float)
                    man.append(dict(id=cid, file=f, cond=label, phase=ph, chunk=tag, delay=d, n=len(x),
                                    t_paired=float(dd.mean() * math.sqrt(len(dd)) / dd.std(ddof=1)),
                                    t_welch=float(stats.ttest_ind(y, x, equal_var=False).statistic)))
    json.dump(man, open(out + "/manifest.json", "w"))
    print(len(man))

main()
