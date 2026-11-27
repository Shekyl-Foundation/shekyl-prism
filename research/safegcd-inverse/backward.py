#!/usr/bin/env python3
"""Backward beam search for inputs needing many divsteps (delta0 = 1/2),
after EUROCRYPT 2026 sec. 4: start from terminal states (delta, 1, 0) and
invert divstep, keeping the `width` smallest states by max(|f|, |g|)
(a cruder smallness measure than the paper's stable norm).
At each depth n, record the smallest f among states that are valid
starting points: delta = 1/2, f odd and 0 <= g <= f. Variable time;
an instrument for the bound table, not the routine under test.
Usage: backward.py MAXSTEPS WIDTH
Output: one line per target b: b, deepest n found with f < 2^b, f, g (hex).
"""
import sys
from fractions import Fraction as Fr

def preds(d2, f, g):
    # d2 = 2*delta (odd). Predecessors of (delta', f', g') = (d2/2, f, g).
    out = []
    # swap: delta = 1 - delta' > 0, g_prev = f (odd), f_prev = f - 2g
    pd2 = 2 - d2
    if pd2 > 0:
        out.append((pd2, f - 2 * g, f))
    # no swap: delta = delta' - 1, f_prev = f
    pd2 = d2 - 2
    out.append((pd2, f, 2 * g))          # g_prev even
    if pd2 < 0:                           # g_prev odd requires delta <= 0
        out.append((pd2, f, 2 * g - f))
    return out

DMAX = 61
TARGETS = [128, 158, 165, 194, 195, 224, 255, 256, 285, 337, 367, 491, 643, 747, 995, 1323]

def main():
    maxn = int(sys.argv[1]); width = int(sys.argv[2])
    beam = {(d2, s, 0) for d2 in range(-41, 43, 2) for s in (1, -1)}
    best_bits = {}
    deepest = {}
    for n in range(1, maxn + 1):
        nxt = set()
        for st in beam:
            for p in preds(*st):
                if p[2] != 0:   # g = 0 earlier would have stopped sooner
                    nxt.add(p)
        # keep the smallest
        # keep the smallest per value of delta, so the beam does not drift
        # to states that can never start from delta = 1/2
        buckets = {}
        for s in nxt:
            buckets.setdefault(s[0], []).append(s)
        beam = set()
        for d2, sts in buckets.items():
            if abs(d2) > DMAX:
                continue
            sts.sort(key=lambda s: max(abs(s[1]), abs(s[2])))
            beam.update(sts[:width])
        cands = [f for (d2, f, g) in beam if d2 == 1 and f > 0 and f & 1 and 0 <= g <= f]
        if cands:
            fbest = min(cands)
            gbest = next(g for (d2, f, g) in beam if d2 == 1 and f == fbest and 0 <= g <= f)
            best_bits[n] = fbest.bit_length()
            for b in TARGETS:
                if fbest.bit_length() <= b:
                    deepest[b] = (n, fbest, gbest)
        if n % 100 == 0:
            print(f"# n={n} beam={len(beam)} best_bits={best_bits.get(n)}", file=sys.stderr, flush=True)
    # For each b: the largest n found with an input of at most b bits.
    for b in TARGETS:
        if b in deepest:
            n, f, g = deepest[b]
            print(f"{b} {n} {f:x} {g:x}")

if __name__ == "__main__":
    main()
