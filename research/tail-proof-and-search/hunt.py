# Copyright (c) 2025-2026, The Shekyl Foundation.  All rights reserved.  BSD-3-Clause
"""Adversarial search at P500 / P664 (SHE-24 follow-up).

Builds on ../certificate-review/hillclimb.py (backward Gaussian-quotient chains in
Gram space, a superset of every admitted class) and adds three strategies:

  * near-tie inputs: an intermediate state G_j of the forward run is snapped so
    that a quotient component is an exact half (h = (k + 1/2) A) or one unit off
    it, then mapped back to G_0 through the recorded unimodular steps;
  * boundary lambda: G_j is perturbed so that lambda_j = Delta/A_j^2 sits at
    2^-11, 2^-9, 1/2 or just below 1 (C_j re-solved), then mapped back;
  * a polish phase: hill climbing directly on G_0 with those intermediate-state
    moves, starting from the best chain-built input.

Every candidate is re-checked against the class bounds (A0^2/Delta <= R0,
max(A0,C0)^2/Delta <= R_C) and measured by running the contract forward
(exact half-up rule and exp64; need = slots to the fixed point, final size
reduction included).

usage: hunt.py SET CLASS SECONDS SEED
"""
import json
import random
import sys
import time
from fractions import Fraction as Fr
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'certificate-review'))
import hillclimb as HC   # noqa: E402
import mycounts as MC    # noqa: E402
import reducer as R      # noqa: E402

ADOPTED = {   # Rick, t43u: 546 / 533 at P500 and the fully self-proven table elsewhere
    'sign-generic': ((280, 360, 415, 546, 719),) * 2,
    'prered-guard256': ((267, 347, 402, 533, 706),) * 2,
    'keygen-D': ((208, 323, 308, 411, 638),) * 2,
    'keygen-Isec': ((22, 21, 22, 21, 22), (22, 22, 23, 22, 24)),
    'challenge': ((197, 257, 297, 396, 526),) * 2,
}


def forward_trace(G, slots):
    """Exact-rule forward run; returns list of (G_before, m, swapped)."""
    G = R.gram_order(G)
    tr = []
    for _ in range(slots):
        A, C, h0, h1 = G
        m0, m1 = R.exact_m(h0, h1, A)
        if m0 == 0 and m1 == 0 and C >= A:
            break
        G2, sw = R.gram_apply(G, m0, m1)
        tr.append((G, (m0, m1), sw))
        G = G2
    return tr, G


def back_one(G2, m, sw):
    """Inverse of reducer.gram_apply for a known quotient and swap flag."""
    m0, m1 = m
    if sw:
        C2, A, g0, ng1 = G2
        g1 = -ng1
    else:
        A, C2, g0, g1 = G2
    h0, h1 = g0 + m0 * A, g1 + m1 * A
    C = C2 + 2 * (m0 * h0 + m1 * h1) - (m0 * m0 + m1 * m1) * A
    return (A, C, h0, h1)


def map_back(Gj, steps):
    G = Gj
    for (_, m, sw) in reversed(steps):
        G = back_one(G, m, sw)
    return G


def admissible(G, R0, RC):
    A, C, h0, h1 = G
    Delta = A * C - h0 * h0 - h1 * h1
    if A <= 0 or C <= 0 or Delta <= 0:
        return False
    A, C = min(A, C), max(A, C)
    return Fr(A * A, Delta) <= R0 and Fr(C * C, Delta) <= RC


