# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Adversarial and honest runs of every quotient rule.

For each input: the fixed schedule of its class with each rule; whether the
basis is reduced at the end (exact quotient of v on u is 0 and N4(u) <=
N4(v)); the first step after which it is reduced (the public count that
input actually needs, so the margin to the certified count is visible);
flags (clamps, big sub-steps); and growth (max N4 over the run vs M0).
Rules: exact (T_exact), clamp64 and clamp128 (T_exact + the exact cleanup,
as the prototype runs), corr64 (T_exact, flag = sticky overflow), exp64
(T_exp64 of the class; any-basis E for bases that are not ideals).
Also: the tail of honest quotient sizes, for corr64's abort rate.
Writes results/adversarial.csv, results/adversarial_summary.txt,
results/quotient_tail.csv.
"""
import csv
import json
import math
import os
import random
import sys
from collections import defaultdict

import gmpy2

from q5common import SETS, params, N4, herm
import gen_inputs as G
from trace_counterexample import crafted, crafted_k1
from variants import RULES, exact_round, bl

HERE = os.path.dirname(os.path.abspath(__file__))


def load_counts():
    c = {}
    with open(os.path.join(HERE, 'results', 'step_counts.csv')) as f:
        for r in csv.DictReader(f):
            c[(r['set'], r['cls'])] = (int(r['T_exact']), int(r['T_exp64']), int(r['T_exp64_any_basis']))
    return c


def reduced(u, v, nu, nv, p):
    h0, h1 = herm(v, u, p)
    return exact_round(h0, nu) == 0 and exact_round(h1, nu) == 0 and nu <= nv


def run_rule(u0, v0, p, steps, rule, cleanup):
    qf = RULES[rule]
    u = list(u0); v = list(v0)
    nu, nv = N4(u, p), N4(v, p)
    M0 = max(nu, nv); Mmax = M0
    if nv < nu:
        u, v, nu, nv = v, u, nv, nu
    first = 0 if reduced(u, v, nu, nv, p) else None
    flags = 0
    for t in range(steps):
        h0, h1 = herm(v, u, p)
        m0, m1, fl = qf(h0, h1, nu)
        flags += fl
        if m0 or m1:
            iu = [-u[1], u[0], -u[3], u[2]]
            v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
            nv = N4(v, p)
            Mmax = max(Mmax, nv)
            if nv < nu:
                u, v, nu, nv = v, u, nv, nu
        red = reduced(u, v, nu, nv, p)
        if red and first is None:
            first = t + 1
        if not red:
            first = None   # must stay reduced to count
    if cleanup:
        h0, h1 = herm(v, u, p)
        m0, m1 = exact_round(h0, nu), exact_round(h1, nu)
        iu = [-u[1], u[0], -u[3], u[2]]
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        nv = N4(v, p)
        if nv < nu:
            u, v, nu, nv = v, u, nv, nu
    return dict(reduced=reduced(u, v, nu, nv, p), first=first, flags=flags, grew=Mmax > M0)


def make_chain(mus):
    """Z[i] continuant w_{k+1} = mu_k w_k + w_{k-1} (w_-1 = 0, w_0 = 1); the
    basis (T, g, h) as make_rolletschek, whose exact reduction replays the
    quotients in reverse (the last mu first)."""
    wp0, wp1, wc0, wc1 = 0, 0, 1, 0
    for (m0, m1) in mus:
        wn0 = (m0 * wc0 - m1 * wc1) + wp0
        wn1 = (m0 * wc1 + m1 * wc0) + wp1
        wp0, wp1, wc0, wc1 = wc0, wc1, wn0, wn1
    T = wc0 * wc0 + wc1 * wc1
    g = (wp0 * wc0 + wp1 * wc1) % T
    h = (wp1 * wc0 - wp0 * wc1) % T
    return T, g, h


def chains(P, rng):
    """Bases with big quotients, sized to T < 2^Tbits."""
    out = []
    lim = 1 << P['Tbits']
    # (a) all big: random mu of 61..200 bits until the size
    for trial in range(3):
        mus = []
        while True:
            b = rng.randrange(61, 201)
            mu = (rng.randrange(1 << (b - 1), 1 << b), rng.randrange(-(1 << (b - 1)), 1 << (b - 1)))
            T, g, h = make_chain(mus + [mu])
            if T >= lim:
                break
            mus.append(mu)
        out.append(('chain_allbig', *make_chain(mus)))
    # (b) Rolletschek (1+-i) body, then one big quotient taking the rest of the size
    for frac in (0.25, 0.5, 0.75):
        mus = []; k = 0
        while True:
            mu = (1, 1) if k % 2 == 0 else (1, -1)
            T, g, h = make_chain(mus + [mu])
            if T.bit_length() > frac * P['Tbits']:
                break
            mus.append(mu); k += 1
        T0, _, _ = make_chain(mus)
        room = (P['Tbits'] - T0.bit_length()) // 2 - 2
        if room > 61:
            mus.append((1 << room, 1))
        out.append((f'chain_roll{int(frac*100)}_big', *make_chain(mus)))
    # (c) alternating: Rolletschek pairs and a 62-bit quotient (just over the window)
    mus = []; k = 0
    while True:
        mu = ((1 << 62) + 1, 1) if k % 3 == 2 else ((1, 1) if k % 2 == 0 else (1, -1))
        T, g, h = make_chain(mus + [mu])
        if T >= lim:
            break
        mus.append(mu); k += 1
    out.append(('chain_roll_62', *make_chain(mus)))
    return out


def run_set(name, scale, counts, rows, tail):
    P = params(name); p, e, B = P['p'], P['e'], P['B']
    rng = random.Random(hash(name) & 0xffffffff ^ 0xC0FFEE)
    qpool = [G.sample_q(e, p, rng) for _ in range(4)] + [G.sample_q(e, p, rng, smallest=True)]
    qroots = [(q, G.root_pair(q, p, rng)) for q, _ in qpool]

    def signing_ideal(at_bound):
        q, (xq, yq) = qroots[rng.randrange(len(qroots))]
        N = G.sample_N(p, B, rng, at_bound=at_bound, avoid=(q,))
        xN, yN = G.root_pair(N, p, rng)
        x, y = G.crt_pair(N, xN, yN, q, xq, yq)
        return N * q, x, y

    def go(fam, cls, u0, v0, ideal=True, rules=('exact', 'clamp64', 'clamp128', 'corr64', 'exp64')):
        Tex, Texp, Tany = counts[(name, cls)]
        st = {}
        for rule in rules:
            if rule == 'exp64':
                r = run_rule(u0, v0, p, Texp if ideal else Tany, rule, False)
                r['T'] = Texp if ideal else Tany
            elif rule in ('clamp64', 'clamp128'):
                r = run_rule(u0, v0, p, Tex, rule, True); r['T'] = Tex + 1
            else:
                r = run_rule(u0, v0, p, Tex, rule, False); r['T'] = Tex
            st[rule] = r
            rows.append(dict(set=name, family=fam, cls=cls, rule=rule, T=r['T'], reduced=int(r['reduced']),
                             first_reduced=r['first'] if r['first'] is not None else '', flags=r['flags'], grew=int(r['grew'])))
        return st

    # honest signing ideals (model and O0); record exact quotient sizes
    for i in range(30 * scale):
        T, x, y = signing_ideal(i % 2 == 0)
        u0, v0 = G.model_basis(T, x, y) if i % 3 else G.o0_basis(T, x, y)
        go('honest_sign' + ('' if i % 3 else '_O0'), 'sign-generic', u0, v0)
    for i in range(100 * scale):
        T, x, y = signing_ideal(i % 2 == 0)
        _, es, _ = G_exact_es(*G.model_basis(T, x, y), p)
        for q2 in es:
            tail[name].append(q2)
    # crafted (Q5) and k = 1
    for sd in range(3):
        T, k, x, y = crafted(P, seed=77 + sd)
        go('crafted_q5', 'sign-generic', *G.model_basis(T, x, y))
        T, k, x, y = crafted_k1(P, seed=78 + 10 * sd)
        go('crafted_k1', 'sign-generic', *G.model_basis(T, x, y))
        go('crafted_k1_O0', 'sign-generic', *G.o0_basis(T, x, y))
    # Rolletschek chains at the size bound (not ideals)
    for mode in ('conj_alt', 'i_rot'):
        T, g, h, k = G.make_rolletschek(1 << P['Tbits'], mode=mode)
        go('rolletschek_' + mode, 'sign-generic', *G.model_basis(T, g, h), ideal=False)
    for fam, T, g, h in chains(P, rng):
        go(fam, 'sign-generic', *G.model_basis(T, g, h), ideal=False)
    # tail stress and ping-pong (from q5-width-proof)
    L = P['L']
    Tt = (1 << (P['Tbits'] - 1)) + 1234567
    K = max(1, math.isqrt(max(1, Tt // max(1, int(30.386 * math.isqrt(p))))))
    Ut = Tt // K
    go('tail_stress', 'sign-generic', [Ut, 0, 0, 0], [-int(0.386 * Ut), -int(0.381 * Ut), K, 0], ideal=False)
    U = 1 << (L // 2 - 2)
    go('pingpong', 'sign-generic', [U, 0, 0, 0], [(U >> 1) + 1, 0, 1, 0], ideal=False)
    # challenge alone, keygen ideal of norm D, keygen pre-reduction of I_sec
    for i in range(4 * scale):
        q, (xq, yq) = qroots[i % len(qroots)]
        go('challenge', 'challenge', *G.model_basis(q, xq, yq))
    D = int(gmpy2.next_prime(1 << (2 * P['W'])))
    for i in range(3 * scale):
        xD, yD = G.root_pair(D, p, rng)
        go('keygen_D', 'keygen-D', *G.model_basis(D, xD, yD))
    keys = []
    for i in range(4 * scale):
        N = G.sample_N(p, B, rng, at_bound=(i % 2 == 0))
        xN, yN = G.root_pair(N, p, rng)
        go('keygen_Isec', 'keygen-Isec', *G.model_basis(N, xN, yN))
        us, vs, nus, nvs, _ = G.exact_lagrange(*G.model_basis(N, xN, yN), p)
        keys.append(('sampled', N, us, vs, nvs))
    sp_ = math.isqrt(p)
    N = G.sample_N(p, B, rng, at_bound=True)
    for kind, kt in (('crafted_skew_max', 1), ('crafted_edge_pass', sp_ * 55 // 100)):
        x, y, k = G.crafted_skew_key(N, p, kt)
        us, vs, nus, nvs, _ = G.exact_lagrange(*G.model_basis(N, x, y), p)
        keys.append((kind, N, us, vs, nvs))
    for kind, N, us, vs, nvs in keys:
        guard = nvs * nvs <= 4 * p * N * N
        q, (xq, yq) = qroots[rng.randrange(len(qroots))]
        w1, w2, _ = G.inter_prereduced_zi(us, vs, xq, yq, q)
        go('prered_' + kind, 'prered-guard2' if guard else 'prered-unguarded', w1, w2)


def G_exact_es(u0, v0, p):
    """Exact early-exit run; returns the list of log2|q|^2 (|q| Euclidean) per logical step."""
    u = list(u0); v = list(v0)
    nu, nv = N4(u, p), N4(v, p)
    if nv < nu:
        u, v, nu, nv = v, u, nv, nu
    out = []
    for steps in range(1, 100000):
        h0, h1 = herm(v, u, p)
        m0, m1 = exact_round(h0, nu), exact_round(h1, nu)
        if m0 == 0 and m1 == 0 and nu <= nv:
            return steps, out, nu
        hh = h0 * h0 + h1 * h1
        out.append(math.log2(hh) - 2 * math.log2(nu) if hh else -99.0)
        iu = [-u[1], u[0], -u[3], u[2]]
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        nv = N4(v, p)
        if nv < nu:
            u, v, nu, nv = v, u, nv, nu


def main():
    scale = int(sys.argv[sys.argv.index('--scale') + 1]) if '--scale' in sys.argv else 1
    names = [a for a in sys.argv[1:] if a.startswith('p') and a in [s[0] for s in SETS]] or [s[0] for s in SETS]
    counts = load_counts()
    for name in names:
        rows = []; tail = defaultdict(list)
        run_set(name, scale, counts, rows, tail)
        json.dump(dict(rows=rows, tail=tail[name]), open(os.path.join(HERE, 'results', f'adv_{name}.json'), 'w'))
        print(name, 'done', len(rows), flush=True)


if __name__ == '__main__':
    main()
