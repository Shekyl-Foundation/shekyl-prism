#!/usr/bin/env python3
"""Iteration bounds for divstep inversion at every Shekyl set and modulus class.

Recomputed from the theorem statements, not copied:
  BY2019 [Bernstein-Yang 2019, Thm 11.2, as restated in EUROCRYPT 2026 sec. 1]:
    delta0 = 1, |f|,|g| <= 2^b, b >= 46  ->  n >= floor((49 b + 80) / 17)
  BCHHMWWY26 [EUROCRYPT 2026, Thm 1, HOL Light]:
    delta0 = 1/2, f odd, 0 <= g <= f <= 2^b  ->  n >= ceil((9437 b + 1) / 4096)
Jumps: the prototype batches 62 divsteps per jump (secp256k1 layout), so
jumps = ceil(n / 62); the divsteps actually run are 62 * jumps.
"""
from fractions import Fraction
import math, sys

SETS = {  # name: (cofactor f, exponent e, coordinate limbs W, constructor limbs D)
    "P248": (5, 248, 11, 22), "P324": (3, 324, 14, 28), "P376": (65, 376, 16, 32),
    "P500": (27, 500, 21, 42), "P664": (17, 664, 27, 54),
}

def by2019(b):
    assert b >= 46
    return (49 * b + 80) // 17

def eurocrypt26(b):
    return -(-(9437 * b + 1) // 4096)

def classes(name):
    f, e, W, D = SETS[name]
    p = f * 2**e - 1
    L = p.bit_length()
    B = -(-L // 2) + 32            # geometry.rs: secret_norm_bits = ceil(L/2) + Q4_SLACK_BITS
    qbits = 2 * e - 5              # geometry.rs: degree_bits = 2e - 5 (exact bit length of q)
    nb = (L + 1) // 2 + 2          # qlapoti-redesign costmodel.py: bound on the qlapoti norm n
    return L, B, qbits, nb, W, D, e

def main():
    rows = []
    hdr = "| set | L = bits(p) | modulus class | b | BY2019 divsteps | 2026 divsteps | ratio | 62-jumps (2019 / 2026) |"
    print(hdr); print("|" + "---|" * 8)
    for name in SETS:
        L, B, qbits, nb, W, D, e = classes(name)
        for cls, b in [("q (challenge degree, public)", qbits),
                       ("n (qlapoti norm, secret)", nb),
                       ("N (secret key norm; no inverse site)", B)]:
            o, n = by2019(b), eurocrypt26(b)
            print(f"| {name} | {L} | {cls} | {b} | {o} | {n} | {n/o:.4f} | {math.ceil(o/62)} / {math.ceil(n/62)} |")
    # checks quoted in the paper, recomputed
    assert eurocrypt26(256) == 590, eurocrypt26(256)
    print()
    print("check: 2026 bound at b = 256 is", eurocrypt26(256), "(paper: 590)")
    print("asymptotic ratio 9437/4096 / (49/17) =", f"{(9437/4096)/(49/17):.4f}")
    print()
    print("| set | W limbs (bits) | D limbs (bits) | q bits | n bound | N bound B | Nq bits | 2N bits |")
    print("|" + "---|" * 8)
    for name in SETS:
        L, B, qbits, nb, W, D, e = classes(name)
        print(f"| {name} | {W} ({64*W}) | {D} ({64*D}) | {qbits} | {nb} | {B} | {B + qbits} | {B + 1} |")

if __name__ == "__main__":
    main()
