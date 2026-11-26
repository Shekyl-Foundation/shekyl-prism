# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Fixed-modulus HNF versus exact-integer HNF (Sage, PARI) on adversarial left O0-ideals.

For every set and every ideal family: several generating sets of the SAME lattice
(HNF columns, J-closed 8, unimodular scrambles with entries up to 2^(2K), extreme
entries near +-2^(2K), zero and duplicate rows, no generators at all), the model
at K = B+1 (Q4 class) and m = 2N, compared with
  (1) Sage HNF of the generators alone (the true lattice),
  (2) PARI mathnf of the generators alone,
  (3) the ideal's own HNF,
and checks index 4N^2, containment of 2N Z^4, the proved width bounds (inside Ctx)
and the trace hash (identical for equal (K, k)).
Usage: python test_adversarial.py P500 [seed]
"""
import sys, random, time, json
from collections import Counter, defaultdict
import lib, gen_ideals
from fmhnf import fixed_modulus_hnf, Ctx
from sets import params, classes
from counts import formula


def J(v):
    return [-v[1], v[0], -v[3], v[2]]


def gens_variants(rng, cols, N, m, K):
    bound = 1 << (2 * K)
    base = [[cols[c][i] if i <= c else 0 for i in range(4)] for c in range(4)]
    V = [('hnf-columns k=4', base)]
    V.append(('J-closed k=8', [r for v in base for r in (v, J(v))]))
    for sb in (6, K // 3, K // 2):
        V.append(('scramble k=4 (U~2^%d)' % sb, lib.scramble(rng, cols, 4, sb, bound)))
    V.append(('scramble k=5', lib.scramble(rng, cols, 5, K // 4 + 1, bound)))
    V.append(('scramble k=8', lib.scramble(rng, cols, 8, K // 4 + 1, bound)))
    # extreme entries: add multiples of m to reach |x| close to 2^(2K)
    sc = lib.scramble(rng, cols, 4, 6, bound)
    ext = []
    for r in sc:
        row = []
        for x in r:
            s = rng.choice((1, -1))
            target = bound - 1 - rng.randrange(0, 3)
            # smallest |x'| >= ... : x' = x + m*w with |x'| <= bound-1, near the top
            if s == 1:
                row.append(x + m * ((target - x) // m))          # in (target-m, target]
            else:
                row.append((x + target) % m - target)             # in [-target, -target+m)
        ext.append(row)
    assert all(abs(x) < bound for r in ext for x in r)
    V.append(('extreme entries ~2^(2K)', ext))
    V.append(('with zero and duplicate rows k=7', base + [[0] * 4, list(base[1]), [0] * 4]))
    return V


def check_one(name, cols, N, m, K, G, ctxs, shapes, widths):
    P, ctx = fixed_modulus_hnf(G, m, K)
    # 'extreme' rows are shifted by multiples of 2N: span(G) may be a proper sublattice,
    # but span(G) + 2N Z^4 is still the ideal, so only the with-modulus references apply.
    exact = not name.endswith('extreme entries ~2^(2K)')
    ref_mod = lib.sage_hnf_cols(G, m)
    ref_true = lib.sage_hnf_cols(G) if exact else ref_mod        # true lattice, no modulus
    ref_pari = lib.pari_hnf_cols(G) if exact else ref_mod
    ref_pari_m = lib.pari_hnf_cols(G, m)
    assert P == ref_true == ref_mod == ref_pari == ref_pari_m == cols, (name, N, 'MISMATCH')
    d = lib.det_of_cols(P)
    assert d == 4 * N * N
    for c in range(4):
        e = [0] * 4
        e[c] = m
        assert lib.lattice_contains(P, e)
    ctxs[len(G)].add(ctx.trace_hash())
    for k_, v in ctx.maxbits.items():
        widths[k_] = max(widths.get(k_, 0), v)
    return ctx


def main(setname, seed):
    rng = random.Random(seed)
    P_, cl = classes(setname)
    p, B = P_['p'], P_['B']
    K = cl['ideal']['K']
    t0 = time.time()
    fam = gen_ideals.families(rng, p, B)
    print('%s: p has %d bits, B=%d, K=%d; %d ideals generated in %.1fs' % (setname, P_['L'], B, K, len(fam), time.time() - t0))
    ctxs = defaultdict(set)
    shapes = Counter()
    widths = {}
    nrun = 0
    maxact = 0
    maxNbits = 0
    for (name, cols, N) in fam:
        m = 2 * N
        Kuse = K if m < (1 << K) else m.bit_length() + 1       # ramified N=p exceeds Q4: widen K (public) for the test
        wide = Kuse != K
        shapes[(gen_ideals.shape_label(cols, N) if not name.startswith('N=') else 'small N', wide)] += 1
        maxNbits = max(maxNbits, N.bit_length())
        if wide:
            tmp = defaultdict(set)
            for vn, G in gens_variants(rng, cols, N, m, Kuse)[:2]:
                ctx = check_one(name + '/' + vn, cols, N, m, Kuse, G, tmp, shapes, {})
                nrun += 1
            continue
        for vn, G in gens_variants(rng, cols, N, m, Kuse):
            ctx = check_one(name + '/' + vn, cols, N, m, Kuse, G, ctxs, shapes, widths)
            maxact = max(maxact, getattr(ctx, 'max_active', 0))
            nrun += 1
    # zero generators: lattice m Z^4, and a few multiples
    for N in (1, 3, int(2 ** B) - 1):
        m = 2 * N
        G = [[0] * 4 for _ in range(4)]
        P, ctx = fixed_modulus_hnf(G, m, K)
        want = lib.sage_hnf_cols(G, m)
        assert P == want and [P[c][c] for c in range(4)] == [m] * 4
        ctxs[4].add(ctx.trace_hash())
        nrun += 1
    for k_, hs in ctxs.items():
        assert len(hs) == 1, ('trace not identical for k=%d' % k_)
    # largest N seen, max active elimination steps
    print('  %d runs, all equal to Sage and PARI (true lattice, with and without 2N rows) and to the ideal HNF' % nrun)
    print('  trace hash identical per k: %s' % {k: list(v)[0][:12] for k, v in sorted(ctxs.items())})
    print('  max active elimination steps observed %d (schedule %d)' % (maxact, 2 * K - 1))
    print('  largest N bits %d (B=%d)' % (maxNbits, B))
    print('  shapes:', dict(shapes))
    print('  observed max bit-lengths:', {k: v for k, v in sorted(widths.items())})
    json.dump({'set': setname, 'K': K, 'runs': nrun, 'maxact': maxact, 'widths': widths,
               'shapes': {('%s|wide=%s' % k): v for k, v in shapes.items()}},
              open('results/adversarial_%s.json' % setname, 'w'), indent=1)


if __name__ == '__main__':
    main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 7)
