# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Measured op count == closed form; trace hash independent of the input; width bounds hold."""
import random, sys
from fmhnf import fixed_modulus_hnf, Ctx
from counts import formula, limb_mults
import lib


def run(K, k, trials, seed):
    rng = random.Random(seed)
    hashes = set()
    for t in range(trials):
        mode = t % 4
        m = rng.randrange(2, 1 << K) if mode else (1 << K) - 1
        if mode == 2:
            m = 2 + rng.randrange(0, 4)                    # tiny modulus, huge K
        bound = 1 << (2 * K)
        G = [[rng.randrange(-bound + 1, bound) for _ in range(4)] for _ in range(k)]
        if mode == 3:
            G = [[0] * 4 for _ in range(k)]                # all-zero generators
        P, ctx = fixed_modulus_hnf(G, m, K)
        f = formula(K, k)
        got = dict(ctx.count)
        for c in f:
            assert got.get(c, 0) == f[c], (K, k, c, got.get(c), f[c])
        assert set(got) == set(f), (set(got) ^ set(f))
        assert ctx.limbmul == limb_mults(K, k), (ctx.limbmul, limb_mults(K, k))
        hashes.add(ctx.trace_hash())
        if t == 0:
            first = ctx
    assert len(hashes) == 1, 'trace depends on the input'
    return first, hashes.pop()


if __name__ == '__main__':
    for (K, k, tr) in [(3, 4, 40), (5, 4, 40), (8, 4, 40), (17, 4, 40), (33, 8, 20), (64, 4, 16), (159, 4, 8), (159, 8, 4)]:
        ctx, h = run(K, k, tr, K * 1000 + k)
        print('K=%4d k=%d trials=%3d  ops=%9d  limbmul=%10d  trace=%s  formula==measured, trace identical' %
              (K, k, tr, ctx.nops, ctx.limbmul, h[:12]))
        sys.stdout.flush()
