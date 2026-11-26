#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
# All rights reserved.
# BSD-3-Clause
"""Report paired timing statistics on the public Stage 2c corpus."""
import argparse
import csv
import gzip
from collections import defaultdict
from math import sqrt
from pathlib import Path
from statistics import mean, stdev

from check_ct_shift import SET_WIDTHS


def summarize(path, pairs):
    groups = defaultdict(list)
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as stream:
        for row in csv.reader(stream):
            if row[:1] != ["timing"]:
                continue
            _, name, cls, index, case, order, fixed, varied = row
            data = groups[name, cls]
            if (int(index) != len(data) or order not in ("true", "false") or
                    not 1 <= int(case) < {"Generic": 5, "Guarded": 6}[cls] or min(int(fixed),int(varied)) <= 0):
                raise ValueError("invalid paired observation")
            data.append(int(varied)-int(fixed))
    if set(groups) != {(name, cls) for name in SET_WIDTHS for cls in ("Generic","Guarded")}:
        raise ValueError("incomplete five-set timing corpus")
    for (name, cls), data in groups.items():
        if len(data) != pairs or len(data) < 2 or not stdev(data):
            raise ValueError(f"invalid sample for {name} {cls}")
        print(f"{name} {cls}: {len(data)} pairs, mean difference {mean(data):.0f} ns, "
              f"paired t={mean(data)*sqrt(len(data))/stdev(data):.2f}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("samples", type=Path)
    parser.add_argument("--pairs", type=int, required=True)
    args = parser.parse_args()
    summarize(args.samples, args.pairs)
