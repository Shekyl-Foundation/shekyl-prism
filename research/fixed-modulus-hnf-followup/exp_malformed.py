# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Follow-up item 2: when may 2N e_i be added, and what a malformed input looks like afterwards.

D1  Delta([u,Ju,v,Jv]) = |det_{Z[i]}(u,v)|^2  (so the index of the decoded Gaussian pair is one Gaussian cross product).
D2  The Cramer membership test of admission is vacuous at det = 0 (it passes every vector); it only means something
    together with |det|^2 = 4N^2.
D3  A FULL-RANK, parity-valid pair whose lattice is a proper sublattice of the ideal I with J + 2N Z^4 = I:
    the output equals the stored HNF of I and prod d = 4N^2, so only Delta(G) != prod d rejects it.  At all five sets.
"""
import random, sys
import common
import lib, gen_ideals
from fmhnf import fixed_modulus_hnf
from sets import classes
from sage.all import matrix, ZZ, random_prime, ZZ as Z


def J(v):
    return [-v[1], v[0], -v[3], v[2]]


def gprod(a, b):
    return [a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0]]


def cross(u, v):
    a, b, c, d = u
    e, f, g, h = v
    p1, p2 = gprod([a, b], [g, h]), gprod([e, f], [c, d])
    return [p1[0] - p2[0], p1[1] - p2[1]]


def contains_cramer(u, v, N, w):
    """Admission's membership test: both Cramer numerators times conj(det) divisible by 4N^2 (both components)."""
    d = cross(u, v)
    conj = [d[0], -d[1]]
    for num in (cross(w, v), cross(u, w)):
        z = gprod(num, conj)
        if z[0] % (4 * N * N) or z[1] % (4 * N * N):
            return False
    return True


def prod_d(cols):
    r = 1
    for c in range(4):
        r *= cols[c][c]
    return r


def main():
    rng = random.Random(21)
    print('D1. Delta([u,Ju,v,Jv]) == |det_Zi(u,v)|^2')
    for _ in range(2000):
        u = [rng.randrange(-10 ** 6, 10 ** 6) for _ in range(4)]
        v = [rng.randrange(-10 ** 6, 10 ** 6) for _ in range(4)]
        d = cross(u, v)
        assert abs(int(matrix(ZZ, [u, J(u), v, J(v)]).det())) == d[0] ** 2 + d[1] ** 2
    print('  2000 random pairs: equal (independent of p)')

    print('D2. the Cramer test at det = 0 and at the right det (toy p = 3, N = 7, U = (14,0,0,0), V = (0,9,1,0))')
    N = 7
    U, V = [14, 0, 0, 0], [0, 9, 1, 0]
    outside = [1, 0, 0, 0]
    inside = [5, 0, 0, 1]
    print('  valid pair   : |d|^2 = %d = 4N^2; contains (5,0,0,1) [in I]: %s; contains (1,0,0,0) [not in I]: %s' % (
        sum(t * t for t in cross(U, V)), contains_cramer(U, V, N, inside), contains_cramer(U, V, N, outside)))
    V3 = [3 * t for t in V]
    print('  dependent v=3u... pair (V, 3V): d = %s; contains (1,0,0,0): %s  <- vacuous pass' % (
        cross(V, V3), contains_cramer(V, V3, N, outside)))
    assert contains_cramer(U, V, N, inside) and not contains_cramer(U, V, N, outside)
    assert contains_cramer(V, V3, N, outside)
    V0 = [0, 0, 0, 0]
    print('  zero v: d = %s; contains (1,0,0,0): %s  <- vacuous pass' % (cross(U, V0), contains_cramer(U, V0, N, outside)))

    print('D3. full-rank, parity-valid pair with J + 2N Z^4 = I but Delta != 4N^2: V\' = V + 2N e_2 (in 2N Z^4 <= I)')
    for name in ('toy p=3 N=7', 'P248', 'P324', 'P376', 'P500', 'P664'):
        if name.startswith('toy'):
            p, N, K = 3, 7, 5
            x, y = gen_ideals.lift_parity(0, 2, N)
        else:
            P, cl = classes(name)
            p, B, K = P['p'], P['B'], cl['ideal']['K']
            top = 1 << B
            N = int(random_prime(top - 1, lbound=top >> 1))
            x, y = gen_ideals.root_mod_prime_power(rng, p, N, 1)
            x, y = gen_ideals.lift_parity(x, y, N)
        gen_ideals.PRIME[0] = p
        cols = lib.sage_hnf_cols(lib.left_ideal_gens([[2 * N, 0, 0, 0], [x, y, 1, 0]], p))
        m = 2 * N
        U, V = [2 * N, 0, 0, 0], [x, y, 1, 0]
        V2 = [V[0], V[1], V[2] + m, V[3]]
        assert V2[0] % 2 == V2[3] % 2 and V2[1] % 2 == V2[2] % 2 or True
        G = [U, J(U), V2, J(V2)]
        Pm, _ = fixed_modulus_hnf(G, m, K)
        d = cross(U, V2)
        Delta = d[0] ** 2 + d[1] ** 2
        par = (V2[0] - V2[3]) % 2 == 0 and (V2[1] - V2[2]) % 2 == 0
        print('  %-12s N bits %3d: output == stored HNF: %s | prod d == 4N^2: %s | Delta/4N^2 = %d | O0 parity ok: %s | Delta == prod d: %s' % (
            name, N.bit_length(), Pm == cols, prod_d(Pm) == 4 * N * N, Delta // (4 * N * N), par, Delta == prod_d(Pm)))
        assert Pm == cols and prod_d(Pm) == 4 * N * N and Delta != 4 * N * N and par
        assert abs(int(matrix(ZZ, G).det())) == Delta
        # and the exact true lattice is a proper sublattice of I
        assert lib.sage_hnf_cols(G) != cols


if __name__ == '__main__':
    main()
