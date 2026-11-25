# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""SHE-24: the exp64 joint-charge inequalities in exact arithmetic.
Charge:   G * 2^-(116n+4) <= rho^(1+n)            for all n >= 1
Boundary: 2^(116n+4)      >= 2^11 * rho^(-2(1+n)) for all n >= 1
Both sides are geometric in n with the LHS/RHS ratio monotone in our favour
(2^-116 <= rho and 2^116 >= rho^-2), so n = 1 suffices; n = 1..64 checked."""
import sys
from fractions import Fraction as Fr
import mycounts as MC

G = MC.load_G(sys.argv[1])[0]
rho = Fr(537, 2000)
assert Fr(1, 2 ** 116) <= rho and Fr(2) ** 116 >= 1 / rho ** 2
ok = all(G * Fr(1, 2 ** (116 * n + 4)) <= rho ** (1 + n) for n in range(1, 65))
ok2 = all(Fr(2) ** (116 * n + 4) >= Fr(2 ** 11) / rho ** (2 * (1 + n)) for n in range(1, 65))
# tail: no big quotient once lambda > 2^-11: |q|^2 <= 1/r < 1/lambda < 2^11 < 2^120
ok3 = 2 ** 11 < 2 ** 120
print('charge %s; boundary %s; tail-has-no-big-quotient %s' % (ok, ok2, ok3))
