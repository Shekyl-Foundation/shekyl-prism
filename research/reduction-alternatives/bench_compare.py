#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Rotate release xq binaries over the same honest fixtures and emit their medians."""
import argparse
import csv
import os
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cpu", type=int, required=True)
    parser.add_argument("binaries", nargs="+", metavar="name=path")
    args = parser.parse_args()
    binaries = []
    for spec in args.binaries:
        name, path = spec.split("=", 1)
        binaries.append((name, Path(path).resolve(strict=True)))
    fixture_dir = Path(__file__).resolve().parents[1] / "clamped-quotient-progress" / "rust-xcheck"
    out = csv.writer(sys.stdout, lineterminator="\n")
    out.writerow(("set", "pass", "variant", "steps", "exp64_median_tsc_ticks", "clamp_control_median_tsc_ticks"))
    for index, size in enumerate(("p500", "p248", "p664", "p324", "p376")):
        for batch in range(3):
            order = list(reversed(binaries)) if batch == 1 else binaries
            offset = (index + batch) % len(order)
            for name, binary in order[offset:] + order[:offset]:
                result = subprocess.run([binary, "bench", size, "41"], cwd=fixture_dir,
                                        env={**os.environ, "RLC_CPU": str(args.cpu)},
                                        text=True, capture_output=True, check=True)
                if f"pinned to cpu {args.cpu}: ok" not in result.stderr:
                    raise RuntimeError(result.stderr)
                measured, _, control, steps, median, *_ = result.stdout.strip().split(",")
                if measured != size:
                    raise ValueError(result.stdout)
                out.writerow((size, batch + 1, name, steps, median, control))
                sys.stdout.flush()


if __name__ == "__main__":
    main()
