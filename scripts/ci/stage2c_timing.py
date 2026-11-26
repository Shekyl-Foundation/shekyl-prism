#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Paired timing of each Stage 2c fixture against its class reference.

A row is one repeat of one contrast: the first corpus row of a class against
one later row, on one schedule. Even repeats run the reference first. The
paired t of a contrast uses that contrast's repeats as its sample. It does
not treat a mixture drawn with replacement as independent pairs.

The reference index and the class census live in `scripts/stage2c_fixtures.py`.
"""
import argparse
import csv
import gzip
import sys
from collections import defaultdict
from math import sqrt
from pathlib import Path
from statistics import mean, stdev

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage2c_fixtures import CLASS_COUNTS, REFERENCE_INDEX, SET_WIDTHS

MEASURED_SCHEDULE = "reviewed"
MIN_REPEATS = 2


def reference_runs_first(repeat):
    """Same rule as `reference_runs_first` in the stage2c example."""
    return repeat % 2 == 0


def load(path, repeats):
    """Differences keyed by `(set, class, schedule, other index)`.

    A row whose order, index, or population is not the contrast design is
    refused. Absence of a contrast is a refusal, not an empty mean.
    """
    if repeats < MIN_REPEATS:
        raise ValueError(f"a contrast needs at least {MIN_REPEATS} repeats")
    groups = defaultdict(list)
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as stream:
        for row in csv.reader(stream):
            if row[:1] != ["timing"]:
                continue
            try:
                _, name, cls, schedule, index, repeat, order, reference, other = row
                other_index = int(index)
                repeat_index = int(repeat)
                reference_ns = int(reference)
                other_ns = int(other)
            except ValueError as error:
                raise ValueError(f"invalid paired observation: {row}") from error
            if cls not in CLASS_COUNTS or schedule != MEASURED_SCHEDULE:
                raise ValueError(f"unexpected contrast {name} {cls} {schedule}")
            upper = CLASS_COUNTS[cls]
            if not REFERENCE_INDEX < other_index < upper:
                raise ValueError(f"other index {other_index} is outside {cls}")
            if repeat_index != len(groups[name, cls, schedule, other_index]):
                raise ValueError("repeats are not contiguous from zero")
            expected_order = "true" if reference_runs_first(repeat_index) else "false"
            if order != expected_order:
                raise ValueError("row order does not follow the even-repeat rule")
            if min(reference_ns, other_ns) <= 0:
                raise ValueError("non-positive sample")
            groups[name, cls, schedule, other_index].append(other_ns - reference_ns)
    expected = {
        (name, cls, MEASURED_SCHEDULE, index)
        for name in SET_WIDTHS
        for cls, count in CLASS_COUNTS.items()
        for index in range(REFERENCE_INDEX + 1, count)
    }
    if set(groups) != expected:
        raise ValueError("incomplete five-set contrast corpus")
    for key, samples in groups.items():
        if len(samples) != repeats or not stdev(samples):
            raise ValueError(f"invalid sample for {key}")
    return groups


def paired_t(samples):
    """Student's t for the mean of paired differences. `samples` has the repeats of one contrast."""
    return mean(samples) * sqrt(len(samples)) / stdev(samples)


def report(groups):
    """Print each contrast and the strongest |t| of each set and class."""
    by_cell = defaultdict(list)
    for (name, cls, schedule, index), samples in sorted(groups.items()):
        statistic = paired_t(samples)
        by_cell[name, cls, schedule].append((abs(statistic), statistic, index, samples))
        print(
            f"{name} {cls} {schedule} other={index}: {len(samples)} repeats, "
            f"mean difference {mean(samples):.0f} ns, paired t={statistic:.2f}"
        )
    summaries = []
    for (name, cls, schedule), contrasts in sorted(by_cell.items()):
        _, statistic, index, _ = max(contrasts)
        summaries.append((name, cls, statistic, index))
        # The label is the magnitude. The signed t is the contrast that attained it.
        print(
            f"{name} {cls} {schedule} max |t|={abs(statistic):.2f} "
            f"(t={statistic:.2f}) at other={index}"
        )
    return summaries


def self_test():
    """A two-repeat corpus passes. Reversed and malformed orders are refused."""
    import tempfile
    rows = []
    # Differences 1 and 3. The sample standard deviation is sqrt(2), so t is 2.
    for name in SET_WIDTHS:
        for cls, count in CLASS_COUNTS.items():
            for index in range(REFERENCE_INDEX + 1, count):
                for repeat, (reference, other) in enumerate(((10, 11), (10, 13))):
                    order = "true" if reference_runs_first(repeat) else "false"
                    rows.append([
                        "timing", name, cls, MEASURED_SCHEDULE, str(index), str(repeat),
                        order, str(reference), str(other),
                    ])
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "contrasts.csv"
        with path.open("w", newline="") as stream:
            csv.writer(stream).writerows(rows)
        groups = load(path, 2)
        statistic = paired_t(next(iter(groups.values())))
        if abs(statistic - 2.0) > 1e-9:
            raise AssertionError(f"paired t was {statistic}")
        for repeat, bad_order in ((0, "false"), (1, "unknown")):
            original = rows[repeat][6]
            rows[repeat][6] = bad_order
            with path.open("w", newline="") as stream:
                csv.writer(stream).writerows(rows)
            try:
                load(path, 2)
            except ValueError:
                rows[repeat][6] = original
            else:
                raise AssertionError(f"accepted order {bad_order!r} at repeat {repeat}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("samples", nargs="?", type=Path)
    parser.add_argument("--repeats", type=int)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    self_test()
    if args.self_test:
        print("stage2c timing self-test passed")
        return
    if args.samples is None or args.repeats is None:
        parser.error("samples and --repeats are required")
    report(load(args.samples, args.repeats))


if __name__ == "__main__":
    main()
