#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Check the five native reducer corpora and their schedules against the exact oracle.

Each vectors/<set>.txt retains the first two honest HNFs, all three Q5 saturation
cases and all six guarded intersections from research commit 35c59c6,
research/normalized-gram/rust/vectors_ng/<set>.txt.
"""
import argparse
from collections import Counter
from pathlib import Path
from stage2c_oracle import columns, native_schedules, norm_form, reduce_exact, signer_parameters

VECTORS = Path(__file__).resolve().parents[1] / "shekyl-prism/examples/stage2c/vectors"
SET_WIDTHS = {"p248": 11, "p324": 14, "p376": 16, "p500": 21, "p664": 27}


def verify_set(name, width):
    _, p, _ = signer_parameters(name)
    counts = Counter()
    for line in (VECTORS / f"{name}.txt").read_text().splitlines():
        cls, *fields = line.split()
        counts[cls] += 1
        t, *values = (int(x, 16) for x in fields)
        u, v = reduce_exact(p, tuple(values[:4]), tuple(values[4:8]))
        f = norm_form(p, columns(u, v), t)
        expected = [*u, *v, f[0][0], f[2][2], f[2][0], f[2][1]]
        if expected != values[8:]:
            raise ValueError(f"incorrect basis or form on {name} {cls} fixture {counts[cls]}")
    if counts != {"sign-generic": 5, "prered-guard256": 6}:
        raise ValueError(f"missing {name} fixtures: {counts}")
    print(f"Exact {name} oracle checked {dict(counts)}")
    schedules = native_schedules(name)
    guarded = schedules["Signing, pre-reduced, guard 256"][1]
    generic = schedules["Signing, generic HNF"][1]
    return f"stage2c {name} limbs={width} guarded={guarded} generic={generic} bases={sum(counts.values())}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native-log", type=Path, help="check actual native runs against the proof rows")
    args = parser.parse_args()
    expected = [verify_set(name, width) for name, width in SET_WIDTHS.items()]
    if args.native_log:
        actual = [line for line in args.native_log.read_text().splitlines() if line.startswith("stage2c ")]
        if Counter(actual) != Counter(expected):
            raise ValueError(f"native coverage differs: expected {expected}, got {actual}")


if __name__ == "__main__":
    main()
