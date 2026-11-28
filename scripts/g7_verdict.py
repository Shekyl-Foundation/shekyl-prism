#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Apply SIGNER_SPEC section 0 to whole-process statistics. See docs/SIGNER_SPEC.md.

Read one JSON file, optionally followed by --rerun FILE. Exit 0 for a completed
observation, 1 for a leak, pending candidate or inconclusive contrast, and 2
for refused input. Noise and a candidate that was not reproduced do not fail.
"""

import argparse
import hashlib
import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path


class Refused(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Refused(message)


def object_pairs(pairs):
    result = {}
    for name, value in pairs:
        require(name not in result, f"duplicate JSON field {name}")
        result[name] = value
    return result


def fields(value, names):
    require(isinstance(value, dict) and set(value) == set(names.split()),
            f"expected fields {names}")


def number(value):
    require(type(value) in (float, int) and math.isfinite(value), "expected finite number")
    return value


def integer(value, minimum):
    require(type(value) is int and value >= minimum, f"expected integer >= {minimum}")
    return value


def names(values, minimum):
    require(isinstance(values, list) and len(values) >= minimum and
            all(type(v) is str and v.strip() for v in values) and len(set(values)) == len(values),
            "expected distinct, nonempty names")


def config_hash(config):
    packed = json.dumps(config, sort_keys=True, separators=(",", ":"), allow_nan=False)
    return hashlib.sha256(packed.encode("ascii")).hexdigest()


def check_config(c):
    fields(c, "tick_ns planned rounds replacement_cap seeds fixtures phases")
    require(number(c["tick_ns"]) > 0, "tick must be positive")
    planned = integer(c["planned"], 3)
    cap = integer(c["replacement_cap"], 0)
    integer(c["rounds"], 4096)
    names(c["seeds"], planned)
    require(len(c["seeds"]) == planned + cap, "plan one seed per launch, including reserves")
    names(c["fixtures"], 2)
    require(isinstance(c["phases"], dict) and c["phases"], "expected phases")
    names(list(c["phases"]), 1)
    for phase in c["phases"].values():
        fields(phase, "levels_ns largest_ns")
        require(number(phase["largest_ns"]) > 0, "largest resolution must be positive")
        levels = phase["levels_ns"]
        require(isinstance(levels, list) and len(levels) == 3 and
                all(type(v) is int and v > 0 for v in levels) and
                sorted(set(levels)) == levels, "expected three increasing nominal delays in ns")


@dataclass(frozen=True)
class Flag:
    decile: bool
    welch: bool
    sign: int


def flag(record, tick):
    direction = record["median_ns"] or record["mean_ns"]
    return Flag(record["rtlf"] and record["flagged_ns"] >= 3 * tick,
                abs(record["t"]) > 4.5, (direction > 0) - (direction < 0))


def support(flags):
    """Number of flagged processes compatible with each direction."""
    return {sign: sum((f.decile or f.welch) and f.sign in (0, sign) for f in flags)
            for sign in (-1, 1)}


def classify(flags, resolution, largest):
    both = [f for f in flags if f.decile and f.welch]
    if max(support(both).values()) >= 2:
        return "Leak"
    counts = support(flags)
    if max(counts.values()) >= 2 or both:
        return "Candidate"
    if max(counts.values()):
        return "Noise"
    if resolution is not None and resolution <= largest:
        return "Pass"
    return "Inconclusive"


def repeated(first, again, rerun_verdict):
    first_signs = {s for s, n in support(first).items() if n >= 2}
    # A both-tests flag is itself sufficient evidence for a candidate sign.
    if not first_signs:
        for f in first:
            if f.decile and f.welch:
                first_signs.update(s for s in (-1, 1) if f.sign in (0, s))
    if rerun_verdict == "Leak":
        return "Leak"
    if any(support(again)[s] >= 2 for s in first_signs):
        return "Persistent leak"
    return "Not reproduced"


def targets(fixtures):
    return [("pooled", arm) for arm in fixtures] + [("keys", "fixed-random")]


def records(process, c):
    fixtures = c["fixtures"]
    ref = process["reference"]
    require(ref in fixtures, "unknown process reference")
    expected = {(phase, kind, arm) for phase, spec in c["phases"].items()
                for kind, arm in (targets(fixtures) + [("null", ref)] +
                                  [("reference", f) for f in fixtures if f != ref] +
                                  [("delay", str(n)) for n in spec["levels_ns"]])}
    indexed = {}
    require(isinstance(process["records"], list), "expected list of records")
    for r in process["records"]:
        fields(r, "phase kind arm rtlf flagged_ns t median_ns mean_ns count")
        key = r["phase"], r["kind"], r["arm"]
        require(all(type(x) is str for x in key) and key in expected and key not in indexed,
                f"unexpected or duplicate record {key}")
        require(integer(r["count"], 1) == c["rounds"], f"{key}: incomplete rounds")
        for name in ("t", "median_ns", "mean_ns", "flagged_ns"):
            number(r[name])
        require(type(r["rtlf"]) is bool and r["flagged_ns"] >= 0 and
                (r["rtlf"] or r["flagged_ns"] == 0), f"{key}: invalid RTLF statistics")
        indexed[key] = (r, flag(r, c["tick_ns"]))
    if missing := expected - indexed.keys():
        raise Refused(f"missing records {sorted(missing)}")
    return indexed


def calibrate(valid, phase, levels):
    detected = []
    for index in valid:
        controls = [index[phase, "delay", str(level)] for level in levels]
        flags = [f.decile for _, f in controls]
        if flags != sorted(flags):
            return None
        detected.append(flags)
    for i, level in enumerate(levels):
        if all(row[i] for row in detected):
            shifts = [index[phase, "delay", str(level)][0]["median_ns"] for index in valid]
            # Delayed minus reference must measure a positive delay in every process.
            return max(shifts) if min(shifts) > 0 else None
    return None


def assess(run, out):
    fields(run, "config processes")
    c = run["config"]
    out["config_sha256"] = config_hash(c)
    check_config(c)
    require(isinstance(run["processes"], list), "expected list of launches")
    valid = []
    ids = set()
    statuses = out["processes"] = []
    references = []
    for i, p in enumerate(run["processes"]):
        fields(p, "id seed reference records")
        require(type(p["id"]) is str and p["id"].strip() and p["id"] not in ids,
                "process IDs must be unique")
        ids.add(p["id"])
        require(i < len(c["seeds"]) and p["seed"] == c["seeds"][i], "launch outside seed plan")
        require(len(valid) < c["planned"], "extra launch after plan completed")
        index = records(p, c)
        void = [phase for phase in c["phases"]
                if any_flag(index[phase, "null", p["reference"]][1])]
        statuses.append(dict(id=p["id"], status="void" if void else "valid", void_phases=void))
        for (phase, kind, arm), (rec, f) in index.items():
            if kind == "reference":
                references.append(dict(process=p["id"], phase=phase, arm=arm, versus=p["reference"],
                                       decile=f.decile, welch=f.welch, median_ns=rec["median_ns"],
                                       mean_ns=rec["mean_ns"], t=rec["t"]))
        if not void:
            valid.append(index)
    require(len(statuses) - len(valid) <= c["replacement_cap"], "replacement cap exceeded")
    require(len(valid) == c["planned"], "planned valid process count not reached")
    resolutions = {}
    verdicts = {}
    evidence = {}
    for phase, spec in c["phases"].items():
        resolution = calibrate(valid, phase, spec["levels_ns"])
        resolutions[phase] = resolution
        rows = verdicts[phase] = []
        for kind, arm in targets(c["fixtures"]):
            flags = [index[phase, kind, arm][1] for index in valid]
            evidence[phase, kind, arm] = flags
            verdict = classify(flags, resolution, spec["largest_ns"])
            row = dict(kind=kind, arm=arm, verdict=verdict)
            if verdict == "Pass":
                row["at_ns"] = resolution
            rows.append(row)
    out.update(resolution_ns=resolutions, verdicts=verdicts, references=references)
    return evidence


def any_flag(f):
    return f.decile or f.welch


def evaluate(first, rerun, report):
    a = assess(first, report)
    rows = report["verdicts"]
    if rerun is not None:
        second = report["rerun"] = {}
        b = assess(rerun, second)
        c, d = first["config"], rerun["config"]
        require(d["planned"] == 3 and d["rounds"] == 4 * c["rounds"], "rerun needs three at 4x rounds")
        require(all(c[k] == d[k] for k in ("fixtures", "phases", "tick_ns")), "rerun design changed")
        require(set(c["seeds"]).isdisjoint(d["seeds"]) and
                {p["id"] for p in first["processes"]}.isdisjoint(p["id"] for p in rerun["processes"]),
                "rerun needs fresh seeds and processes")
        require(any(row["verdict"] == "Candidate" for rr in rows.values() for row in rr),
                "rerun requires a first-run candidate")
        for phase, contrasts in rows.items():
            for row, new in zip(contrasts, second["verdicts"][phase]):
                key = phase, row["kind"], row["arm"]
                if row["verdict"] == "Candidate":
                    row["first_verdict"] = "Candidate"
                    row["verdict"] = repeated(a[key], b[key], new["verdict"])
                elif new["verdict"] in ("Candidate", "Leak") and row["verdict"] != "Leak":
                    # A different contrast can first become a candidate during this repeat.
                    row["first_verdict"] = row["verdict"]
                    row.pop("at_ns", None)
                    row["verdict"] = new["verdict"]
    return int(any(row["verdict"] in ("Leak", "Persistent leak", "Candidate", "Inconclusive")
                   for phase in rows.values() for row in phase))


def read(path):
    run = json.loads(Path(path).read_text(), object_pairs_hook=object_pairs)
    require(isinstance(run, dict), "expected run object")
    return run


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", nargs="?")
    parser.add_argument("--rerun")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest and not (args.run or args.rerun):
        from ci.test_g7_verdict import selftest
        selftest()
        return 0
    if not args.run or args.selftest:
        parser.error("supply a run and optional --rerun, or --selftest")
    report = {}
    try:
        status = evaluate(read(args.run), read(args.rerun) if args.rerun else None, report)
    except (ValueError, OSError, OverflowError, KeyError, TypeError) as error:
        for part in (report, report.get("rerun", {})):
            for key in ("verdicts", "resolution_ns", "references"):
                part.pop(key, None)
        report["refused"] = str(error)
        status = 2
    print(json.dumps(report, indent=2, allow_nan=False))
    return status


if __name__ == "__main__":
    sys.exit(main())
