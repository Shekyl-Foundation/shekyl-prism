#!/usr/bin/env python3
"""Turn the P248 raw CSVs into two-class RTLF inputs, with the t statistics.

  extract.py OUTDIR RUN=GLOB...      (RUN is a label, GLOB the raw CSVs)

For every contrast it writes OUTDIR/inputs/<id>.csv in RTLF's classic layout
(V1 = "1" for the reference class X, "2" for the other class Y; V2 = ns) and
one manifest row with:
  t_paired  the G7 statistic: mean*sqrt(n)/sd of the paired differences
            (other - reference), as in scripts/ci/stage2c_timing.py;
  t_welch   Welch's unpaired t of the two samples (the dudect statistic);
  mean_diff_ns, median_diff_ns (unpaired medians), n per class.

Contrast kinds:
  orig      a G7 `timing` row: reference vs fixture `idx` (both in one pair);
  matched   the same for the matched-preamble mode; idx == len(class) is the
            null arm (reference again);
  aa        orig-mode A/A: the reference samples of one arm, split at random
            into halves (seeded), Welch and RTLF only (no pairing);
  split     phase-split rounds: fixture f vs fixture 0 for one phase; the null
            fixture (reference copy) is tagged null=1.
"""
import collections, csv, glob, hashlib, json, math, os, random, sys
import numpy as np
from scipy import stats

def welch(x, y):
    r = stats.ttest_ind(y, x, equal_var=False)
    return float(r.statistic)

def paired(x, y):
    d = np.asarray(y, float) - np.asarray(x, float)
    sd = d.std(ddof=1)
    return float(d.mean() * math.sqrt(len(d)) / sd) if sd else float("nan")

def write(outdir, cid, x, y):
    p = os.path.join(outdir, "inputs", cid + ".csv")
    with open(p, "w") as f:
        f.write("V1,V2\n")
        for v in x:
            f.write("1,%d\n" % max(int(v), 1))  # RTLF drops zeros; nop rows can be 0
        for v in y:
            f.write("2,%d\n" % max(int(v), 1))
    return p

def main():
    outdir = sys.argv[1]
    os.makedirs(os.path.join(outdir, "inputs"), exist_ok=True)
    man = []
    for spec in sys.argv[2:]:
        run, pat = spec.split("=", 1)
        for path in sorted(glob.glob(pat)):
            base = os.path.splitext(os.path.basename(path))[0]
            arms = collections.defaultdict(list)   # (tag, cls, idx) -> [(rep, ref, oth)]
            cells = collections.defaultdict(dict)  # (cls, phase, fx) -> round -> ns
            for line in open(path):
                p = line.strip().split(",")
                if len(p) > 1 and p[1] != "p248":
                    continue  # the unpatched a5a6f60 binary prints all five sets
                if p[0] in ("timing", "matched"):
                    arms[(p[0], p[2], int(p[4]))].append((int(p[5]), int(p[7]), int(p[8])))
                elif p[0] == "split":
                    cells[(p[2], p[4], int(p[3]))][int(p[5])] = int(p[7])
            nfix = collections.Counter()
            for (tag, cls, idx) in arms:
                nfix[(tag, cls)] = max(nfix[(tag, cls)], idx)
            for (tag, cls, idx), rows in sorted(arms.items()):
                rows.sort()
                x = [r for _, r, _ in rows]; y = [o for _, _, o in rows]
                kind = "orig" if tag == "timing" else "matched"
                null = int(kind == "matched" and idx == nfix[(tag, cls)])
                cid = "%s__%s__%s__%s__i%d" % (run, base, kind, cls, idx)
                f = write(outdir, cid, x, y)
                man.append(dict(id=cid, file=f, run=run, src=base, kind=kind, cls=cls, idx=idx,
                                phase="full", null=null, n=len(x), t_paired=paired(x, y), t_welch=welch(x, y),
                                mean_diff_ns=float(np.mean(y) - np.mean(x)),
                                median_diff_ns=float(np.median(y) - np.median(x))))
                if kind == "orig":
                    rng = random.Random(int(hashlib.sha256(cid.encode()).hexdigest()[:8], 16))
                    xs = x[:]; rng.shuffle(xs); h = len(xs) // 2
                    a, b = xs[:h], xs[h:2 * h]
                    cid2 = cid.replace("__orig__", "__aa__")
                    f = write(outdir, cid2, a, b)
                    man.append(dict(id=cid2, file=f, run=run, src=base, kind="aa", cls=cls, idx=idx,
                                    phase="full", null=1, n=h, t_paired=float("nan"), t_welch=welch(a, b),
                                    mean_diff_ns=float(np.mean(b) - np.mean(a)),
                                    median_diff_ns=float(np.median(b) - np.median(a))))
            classes = sorted({k[0] for k in cells})
            for cls in classes:
                fxs = sorted({k[2] for k in cells if k[0] == cls})
                nullfx = max(fxs)
                for ph in ["adm", "red", "full", "inv", "herm"]:
                    ref = cells[(cls, ph, 0)]
                    for fx in fxs[1:]:
                        oth = cells[(cls, ph, fx)]
                        rounds = sorted(set(ref) & set(oth))
                        x = [ref[r] for r in rounds]; y = [oth[r] for r in rounds]
                        cid = "%s__%s__split__%s__%s__f%d" % (run, base, cls, ph, fx)
                        f = write(outdir, cid, x, y)
                        man.append(dict(id=cid, file=f, run=run, src=base, kind="split", cls=cls, idx=fx,
                                        phase=ph, null=int(fx == nullfx), n=len(x), t_paired=paired(x, y),
                                        t_welch=welch(x, y), mean_diff_ns=float(np.mean(y) - np.mean(x)),
                                        median_diff_ns=float(np.median(y) - np.median(x))))
    with open(os.path.join(outdir, "manifest.json"), "w") as f:
        json.dump(man, f)
    print(len(man), "contrasts")

main()
