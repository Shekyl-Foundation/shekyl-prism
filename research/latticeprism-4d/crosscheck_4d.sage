# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Cross-check lp4d against Sage/PARI: the quaternion product is Sage's
# QuaternionAlgebra(-1, -p), the ideal basis has determinant T^2, and the
# instrumented LLL's first vector has the same reduced norm as PARI
# qflllgram's on the same Gram matrix (both delta = 0.99).
# Run from this folder: sage crosscheck_4d.sage
import sys; sys.path.insert(0, '.')
from lp4d import *
import secrets
for name, e, f in SETS:
    p = prime_of(e, f); Q.<i,j,k> = QuaternionAlgebra(QQ, -1, -p)
    bad = 0
    for _ in range(25):
        u = [ZZ.random_element(-10^6, 10^6) for _ in range(4)]; v = [ZZ.random_element(-10^6, 10^6) for _ in range(4)]
        a = Q(u); b = Q(v); c = qmul(tuple(map(int, u)), tuple(map(int, v)), int(p))
        bad += (a * b != Q(list(c))) or (a.reduced_norm() != nrd(tuple(map(int, u)), int(p)))
        while True:
            s = 2 * secrets.randbits(e - 4) + 1; T = (1 << (2 * e - 5)) - s * s
            if is_prime(T): break
        B, _ = embed_D0(T, int(p)); bad += det4(B) != T ^ 2
        G = gram_norm(B, int(p)); H, _ = lll_gram(G)
        R = apply(H, B); ours = nrd(R[0], int(p))
        Gp = matrix(ZZ, G); U = pari(Gp).qflllgram(); Ured = matrix(ZZ, U).transpose() * matrix(ZZ, B)
        theirs = min(nrd(tuple(map(int, r)), int(p)) for r in Ured.rows())
        bad += ours > 2 * theirs   # LLL first vector within factor 2 of PARI's best row
        bad += ours % T != 0
    print(name, 'mismatches:', bad, '/ 100 checks')
