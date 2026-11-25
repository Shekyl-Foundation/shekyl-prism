# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""SHE-24 adversarial search (own code).  Searches integer Hermitian Gram
matrices (A, C, h0 + i h1) -- a superset of every admitted class, since the
certificate is claimed for every Gram with A0^2/Delta <= R0 (and, for exp64,
C0^2/Delta <= R_C) -- for inputs whose fixed-point step need exceeds the
claimed count.

An input is a backward chain: a post-last-swap state plus a list of
Gaussian quotients prepended by the inverse step
    (A, C, h) -> (C, A + 2 Re(mu h) + |mu|^2 C, conj(h) + mu C),
extended until A0^2/Delta would exceed R0.  Optionally a 'head' quotient of
chosen size is applied last (first forward step), constrained by R_C.
The true need is measured by running the contract forward (reducer.py), so
the construction only has to produce good candidates.

usage: hillclimb.py SET CLASS SECONDS SEED
"""
import json
import random
import sys
import time
from fractions import Fraction as Fr

import mycounts as MC
import reducer as R

SMALL_MU = [(a, b) for a in range(-3, 4) for b in range(-3, 4) if a * a + b * b >= 2]
PREF_MU = [(1, 1), (1, -1), (-1, 1), (-1, -1)]


def build(start, mus, head, R0, RC):
    A, C, h0, h1 = start
    Delta = A * C - h0 * h0 - h1 * h1
    if Delta <= 0 or not (A < C):
        return None
    G = (A, C, h0, h1)
    used = 0
    for (m0, m1) in mus:
        A, C, h0, h1 = G
        # inverse step: previous u = v, previous v = u + mu v
        hc0, hc1 = h0, -h1                                  # conj(h)
        nA = C
        nh0 = hc0 + m0 * C
        nh1 = hc1 + m1 * C
        nC = A + 2 * (m0 * h0 - m1 * h1) + (m0 * m0 + m1 * m1) * C   # A + 2Re(mu h) + |mu|^2 C
        if nC <= nA:
            break
        if Fr(nA * nA, Delta) > R0:
            break
        G = (nA, nC, nh0, nh1)
        used += 1
    if head is not None:
        A, C, h0, h1 = G
        m0, m1 = head
        hc0, hc1 = h0, -h1
        nA, nh0, nh1 = C, hc0 + m0 * C, hc1 + m1 * C
        nC = A + 2 * (m0 * h0 - m1 * h1) + (m0 * m0 + m1 * m1) * C
        # the head enters as the FIRST forward step: swap roles so that the
        # big quotient is q_1 = h/A with A the shorter vector
        if nC > nA and Fr(nA * nA, Delta) <= R0 and Fr(nC * nC, Delta) <= RC:
            G = (nA, nC, nh0, nh1)
    A, C, h0, h1 = G
    if Fr(A * A, Delta) > R0 or Fr(max(A, C) ** 2, Delta) > RC:
        return None
    return G, used


def evaluate(G, T, T64):
    _, need, sw, _ = R.run(G, T + 50, 'exact')
    _, need64, _, big = R.run(G, T64 + 50, 'exp64')
    return need, need64, big


def random_start(rng):
    A = rng.getrandbits(48) | (1 << 47)
    C = A + rng.getrandbits(rng.choice([8, 30, 46, 47]))
    if rng.random() < 0.5:
        C = A + 1 + rng.getrandbits(46)
    lim = C // 2
    h0 = rng.choice([-lim, lim - 1, rng.randrange(-lim, lim)])
    h1 = rng.choice([-lim, lim - 1, rng.randrange(-lim, lim)])
    if A * C - h0 * h0 - h1 * h1 <= 0:
        h0, h1 = h0 // 2, h1 // 2
    return (A, C, h0, h1)


def mutate(rng, start, mus, head):
    start = list(start)
    mus = list(mus)
    c = rng.random()
    if c < 0.30 and mus:
        i = rng.randrange(min(len(mus), 40)) if rng.random() < 0.7 else rng.randrange(len(mus))
        mus[i] = rng.choice(PREF_MU if rng.random() < 0.7 else SMALL_MU)
    elif c < 0.40:
        i = rng.randrange(min(len(mus), 30) + 1)
        mus.insert(i, rng.choice(SMALL_MU))
    elif c < 0.50 and mus:
        del mus[rng.randrange(min(len(mus), 30))]
    elif c < 0.80:
        A, C, h0, h1 = start
        j = rng.randrange(4)
        start[j] += rng.choice([-1, 1]) * (1 << rng.randrange(0, 46))
        A, C, h0, h1 = start
        if not (0 < A < C) or A * C - h0 * h0 - h1 * h1 <= 0 or abs(h0) > C // 2 or abs(h1) > C // 2:
            return None
    elif c < 0.90:
        start = list(random_start(rng))
    else:
        if head is None or rng.random() < 0.3:
            head = None if rng.random() < 0.5 else (rng.getrandbits(rng.randrange(60, 400)), rng.getrandbits(rng.randrange(1, 400)))
        else:
            head = (head[0] + rng.randrange(-5, 6), head[1] + rng.randrange(-5, 6))
    return tuple(start), mus, head


def main():
    sset, cls, seconds, seed = sys.argv[1], sys.argv[2], float(sys.argv[3]), int(sys.argv[4])
    R0, RC = MC.classes(sset)[cls]
    G = MC.load_G(sys.argv[5])[0] if len(sys.argv) > 5 else None
    T, T64 = MC.CLAIMED[cls][0][MC.SETS.index(sset)], MC.CLAIMED[cls][1][MC.SETS.index(sset)]
    rng = random.Random(seed)
    t0 = time.time()
    # seeds: periodic Rolletschek-type chains with random phases
    pats = [[(1, 1), (1, -1)], [(1, 1)], [(1, -1)], [(1, 1), (-1, 1)], [(-1, -1), (1, -1)]]
    best = None
    evals = 0
    restarts = 0
    record = {'need': -1, 'need64': -1}
    while time.time() - t0 < seconds:
        pat = rng.choice(pats)
        cur = (random_start(rng), [pat[i % len(pat)] for i in range(3000)], None)
        b = build(cur[0], cur[1], cur[2], R0, RC)
        if b is None:
            continue
        n, n64, big = evaluate(b[0], T, T64)
        cur_score = (n if n is not None else 10**9, n64 if n64 is not None else 10**9)
        restarts += 1
        stall = 0
        while stall < 400 and time.time() - t0 < seconds:
            m = mutate(rng, *cur)
            if m is None:
                continue
            b = build(m[0], m[1], m[2], R0, RC)
            if b is None:
                continue
            n, n64, big = evaluate(b[0], T, T64)
            evals += 1
            n = 10**9 if n is None else n
            n64 = 10**9 if n64 is None else n64
            sc = (n, n64)
            if max(sc) >= max(cur_score) and sum(sc) >= sum(cur_score):
                if sc != cur_score:
                    stall = 0
                else:
                    stall += 1
                cur, cur_score = m, sc
            else:
                stall += 1
            if n > record['need'] or n64 > record['need64']:
                A, C, h0, h1 = b[0]
                Delta = A * C - h0 * h0 - h1 * h1
                record = {'need': max(n, record['need']), 'need64': max(n64, record['need64']),
                          'gram': [A, C, h0, h1], 'big_substeps': big,
                          'log2_A0sq_over_Delta': (A * A).bit_length() - Delta.bit_length()}
    out = dict(set=sset, cls=cls, seconds=seconds, seed=seed, evals=evals, restarts=restarts,
               log2_R0=R0.numerator.bit_length() - R0.denominator.bit_length(),
               claimed_exact=T, claimed_exp64=T64, best_need_exact=record['need'],
               best_need_exp64=record['need64'], margin_exact=T - record['need'],
               margin_exp64=T64 - record['need64'], witness=record.get('gram'))
    print(json.dumps(out))


if __name__ == '__main__':
    main()
