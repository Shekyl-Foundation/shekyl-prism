# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Per-set widths (bits and 64-bit limbs) and public operation counts of the fixed-modulus HNF."""
import csv
from sets import SETS, classes
from counts import formula, limb_mults

L = lambda b: -(-b // 64)


def rows():
    out = []
    for s in SETS:
        P, cl = classes(s[0])
        for cname, c in cl.items():
            K = c['K']
            F = 2 * K - 1
            r = dict(set=P['name'], cls=cname, Nbits=c['Nbits'], K=K, F=F)
            r['entry_limbs'] = L(K)                      # canonical entries, pivots, m
            r['red_in_limbs'] = L(2 * K)                 # |y| < 2^K m, R
            r['prod_sched_limbs'] = L(3 * K)             # Barrett product inside the schedule
            r['prod_entry_limbs'] = L(4 * K - 1)         # Barrett product when entries reach 2^(2K)
            r['W'] = P['W']
            r['LIMBS'] = P['LIMBS']
            for k in (4, 8):
                f = formula(K, k)
                r['reds_k%d' % k] = 4 * k + 6 * k * F + 4
                r['ops_k%d' % k] = sum(f.values())
                r['limbmul_k%d' % k] = limb_mults(K, k)
            out.append(r)
    return out


if __name__ == '__main__':
    R = rows()
    with open('results/widths_counts.csv', 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(R[0].keys()))
        w.writeheader()
        w.writerows(R)
    hdr = ('set', 'cls', 'K', 'entry_limbs', 'red_in_limbs', 'prod_sched_limbs', 'prod_entry_limbs', 'W', 'LIMBS',
           'reds_k4', 'ops_k4', 'limbmul_k4')
    print(' '.join('%12s' % h for h in hdr))
    for r in R:
        print(' '.join('%12s' % r[h] for h in hdr))
