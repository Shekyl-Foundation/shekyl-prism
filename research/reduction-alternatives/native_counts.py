#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Integer inversion of the O0 certificate bounds; all norms in nrd units."""

import csv
from fractions import Fraction
from math import isqrt
from pathlib import Path
import sys

from check_native import STEPS, params

ROOT = Path(__file__).resolve().parents[2]
OLD = (Fraction(31, 100), Fraction(87, 1000))
THREE = Fraction(47, 2000)


def schedule(phi_sq_over_delta, norm_bound, rates):
    # best[k] contracts k links. Each final block can follow any certified prefix.
    best = [Fraction(1)]
    while best[-1] ** 2 * phi_sq_over_delta >= 1000:
        k = len(best)
        best.append(min(best[k - i] * rho for i, rho in enumerate(rates, 1) if i <= k))
    exact = max(1, len(best) - 1) + 5
    extra = 0
    while 4 ** (58 * (extra + 1) + 2) < norm_bound:
        extra += 1
    return exact, exact + extra


def secret_norm(name):
    vectors = (ROOT / f"prism-vectors/data/signer/{name}.txt").read_text().splitlines()
    (d,) = [
        int.from_bytes(bytes.fromhex(row.split()[1]), "little")
        for row in vectors
        if row.startswith("secret_norm ")
    ]
    return d


def input_bounds(name):
    info = params(name)
    p, e = info["p"], info["e"]
    n, q = 1 << info["B"], 1 << (2 * e - 5)
    for label, t in (
        ("signing", n * q),
        ("norm-D", secret_norm(name)),
        ("secret-pre", n),
        ("challenge", q),
    ):
        yield label, Fraction(16 * t * t, p), 2 * t * t + p
    yield "guard-256", 16 * 256**2 * q * q, (1 + q) ** 2 * 256 * (1 + isqrt(p)) * n


def main():
    out = csv.writer(sys.stdout, lineterminator="\n")
    out.writerow(("set", "class", "old_exact", "old_exp64", "new_exact", "new_exp64"))
    for name in STEPS:
        for label, phi, bound in input_bounds(name):
            old = schedule(phi, bound, OLD)
            new = schedule(phi, bound, (*OLD, THREE))
            if label == "signing" and old[1] != STEPS[name]:
                raise RuntimeError((name, old, STEPS[name]))
            out.writerow((name, label, *old, *new))


if __name__ == "__main__":
    main()
