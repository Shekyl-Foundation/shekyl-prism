# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Closed-form public operation count of fixed_modulus_hnf(K, k), and the limb tables.

formula(K, k) is derived by hand in REPORT.md section 4 from the loop structure;
test_counts.py checks it equals the count measured by the instrumented model.
"""
import sympy as sp
from math import ceil

CATS = ['abs', 'add', 'bit', 'bitlen', 'cmp', 'mul', 'neg', 'sel', 'shl_public', 'shl_secret',
        'shr_public', 'signbit', 'sub']


def formula(K, k):
    F = 2 * K - 1
    rho = 4 * k + 6 * k * F + 4                    # number of Barrett reductions
    recip_it = 2 * K + 1
    back_it = 6 * K                                # 6 (c, l) pairs x K bits
    f = {}
    f['abs'] = rho
    f['mul'] = 2 * rho + 4
    f['shr_public'] = rho
    f['sub'] = 14 * k * F + back_it + 3 * rho + 4 + recip_it
    f['add'] = (recip_it + 1) + back_it
    f['cmp'] = 8 * k * F + 4 * k + 2 * rho + recip_it + back_it
    f['bit'] = 3 * rho + recip_it + back_it
    f['signbit'] = 4 * k * F + rho
    f['sel'] = 40 * k * F + 20 * k + 2 * rho + recip_it + back_it
    f['bitlen'] = 8 * k * F
    f['shl_secret'] = 10 * k * F
    f['neg'] = 10 * k * F
    f['shl_public'] = 2 * recip_it + 2 * back_it
    return f


def limb_mults(K, k):
    """Schoolbook limb products of every `mul`."""
    F = 2 * K - 1
    rho = 4 * k + 6 * k * F + 4
    L = lambda b: -(-b // 64)
    tight = rho - 4 * k                               # all Reds except the entry reductions
    return (tight * (L(2 * K) * L(2 * K) + L(K) * L(K))
            + 4 * k * (L(2 * K) * L(2 * K) + L(2 * K - 1) * L(K))
            + 4 * L(K) * L(K))


def symbolic():
    K, k = sp.symbols('K k')
    F = 2 * K - 1
    rho = sp.expand(4 * k + 6 * k * F + 4)
    return {'Barrett reductions': rho,
            'pair steps': sp.expand(4 * k * F)}


if __name__ == '__main__':
    print(symbolic())
