#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Summarize real first-prime searches from sample_e_prime (unsalted v1).

Generate with: cargo run --release --locked -p prism-compare --bin
sample_e_prime -- 1500 > samples.txt
Then: python3 scripts/measure_unsalted_density.py samples.txt

Each output line is one successful search, NOT one candidate. The number
of tested candidates is sum(j+1). This measures the implemented BPSW
predicate, not proven primality or native signing acceptance.
"""

import argparse
import json
import math
from pathlib import Path

ROWS = {"NIST-I": (248, 128), "NIST-III": (376, 192), "NIST-V": (500, 256)}


def summarize(text: str, count: int) -> list[dict]:
    rows = {name: [] for name in ROWS}
    for line in text.splitlines():
        if not line.strip():
            continue
        name, exponent, index, scalar_hex, degree_hex = line.split()
        e, _ = ROWS[name]
        r, j = int(exponent), int(index)
        scalar = int.from_bytes(bytes.fromhex(scalar_hex), "little")
        degree = int.from_bytes(bytes.fromhex(degree_hex), "little")
        if r != e - 2 or not 0 <= j < 65536:
            raise ValueError("unexpected exponent/index")
        if not (0 < scalar < 1 << (r - 1) and scalar & 1):
            raise ValueError("scalar outside the verifier challenge domain")
        if degree != (1 << (2 * r - 1)) - scalar * scalar:
            raise ValueError("reported degree disagrees with scalar")
        rows[name].append(j)
    result = []
    for name, indices in rows.items():
        if len(indices) != count or count < 1:
            raise ValueError(f"{name}: expected {count} successful searches, got {len(indices)}")
        e, nominal = ROWS[name]
        candidates = sum(j + 1 for j in indices)
        density = count / candidates
        exponent = ((e - 4) - math.log2(density)) / 2
        result.append(dict(row=name, successful_searches=count,
                           tested_candidates=candidates, density=density,
                           free_bits=e - 4, collision_log2_queries=exponent,
                           nominal=nominal, below_nominal=exponent < nominal))
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("samples", type=Path)
    parser.add_argument("--count", type=int, default=1500)
    parser.add_argument("--emit-samples", action="store_true")
    args = parser.parse_args()
    text = args.samples.read_text()
    print(json.dumps({"predicate": "implemented BPSW", "rows": summarize(text, args.count)}, indent=2))
    if args.emit_samples:
        for line in text.splitlines():
            if line.strip():
                print("SAMPLE " + line)


if __name__ == "__main__":
    main()
