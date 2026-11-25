# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Verification of Rick's finding (2026-11-25): for the O0-numerator basis
U = (2T, 0, 0, 0), V = (x, y, 1, 0) the Hermitian (Gram) determinant
Delta = N4(U) N4(V) - h0^2 - h1^2 is 4 p T^2, not 16 p T^2.
(a) symbolically in Sage, with the reducer's exact form and Hermitian form;
(b) numerically on real O0 numerator bases (gen_inputs.o0_basis of sampled
signing ideals, challenge ideals, key-generation ideals), several random
T, x, y per set, at all five sets, before and after reduction.
Run: sage verify_finding.sage   (writes results/verify_finding.txt)"""
import os, random, sys
sys.path.insert(0, os.path.dirname(os.path.abspath('verify_finding.sage')))
import gmpy2
from q5common import SETS, params, N4 as N4py, herm as hermpy
import gen_inputs as G
from variants import exact_logical

OUT = []
def say(s):
    print(s); OUT.append(str(s))

# (a) symbolic: the reducer's form and Hermitian form, written out exactly as in
# q5common.N4 / q5common.herm (and zi.rs nrd / herm)
R.<T, x, y, p, a0, a1, a2, a3, b0, b1, b2, b3> = PolynomialRing(ZZ)
def N4(v): return v[0]^2 + v[1]^2 + p*(v[2]^2 + v[3]^2)
def herm(v, u):
    return (v[0]*u[0] + v[1]*u[1] + p*(v[2]*u[2] + v[3]*u[3]),
            v[1]*u[0] - v[0]*u[1] + p*(v[3]*u[2] - v[2]*u[3]))
def Delta(U, V):
    h0, h1 = herm(V, U)
    return N4(U)*N4(V) - h0^2 - h1^2
say('== (a) symbolic (Sage, ZZ[T, x, y, p])')
dO0 = Delta([2*T, 0, 0, 0], [x, y, 1, 0]).factor()
dM = Delta([T, 0, 0, 0], [x, y, 1, 0]).factor()
say(f'  O0 numerators  U = (2T,0,0,0), V = (x,y,1,0): Delta = {dO0}')
say(f'  model order    U = (T,0,0,0),  V = (x,y,1,0): Delta = {dM}')
assert Delta([2*T, 0, 0, 0], [x, y, 1, 0]) == 4*p*T^2
assert Delta([T, 0, 0, 0], [x, y, 1, 0]) == p*T^2
# invariance under the step (v - m0 u - m1 iu) and the swap, general vectors
m0, m1 = var_m = PolynomialRing(R, 'm0,m1').gens()
U = [a0, a1, a2, a3]; V = [b0, b1, b2, b3]
iU = [-U[1], U[0], -U[3], U[2]]
V2 = [V[r] - m0*U[r] - m1*iU[r] for r in range(4)]
assert Delta(U, V2) == Delta(U, V) and Delta(V, U) == Delta(U, V)
say('  Delta is invariant under v <- v - m0 u - m1 iu and under the swap (general u, v, m: checked)')
# the general closed form (Lagrange identity): Delta = p * (cross terms)
say(f'  general: Delta(u, v) = {Delta(U, V).factor()}')
say('  so Delta = 4 p T^2 for the O0 numerator basis: the finding is CONFIRMED; 16 p T^2 is refuted')
say('  (the O0 basis is not 2x the model basis: its j-numerator is 1, not 2)')

# (b) numerical on real O0 numerator bases
say('== (b) numerical, real O0 numerator bases (o0_basis), five random ideals per set and class')
for s in SETS:
    P = params(s[0]); p_ = P['p']; e = P['e']; B = P['B']
    rng = random.Random(int(2026 + e))
    n_ok = 0; n = 0; lam_ok = 0
    for i in range(5):
        q, _ = G.sample_q(e, p_, rng); xq, yq = G.root_pair(q, p_, rng)
        N = G.sample_N(p_, B, rng, at_bound=(i % 2 == 0), avoid=(q,))
        xN, yN = G.root_pair(N, p_, rng)
        X, Y = G.crt_pair(N, xN, yN, q, xq, yq)
        D = int(gmpy2.next_prime(1 << (2 * P['W'])))
        xD, yD = G.root_pair(D, p_, rng)
        for cls, T_, xx, yy in (('sign', N * q, X, Y), ('challenge', q, xq, yq), ('keygen-D', D, xD, yD), ('keygen-Isec', N, xN, yN)):
            u, v = G.o0_basis(T_, xx, yy)
            hh = hermpy(v, u, p_)
            d = N4py(u, p_) * N4py(v, p_) - hh[0] ** 2 - hh[1] ** 2
            n += 1
            ok = d == 4 * p_ * T_ * T_ and d != 16 * p_ * T_ * T_
            # after full exact reduction: same Delta; lambda_1 = N4(u) is a multiple of 4T (N4 = 4 nrd, nrd in T Z)
            ur, vr, nur, nvr, _ = G.exact_lagrange(u, v, p_)
            hr = hermpy(vr, ur, p_)
            dr = nur * nvr - hr[0] ** 2 - hr[1] ** 2
            ok = ok and dr == d
            lam_ok += nur % (4 * T_) == 0
            # and the model basis of the same ideal: p T^2
            um, vm = G.model_basis(T_, xx, yy)
            hm = hermpy(vm, um, p_)
            ok = ok and N4py(um, p_) * N4py(vm, p_) - hm[0] ** 2 - hm[1] ** 2 == p_ * T_ * T_
            n_ok += ok
            if i == 0 and cls == 'sign':
                say(f'  {s[0]} sign T {T_.bit_length()} b: Delta/(p T^2) = {d // (p_ * T_ * T_)} (exact: {d % (p_ * T_ * T_) == 0}); after reduction {dr // (p_ * T_ * T_)}')
    say(f'  {s[0]}: {n_ok}/{n} O0 bases have Delta = 4 p T^2 (not 16 p T^2), unchanged by reduction, and model Delta = p T^2; '
        f'lambda_1 a multiple of 4T: {lam_ok}/{n}')
    assert n_ok == n and lam_ok == n
open('results/verify_finding.txt', 'w').write('\n'.join(OUT) + '\n')
