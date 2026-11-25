#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Differential checks on public O0 fixtures for the integral class Gram matrix."""
import sys
from pathlib import Path

RESEARCH = Path(__file__).resolve().parents[1]
EXPERIMENT = RESEARCH / "clamped-quotient-progress"
sys.path.insert(0, str(EXPERIMENT))
from q5common import N4, herm, params, zi_mul
from variants import q_exp64, run

# Generic signing rows from SIGNER_SPEC 2.3, PR #45 at 7b6dc21.
STEPS = {"p248": 312, "p324": 401, "p376": 463, "p500": 609, "p664": 803}


def class_gram(u, v, p, t):
    entries = [N4(u, p), N4(v, p), *herm(v, u, p)]
    if any(n % (2*t) for n in entries):
        raise ValueError("nonintegral class Gram matrix")
    return tuple(n // (2*t) for n in entries)


def check_one(p, t, x, y, steps):
    # Lift each residue to its O0 parity in [0, 2T).
    u = [2*t, 0, 0, 0]
    v = [x + (x % 2)*t, y + ((y+1) % 2)*t, 1, 0]
    ref = run(u, v, p, steps, "exact")
    a, c, h0, h1 = class_gram(u, v, p, t)
    if c < a:
        u, v, a, c, h1 = v, u, c, a, -h1
    width = max(a, c, abs(h0), abs(h1)).bit_length()
    raw = (max(a, c, abs(h0), abs(h1)) * 2*t).bit_length()
    for _ in range(steps):
        assert a*c - h0*h0 - h1*h1 == p
        m0, m1, _ = q_exp64(h0, h1, a)
        r0, r1 = h0 - m0*a, h1 - m1*a
        c -= m0*(h0 + r0) + m1*(h1 + r1)
        h0, h1 = r0, r1
        v = [x - y for x, y in zip(v, zi_mul(m0, m1, u))]
        if c < a:
            u, v, a, c, h1 = v, u, c, a, -h1
        assert class_gram(u, v, p, t) == (a, c, h0, h1)
    assert (u, v) == (ref["u"], ref["v"])
    assert a <= c and all(-a <= 2*h < a for h in (h0, h1))
    return raw, width


def main():
    for name, steps in STEPS.items():
        par = params(name)
        p = par["p"]
        inputs = set()
        widths = []
        for line in (EXPERIMENT / "rust-xcheck" / "vectors" / (name + ".txt")).read_text().splitlines():
            _, t, x, y, *_ = line.split()
            t, x, y = (int(n, 16) for n in (t, x, y))
            if t <= 0 or t % 2 == 0 or t >= 1 << par["Tbits"]:
                continue
            x, y = x % t, y % t
            if (x*x + y*y + p) % t:
                continue
            inputs.add((t, x, y))
        assert inputs
        for t, x, y in sorted(inputs):
            widths.append(check_one(p, t, x, y, steps))
        print(name, len(inputs), "distinct lifts; max initial Gram bits (raw, normalized):",
              max(w[0] for w in widths), max(w[1] for w in widths))


if __name__ == "__main__":
    main()
