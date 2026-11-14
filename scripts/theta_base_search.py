#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""When can the shift search of `base_change_theta` fail, for any `zeta`?

`nonvanishing` in `prism-isogeny/src/theta_base.rs` tries four half-period
shifts and takes the first whose sums are not all zero. Each sum is a
polynomial in `zeta` of degree at most 3 with non-negative integer
coefficients that add up to at most 4, because the code reduces every
exponent modulo 4 whatever `zeta` is. The search fails exactly when `zeta`
is a common root of every such polynomial over the four shifts.

This script walks all 4^8 pairs of blocks `(C, D)`, collects the set of
polynomials of each, and takes the greatest common divisor of the set over
the integers. A constant divisor (1, 2 or 4) has no root in a field of odd
characteristic, so the search cannot fail for that pair at any `zeta`. The
only other divisor that occurs is `zeta^2 + 3`.

The polynomials have coefficients at most 4 and degree at most 3, so a
common root modulo `p` that is not one over the rationals would need `p`
to divide a resultant far smaller than any prime here; the Rust tests
check the three real fields directly at the roots of `zeta^2 + 3` and at
other values.

Needs sympy. Prints the divisors found, the number of pairs with each, and
how many pairs with a non-constant divisor are the right half of a matrix
of Sp_4(Z/4Z). Exits non-zero if the result is not the one the doc comment
of `nonvanishing` states.
"""
import itertools
import sys
from functools import reduce

from sympy import ZZ, Poly, gcd, symbols

Z = symbols("z")


def polynomials(c, d):
    """Every nonzero sum of the four shifts, as coefficient tuples."""
    found = set()
    for ir0 in range(2):
        for ir1 in range(2):
            sums = [[0] * 4 for _ in range(4)]
            for j0 in range(2):
                for j1 in range(2):
                    k0 = (c[0][0] * j0 + c[0][1] * j1) % 4
                    k1 = (c[1][0] * j0 + c[1][1] * j1) % 4
                    l0 = (d[0][0] * j0 + d[0][1] * j1) % 4
                    l1 = (d[1][0] * j0 + d[1][1] * j1) % 4
                    exponent = (-((k0 + 2 * ir0) * l0) - ((k1 + 2 * ir1) * l1)) % 4
                    index = ((k0 + ir0) % 4) % 2 + 2 * (((k1 + ir1) % 4) % 2)
                    sums[index][exponent] += 1
            found.update(tuple(s) for s in sums if any(s))
    return frozenset(found)


def symplectic_half(c, d):
    """Whether `(C; D)` can be the right two columns of a symplectic matrix."""
    form = c[0][0] * d[0][1] + c[1][0] * d[1][1] - c[0][1] * d[0][0] - c[1][1] * d[1][0]
    first = [c[0][0] % 2, c[1][0] % 2, d[0][0] % 2, d[1][0] % 2]
    second = [c[0][1] % 2, c[1][1] % 2, d[0][1] % 2, d[1][1] % 2]
    return form % 4 == 0 and any(first) and any(second) and first != second


def main():
    divisor_of = {}
    counts = {}
    symplectic = {}
    for entries in itertools.product(range(4), repeat=8):
        c = [[entries[0], entries[1]], [entries[2], entries[3]]]
        d = [[entries[4], entries[5]], [entries[6], entries[7]]]
        sums = polynomials(c, d)
        if sums not in divisor_of:
            polys = [Poly(sum(a * Z**k for k, a in enumerate(p)), Z, domain=ZZ) for p in sums]
            divisor_of[sums] = str(reduce(gcd, polys).as_expr())
        divisor = divisor_of[sums]
        counts[divisor] = counts.get(divisor, 0) + 1
        symplectic[divisor] = symplectic.get(divisor, 0) + symplectic_half(c, d)
    for divisor in sorted(counts):
        print(f"divisor {divisor}: {counts[divisor]} pairs, "
              f"{symplectic[divisor]} of them symplectic halves")
    expected = {"1", "2", "4", "z**2 + 3"}
    if set(counts) != expected or counts["z**2 + 3"] != 1152 or symplectic["z**2 + 3"] != 0:
        sys.exit("the search's failure set is not the one theta_base.rs documents")
    if sum(counts.values()) != 4**8 or sum(symplectic.values()) != 11520:
        sys.exit("the walk did not cover every pair")
    print("the search fails only when zeta^2 = -3, on 1152 pairs, none a symplectic half")


if __name__ == "__main__":
    main()
