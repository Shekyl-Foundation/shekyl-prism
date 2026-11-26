# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Hill-climb for the worst scalar pair (a, b) < 2^K at each set's K: observed steps vs the proved 2K-1."""
import random, sys
from test_lemmas import pair_scalar
from sets import SETS, classes

def search(K, iters, seed):
    rng = random.Random(seed)
    best = (0, None)
    for restart in range(8):
        a = rng.getrandbits(K) | 1
        b = rng.getrandbits(K) | 1
        cur = pair_scalar(a, b)[0]
        for _ in range(iters):
            na, nb = a, b
            for _ in range(rng.choice((1, 1, 2, 3))):
                bit = 1 << rng.randrange(K)
                if rng.random() < .5:
                    na ^= bit
                else:
                    nb ^= bit
            if not (0 < na < (1 << K) and 0 < nb < (1 << K)):
                continue
            s = pair_scalar(na, nb)[0]
            if s >= cur:
                a, b, cur = na, nb, s
        if cur > best[0]:
            best = (cur, (a, b))
    return best

if __name__ == '__main__':
    for s in SETS:
        K = classes(s[0])[1]['ideal']['K']
        steps, arg = search(K, 3000, K)
        print('%s K=%3d: worst observed active steps %4d (%.2f K) vs proved 2K-1 = %d' % (s[0], K, steps, steps / K, 2 * K - 1))
        sys.stdout.flush()
