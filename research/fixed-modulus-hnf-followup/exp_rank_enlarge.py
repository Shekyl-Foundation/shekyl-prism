# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Follow-up items 1 and 2: zero candidate pivots, rank deficiency, and silent enlargement.

Part A  toy worked examples (p = 3, N = 7 and N = 3), printed with the actual matrices.
Part B  every subset of the HNF columns of generic and non-generic ideals at the five sets.
Part C  the exact criterion  (Delta(G) != 0 and Delta(G) == prod d_c)  <=>  (rank 4 and m Z^4 <= span G),
        checked against Sage on random and structured generators.
"""
import itertools, random, sys
import common
import lib, gen_ideals
from fmhnf import fixed_modulus_hnf
from sets import classes
from sage.all import matrix, ZZ, QQ


def J(v):
    return [-v[1], v[0], -v[3], v[2]]


def prod_d(cols):
    r = 1
    for c in range(4):
        r *= cols[c][c]
    return r


def mat(cols):
    return [[cols[c][i] if i <= c else 0 for c in range(4)] for i in range(4)]       # rows = coordinates


def delta(G):
    """|det| of 4 generator rows (exact integer 4x4 determinant)."""
    return abs(int(matrix(ZZ, G).det()))


def span_contains_mZ4(G, m):
    """Exact: is m e_c in span_Z(G) for every c?  (HNF of G alone, then reduce.)"""
    M = matrix(ZZ, [list(reversed(r)) for r in G])
    if M.rank() < 4:
        return False
    H = M.hermite_form()
    cols = {}
    nz = [r for r in H.rows() if any(r)]
    cc = [[int(nz[j][3 - i]) if i <= 3 - j else 0 for i in range(4)] for j in range(4)]
    cols = [[cc[3 - c][i] for i in range(c + 1)] for c in range(4)]
    return all(lib.lattice_contains(cols, [m if i == c else 0 for i in range(4)]) for c in range(4))


def show(name, G, m, K, expect_ideal=None):
    P, ctx = fixed_modulus_hnf(G, m, K)
    d = delta(G) if len(G) == 4 else None
    rk = matrix(ZZ, G).rank()
    print('  %-52s rank %d  Delta=%s  pivots %s  prod d = %d' % (name, rk, d if d is not None else 'n/a (k=%d)' % len(G),
          [P[c][c] for c in range(4)], prod_d(P)))
    return P


def part_a():
    print('A. toy examples, p = 3')
    p = 3
    gen_ideals.PRIME[0] = p
    N = 7
    x, y = gen_ideals.lift_parity(0, 2, N)               # (0, 9): 0 + 81 + 3 = 84 = 7 * 12
    gens = lib.left_ideal_gens([[2 * N, 0, 0, 0], [x, y, 1, 0]], p)
    cols = lib.sage_hnf_cols(gens)
    m = 2 * N
    print(' ideal N=7, root (x,y)=(%d,%d), HNF columns (col c = entries 0..c): %s   index %d = 4N^2' % (x, y, cols, prod_d(cols)))
    base = [[cols[c][i] if i <= c else 0 for i in range(4)] for c in range(4)]
    K = 5
    print(' (i) the m e_i columns are redundant in the HNF itself: col0 = (14,0,0,0) = m e_0, col1 = (0,14,0,0) = m e_1.')
    for name, G in (('full columns (rank 4)', base),
                    ('drop col0 and col1 -> rank 2', [base[2], base[3]]),
                    ('only the root column (0,9,1,0) -> rank 1', [base[2]]),
                    ('drop col0 only -> rank 3', [base[1], base[2], base[3]]),
                    ('no generators at all', [[0] * 4]),
                    ('u, Ju, v, Jv with v = 0 (dependent Gaussian pair)', [base[0], J(base[0]), [0] * 4, [0] * 4]),
                    ('u, Ju, v, Jv with v = 3u (dependent)', [base[2], J(base[2]), [3 * t for t in base[2]], J([3 * t for t in base[2]])])):
        P = show(name, G, m, K)
        same = (P == cols)
        print('        output == the ideal HNF: %s' % same)
        assert same == (name.startswith('full') or name.startswith('drop') or 'v = 3u' in name), name
        if not same:
            print('        (output index %d is %s the ideal index %d)' % (prod_d(P), 'a multiple of' if prod_d(P) % prod_d(cols) == 0 else 'unrelated to', prod_d(cols)))
    print(' (a rank-deficient set that does not reach the ideal gives a different, larger-index lattice: prod d != 4N^2.)')
    print(' => every rank-deficient generating set that reaches the ideal after adding 2N e_i is accepted by a')
    print('    comparison with the stored HNF or by "prod d == 4N^2".  Only the exact Delta(G) = 0 test sees them.')
    # Prop 2: dropping col c with d_c = m
    print(' (ii) Prop 2: d_c = m  =>  columns minus col_c have rank 3 and the same output.  pivots %s, m = %d' % (
        [cols[c][c] for c in range(4)], m))

    print(' (iii) full-rank, index 4N^2, inside O0num, NOT left-O0-closed: N = 3, m = 6')
    N = 3
    m = 6
    b = [[2, 0, 0, 0], [0, 2, 0, 0], [0, 1, 1, 0], [1, 0, 0, 1]]
    G = [b[0], b[1], b[2], [9 * t for t in b[3]]]       # cyclic quotient Z/9 inside O0num: index 4*9 = 36 = 4 N^2
    Lc = lib.sage_hnf_cols(G)
    print('     L has HNF pivots %s (index %d = 4N^2 = 36), all generators pass the O0 parity test' % ([Lc[c][c] for c in range(4)], prod_d(Lc)))
    P = show('add 6 e_i', G, m, 4)
    print('     output index %d != 36: a strictly larger lattice (index ratio %d). Contains 6 Z^4? %s' % (
        prod_d(P), 36 // prod_d(P), span_contains_mZ4(G, 6)))
    print(' (iv) full rank, index 4N^2, contains 2N Z^4, but not O0-stable and not in O0num (parity fails): N = 7')
    N = 7
    m = 14
    colsx = [[14], [0, 14], [1, 2, 1], [14 - 2, 1, 0, 1]]      # (x,y,1,0) with x odd: a != d mod 2
    G = [[colsx[c][i] if i <= c else 0 for i in range(4)] for c in range(4)]
    P = show('non-ideal HNF with pivots (14,14,1,1)', G, m, 5)
    assert P == colsx
    print('     HNF is computed correctly (= the input), prod d = 196 = 4N^2 passes, but G is not an ideal lattice: only the')
    print('     parity / closure tests see it.  The HNF algorithm is correct for any lattice containing m Z^4; ideal-ness is separate.')


def part_b(rng):
    print('B. subsets of the HNF columns (k = 1..4 of the columns) at the five sets: which reproduce the ideal?')
    out = {}
    for name in ('P248', 'P324', 'P376', 'P500', 'P664'):
        P_, cl = classes(name)
        p, B, K = P_['p'], P_['B'], cl['ideal']['K']
        gen_ideals.PRIME[0] = p
        top = 1 << B
        from sage.all import random_prime
        fams = []
        l = int(random_prime(top - 1, lbound=top >> 1))
        x, y = gen_ideals.root_mod_prime_power(rng, p, l, 1)
        fams.append(('prime N~2^B generic', l, gen_ideals.ideal_from_root(l, x, y)))
        fams.append(('scalar 3*O0 (N=9)', 9, lib.left_ideal_gens([[6, 0, 0, 0]], p)))
        fams.append(('O0 (N=1)', 1, lib.left_ideal_gens([[2, 0, 0, 0]], p)))
        fams.append(('even N=2 (1+i)', 2, lib.left_ideal_gens([[2, 2, 0, 0]], p)))
        for fname, N, gens in fams:
            cols = lib.sage_hnf_cols(gens)
            assert lib.ideal_norm_from_cols(cols) == N
            m = 2 * N
            base = [[cols[c][i] if i <= c else 0 for i in range(4)] for c in range(4)]
            good = []
            for r in range(1, 5):
                for sub in itertools.combinations(range(4), r):
                    G = [base[c] for c in sub]
                    P, _ = fixed_modulus_hnf(G, m, K)
                    if P == cols:
                        good.append((sub, matrix(ZZ, G).rank()))
            deficient = [s for s, rk in good if rk < 4]
            pivs = [cols[c][c] for c in range(4)]
            eqm = [c for c in range(4) if cols[c][c] == m]
            print('  %s %-22s pivots==m at c=%s; subsets reproducing the HNF: %d of 15, of which rank-deficient: %d (e.g. %s)' % (
                name, fname, eqm, len(good), len(deficient), deficient[:2]))
            # Prop 2 check
            for c in eqm:
                G = [base[t] for t in range(4) if t != c]
                P, _ = fixed_modulus_hnf(G, m, K)
                assert P == cols and matrix(ZZ, G).rank() == 3
            out[(name, fname)] = (len(good), len(deficient))
    return out


def part_c(rng):
    print('C. criterion  Delta(G) != 0 and Delta(G) == prod d_c   <=>   rank 4 and m Z^4 <= span G')
    n = 0
    tally = {'accept': 0, 'reject-deficient': 0, 'reject-enlarged': 0}
    for it in range(6000):
        m = rng.choice((2, 4, 6, 10, 14, 30, 2 * 7 * 11))
        K = m.bit_length() + rng.choice((0, 1, 3))
        mode = rng.randrange(6)
        if mode == 0:       # random full lattices containing mZ^4 via HNF + scramble, k=4
            ds = [d for d in range(1, m + 1) if m % d == 0]
            d = [rng.choice(ds) for _ in range(4)]
            cols = [[0] * (c + 1) for c in range(4)]
            for c in range(4):
                cols[c][c] = d[c]
                for l in range(c):
                    cols[c][l] = rng.randrange(d[l])
            if not all(lib.lattice_contains(cols, [m if i == c else 0 for i in range(4)]) for c in range(4)):
                continue
            G = lib.scramble(rng, cols, 4, 4, 1 << (2 * K))
        elif mode == 1:     # same but one row replaced by a combination of the others (rank <= 3)
            G = [[rng.randrange(-30, 30) for _ in range(4)] for _ in range(3)]
            G.append([a + 2 * b - c for a, b, c in zip(*G)])
        elif mode == 2:     # random small rows
            G = [[rng.randrange(-m, m + 1) for _ in range(4)] for _ in range(4)]
        elif mode == 3:     # rows multiples of a common factor
            g = rng.choice((2, 3, 5))
            G = [[g * rng.randrange(-5, 6) for _ in range(4)] for _ in range(4)]
        elif mode == 4:     # a valid lattice with one generator weakened (times a non-unit)
            ds = [dd for dd in range(1, m + 1) if m % dd == 0]
            d = [rng.choice(ds) for _ in range(4)]
            cols = [[0] * (c + 1) for c in range(4)]
            for c in range(4):
                cols[c][c] = d[c]
                for l in range(c):
                    cols[c][l] = rng.randrange(d[l])
            G = [[cols[c][i] if i <= c else 0 for i in range(4)] for c in range(4)]
            G[rng.randrange(4)] = [rng.choice((2, 3)) * t for t in G[rng.randrange(4)]]
        else:               # columns with one entirely zero row
            G = [[rng.randrange(-5, 6) for _ in range(4)] for _ in range(4)]
            G[rng.randrange(4)] = [0] * 4
        if any(abs(x) >= (1 << (2 * K)) for r in G for x in r):
            continue
        P, _ = fixed_modulus_hnf(G, m, K)
        D = delta(G)
        crit = (D != 0 and D == prod_d(P))
        truth = span_contains_mZ4(G, m)
        assert crit == truth, (G, m, D, prod_d(P), truth)
        if D == 0:
            tally['reject-deficient'] += 1
        elif crit:
            tally['accept'] += 1
        else:
            tally['reject-enlarged'] += 1
        n += 1
    print('  %d cases, criterion == exact truth in every case; %s' % (n, tally))
    # criterion does not hold with the product alone: count accepted-by-product-only rank-deficient inputs
    cnt = 0
    tot = 0
    for it in range(3000):
        m = rng.choice((2, 4, 6, 14))
        K = m.bit_length() + 1
        ds = [d for d in range(1, m + 1) if m % d == 0]
        d = [rng.choice(ds) for _ in range(4)]
        cols = [[0] * (c + 1) for c in range(4)]
        for c in range(4):
            cols[c][c] = d[c]
            for l in range(c):
                cols[c][l] = rng.randrange(d[l])
        if not all(lib.lattice_contains(cols, [m if i == c else 0 for i in range(4)]) for c in range(4)):
            continue
        base = [[cols[c][i] if i <= c else 0 for i in range(4)] for c in range(4)]
        sub = [base[c] for c in range(4) if rng.random() < 0.6]
        if not sub:
            continue
        P, _ = fixed_modulus_hnf(sub, m, K)
        tot += 1
        if P == cols and matrix(ZZ, sub).rank() < 4:
            cnt += 1
    print('  random column subsets of valid lattices: %d of %d are rank-deficient yet reproduce the full HNF exactly' % (cnt, tot))
    return n, tally, cnt, tot


if __name__ == '__main__':
    rng = random.Random(3)
    part_a()
    part_b(rng)
    part_c(rng)
