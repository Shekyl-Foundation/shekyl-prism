#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Verify the native prototype's public input/output fixtures with the exact oracle.

Inputs and original terminal bases come from research commit 35c59c6,
research/normalized-gram/rust/vectors_ng/p500.txt. Keep the two honest HNFs,
three Q5 saturation cases and six guarded intersections represented here.
"""
from pathlib import Path
from stage2c_oracle import columns, norm_form, reduce_exact, signer_parameters


def main():
    _, p, _ = signer_parameters("p500")
    file = Path(__file__).resolve().parents[1] / "shekyl-prism/examples/stage2c_p500/vectors.txt"
    counts = {}
    for line in file.read_text().splitlines():
        cls, *fields = line.split()
        counts[cls] = counts.get(cls, 0) + 1
        t, *values = (int(x, 16) for x in fields)
        u, v = reduce_exact(p, tuple(values[:4]), tuple(values[4:8]))
        f = norm_form(p, columns(u, v), t)
        expected = [*u, *v, f[0][0], f[2][2], f[2][0], f[2][1]]
        if expected != values[8:]:
            raise ValueError(f"incorrect basis or form on {cls} fixture {counts[cls]}")
    if counts != {"sign-generic": 5, "prered-guard256": 6}:
        raise ValueError(f"missing P500 fixtures: {counts}")
    print(f"Exact P500 oracle checked {counts}")


if __name__ == "__main__":
    main()
