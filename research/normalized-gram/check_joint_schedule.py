#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Replay the existing ng check corpus at SHE-23's candidate public counts."""
import argparse
import csv
from pathlib import Path
from shutil import copyfile
import subprocess
from tempfile import TemporaryDirectory

import gen_ng

HERE = Path(__file__).resolve().parent
VECTORS = HERE / "rust/vectors_ng"
COUNTS = HERE.parent / "tighter-progress-certificate/results/step_counts_new.csv"


def candidate_line(line, counts, name):
    fields = line.split()
    if fields[14] == "0":
        return line
    prime, cls, kind, shape = fields[:4]
    norm, *coords = [int(value, 16) for value in fields[4:13]]
    exact, exp64 = counts[name, cls]
    result, _ = gen_ng.reference(
        int(prime, 16), cls, kind, shape, norm, coords[:4], coords[4:], exp64, exact, None
    )
    new = result.split()
    # Only the public count and the two padded physical trace hashes can change.
    if new[:13] != fields[:13] or new[14:21] != fields[14:21] or new[23:] != fields[23:]:
        raise ValueError(f"{name} {cls} {kind}: candidate differs from pinned oracle")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ng", type=Path, help="path to the compiled ng binary")
    args = parser.parse_args()
    with COUNTS.open() as source:
        counts = {
            (row["set"], row["cls"]): (int(row["T_exact"]), int(row["T_exp64"]))
            for row in csv.DictReader(source)
        }
    with TemporaryDirectory() as temp:
        directory = Path(temp) / "vectors_ng"
        directory.mkdir()
        for name in gen_ng.n.NAMES:
            file = name + ".txt"
            rows = (VECTORS / file).read_text().splitlines()
            (directory / file).write_text(
                "".join(candidate_line(row, counts, name) + "\n" for row in rows)
            )
        copyfile(VECTORS / "toy.txt", directory / "toy.txt")
        subprocess.run([args.ng.resolve(), "check"], cwd=temp, check=True)


if __name__ == "__main__":
    main()