def build2(start, mus, head, R0, RC):
    """Like hillclimb.build, but the chain is TRUNCATED at the longest prefix that
    satisfies BOTH class bounds (A^2/Delta <= R0 and C^2/Delta <= R_C), instead of
    rejecting the whole candidate when the C bound fails at the end.  This matters
    for the guarded class, where R_C is close to R0."""
    A, C, h0, h1 = start
    Delta = A * C - h0 * h0 - h1 * h1
    if Delta <= 0 or not (A < C):
        return None
    G = (A, C, h0, h1)
    best = G if admissible(G, R0, RC) else None
    for (m0, m1) in mus:
        A, C, h0, h1 = G
        nA, nh0, nh1 = C, h0 + m0 * C, -h1 + m1 * C
        nC = A + 2 * (m0 * h0 - m1 * h1) + (m0 * m0 + m1 * m1) * C
        if nC <= nA or Fr(nA * nA, Delta) > R0:
            break
        G = (nA, nC, nh0, nh1)
        if admissible(G, R0, RC):
            best = G
    if head is not None and best is not None:
        A, C, h0, h1 = best
        m0, m1 = head
        nA, nh0, nh1 = C, h0 + m0 * C, -h1 + m1 * C
        nC = A + 2 * (m0 * h0 - m1 * h1) + (m0 * m0 + m1 * m1) * C
        if nC > nA and admissible((nA, nC, nh0, nh1), R0, RC):
            best = (nA, nC, nh0, nh1)
    return None if best is None else (best, 0)


TARGETS = [Fr(1, 2048), Fr(1, 512), Fr(1, 2), Fr(1023, 1024), Fr(1, 65536)]


