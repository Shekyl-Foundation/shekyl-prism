# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""What-if for ledger O29: per-pass success of the norm equation against the
power of two it targets (e-2 as in the reference, e-1, e), and the share of
passes whose M4 is positive. Same simulated inputs per exponent. Whether
the dimension-two chain can use e-1 or e is NOT addressed here (that is
O29's open question); this only measures what the search would gain."""
import csv
import multiprocessing as mp
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))


def job(args):
    from fne import (SET_MAP, prime_of, sec_degree, tape, random_prime_ideal,
                     prime_norm_equivalent, intersect_coprime, prepare_small,
                     qlapoti, Ctr, gmpy2)
    name, n = args
    out = HERE / 'data' / name / 'diag_exponent.csv'
    if out.exists():
        return str(out), 'skip'
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    SEC = sec_degree(name)
    rows = []
    for i in range(n):
        kid = 6 * 10 ** 6 + i
        rng = tape(b'diag', name, kid)
        Is, _ = prime_norm_equivalent(random_prime_ideal(SEC, p, rng), p, rng)
        while True:
            s = rng.getrandbits(e - 4) | 1
            q = (1 << (2 * e - 5)) - s * s
            if gmpy2.is_prime(q):
                break
        S = prepare_small(intersect_coprime(Is, random_prime_ideal(q, p, rng)), p)
        for e2 in (e - 2, e - 1, e):
            for pol in ('ref', 'fresh'):
                c = Ctr()
                kw = {} if pol == 'ref' else dict(policy='fresh')
                qlapoti(S, p, e2, tape(b'diagt', name, kid, e2, pol), c, **kw)
                rows.append(dict(set=name, i=i, e2_minus_e=e2 - e, policy=pol,
                                 passes=c.passes, success=int(c.success_pass > 0),
                                 m4pos=c.m4pos, cvp=c.cvp, mod_ok=c.mod_ok,
                                 n_bits=int(S.norm).bit_length()))
    with open(out, 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    return str(out), len(rows)


if __name__ == '__main__':
    n = {'p248': 300, 'p324': 300, 'p376': 150, 'p500': 150, 'p664': 100}
    with mp.get_context('spawn').Pool(2) as pool:
        for r in pool.imap_unordered(job, list(n.items())):
            print(time.strftime('%H:%M:%S'), r, flush=True)
    print('DONE', flush=True)
