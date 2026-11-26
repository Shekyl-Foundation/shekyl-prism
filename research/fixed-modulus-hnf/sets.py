# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Public parameters of the five Shekyl sets and the modulus classes of the HNF.

Mirrors research/stage2c-design-notes/widths.py and dev (470d537)
shekyl-prism/src/ideal/geometry.rs:  B = ceil(L/2) + 32 (Q4),  L = bitlen(p).
A class fixes the public bound K with m = 2*N_class < 2^K.
"""
# name, e, f, WIDTH_BITS, LIMBS, D - 2^(2 WIDTH_BITS), (prism-int ideal widths W, 2W) from dev ideal/mod.rs
SETS = [
    ('P248', 248, 5, 256, 26, 75, 11),
    ('P324', 324, 3, 384, 33, 183, 14),
    ('P376', 376, 65, 384, 38, 183, 16),
    ('P500', 500, 27, 512, 49, 643, 21),
    ('P664', 664, 17, 768, 65, 75, 27),
]
Q4_SLACK = 32


def params(name):
    for (n, e, f, wb, limbs, doff, iw) in SETS:
        if n == name:
            p = f * (1 << e) - 1
            L = p.bit_length()
            B = -(-L // 2) + Q4_SLACK
            qbits = 2 * e - 5                  # challenge degree q < 2^qbits
            Dn = (1 << (2 * wb)) + doff        # keygen norm D (widths.py)
            return dict(name=n, e=e, f=f, p=p, L=L, B=B, qbits=qbits, D=Dn,
                        LIMBS=limbs, W=iw, Wbits=wb)
    raise KeyError(name)


def classes(name):
    """Public K per class: m = 2N < 2^K.  N bound given alongside."""
    P = params(name)
    out = {}
    # decode / secret ideal / keygen secret ideal: Q4, N < 2^B
    out['ideal'] = dict(Nbits=P['B'], K=P['B'] + 1)
    # challenge ideal alone: N = q < 2^qbits
    out['challenge'] = dict(Nbits=P['qbits'], K=P['qbits'] + 1)
    # keygen norm D (about 2^(2 WIDTH_BITS)), N = D
    out['keygen-D'] = dict(Nbits=P['D'].bit_length(), K=P['D'].bit_length() + 1)
    # generic signing T = N q < 2^(B+qbits)
    out['sign-T'] = dict(Nbits=P['B'] + P['qbits'], K=P['B'] + P['qbits'] + 1)
    return P, out


if __name__ == '__main__':
    for s in SETS:
        P, c = classes(s[0])
        print(P['name'], 'L', P['L'], 'B', P['B'], {k: v['K'] for k, v in c.items()})
