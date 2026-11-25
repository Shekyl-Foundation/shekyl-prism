#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Run native_gram against the generic signing row for each set."""

import argparse
from pathlib import Path
import subprocess
import sys

from native_counts import OLD, THREE, STEPS, input_bounds, schedule


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["check", "bench", "memcheck"])
    parser.add_argument("binary", type=Path)
    parser.add_argument("vectors", type=Path)
    args = parser.parse_args()
    names = list(STEPS)
    passes = 3 if args.mode == "bench" else 1
    if args.mode == "bench":
        print("pass,set,kernel,steps,median_ns")
    for index in range(passes):
        for name in names[index:] + names[:index]:
            _, phi, bound = next(
                row for row in input_bounds(name) if row[0] == "signing"
            )
            old = schedule(phi, bound, OLD)[1]
            new = schedule(phi, bound, (*OLD, THREE))[1]
            cmd = [
                str(args.binary.resolve()),
                args.mode,
                name,
                str(args.vectors),
                str(old),
                str(new),
            ]
            if args.mode == "bench":
                result = subprocess.run(cmd, check=True, capture_output=True, text=True)
                for line in result.stdout.splitlines():
                    print(f"{index + 1},{line}", flush=True)
            elif args.mode == "memcheck":
                probe = ["valgrind", "--error-exitcode=77", *cmd]
                subprocess.run(probe, check=True)
                if index == 0 and name == names[0]:
                    control = subprocess.run([*probe, "control"])
                    if control.returncode != 77:
                        sys.exit(
                            f"positive control exited {control.returncode}, expected 77"
                        )
            else:
                subprocess.run(cmd, check=True)


if __name__ == "__main__":
    main()
