#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The secret norms of the signer's constants, checked with sympy.

`scripts/signer_constants.py` finds each set's secret norm, the next
prime after `2^(2 * WIDTH_BITS)`, with its own Baillie-PSW test and no
library. This asks a second implementation the same question: sympy's
`nextprime` of the same power of two must be the committed value, at all
five sets.

The nightly runs it, with sympy 1.14.0. It is not a proof of primality:
sympy's test at these sizes is a probable-prime test too. A proof is
ledger O19's.
"""
import sys
from pathlib import Path

import sympy

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "prism-vectors" / "data" / "signer"
SETS = ("p248", "p324", "p376", "p500", "p664")


def row(text, name):
    found = [line.split(" ", 1)[1] for line in text.splitlines() if line.startswith(name + " ")]
    if len(found) != 1:
        raise SystemExit(f"{len(found)} rows `{name}`")
    return found[0]


def main():
    checked = 0
    for name in SETS:
        text = (DATA / f"{name}.txt").read_text()
        e = int(row(text, "e"))
        norm = int.from_bytes(bytes.fromhex(row(text, "secret_norm")), "little")
        width = (e + 127) // 128 * 128
        expected = sympy.nextprime(1 << (2 * width))
        if norm != expected:
            print(f"FAIL: {name}: the secret norm is not sympy's next prime after 2^{2 * width}", file=sys.stderr)
            return 1
        print(f"{name}: 2^{2 * width} + {norm - (1 << (2 * width))}")
        checked += 1
    if checked != len(SETS):
        return 1
    print(f"sympy {sympy.__version__} agrees on the secret norm of {checked} sets")
    return 0


if __name__ == "__main__":
    sys.exit(main())
