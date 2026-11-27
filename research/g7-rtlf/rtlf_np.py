#!/usr/bin/env python3
"""Vectorised port of the RTLF decision procedure, for bulk runs.

Same algorithm as the pinned rtlf.R (tls-attacker/RTLF 2497d66): for each of
the two classes, B bootstrap replicates draw two resamples of size n (n =
min of the class sizes) with replacement and record |decile_i(x1) - decile_i(x2)|
for deciles 10..90% (R quantile type 2); the per-decile (1 - alpha/9) quantile
(type 2) of those B values is the class threshold; the larger of the two
class thresholds is the decision threshold; decile i is flagged when
|decile_i(X) - decile_i(Y)| of the full samples exceeds it. Detect = any decile.

Differences from the reference are only in the random stream (numpy PCG64
instead of R's Mersenne Twister), so thresholds agree to bootstrap noise. The
agreement with the R reference on the same files is reported in REPORT.md.

  rtlf_np.py LISTFILE OUT.tsv ALPHA [B] [WORKERS]
"""
import hashlib, sys
import numpy as np
from multiprocessing import Pool

P = np.arange(1, 10) / 10.0

def dec_type2(a):
    """R quantile(type=2) at 0.1..0.9 along the last axis of a (sorted)."""
    n = a.shape[-1]
    np_ = P * n
    j = np.floor(np_ + 1e-9).astype(int)
    exact = np.abs(np_ - np.round(np_)) < 1e-9
    j = np.where(exact, np.round(np_).astype(int), j)
    lo = np.clip(j - 1, 0, n - 1)           # 0-indexed x[j]
    hi = np.clip(j, 0, n - 1)               # x[j+1]
    ceil_idx = np.clip(np.ceil(np_ - 1e-9).astype(int) - 1, 0, n - 1)
    return np.where(exact, 0.5 * (a[..., lo] + a[..., hi]), a[..., ceil_idx])

def q_type2(v, p):
    s = np.sort(v, axis=0); n = s.shape[0]
    h = n * p
    if abs(h - round(h)) < 1e-9:
        j = int(round(h)); return 0.5 * (s[max(j - 1, 0)] + s[min(j, n - 1)])
    return s[int(np.ceil(h)) - 1]

def boot(x, n, B, rng):
    out = np.empty((B, 9))
    step = max(1, int(4e7 // max(n, 1)))
    for s in range(0, B, step):
        b = min(step, B - s)
        i1 = rng.integers(0, len(x), (b, n)); i2 = rng.integers(0, len(x), (b, n))
        a1 = np.sort(x[i1], axis=1); a2 = np.sort(x[i2], axis=1)
        out[s:s + b] = np.abs(dec_type2(a1) - dec_type2(a2))
    return out

def one(args):
    f, alpha, B = args
    seed = int(hashlib.sha256(f.encode()).hexdigest()[:8], 16)
    rng = np.random.default_rng(seed)
    v1, v2 = [], []
    for line in open(f):
        if line[0] == "1": v1.append(float(line[2:]))
        elif line[0] == "2": v2.append(float(line[2:]))
    x, y = np.array(v1), np.array(v2)
    n = max(100, min(len(x), len(y)))
    t = np.abs(dec_type2(np.sort(x)) - dec_type2(np.sort(y)))
    ap = alpha / 9
    m1 = boot(x, min(n, len(x)), B, rng); m2 = boot(y, min(n, len(y)), B, rng)
    th = np.maximum([q_type2(m1[:, k], 1 - ap) for k in range(9)], [q_type2(m2[:, k], 1 - ap) for k in range(9)])
    sig = t > th
    return "\t".join([f, str(min(len(x), len(y))), str(int(sig.any())), ";".join(str((k + 1) * 10) for k in np.where(sig)[0]),
                      ";".join("%g" % v for v in t), ";".join("%g" % v for v in th)])

if __name__ == "__main__":
    lst, out, alpha = sys.argv[1], sys.argv[2], float(sys.argv[3])
    B = int(sys.argv[4]) if len(sys.argv) > 4 else 10000
    w = int(sys.argv[5]) if len(sys.argv) > 5 else 2
    files = [l.strip() for l in open(lst) if l.strip()]
    with Pool(w) as p:
        rows = p.map(one, [(f, alpha, B) for f in files], chunksize=1)
    open(out, "w").write("file\tn\tdetect\tsig_deciles\tdiff\tthreshold\n" + "\n".join(rows) + "\n")