def intermediate_move(rng, G0, R0, RC):
    tr, _ = forward_trace(G0, 4000)
    if len(tr) < 2:
        return None
    # bias toward the tail (last ~12 steps) and the boundary region
    j = rng.randrange(max(0, len(tr) - 14), len(tr)) if rng.random() < 0.6 else rng.randrange(len(tr))
    A, C, h0, h1 = tr[j][0]
    if A % 2:   # scale so exact halves exist
        A, C, h0, h1 = 2 * A, 2 * C, 2 * h0, 2 * h1
    kind = rng.random()
    if kind < 0.45:          # near-tie snap of one component
        comp = rng.randrange(2)
        h = (h0, h1)[comp]
        k = (2 * h + A) // (2 * A)
        sgn = rng.choice([-1, 1])
        new = k * A + sgn * (A // 2) + rng.choice([0, 0, 0, -1, 1])
        if comp == 0:
            h0 = new
        else:
            h1 = new
        if rng.random() < 0.3:   # double tie
            other = (h1, h0)[comp]
            k2 = (2 * other + A) // (2 * A)
            o2 = k2 * A + rng.choice([-1, 1]) * (A // 2)
            if comp == 0:
                h1 = o2
            else:
                h0 = o2
        need_c = (h0 * h0 + h1 * h1) // A + 1
        if C < need_c:
            C = need_c
    elif kind < 0.8:         # boundary lambda: Delta = t A^2
        t = rng.choice(TARGETS)
        C = (t * A * A + h0 * h0 + h1 * h1) / A
        C = int(C) + rng.choice([0, 1, -1])
    else:                     # random small perturbation of the intermediate state
        b = rng.randrange(0, max(1, A.bit_length() - 2))
        d = rng.choice([-1, 1]) << rng.randrange(0, b + 1)
        c = rng.randrange(3)
        if c == 0:
            h0 += d
        elif c == 1:
            h1 += d
        else:
            C += d
    Gj = (A, C, h0, h1)
    if A * C - h0 * h0 - h1 * h1 <= 0:
        return None
    steps = tr[:j]
    if tr[j][0][0] % 2:      # we scaled: earlier steps are unchanged under scaling
        pass
    G = map_back(Gj, steps)
    return G if admissible(G, R0, RC) else None


def measure(G, T, T64):
    n, n64, big = HC.evaluate(G, T, T64)
    return (10**9 if n is None else n), (10**9 if n64 is None else n64), big


def main():
    sset, cls, seconds, seed = sys.argv[1], sys.argv[2], float(sys.argv[3]), int(sys.argv[4])
    R0, RC = MC.classes(sset)[cls]
    i = MC.SETS.index(sset)
    T, T64 = MC.CLAIMED[cls][0][i], MC.CLAIMED[cls][1][i]
    rng = random.Random(seed)
    t0 = time.time()
    pats = [[(1, 1), (1, -1)], [(1, 1)], [(1, -1)], [(1, 1), (-1, 1)], [(-1, -1), (1, -1)]]
    rec = {'need': -1, 'need64': -1, 'gram': None, 'strategy': None}
    evals = {'chain': 0, 'polish': 0}
    pool = []   # best Grams for the polish phase

    def note(G, n, n64, big, strat):
        if n > rec['need'] or n64 > rec['need64'] or (n == rec['need'] and n64 == rec['need64'] and rec['gram'] is None):
            A, C, h0, h1 = G
            Dl = A * C - h0 * h0 - h1 * h1
            rec.update(need=max(n, rec['need']), need64=max(n64, rec['need64']), gram=[A, C, h0, h1],
                       big=big, strategy=strat,
                       log2_A0sq_over_Delta=(min(A, C) ** 2).bit_length() - Dl.bit_length())

    chain_budget = seconds * 0.55
    # ---- phase 1: chain hill climbing (hillclimb.py moves) plus tie-start seeds
    while time.time() - t0 < chain_budget:
        pat = rng.choice(pats)
        start = HC.random_start(rng)
        if rng.random() < 0.4:   # tie start: residual exactly -1/2 or +1/2 - 1/C
            A, C, h0, h1 = start
            C += C % 2
            h0 = rng.choice([-C // 2, C // 2 - 1, h0])
            h1 = rng.choice([-C // 2, C // 2 - 1, h1])
            if A * C - h0 * h0 - h1 * h1 > 0 and A < C:
                start = (A, C, h0, h1)
        cur = (start, [pat[k % len(pat)] for k in range(3000)], None)
        b = build2(cur[0], cur[1], cur[2], R0, RC)
        if b is None:
            continue
        n, n64, big = measure(b[0], T, T64)
        cs = (n, n64)
        stall = 0
        while stall < 300 and time.time() - t0 < chain_budget:
            m = HC.mutate(rng, *cur)
            if m is None:
                continue
            b = build2(m[0], m[1], m[2], R0, RC)
            if b is None:
                continue
            n, n64, big = measure(b[0], T, T64)
            evals['chain'] += 1
            sc = (n, n64)
            if max(sc) >= max(cs) and sum(sc) >= sum(cs):
                stall = 0 if sc != cs else stall + 1
                cur, cs = m, sc
            else:
                stall += 1
            note(b[0], n, n64, big, 'chain')
        if b is not None:
            pool.append((cs, cur, b[0]))
            pool.sort(key=lambda x: (-max(x[0]), -sum(x[0])))
            del pool[8:]
    # ---- phase 2: polish with intermediate-state moves (near ties, boundary lambda)
    while time.time() - t0 < seconds and pool:
        cs, _, G = pool[rng.randrange(len(pool))]
        stall = 0
        while stall < 400 and time.time() - t0 < seconds:
            G2 = intermediate_move(rng, G, R0, RC)
            if G2 is None:
                stall += 1
                continue
            n, n64, big = measure(G2, T, T64)
            evals['polish'] += 1
            sc = (n, n64)
            if max(sc) >= max(cs) and sum(sc) >= sum(cs):
                stall = 0 if sc != cs else stall + 1
                G, cs = G2, sc
            else:
                stall += 1
            note(G2, n, n64, big, 'polish')
    aE, a64 = ADOPTED[cls][0][i], ADOPTED[cls][1][i]
    out = dict(set=sset, cls=cls, seconds=seconds, seed=seed, evals=evals,
               claimed_exact=T, claimed_exp64=T64, adopted_exact=aE, adopted_exp64=a64,
               best_need_exact=rec['need'], best_need_exp64=rec['need64'],
               margin_vs_claimed=(T - rec['need'], T64 - rec['need64']),
               margin_vs_adopted=(aE - rec['need'], a64 - rec['need64']),
               witness=rec['gram'], witness_strategy=rec['strategy'], big_substeps=rec.get('big'))
    print(json.dumps(out), flush=True)


if __name__ == '__main__':
    main()
