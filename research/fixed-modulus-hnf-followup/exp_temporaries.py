# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Follow-up item 3: the largest temporary before modular reduction, per set and per entry class.

Derived bounds (REPORT section 4) against maxima observed on instrumented runs of the merged model:
  entry     : input of the entry reduction  (decode: raw HNF columns / guarded pre-reduced pair; worst case: 2^(2K))
  sched-real: input of Red inside a pair step whose pivot pair is active (both pivots nonzero)
  sched-mask: the same on the dummy path of a step whose smaller pivot is already zero (computed, then discarded)
  canon     : input of Red in the canonical-form step (q * P_l[l'] subtracted)
  Barrett product a*R (tight / entry)
"""
import random, sys, math
from math import isqrt
import common
import fmhnf
import lib, gen_ideals
from fmhnf import fixed_modulus_hnf
from sets import classes, SETS

L64 = lambda b: -(-b // 64)
STATS = {}
FLAG = {'in_pair': False, 'masked': False}

_red, _pair = fmhnf.red, fmhnf.pair_step


def red_w(ctx, y, m, R, K, tight=True):
    if not tight:
        ph = 'entry'
    elif FLAG['in_pair']:
        ph = 'sched-mask' if FLAG['masked'] else 'sched-real'
    else:
        ph = 'canon'
    b = abs(y).bit_length()
    if b > STATS.get(ph, 0):
        STATS[ph] = b
    return _red(ctx, y, m, R, K, tight)


def pair_w(ctx, X, Y, c, m, R, K):
    FLAG['in_pair'] = True
    FLAG['masked'] = min(X[c], Y[c]) == 0
    try:
        return _pair(ctx, X, Y, c, m, R, K)
    finally:
        FLAG['in_pair'] = False


fmhnf.red, fmhnf.pair_step = red_w, pair_w


def J(v):
    return [-v[1], v[0], -v[3], v[2]]


def B_(u, v, p):
    return u[0] * v[0] + u[1] * v[1] + p * (u[2] * v[2] + u[3] * v[3])


def gm(g, w):
    return [g[0] * w[i] + g[1] * J(w)[i] for i in range(4)]


def rdiv(a, b):
    return (2 * a + b) // (2 * b)


def lagrange(u, v, p):
    while True:
        if B_(v, v, p) < B_(u, u, p):
            u, v = v, u
        A = B_(u, u, p)
        g = (rdiv(B_(u, v, p), A), rdiv(B_(J(u), v, p), A))
        w = [v[i] - gm(g, u)[i] for i in range(4)]
        if B_(w, w, p) >= B_(v, v, p):
            return u, w
        v = w


def guard_pair(u, v, p, N):
    """Shear v by the largest real multiple of u with N4(v) <= 1024 sqrt(p) N (the factor-256 guard)."""
    cap = 1024 * (isqrt(p) + 1) * N
    lo, hi = 0, 1 << 400
    best = v
    while lo < hi:
        mid = (lo + hi + 1) // 2
        w = [v[i] - mid * u[i] for i in range(4)]
        if B_(w, w, p) <= cap:
            lo = mid
            best = w
        else:
            hi = mid - 1
    return best


def bounds(name):
    P, cl = classes(name)
    p, B, K = P['p'], P['B'], cl['ideal']['K']
    gbits = isqrt(1024 * (isqrt(p) + 1) * ((1 << B) - 1)).bit_length()
    return dict(K=K, B=B, L=P['L'], raw=K, guard=gbits, worst=2 * K,
                sched_mask=2 * K, sched_real=2 * K - 1, canon=2 * K, prod_tight=3 * K, prod_entry=4 * K - 1)


def run_set(name, rng):
    P, cl = classes(name)
    p, B, K = P['p'], P['B'], cl['ideal']['K']
    gen_ideals.PRIME[0] = p
    top = 1 << B
    from sage.all import random_prime
    STATS.clear()
    per_class = {}
    cases = []
    for _ in range(3):
        l = int(random_prime(top - 1, lbound=top >> 1))
        cases.append((l, gen_ideals.root_mod_prime_power(rng, p, l, 1)))
    l = int(__import__('sage.all', fromlist=['ZZ']).ZZ(top).previous_prime())
    cases.append((l, gen_ideals.root_mod_prime_power(rng, p, l, 1)))
    classes_seen = {'raw': 0, 'guard': 0, 'worst': 0}
    for N, (x, y) in cases:
        x, y = gen_ideals.lift_parity(x, y, N)
        m = 2 * N
        U, V = [2 * N, 0, 0, 0], [x, y, 1, 0]
        ur, vr = lagrange(U, V, p)
        vg = guard_pair(ur, vr, p, N)
        assert B_(vg, vg, p) <= 1024 * (isqrt(p) + 1) * N
        gens = lib.left_ideal_gens([[2 * N, 0, 0, 0], [x, y, 1, 0]], p)
        cols = lib.sage_hnf_cols(gens)
        # decode class, raw columns
        for cls, (u, v) in (('raw', (U, V)), ('guard', (ur, vg))):
            G = [u, J(u), v, J(v)]
            STATS.pop('entry', None)
            Pm, ctx = fixed_modulus_hnf(G, m, K)
            assert Pm == cols
            classes_seen[cls] = max(classes_seen[cls], max(abs(t).bit_length() for r in G for t in r))
            per_class.setdefault(cls, {})['entry_in'] = max(per_class.get(cls, {}).get('entry_in', 0), STATS.get('entry', 0))
        # worst-case entries: the range gate's maximum
        bound = 1 << (2 * K)
        base = [list(U), J(U), list(V), J(V)]
        ext = []
        for r in base:
            row = []
            for t in r:
                s = rng.choice((1, -1))
                target = bound - 1
                row.append(t + m * ((target - t) // m) if s == 1 else (t + target) % m - target)
            ext.append(row)
        STATS.pop('entry', None)
        Pm, ctx = fixed_modulus_hnf(ext, m, K)
        assert Pm == cols
        classes_seen['worst'] = max(classes_seen['worst'], max(abs(t).bit_length() for r in ext for t in r))
        per_class.setdefault('worst', {})['entry_in'] = max(per_class.get('worst', {}).get('entry_in', 0), STATS.get('entry', 0))
        per_class['maxbits'] = {k: max(per_class.get('maxbits', {}).get(k, 0), v) for k, v in ctx.maxbits.items()}
    # worst-case entries with a tiny modulus: the Barrett product of the entry reduction is largest when m is small
    for N, g in ((1, [[2, 0, 0, 0]]), (2, [[2, 2, 0, 0]]), (3, [[6, 0, 0, 0]])):
        cols = lib.sage_hnf_cols(lib.left_ideal_gens(g, p))
        N = lib.ideal_norm_from_cols(cols)
        m = 2 * N
        base = [[cols[c][i] if i <= c else 0 for i in range(4)] for c in range(4)]
        bound = 1 << (2 * K)
        ext = [[t + m * ((bound - 1 - t) // m) for t in r] for r in base]
        assert all(abs(t) < bound for r in ext for t in r)
        Pm, ctx = fixed_modulus_hnf(ext, m, K)
        assert Pm == cols
        per_class['maxbits'] = {k: max(per_class['maxbits'].get(k, 0), v) for k, v in ctx.maxbits.items()}
    return dict(stats=dict(STATS), seen=classes_seen, per=per_class)


def main():
    rng = random.Random(9)
    rows = []
    print('%-5s %4s | %-30s | %-30s' % ('set', 'K', 'bound bits (limbs)', 'observed bits'))
    for s in SETS:
        name = s[0]
        b = bounds(name)
        r = run_set(name, rng)
        st = r['stats']
        print('%-5s %4d  B=%d L=%d' % (name, b['K'], b['B'], b['L']))
        for key, label in (('raw', 'entries, decode: raw HNF columns (<2N)'),
                           ('guard', 'entries, decode: guarded pre-reduced pair'),
                           ('worst', 'entries, worst case (range gate 2^(2K))'),
                           ('sched_real', 'Red input, schedule, active step'),
                           ('sched_mask', 'Red input, schedule, dummy (masked) step'),
                           ('canon', 'Red input, canonical-form step'),
                           ('prod_tight', 'Barrett product, schedule/decode'),
                           ('prod_entry', 'Barrett product, worst-case entries')):
            obs = {'raw': r['seen']['raw'], 'guard': r['seen']['guard'], 'worst': r['seen']['worst'],
                   'sched_real': st.get('sched-real', 0), 'sched_mask': st.get('sched-mask', 0),
                   'canon': st.get('canon', 0),
                   'prod_tight': r['per']['maxbits'].get('red_prod_t', 0),
                   'prod_entry': r['per']['maxbits'].get('red_prod_u', 0)}[key]
            assert obs <= b[key], (name, key, obs, b[key])
            print('   %-46s bound %5d b = %2d limbs | observed %5d b = %2d limbs' % (label, b[key], L64(b[key]), obs, L64(obs)))
            rows.append((name, key, b[key], L64(b[key]), obs, L64(obs)))
    import csv
    with open('results/temporaries.csv', 'w', newline='') as fh:
        w = csv.writer(fh)
        w.writerow(['set', 'quantity', 'bound_bits', 'bound_limbs', 'observed_bits', 'observed_limbs'])
        w.writerows(rows)


if __name__ == '__main__':
    main()
