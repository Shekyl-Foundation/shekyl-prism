#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Convert the committed whole-process runV2 statistics to G7 JSON on stdout.

Run in a checkout with the pinned research object available. No raw timing
data, R installation, network or floor device is needed.
"""

import csv
import json
import subprocess
from pathlib import PurePosixPath

SOURCE = "1cc5d2650cb017ddf2bab7587b404b58db05f8c8:research/g7-rtlf/v2/results/runV2/"
PHASES = {"adm": "reducer_constructor", "herm": "hermitian", "full": "full"}


def source(name):
    return subprocess.check_output(["git", "show", SOURCE + name], text=True)


def convert():
    configs = {p: dict(levels_ns=[100, 200, 400], largest_ns=250) for p in PHASES.values()}
    configs["full"] = dict(levels_ns=[1600, 3200, 6400], largest_ns=2000)
    config = dict(tick_ns=18.5, planned=3, rounds=8192, replacement_cap=0,
                  seeds=["101", "102", "103"], fixtures=["f0", "f1", "f2", "f3", "f4"],
                  phases=configs)
    tests = {}
    for row in csv.DictReader(source("runV2_a01.tsv").splitlines(), delimiter="\t"):
        name = PurePosixPath(row["file"]).name
        if name in tests:
            raise ValueError(f"duplicate RTLF row {name}")
        tests[name] = row
    processes = {}
    for r in json.loads(source("manifest.json")):
        if r["span"] != "all":
            continue
        metadata = dict(id=r["src"], seed=r["seed"], reference=r["ref"])
        process = processes.setdefault(r["src"], dict(**metadata, records=[]))
        if any(process[k] != v for k, v in metadata.items()):
            raise ValueError("inconsistent process metadata")
        phase = PHASES[r["phase"]]
        kind, arm = r["kind"], r["arm"]
        versus = {"loo": "others", "keys": "kfix"}.get(kind, r["ref"])
        if r["versus"] != versus:
            raise ValueError("unexpected comparator")
        direction = 1
        if kind == "inject":
            kind = "delay"
            arm = str(configs[phase]["levels_ns"][["injL", "injM", "injH"].index(arm)])
        elif kind == "keys" and arm == "krand":
            arm, direction = "fixed-random", -1
        elif kind == "null" and arm == "null":
            arm = r["ref"]
        else:
            kind = {"loo": "pooled", "fixture": "reference"}[kind]
        test = tests.pop(PurePosixPath(r["file"]).name)
        deciles = [int(d) for d in test["sig_deciles"].split(";") if d]
        diffs = dict(zip(range(10, 100, 10), map(float, test["diff"].split(";")), strict=True))
        if int(test["n"]) != r["n"] or int(test["detect"]) != bool(deciles):
            raise ValueError("RTLF count or detection disagrees with record")
        process["records"].append(dict(phase=phase, kind=kind, arm=arm, count=r["n"],
            rtlf=bool(deciles), flagged_ns=max((abs(diffs[d]) for d in deciles), default=0),
            t=direction*r["t_welch"], median_ns=direction*r["median_diff_ns"],
            mean_ns=direction*r["mean_diff_ns"]))
    if any(name.endswith("__all.csv") for name in tests):
        raise ValueError("unmatched whole-process RTLF record")
    return dict(config=config, processes=sorted(processes.values(), key=lambda p: int(p["seed"])))


if __name__ == "__main__":
    print(json.dumps(convert(), indent=2, allow_nan=False))
