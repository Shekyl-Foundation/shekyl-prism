#!/usr/bin/env python3
"""Random admitted generic keys for P248: lines `n x y` (hex).

n is a random prime in [2^157, 2^158) (the corpus norms are 158-bit primes) and
(x, y) a root of x^2 + y^2 + p = 0 mod n, p = 5*2^248 - 1, with y uniform mod n
(so each key has a fresh root), then lifted to x even and y odd in [0, 2n) as GenericHnf requires. Seeded, reproducible. The harness turns each
line into (norm, GenericHnf generators) exactly as `Basis::secret_candidate`
does. Usage: genpool.py COUNT SEED > pool.txt
"""
import random, sys
import sympy
from sympy.ntheory import sqrt_mod
cnt, seed = int(sys.argv[1]), int(sys.argv[2])
rng = random.Random(seed)
p = 5 * 2**248 - 1
out = 0
while out < cnt:
    n = rng.randrange(2**157, 2**158) | 1
    if not sympy.isprime(n):
        continue
    for _ in range(40):
        y = rng.randrange(n)
        r = sqrt_mod((-p - y * y) % n, n)
        if r is not None:
            r = int(r)
            x = r if r % 2 == 0 else r + n      # v0 even, 0 <= x < 2n
            yy = y if y % 2 == 1 else y + n     # v1 odd,  0 < y < 2n
            assert (x * x + yy * yy + p) % n == 0
            sys.stdout.write("%x %x %x\n" % (n, x, yy)); out += 1
            break
