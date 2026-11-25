# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Shared definitions for the Q5 width study.

Parameter sets (the only Shekyl sets): P248, P324, P376, P500, P664.
p = f * 2^e - 1, L = bits(p), W = WIDTH_BITS, B = ceil(L/2) + 32 (Q4),
LIMBS from SIGNER_SPEC 2.1.2 (prism-int width = 64 * LIMBS bits of
magnitude, sign held beside).

Two coordinate systems, one algorithm:
  * 'model': the research models' order Z<1,i,j,k>; integer coordinates,
    form N4(x) = x0^2 + x1^2 + p (x2^2 + x3^2) = nrd(x).
  * 'O0': the signer's maximal order <1, i, (i+j)/2, (1+k)/2> held as
    numerators over 2 (SIGNER_SPEC Q1); the same integer form N4 on the
    numerators equals 4 nrd. This is what a prism-int port stores.
Left multiplication by i is (x0,x1,x2,x3) -> (-x1, x0, -x3, x2) in both.
"""
import math

SETS = [  # name, e, f, W, LIMBS (SIGNER_SPEC 2.1.2), T_prov(sec), T_prov(I_q), T_prov(pre-reduced, guarded)
    ('p248', 248, 5, 256, 26, 301, 211, 283),
    ('p324', 324, 3, 384, 33, 387, 276, 369),
    ('p376', 376, 65, 384, 38, 446, 319, 428),
    ('p500', 500, 27, 512, 49, 587, 425, 569),
    ('p664', 664, 17, 768, 65, 773, 565, 755),
]

# T_prov for the unguarded pre-reduced variant (keygen-precompute REPORT 1.2)
TPROV_PRE_UNGUARDED = {'p248': 354, 'p324': 461, 'p376': 536, 'p500': 712, 'p664': 945}

# Rust prototype widths (research/rust-lagrange-ct/src/sets.rs): C coordinate
# limbs (two's complement), N = 2C norm limbs.
RUST_C = {'p248': 11, 'p324': 14, 'p376': 16, 'p500': 21, 'p664': 27}


def params(name):
    for s in SETS:
        if s[0] == name:
            _, e, f, W, LIMBS, tp, tq, tpre = s
            p = f * (1 << e) - 1
            L = p.bit_length()
            B = (L + 1) // 2 + 32
            return dict(name=name, e=e, f=f, p=p, L=L, W=W, B=B, LIMBS=LIMBS,
                        width=64 * LIMBS, qbits=2 * e - 5, Tbits=B + 2 * e - 5,
                        tprov_sec=tp, tprov_q=tq, tprov_pre=tpre,
                        tprov_pre_ung=TPROV_PRE_UNGUARDED[name])
    raise KeyError(name)


def N4(v, p):
    return v[0] * v[0] + v[1] * v[1] + p * (v[2] * v[2] + v[3] * v[3])


def herm(v, u, p):
    """Z[i]-part (h0, h1) of v * conj(u) in the N4 scaling."""
    h0 = v[0] * u[0] + v[1] * u[1] + p * (v[2] * u[2] + v[3] * u[3])
    h1 = v[1] * u[0] - v[0] * u[1] + p * (v[3] * u[2] - v[2] * u[3])
    return h0, h1


def times_i(u):
    return [-u[1], u[0], -u[3], u[2]]


def zi_mul(c0, c1, u):
    """(c0 + c1 i) * u, left multiplication."""
    return [c0 * u[0] - c1 * u[1], c0 * u[1] + c1 * u[0],
            c0 * u[2] - c1 * u[3], c0 * u[3] + c1 * u[2]]


def bits(x):
    return abs(int(x)).bit_length()


def isqrt_ceil(n):
    r = math.isqrt(n)
    return r if r * r == n else r + 1
