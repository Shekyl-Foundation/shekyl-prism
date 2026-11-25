# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""SHE-24 item (3) and exp64 lemmas: the p=3, T=5 fixture, the
immediate-fixed-point property with half ties, Gram/coordinate agreement,
and randomized checks of exp64 Lemmas W, U and B2 (own implementation)."""
import random
from fractions import Fraction as Fr
import reducer as R

out = []


def say(s):
    print(s)
    out.append(s)


# ---- (3) fixture p = 3, T = 5 -------------------------------------------
p = 3
U, V = (10, 0, 0, 0), (4, 9, 1, 0)
U, V = R.order(p, U, V)
A0 = R.D(p, U, U)
G0 = R.gram(p, U, V)
say('fixture p=3 T=5: start U=%s V=%s Gram (A,C,h0,h1)=%s Delta=%d (4pT^2=%d)'
    % (U, V, G0, G0[0] * G0[1] - G0[2] ** 2 - G0[3] ** 2, 4 * p * 25))
steps = []
for s in range(1, 6):
    A = R.D(p, U, U)
    h = (R.D(p, V, U), R.D(p, V, R.J(U)))
    U, V, m, sw = R.coord_step(p, U, V)
    steps.append((s, h, A, m, sw, U, V, R.D(p, U, U), R.D(p, V, V)))
    say('  step %d: h=(%d,%d) A=%d -> q=(%s,%s) m=%s swap=%s; U=%s N4=%d, V=%s N4=%d'
        % (s, h[0], h[1], A, Fr(h[0], A), Fr(h[1], A), m, sw, U, R.D(p, U, U), V, R.D(p, V, V)))
fp = next(s for (s, h, A, m, sw, *_rest) in steps if m == (0, 0) and not sw) - 1
say('  fixed point reached after %d active steps (1 swap + the half-tie size reduction); '
    'a schedule of 1 step would stop at U=%s V=%s (not size-reduced)' % (fp, steps[0][5], steps[0][6]))
assert fp == 2 and steps[0][4] and not steps[1][4] and steps[1][3] == (2, 1)
assert steps[1][1][1] * 2 == steps[1][2]          # +1/2 tie on the i-component, rounded up
# exhaustive minimum check of the result (tiny lattice)
Uf, Vf = steps[-1][5], steps[-1][6]
basis = [Uf, R.J(Uf), Vf, R.J(Vf)]
best = min(R.D(p, v, v) for v in (tuple(sum(c[i] * basis[i][k] for i in range(4)) for k in range(4))
            for c in __import__('itertools').product(range(-3, 4), repeat=4)) if any(v))
say('  final N4(U)=%d, exhaustive shortest nonzero N4 in span (|c|<=3) = %d' % (R.D(p, Uf, Uf), best))
assert R.D(p, Uf, Uf) == best

# ---- immediate fixed point incl. half ties --------------------------------
rng = random.Random(24)
cnt = 0
for _ in range(20000):
    A = rng.randrange(2, 10**6) * 2
    h0 = rng.choice([-A // 2, A // 2 - 1, rng.randrange(-A // 2, A // 2)])
    h1 = rng.choice([-A // 2, A // 2 - 1, rng.randrange(-A // 2, A // 2)])
    assert R.half_up(h0, A) == 0 and R.half_up(h1, A) == 0
    assert R.half_up(A // 2, A) == 1 and R.half_up(-A // 2, A) == 0
    cnt += 1
say('immediate fixed point: residuals in [-A/2, A/2) round to 0 (incl. -A/2), +A/2 rounds to 1: %d cases OK' % cnt)

# ---- Gram form == coordinate form ----------------------------------------
for trial in range(300):
    p = rng.choice([3, 7, 11, 19, 10007, (27 << 100) - 1])
    T = rng.randrange(1, 10**12)
    U, V = (2 * T, 0, 0, 0), (rng.randrange(0, 2 * T), rng.randrange(0, 2 * T), 1, 0)
    U, V = R.order(p, U, V)
    G = R.gram_order(R.gram(p, U, V))
    for s in range(60):
        A, C, h0, h1 = G
        G, _ = R.gram_apply(G, *R.exact_m(h0, h1, A))
        U, V, _, _ = R.coord_step(p, U, V)
        assert G == R.gram(p, U, V), (trial, s)
say('Gram recurrence equals the coordinate reducer on 300 random O0 bases x 60 steps')

# ---- exp64 Lemma W (window = exact, incl. ties and signs) ----------------
nw = 0
for _ in range(200000):
    bits = rng.choice([1, 2, 5, 63, 64, 65, 66, 100, 200, rng.randrange(1, 400)])
    A = rng.choice([rng.getrandbits(bits) | (1 << (bits - 1)), (1 << bits) - 1, 1 << (bits - 1)])
    if A < 1:
        continue
    eb = rng.randrange(-5, 61)
    hb = max(0, A.bit_length() + eb)
    def rh():
        c = rng.random()
        if c < 0.2 and A % 2 == 0:
            m = rng.getrandbits(max(0, eb + 1)) if eb >= 0 else 0
            return rng.choice([1, -1]) * (m * A) + rng.choice([A // 2, -A // 2])
        x = rng.getrandbits(hb) if hb else 0
        return x if rng.random() < .5 else -x
    h0, h1 = rh(), rh()
    m0, m1, path = R.exp64_m(h0, h1, A)
    if path != 'window':
        continue
    assert (m0, m1) == R.exact_m(h0, h1, A), (h0, h1, A)
    nw += 1
say('exp64 Lemma W: window quotient == floor((2h+A)/(2A)) on %d random cases (ties, both signs, A of 1..400 bits)' % nw)

# ---- exp64 Lemma U and B2 on logical steps -------------------------------
nb = 0
worst_margin = None
minDrop = 10**9
for _ in range(20000):
    abits = rng.choice([10, 63, 64, 65, 128, 300, 1000])
    A = rng.getrandbits(abits) | (1 << (abits - 1)) | 1
    e = rng.randrange(61, 700)
    h0 = rng.getrandbits(abits + e) * rng.choice([1, -1])
    h1 = rng.getrandbits(rng.randrange(1, abits + e + 1)) * rng.choice([1, -1])
    if rng.random() < 0.5:
        h0, h1 = h1, h0
    Delta = rng.randrange(1, A * A)                   # any positive determinant
    C = (Delta + h0 * h0 + h1 * h1)                   # A*C = Delta + |h|^2 needs divisibility:
    C = -(-C // A)                                    # round C up: Delta' = A*C - |h|^2 >= Delta > 0
    G = (A, C, h0, h1)
    q = (Fr(h0, A), Fr(h1, A))
    absq2 = q[0] ** 2 + q[1] ** 2
    n = 0
    e_prev = max(R.bl(h0), R.bl(h1)) - A.bit_length()
    while True:
        Acur, Ccur, a0, a1 = G
        m0, m1, path = R.exp64_m(a0, a1, Acur)
        if path == 'window':
            break
        n += 1
        qa, qb = Fr(a0, Acur), Fr(a1, Acur)
        for qq, mm in ((qa, m0), (qb, m1)):
            assert mm == 0 or (mm > 0) == (qq > 0)
            assert abs(mm) < abs(qq) or mm == 0
        G2, sw = R.gram_apply(G, m0, m1)
        assert not sw, 'big sub-step swapped'
        assert G2[1] < Ccur, 'v lengthened'
        e_new = max(R.bl(G2[2]), R.bl(G2[3])) - Acur.bit_length()
        minDrop = min(minDrop, e_prev - e_new)
        assert e_new <= e_prev - 58
        e_prev = e_new
        G = G2
    # completes the logical step exactly
    Gex, _ = R.gram_apply((A, C, h0, h1), *R.exact_m(h0, h1, A))
    Gw, _ = R.gram_apply(G, m0, m1)
    assert Gw == Gex
    if n:
        nb += 1
        # B2: log2|q| > 58 n + 2  <=>  |q|^2 > 2^(116 n + 4)
        assert absq2 > Fr(2) ** (116 * n + 4)
        import math
        lg = (absq2.numerator.bit_length() - absq2.denominator.bit_length()) / 2
        margin = lg - (58 * n + 2)
        worst_margin = margin if worst_margin is None else min(worst_margin, margin)
say('exp64 Lemma U/B2: %d logical steps with big sub-steps; no sub-step swapped or lengthened v; '
    'every logical step equals the exact step; e dropped by >= %d per sub-step (bound 58); '
    'log2|q| - (58n+2) >= %.1f (bound > 0)' % (nb, minDrop, worst_margin))

open('results/test_contract.txt', 'w').write('\n'.join(out) + '\n')
