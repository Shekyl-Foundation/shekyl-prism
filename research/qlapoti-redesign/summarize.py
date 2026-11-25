# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Reduce the raw sweep logs (data/<set>/sweep_*.jsonl, not committed:
regenerate with collect.py) to per-generator summaries
(results/gen_<set>.csv) and per-ideal draw counts (results/gd_<set>.csv).

Per (ideal S, generator g, exponent offset de) the row holds, over the
LAM swept lambdas: counts of each outcome code; the per-slot rates of a
fixed schedule under several lambda policies; two-stage sieve counts;
and the maxima of the inner loops (Gauss steps, Cornacchia Euclid steps,
M4 bits)."""
import csv
import glob
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SIEVE_K = (6, 8, 10, 12, 14, 16)


def primes_upto(B):
    s = bytearray([1]) * (B + 1)
    s[0:2] = b'\x00\x00'
    for i in range(2, int(B ** 0.5) + 1):
        if s[i]:
            s[i * i::i] = bytearray(len(s[i * i::i]))
    return [i for i in range(B + 1) if s[i]]


def skip_cap(codes, C):
    """Fixed schedule with a capped coprime-lambda search: each slot
    examines up to C+1 consecutive lambdas and uses the first coprime one;
    a slot with none is wasted. Returns (slots, successes, coprime_slots)."""
    i, slots, succ, used = 0, 0, 0, 0
    L = len(codes)
    while i < L:
        slots += 1
        j = i
        while j < L and j - i <= C and codes[j] == '0':
            j += 1
        if j < L and j - i <= C:
            used += 1
            succ += codes[j] == '6'
            i = j + 1
        else:
            i = i + C + 1
    return slots, succ, used


def main(names):
    out = HERE / 'results'
    out.mkdir(exist_ok=True)
    for name in names:
        files = sorted(glob.glob(str(HERE / 'data' / name / 'sweep_*.jsonl')))
        rows, gdrows = [], []
        for fn in files:
            for line in open(fn):
                r = json.loads(line)
                sid = f"{r['kid']}:{r['mid']}"
                gdrows.append(dict(sid=sid, u=r['u'], n_bits=r['n_bits'], v2=r['v2'],
                                   sf='.'.join(map(str, r['sf'])),
                                   gd=' '.join(map(str, r['gd']))))
                for gi, g in enumerate(r['gens']):
                    for de in ('2', '1', '0'):
                        c = g['c' + de]
                        L = len(c)
                        if L == 0:
                            continue
                        det = g['d' + de]
                        row = dict(sid=sid, u=round(r['u'], 5), n_bits=r['n_bits'],
                                   v2=r['v2'], g=gi, de=int(de), L=L,
                                   draws=g['draws'], gauss=g['gauss'],
                                   suitable=g['suitable'], r0bits=g['r0bits'],
                                   lat_short2=g['lat_short2'], lat_long2=g['lat_long2'],
                                   mb_min=g['mb' + de][0], mb_max=g['mb' + de][1],
                                   corn_max=max([d[2] for d in det], default=0))
                        for k in '0123456':
                            row['n' + k] = c.count(k)
                        for C in (0, 1, 2, 3, 4, 6, 8):
                            s, sc, u = skip_cap(c, C)
                            row[f'sk{C}_slots'] = s
                            row[f'sk{C}_succ'] = sc
                        pr = [q for q in primes_upto(L) if q >= 3]
                        row['pr_slots'] = len(pr)
                        row['pr_succ'] = sum(c[q - 1] == '6' for q in pr)
                        row['pr_cop'] = sum(c[q - 1] != '0' for q in pr)
                        odd = list(range(1, L + 1, 2))
                        row['odd_slots'] = len(odd)
                        row['odd_succ'] = sum(c[q - 1] == '6' for q in odd)
                        for K in SIEVE_K:
                            surv = [d for d in det if d[3] == 0 or d[3] >= K]
                            row[f'sv{K}'] = len(surv)
                        rows.append(row)
        with open(out / f'gen_{name}.csv', 'w', newline='') as fh:
            w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
            w.writeheader()
            w.writerows(rows)
        with open(out / f'gd_{name}.csv', 'w', newline='') as fh:
            w = csv.DictWriter(fh, fieldnames=list(gdrows[0].keys()))
            w.writeheader()
            w.writerows(gdrows)
        print(name, len(gdrows), 'ideals', len(rows), 'rows', flush=True)


if __name__ == '__main__':
    main(sys.argv[1:] or ['p248', 'p324', 'p376', 'p500', 'p664'])
