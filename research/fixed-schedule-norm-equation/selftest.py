# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Quick checks of the port (run: python selftest.py)."""
import time
from fne import *

for name, e, f in SETS:
    p = prime_of(e, f)
    rng = tape(b'self', name)
    t0 = time.time()
    SEC = sec_degree(name)
    I0 = random_prime_ideal(SEC, p, rng)
    assert lat_index_ok(I0, p)
    Isec, info = prime_norm_equivalent(I0, p, rng)
    assert lat_index_ok(Isec, p)
    N = Isec.norm
    s = 1
    while True:
        q = (1 << (2 * e - 5)) - s * s
        if gmpy2.is_prime(q):
            break
        s += 2
    Iq = random_prime_ideal(q, p, rng)
    L = intersect_coprime(Isec, Iq)
    assert lat_index_ok(L, p)
    S = prepare_small(L, p)
    assert lat_index_ok(S, p)
    c = Ctr()
    r = qlapoti(S, p, e - 2, rng, c, verify=True)
    print(name, 'p bits', p.bit_length(), 'N bits', int(N).bit_length(),
          'draws', info['draws'], 'n_small bits', int(S.norm).bit_length(),
          'den', S.den, 'ok', r is not None, c.asdict(),
          'time %.2fs' % (time.time() - t0))
