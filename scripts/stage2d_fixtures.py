#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
# All rights reserved.
# BSD-3-Clause
"""Recover producer inputs for the committed guarded intersections using public integers."""
import argparse
from collections import Counter
from pathlib import Path
from math import gcd, isqrt

from stage2c_fixtures import SET_WIDTHS, VECTORS
from stage2c_oracle import combine, half_up_quotient, inner, native_schedules, signer_parameters, times_i


def root(w, modulus):
    a, b, c, d = w
    inv = pow(c*c+d*d, -1, modulus)
    x = (a*c+b*d)*inv % modulus
    y = (b*c-a*d)*inv % modulus
    return x + (x % 2)*modulus, y + (1-y % 2)*modulus


def inputs(name):
    e, p, _ = signer_parameters(name)
    for row in (VECTORS / f"{name}.txt").read_text().splitlines():
        cls, *fields = row.split()
        if cls != "prered-guard256":
            continue
        t, *coords = [int(v, 16) for v in fields[:9]]
        q = gcd(t, *coords[:4])
        n = t // q
        v = tuple(x//q for x in coords[:4])
        z = tuple(coords[4:])
        c = inner(p, v, v)
        g, h = (half_up_quotient(inner(p, z, w), c) for w in (v, times_i(v)))
        u = combine((z, v, times_i(v)), (1, -g, -h))
        xn, yn = root(v, n)
        xq, yq = root(z, q)
        for x, y, m in ((xn, yn, n), (xq, yq, q)):
            assert (x*x+y*y+p) % (4*m) == 0
        assert q % 4 == 3 and gcd(n, q) == 1
        square = (1 << (2*e-5))-q
        scalar = isqrt(square)
        assert scalar**2 == square and scalar.bit_length() <= e-3
        # q divides every nonzero reduced norm in the challenge ideal.
        # Even the largest norm allowed by Q4 and the guard lies below q.
        nmax = 1 << ((p.bit_length()+1)//2+32)
        assert (256*nmax)**2*p < (1 << (2*e-6))**2
        yield " ".join(format(value, "x") for value in (n, xn, yn, *u, *v, scalar, xq, yq))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--native-log", type=Path)
    args = parser.parse_args()
    expected = []

    for name in SET_WIDTHS:
        rows = list(inputs(name))
        if len(rows) != 6:
            raise ValueError(f"{name} has {len(rows)} guarded inputs")
        text = "\n".join(rows) + "\n"
        path = VECTORS.parent / "ideals" / f"{name}.txt"
        count = native_schedules(name)["Secret ideal pre-reduction, Q4"][1]
        expected.append(f"stage2d {name} secret_steps={count} guarded_intersections={len(rows)}")
        if args.write:
            path.parent.mkdir(exist_ok=True)
            path.write_text(text)
        elif path.read_text() != text:
            raise ValueError(f"{name} ideal inputs differ")
    if args.native_log:
        actual = [s for s in args.native_log.read_text().splitlines() if s.startswith("stage2d ")]
        if Counter(actual) != Counter(expected):
            raise ValueError(f"native secret-ideal schedule or corpus differs: {actual}")
