#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Reproduce integer bounds in docs/SECURITY_ARGUMENT.md, without building Rust.

This checks the pinned parameter layout, not primality, isogeny soundness,
prime density, or a cryptographic security level. A changed source layout
requires reviewing this reader; an empty or partial scan is an error.
"""

import math
from pathlib import Path
import re


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    text = (root / "prism-field/src/lib.rs").read_text()
    for declaration in (
        "const CHAIN_STEPS: u32 = $e - 2;",
        "const CHALLENGE_BITS: u32 = $e - 3;",
        "const SIGNATURE_ELEMENTS: usize = 5;",
    ):
        if text.count(declaration) != 1:
            raise ValueError(f"parameter declaration changed: {declaration}")
    rows = re.findall(
        r'parameter_set!\(\s*p(\d+),\s*P\d+,\s*f = (\d+),\s*e = (\d+),'
        r'\s*FpP\d+,\s*Fp2P\d+,'
        r'\s*\[(.*?)\]\s*\);',
        text,
        re.S,
    )
    if [(row[0], row[2]) for row in rows] != [("248", "248"), ("376", "376"), ("500", "500")]:
        raise ValueError("expected exactly the three parameter sets in source order")
    # The collision target and the row name each row of the published
    # argument was written against. Both are inputs of this analysis and of
    # the table in docs/SECURITY_ARGUMENT.md, keyed by the exponent. No Rust
    # code carries either: the category a prime is assessed at is
    # documentation, and the row name is the string that argument used.
    targets = {"248": 128, "376": 192, "500": 256}
    names = {"248": "NIST-I", "376": "NIST-III", "500": "NIST-V"}
    rows = [(names[e], targets[e], e, limbs, f) for _name, f, e, limbs in rows]
    cofactors = {e: f for _label, _target, e, _limbs, f in rows}
    print("| Row | e | r | free bits | q bits | N² > 4p | 4q < N² | rho threshold |")
    print("| --- | ---: | ---: | ---: | ---: | --- | --- | --- |")
    for name, target, exponent, limbs, _f in rows:
        e, target = int(exponent), int(target)
        values = [int(x, 16) for x in re.findall(r"0x([0-9A-F]+)u64", limbs)]
        if len(values) != math.ceil(e / 64):
            raise ValueError(f"unexpected field limb count for {name}")
        p = sum(value << (64 * i) for i, value in enumerate(values))
        if p != (int(cofactors[exponent]) << e) - 1:
            raise ValueError(f"field modulus/cofactor disagree for {name}")
        r = e - 2
        n = 1 << e
        d = 1 << (2 * r - 1)
        maximum_s = (1 << (r - 1)) - 1
        q_min, q_max = d - maximum_s**2, d - 1
        # Integer upper bound avoids floating point in the degree-norm proof.
        norm_upper = (math.isqrt(d) + 1 + maximum_s) ** 2
        if not (n * n > 4 * p and norm_upper < n * n and 4 * q_max < n * n):
            raise ValueError(f"acceptance inequalities fail for {name}")
        if not (q_min > p and q_min.bit_length() == q_max.bit_length() == 2 * r - 1):
            raise ValueError(f"degree interval changed for {name}")
        if not (q_min % 8 == q_max % 8 == 7):
            raise ValueError(f"odd challenge endpoints changed for {name}")
        free = r - 2
        # sqrt(2^free/rho) >= 2^target requires rho <= 2^(free-2*target).
        threshold = free - 2 * target
        print(f"| {name} | {e} | {r} | {free} | {2*r-1} | yes | yes | 2^{threshold} |")
    print("Exact integer bounds passed. No primality or security-level claim checked.")


if __name__ == "__main__":
    main()
