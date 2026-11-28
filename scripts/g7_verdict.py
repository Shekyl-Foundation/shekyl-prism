#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Apply SIGNER_SPEC section 0 to whole-process statistics.

Read one JSON run, and --rerun when a first run has a candidate. Exit 0
when every contrast is closed: a pass, noise, or not reproduced. Exit 1
when a contrast is a leak, a persistent leak, a candidate, or inconclusive.
Exit 2 when the input is refused or the checker fails. A refusal prints
only {"refused": "..."}: no verdicts and no process list.

The decision is one ruling. classify names the verdict and the directions
that qualified a candidate or a leak. follow_up consumes that set. It does
not recover the signs from the flags a second time.

A plan is three or four processes (TESTED_PLANNED). Those are the
populations the self-test compares exhaustively with an independent pair
oracle. Extending the set means extending that comparison in the same
change. The self-test is python3 -B scripts/ci/test_g7_verdict.py.
"""

import argparse
import hashlib
import json
import math
import sys
import traceback
from dataclasses import dataclass
from pathlib import Path

# The rule in section 0. These names are the rule's quantities.
WELCH_MAGNITUDE = 4.5
DECILE_TICKS = 3
AGREEING_PROCESSES = 2
MINIMUM_ROUNDS = 4096
MINIMUM_FIXTURES = 2
RERUN_PROCESSES = 3
RERUN_ROUND_FACTOR = 4
DELAY_LEVELS = 3
TESTED_PLANNED = frozenset({3, 4})

SIGN_NEGATIVE = -1
SIGN_ZERO = 0
SIGN_POSITIVE = 1
DIRECTIONS = (SIGN_NEGATIVE, SIGN_POSITIVE)

LEAK = "Leak"
CANDIDATE = "Candidate"
NOISE = "Noise"
PASS = "Pass"
INCONCLUSIVE = "Inconclusive"
PERSISTENT_LEAK = "Persistent leak"
NOT_REPRODUCED = "Not reproduced"

OPEN = frozenset({LEAK, PERSISTENT_LEAK, CANDIDATE, INCONCLUSIVE})

EXIT_COMPLETE = 0
EXIT_OPEN = 1
EXIT_REFUSED = 2

DESIGN_FIELDS = ("fixtures", "phases", "tick_ns")


class Refused(ValueError):
    """The run is outside the rule. The report is not a verdict."""


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
            all(type(value) is str and value.strip() for value in values) and
            len(set(values)) == len(values),
            "expected distinct, nonempty names")


def config_hash(config):
    """SHA-256 of the plan: sorted object keys, original array order, ASCII."""
    packed = json.dumps(config, sort_keys=True, separators=(",", ":"), allow_nan=False)
    return hashlib.sha256(packed.encode("ascii")).hexdigest()


def check_config(config):
    fields(config, "tick_ns planned rounds replacement_cap seeds fixtures phases")
    require(number(config["tick_ns"]) > 0, "tick must be positive")
    planned = integer(config["planned"], min(TESTED_PLANNED))
    require(planned in TESTED_PLANNED,
            f"planned process count must be one of {sorted(TESTED_PLANNED)}")
    integer(config["replacement_cap"], 0)
    integer(config["rounds"], MINIMUM_ROUNDS)
    names(config["seeds"], planned)
    require(len(config["seeds"]) == planned + config["replacement_cap"],
            "plan one seed per launch, including reserves")
    names(config["fixtures"], MINIMUM_FIXTURES)
    require(isinstance(config["phases"], dict) and config["phases"], "expected phases")
    names(list(config["phases"]), 1)
    for phase in config["phases"].values():
        fields(phase, "levels_ns largest_ns")
        require(number(phase["largest_ns"]) > 0, "largest resolution must be positive")
        levels = phase["levels_ns"]
        require(isinstance(levels, list) and len(levels) == DELAY_LEVELS and
                all(type(value) is int and value > 0 for value in levels) and
                sorted(set(levels)) == levels,
                "expected three increasing nominal delays in ns")


@dataclass(frozen=True)
class Flag:
    """One process's two tests and the sign of the contrast.

    decile is the RTLF result at α = 0.01 with a difference of at least
    DECILE_TICKS timer ticks. welch is a t statistic beyond WELCH_MAGNITUDE.
    sign is SIGN_NEGATIVE, SIGN_ZERO, or SIGN_POSITIVE. Zero agrees with
    either direction.
    """

    decile: bool
    welch: bool
    sign: int


@dataclass(frozen=True)
class Ruling:
    """The verdict of one contrast, and the directions that qualified it.

    signs is empty for noise, a pass, and an inconclusive result. A leak or
    a candidate carries the directions the matching row used: an agreeing
    pair's directions, or, when the only candidate is one both-tests
    process, that process's directions.
    """

    verdict: str
    signs: frozenset


@dataclass(frozen=True)
class Sample:
    """One record after the tick rule has been applied. The raw record is not kept."""

    median_ns: float
    mean_ns: float
    t: float
    observed: Flag


@dataclass(frozen=True)
class Contrast:
    """A ruling, the valid-process flags, and whether its phase met its limit."""

    ruling: Ruling
    flags: tuple
    adequate_resolution: bool


def sign_of(median_ns, mean_ns):
    """The median's sign, or the mean's sign when the median is zero."""
    direction = mean_ns if median_ns == 0 else median_ns
    if direction > 0:
        return SIGN_POSITIVE
    if direction < 0:
        return SIGN_NEGATIVE
    return SIGN_ZERO


def agrees(observed, direction):
    return observed == SIGN_ZERO or observed == direction


def either_test(flag):
    return flag.decile or flag.welch


def both_tests(flag):
    return flag.decile and flag.welch


def flag(record, tick_ns):
    """The two tests and the sign of one record."""
    return Flag(
        record["rtlf"] and record["flagged_ns"] >= DECILE_TICKS * tick_ns,
        abs(record["t"]) > WELCH_MAGNITUDE,
        sign_of(record["median_ns"], record["mean_ns"]),
    )


def support(flags):
    """How many flagged processes agree with each direction."""
    return {
        direction: sum(either_test(item) and agrees(item.sign, direction) for item in flags)
        for direction in DIRECTIONS
    }


def agreeing_signs(flags, width):
    return frozenset(
        direction for direction, count in support(flags).items() if count >= width
    )


def single_process_signs(flags):
    """Directions qualified by a both-tests flag that has no agreeing pair."""
    return frozenset(
        direction
        for item in flags
        if both_tests(item)
        for direction in DIRECTIONS
        if agrees(item.sign, direction)
    )


def within_limit(resolution, largest):
    return resolution is not None and resolution <= largest


def classify(flags, resolution, largest):
    """The first matching row of the decision table.

    resolution is the phase's measured resolution, or None when the phase
    has none. largest is that phase's largest resolution for a pass.
    """
    leak_signs = agreeing_signs(
        [item for item in flags if both_tests(item)], AGREEING_PROCESSES
    )
    if leak_signs:
        return Ruling(LEAK, leak_signs)
    pair_signs = agreeing_signs(flags, AGREEING_PROCESSES)
    if pair_signs:
        return Ruling(CANDIDATE, pair_signs)
    single = single_process_signs(flags)
    if single:
        return Ruling(CANDIDATE, single)
    if any(either_test(item) for item in flags):
        return Ruling(NOISE, frozenset())
    if within_limit(resolution, largest):
        return Ruling(PASS, frozenset())
    return Ruling(INCONCLUSIVE, frozenset())


def follow_up(first, again):
    """The candidate's verdict after its re-run.

    first is the first-run ruling, including the directions that qualified
    it. again is the re-run contrast. A row-1 re-run is a leak. Two
    re-run processes that support a qualified direction make a persistent
    leak. Only Noise or Pass with adequate re-run resolution retires the
    candidate.
    """
    if again.ruling.verdict == LEAK:
        return LEAK
    again_support = support(again.flags)
    if any(again_support[sign] >= AGREEING_PROCESSES for sign in first.signs):
        return PERSISTENT_LEAK
    if again.ruling.verdict in (CANDIDATE, INCONCLUSIVE):
        return CANDIDATE
    if again.ruling.verdict in (NOISE, PASS):
        return NOT_REPRODUCED if again.adequate_resolution else CANDIDATE
    raise RuntimeError(f"unexpected re-run verdict {again.ruling.verdict}")


def targets(fixtures):
    return [("pooled", arm) for arm in fixtures] + [("keys", "fixed-random")]


def expected_records(config, reference):
    fixtures = config["fixtures"]
    require(reference in fixtures, "unknown process reference")
    found = set()
    for phase, spec in config["phases"].items():
        for kind, arm in (
            targets(fixtures)
            + [("null", reference)]
            + [("reference", fixture) for fixture in fixtures if fixture != reference]
            + [("delay", str(level)) for level in spec["levels_ns"]]
        ):
            found.add((phase, kind, arm))
    return found


def read_records(process, config):
    """Every planned record of one launch, keyed by phase, kind, and arm."""
    expected = expected_records(config, process["reference"])
    indexed = {}
    require(isinstance(process["records"], list), "expected list of records")
    for record in process["records"]:
        fields(record, "phase kind arm rtlf flagged_ns t median_ns mean_ns count")
        key = record["phase"], record["kind"], record["arm"]
        require(all(type(part) is str for part in key) and key in expected and key not in indexed,
                f"unexpected or duplicate record {key}")
        require(integer(record["count"], 1) == config["rounds"], f"{key}: incomplete rounds")
        for name in ("t", "median_ns", "mean_ns", "flagged_ns"):
            number(record[name])
        require(type(record["rtlf"]) is bool and record["flagged_ns"] >= 0 and
                (record["rtlf"] or record["flagged_ns"] == 0),
                f"{key}: invalid RTLF statistics")
        indexed[key] = Sample(
            record["median_ns"], record["mean_ns"], record["t"], flag(record, config["tick_ns"])
        )
    if missing := expected - indexed.keys():
        raise Refused(f"missing records {sorted(missing)}")
    return indexed


def solid_delay(detections):
    """True when every miss sits below every detection.

    A higher miss after a detection leaves the phase with no resolution.
    """
    seen = False
    for detected in detections:
        if seen and not detected:
            return False
        seen = seen or detected
    return True


def resolution_of(valid, phase, levels):
    """The largest positive median at the lowest delay every process detects.

    valid is the record maps of the valid processes. The Welch flag does not
    detect a delay. None means the phase has no resolution.
    """
    detected = []
    for records in valid:
        flags = [records[phase, "delay", str(level)].observed.decile for level in levels]
        if not solid_delay(flags):
            return None
        detected.append(flags)
    for offset, level in enumerate(levels):
        if not all(row[offset] for row in detected):
            continue
        shifts = [records[phase, "delay", str(level)].median_ns for records in valid]
        if min(shifts) <= 0:
            return None
        return max(shifts)
    return None


def read_launches(run):
    """Validated launches. Raises Refused before any report exists."""
    fields(run, "config processes")
    config = run["config"]
    check_config(config)
    require(isinstance(run["processes"], list), "expected list of launches")
    launches = []
    seen = set()
    valid_count = 0
    for index, process in enumerate(run["processes"]):
        fields(process, "id seed reference records")
        require(type(process["id"]) is str and process["id"].strip() and process["id"] not in seen,
                "process IDs must be unique")
        seen.add(process["id"])
        require(index < len(config["seeds"]) and process["seed"] == config["seeds"][index],
                "launch outside seed plan")
        require(valid_count < config["planned"], "extra launch after plan completed")
        records = read_records(process, config)
        void = [
            phase for phase in config["phases"]
            if either_test(records[phase, "null", process["reference"]].observed)
        ]
        if not void:
            valid_count += 1
        launches.append({
            "id": process["id"],
            "reference": process["reference"],
            "records": records,
            "void": void,
        })
    require(len(launches) - valid_count <= config["replacement_cap"], "replacement cap exceeded")
    require(valid_count == config["planned"], "planned valid process count not reached")
    return config, launches


def references_of(launches):
    rows = []
    for launch in launches:
        for key, record in launch["records"].items():
            phase, kind, arm = key
            if kind != "reference":
                continue
            rows.append({
                "process": launch["id"],
                "phase": phase,
                "arm": arm,
                "versus": launch["reference"],
                "decile": record.observed.decile,
                "welch": record.observed.welch,
                "median_ns": record.median_ns,
                "mean_ns": record.mean_ns,
                "t": record.t,
            })
    return rows


def contrasts_of(valid, config):
    """Rulings for the planned contrasts, and the report rows in plan order."""
    contrasts = {}
    verdicts = {}
    resolutions = {}
    for phase, spec in config["phases"].items():
        resolution = resolution_of(valid, phase, spec["levels_ns"])
        resolutions[phase] = resolution
        adequate_resolution = within_limit(resolution, spec["largest_ns"])
        phase_contrasts = {}
        rows = []
        for kind, arm in targets(config["fixtures"]):
            flags = tuple(records[phase, kind, arm].observed for records in valid)
            ruling = classify(flags, resolution, spec["largest_ns"])
            phase_contrasts[kind, arm] = Contrast(ruling, flags, adequate_resolution)
            row = {"kind": kind, "arm": arm, "verdict": ruling.verdict}
            if ruling.verdict == PASS:
                row["at_ns"] = resolution
            rows.append(row)
        contrasts[phase] = phase_contrasts
        verdicts[phase] = rows
    return contrasts, resolutions, verdicts


def assess(run):
    """The finished report and the contrast rulings. Refusals raise."""
    config, launches = read_launches(run)
    valid = [item["records"] for item in launches if not item["void"]]
    contrasts, resolutions, verdicts = contrasts_of(valid, config)
    report = {
        "config_sha256": config_hash(config),
        "processes": [
            {"id": item["id"], "status": "void" if item["void"] else "valid",
             "void_phases": item["void"]}
            for item in launches
        ],
        "resolution_ns": resolutions,
        "verdicts": verdicts,
        "references": references_of(launches),
    }
    return report, contrasts


def require_rerun(first, second, report):
    first_config, second_config = first["config"], second["config"]
    require(second_config["planned"] == RERUN_PROCESSES and
            second_config["rounds"] == RERUN_ROUND_FACTOR * first_config["rounds"],
            "rerun needs three processes at four times the rounds")
    require(all(first_config[key] == second_config[key] for key in DESIGN_FIELDS),
            "rerun design changed")
    require(set(first_config["seeds"]).isdisjoint(second_config["seeds"]),
            "rerun needs fresh seeds")
    require({process["id"] for process in first["processes"]}.isdisjoint(
        process["id"] for process in second["processes"]),
            "rerun needs fresh processes")
    require(any(row["verdict"] == CANDIDATE
                for rows in report["verdicts"].values() for row in rows),
            "rerun requires a first-run candidate")


def apply_rerun(report, first, second):
    """Replace candidate rows from the re-run. Rows are matched by kind and arm."""
    for phase, rows in report["verdicts"].items():
        for row in rows:
            key = row["kind"], row["arm"]
            opening = first[phase][key]
            follow = second[phase][key]
            if row["verdict"] == CANDIDATE:
                row["first_verdict"] = CANDIDATE
                row["verdict"] = follow_up(opening.ruling, follow)
            elif follow.ruling.verdict in (CANDIDATE, LEAK) and row["verdict"] != LEAK:
                row["first_verdict"] = row["verdict"]
                row.pop("at_ns", None)
                row["verdict"] = follow.ruling.verdict


def opens(verdicts):
    return any(row["verdict"] in OPEN for rows in verdicts.values() for row in rows)


def evaluate(first, rerun=None):
    """Return the report and the exit status. The report exists only complete."""
    report, contrasts = assess(first)
    if rerun is not None:
        second, second_contrasts = assess(rerun)
        require_rerun(first, rerun, report)
        apply_rerun(report, contrasts, second_contrasts)
        report["rerun"] = second
    status = EXIT_OPEN if opens(report["verdicts"]) else EXIT_COMPLETE
    return report, status


def read(path):
    try:
        text = Path(path).read_text()
    except (OSError, UnicodeError) as error:
        raise Refused(f"cannot read {path}: {error}") from error
    try:
        run = json.loads(text, object_pairs_hook=object_pairs)
    except json.JSONDecodeError as error:
        raise Refused(f"invalid JSON: {error.msg}") from error
    require(isinstance(run, dict), "expected run object")
    return run


def emit(body):
    print(json.dumps(body, indent=2, allow_nan=False))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run")
    parser.add_argument("--rerun")
    args = parser.parse_args(argv)
    try:
        report, status = evaluate(read(args.run), read(args.rerun) if args.rerun else None)
    except Refused as error:
        emit({"refused": str(error)})
        return EXIT_REFUSED
    except Exception as error:
        traceback.print_exc(file=sys.stderr)
        emit({"refused": f"checker failed: {error}"})
        return EXIT_REFUSED
    emit(report)
    return status


if __name__ == "__main__":
    sys.exit(main())
