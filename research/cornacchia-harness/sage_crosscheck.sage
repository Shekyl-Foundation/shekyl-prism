# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

# Cross-check the harness against Sage / PARI on fresh primes.
# Run: sage sage_crosscheck.sage   (from this folder)
import sys; sys.path.insert(0, '.')
from cornacchia_variants import *
gen = SGenerator()
for b in [376, 384, 500, 664]:
    bad = 0
    for _ in range(200):
        while True:
            M = ZZ.random_element(2^(b-1), 2^b)
            M = M - (M % 4) + 1
            if M.nbits() == b and M.is_prime(): break
        u, _ = sqrt_minus_one(int(M))
        assert (u^2 + 1) % M == 0
        ref = sorted(two_squares(M))                          # Sage (PARI-backed)
        par = sorted(int(t) for t in pari(f'qfbcornacchia(1,{M})'))  # PARI Cornacchia
        a = sorted(cornacchia_classical(int(M), u)[0])
        l = sorted(lattice_plain(int(M), u)[0])
        bb = sorted(lattice_randomized(int(M), u, gen.next())[0])
        bad += not (ref == par == a == l == bb)
    print(b, 'mismatches:', bad, '/ 200')
