#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Known answers for arithmetic modulo 2^e, from Python's integers.

The verifier's matrices have entries modulo `2^e` for the exponent `e` of
a parameter set (`prism-isogeny/src/modular.rs`, `Residue`). The matrices
themselves have known answers only at the three sets with a model file.
This script writes products, inverses and the powers the matrices are
built from, at every set, with nothing of this workspace in it: Python's
arbitrary-precision integers and `pow(s, -1, 2**e)`.

    residue_known_answers.py --check    what CI runs
    residue_known_answers.py --write    rewrite the file

A Rust test reads the file and holds `Residue` to every row.

The operands are chosen for the limbs: at each 64-bit boundary that
matters (the first, bit 512 where the earlier sets' eight limbs ended,
and the last one below `e`) there are values with every bit set below
the boundary and a bit above it, so that products carry across it; there
are products that land exactly on bit `e − 1` and on bit `e`, which is
zero; and there are random operands that fill the width.
"""
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "prism-vectors" / "data" / "isogeny" / "residue_known_answers.tsv"
# Every parameter set, by name, with its exponent.
SETS = {"p248": 248, "p324": 324, "p376": 376, "p500": 500, "p664": 664}
RANDOM_ROWS = 8


def boundaries(e):
    """The limb boundaries the operands are built around, below `e`."""
    last = (e - 1) // 64 * 64
    return sorted({64, last} | ({512} if e > 512 else set()))


def hexle(value, e):
    return value.to_bytes((e + 7) // 8, "little").hex()


def rows(name, e):
    modulus = 1 << e
    rng = random.Random(f"shekyl-prism-residue-{name}")
    pairs = [(modulus - 1, modulus - 1), (1 << (e - 1), 2), (1 << (e - 1), 3)]
    scalars = [1, 3, modulus - 1, (1 << (e - 1)) + 1]
    for b in boundaries(e):
        below, above = (1 << b) - 1, (1 << b) + 1
        pairs += [
            (below, above),
            (below, below),
            (above, above),
            ((1 << (e - 1)) + above, below),
            (1 << b, 1 << (e - 1 - b)),
            (1 << b, 1 << (e - b)),
        ]
        scalars += [above, (1 << (e - 1)) + above, modulus - below]
    for _ in range(RANDOM_ROWS):
        pairs.append((rng.getrandbits(e), rng.getrandbits(e)))
        scalars.append(rng.getrandbits(e) | 1)
    lines = []
    for a, b in pairs:
        lines.append("\t".join(["MUL", name, hexle(a, e), hexle(b, e), hexle(a * b % modulus, e)]))
    for s in scalars:
        t = pow(s, -1, modulus)
        assert s * t % modulus == 1
        lines.append(
            "\t".join(["INV", name, hexle(s, e), hexle(t, e), hexle(s * s % modulus, e), hexle(t * t % modulus, e)])
        )
    return lines


def text():
    header = [
        "# Arithmetic modulo 2^e, from scripts/residue_known_answers.py. Do not edit.",
        "# Python integers and pow(s, -1, 2**e); nothing of this workspace.",
        "# Values are little-endian hex, e bits in whole bytes.",
        "# MUL set a b a*b",
        "# INV set s 1/s s^2 (1/s)^2",
    ]
    body = [line for name, e in SETS.items() for line in rows(name, e)]
    return "\n".join(header + body) + "\n"


def main():
    if sys.argv[1:] not in (["--check"], ["--write"]):
        raise SystemExit("usage: residue_known_answers.py --check | --write")
    out = text()
    if sys.argv[1] == "--write":
        OUT.write_text(out)
        print(f"wrote {OUT.name}")
        return
    if not OUT.is_file() or OUT.read_text() != out:
        raise SystemExit(f"{OUT.name} is not what this script writes; run --write and commit it")
    counts = {name: sum(1 for line in out.splitlines() if line.split("\t")[1:2] == [name]) for name in SETS}
    print(f"{OUT.name} is current: " + ", ".join(f"{name} {count}" for name, count in counts.items()))


if __name__ == "__main__":
    main()
