#!/usr/bin/env python3
"""v2 CSVs -> two-class RTLF inputs + manifest.

  extract_v2.py OUTDIR RUNLABEL FILE...

Per process file (one `v2meta` line, then `v2,set,class,arm,phase,round,position,ns`):
  fixture arm fN  vs the process reference fixture (named in the meta line)
  null, injL, injM, injH  vs the reference fixture (they are copies of it)
  krand vs kfix           (fixed key vs fresh random key)
Each contrast over the whole run (n = rounds) and, for the control arms, also in
chunks of CHUNK rounds. Class "1" is the first named arm (reference or fixed key),
class "2" the other. Statistics recorded: Welch t, round-paired t, mean and median
differences.
"""
import collections, json, math, os, sys
import numpy as np
from scipy import stats

CHUNK = 4096

def write(out, cid, x, y):
    f = os.path.join(out, "inputs", cid + ".csv")
    with open(f, "w") as fh:
        fh.write("V1,V2\n" + "".join("1,%d\n" % max(v, 1) for v in x) + "".join("2,%d\n" % max(v, 1) for v in y))
    return f

def stat(x, y):
    x = np.asarray(x, float); y = np.asarray(y, float); d = y - x
    sd = d.std(ddof=1)
    return dict(t_welch=float(stats.ttest_ind(y, x, equal_var=False).statistic),
                t_paired=float(d.mean() * math.sqrt(len(d)) / sd) if sd else float("nan"),
                mean_diff_ns=float(y.mean() - x.mean()), median_diff_ns=float(np.median(y) - np.median(x)),
                sd_pair_ns=float(sd))

def main():
    out, run, files = sys.argv[1], sys.argv[2], sys.argv[3:]
    os.makedirs(out + "/inputs", exist_ok=True)
    man = []
    for path in files:
        base = os.path.splitext(os.path.basename(path))[0]
        meta = {}; cells = collections.defaultdict(dict)
        for line in open(path):
            p = line.strip().split(",")
            if p[0] == "v2meta":
                meta = dict(kv.split("=", 1) for kv in p[2:] if "=" in kv); meta["set"] = p[1]
            elif p[0] == "v2":
                cells[(p[3], p[4])][int(p[5])] = int(p[7])
        ref = "f" + meta["reference"].lstrip("f")
        arms = sorted({a for a, _ in cells})
        for ph in ("adm", "herm", "full"):
            pairs = []
            for a in arms:
                if a in (ref, "kfix", "krand"):
                    continue
                kind = "fixture" if a.startswith("f") else ("null" if a == "null" else "inject")
                pairs.append((ref, a, kind))
            pairs.append(("kfix", "krand", "keys"))
            for (r, a, kind) in pairs:
                cr, ca = cells[(r, ph)], cells[(a, ph)]
                rounds = sorted(set(cr) & set(ca))
                x = [cr[k] for k in rounds]; y = [ca[k] for k in rounds]
                spans = [("all", slice(0, len(x)))]
                if kind in ("null", "inject") and len(x) >= 2 * CHUNK:
                    spans += [("c%d" % i, slice(i * CHUNK, (i + 1) * CHUNK)) for i in range(len(x) // CHUNK)]
                for tag, sl in spans:
                    cid = "%s__%s__%s__%s__%s__%s" % (run, base, ph, a, "vs" + r, tag)
                    f = write(out, cid, x[sl], y[sl])
                    man.append(dict(id=cid, file=f, run=run, src=base, seed=meta.get("seed"), perm=meta.get("perm"),
                                    ref=ref, phase=ph, arm=a, versus=r, kind=kind, span=tag, n=len(x[sl]), **stat(x[sl], y[sl])))
        # fixture identity offsets: each fixture's median minus the median over all fixtures, per phase
    json.dump(man, open(out + "/manifest.json", "w"))
    print(len(man), "contrasts")

main()
