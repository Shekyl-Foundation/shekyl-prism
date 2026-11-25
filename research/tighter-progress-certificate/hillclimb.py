# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Adversarial local search for inputs that need more exact steps than the
new count.  A candidate is a Z[i] continuant chain (quotient list, the
exact reduction replays it in reverse) completed greedily with (1+-i)
quotients up to the class's size bound, then optionally a small
perturbation (dx, dy) of the basis.  Objective: exact steps needed
(including the final size reduction).  Hill climbing with restarts from
Rolletschek and random starts; mutations change, insert or delete one
quotient, or perturb (dx, dy).  Writes results/hillclimb.txt."""
import csv, math, os, random, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, 'vendor'))
from q5common import params
import gen_inputs as G
from adversarial import make_chain as _make_chain


def make_chain(mus):
    try:
        return _make_chain(mus)
    except ZeroDivisionError:      # degenerate continuant (w = 0)
        return 0, 0, 0
import ngram as NG

MUS = [(1, 1), (1, -1), (-1, 1), (-1, -1), (2, 0), (0, 2), (2, 1), (1, 2), (2, -1), (1, -2), (2, 2), (3, 0), (2, -2), (3, 1)]


def complete(prefix, lim, rng, alt=True):
    mus = list(prefix); k = len(mus)
    while True:
        mu = ((1, 1) if k % 2 == 0 else (1, -1)) if alt else rng.choice(MUS[:4])
        T, g, h = make_chain(mus + [mu])
        if T >= lim:
            return mus
        if T == 0:
            mu = (1, 1)
        mus.append(mu); k += 1


def score(mus, d, lim, p):
    T, g, h = make_chain(mus)
    if T >= lim or T < 2:
        return -1, None
    u, v = G.model_basis(T, (g + d[0]) % T, (h + d[1]) % T)
    return NG.exact_steps_needed(u, v, p), (T, g, h)


def run(name, cls, count, lim, seconds, seed):
    P = params(name); p = P['p']; rng = random.Random(seed)
    best_overall = (0, None)
    t0 = time.time(); evals = 0
    while time.time() - t0 < seconds:
        # restart: a Rolletschek start or a random short prefix, completed
        pre = [rng.choice(MUS) for _ in range(rng.randrange(0, 6))]
        mus = complete(pre, lim, rng, alt=rng.random() < 0.7)
        d = (0, 0)
        cur, _ = score(mus, d, lim, p); evals += 1
        stall = 0
        while stall < 60 and time.time() - t0 < seconds:
            m2 = list(mus); d2 = d
            r = rng.random()
            pos = rng.randrange(len(m2)) if m2 else 0
            if r < 0.3 and m2:
                m2[pos] = rng.choice(MUS)
            elif r < 0.45:
                m2.insert(pos, rng.choice(MUS))
            elif r < 0.6 and m2:
                del m2[pos]
            elif r < 0.8 and m2:
                # change the first quotients (the end of the reduction: the tail)
                k = rng.randrange(1, min(6, len(m2)) + 1)
                m2[:k] = [rng.choice(MUS) for _ in range(k)]
            else:
                d2 = (rng.randrange(-3, 4), rng.randrange(-3, 4))
            # trim or complete to the bound
            while m2 and make_chain(m2)[0] >= lim:
                m2.pop()
            m2 = complete(m2, lim, rng)
            s, _ = score(m2, d2, lim, p); evals += 1
            if s >= cur:
                stall = 0 if s > cur else stall + 1
                mus, d, cur = m2, d2, s
            else:
                stall += 1
        if cur > best_overall[0]:
            best_overall = (cur, (mus[:8], d, len(mus)))
    return best_overall, evals


if __name__ == '__main__':
    name, cls, count, bits, secs = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5])
    seed = int(sys.argv[6]) if len(sys.argv) > 6 else 1
    (best, info), ev = run(name, cls, count, 1 << bits, secs, seed)
    T, g, h, k = G.make_rolletschek(1 << bits)
    roll = NG.exact_steps_needed(*G.model_basis(T, g, h), params(name)['p'])
    print(f'{name} {cls}: {ev} evaluations, best need {best} (Rolletschek {roll}), count {count}, '
          f'margin {count - best}, best starts {info}', flush=True)
    assert best <= count
