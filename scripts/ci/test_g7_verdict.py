# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Self-test for the G7 verdict checker.

Run from the repository root:

    python3 -B scripts/ci/test_g7_verdict.py

No measurements are made. The pair oracle does not call the checker.
"""

import io
import json
import math
import sys
import tempfile
from contextlib import redirect_stdout
from itertools import combinations, permutations, product
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from g7_verdict import (  # noqa: E402
    CANDIDATE,
    DECILE_TICKS,
    EXIT_COMPLETE,
    EXIT_OPEN,
    EXIT_REFUSED,
    INCONCLUSIVE,
    LEAK,
    MINIMUM_ROUNDS,
    NOISE,
    NOT_REPRODUCED,
    PASS,
    PERSISTENT_LEAK,
    SIGN_NEGATIVE,
    SIGN_POSITIVE,
    WELCH_MAGNITUDE,
    Flag,
    Refused,
    classify,
    evaluate,
    flag,
    follow_up,
    main,
    config_hash,
)

# The fixture's timer, not the rule's. Boundaries are read from the rule.
FIXTURE_TICK_NS = 2
RERUN_ROUNDS = MINIMUM_ROUNDS * 4
# Two tests and three signs. 12 ** 3 = 1,728 and 12 ** 4 = 20,736.
FLAG_STATES = 12
VERDICT_ORDER = [PASS, NOISE, CANDIDATE, LEAK]


def pair_oracle(states, resolution, largest):
    """The decision table by pairs. It does not call the checker.

    Each state is (decile, welch, sign). Two signs agree when their product
    is non-negative, which is how a zero sign agrees with either direction.
    """
    pairs = list(combinations(states, 2))
    for (decile, welch, sign), (other_decile, other_welch, other) in pairs:
        if decile and welch and other_decile and other_welch and sign * other >= 0:
            return LEAK
    for (decile, welch, sign), (other_decile, other_welch, other) in pairs:
        if ((decile or welch) and (other_decile or other_welch) and sign * other >= 0):
            return CANDIDATE
    if any(decile and welch for decile, welch, _sign in states):
        return CANDIDATE
    if any(decile or welch for decile, welch, _sign in states):
        return NOISE
    if resolution is None or resolution > largest:
        return INCONCLUSIVE
    return PASS


def equal(actual, expected, context):
    if actual != expected:
        raise AssertionError(f"{context}: got {actual}, expected {expected}")


def exhaustive(width):
    """Totality, order, reversal, and monotone flags, for one population."""
    states = list(product((False, True), (False, True), (SIGN_NEGATIVE, 0, SIGN_POSITIVE)))
    equal(len(states), FLAG_STATES, "flag states")
    count = 0
    for combo in product(states, repeat=width):
        flags = [Flag(*state) for state in combo]
        result = classify(flags, 1, 1).verdict
        equal(result, pair_oracle(combo, 1, 1), combo)
        for perm in permutations(flags):
            equal(classify(perm, 1, 1).verdict, result, combo)
        flipped = [Flag(decile, welch, -sign) for decile, welch, sign in combo]
        equal(classify(flipped, 1, 1).verdict, result, combo)
        for index, (decile, welch, sign) in enumerate(combo):
            for added in (Flag(True, welch, sign), Flag(decile, True, sign)):
                augmented = flags.copy()
                augmented[index] = added
                worsened = VERDICT_ORDER.index(classify(augmented, 1, 1).verdict)
                equal(worsened >= VERDICT_ORDER.index(result), True, combo)
        count += 1
    equal(count, FLAG_STATES ** width, "exhaustive domain")
    print(f"{count} {width}-process combinations: totality, permutations, reversal, adding flags")


def run_fixture(count=3, cap=0, rounds=MINIMUM_ROUNDS, prefix="a", planned=3):
    planned_seeds = planned + cap
    seeds = [prefix + str(index) for index in range(max(planned_seeds, count))]
    config = dict(
        tick_ns=FIXTURE_TICK_NS, planned=planned, rounds=rounds, replacement_cap=cap,
        seeds=seeds[:planned_seeds], fixtures=["a", "b"],
        phases={"phase": dict(levels_ns=[10, 20, 40], largest_ns=10)},
    )
    processes = []
    for seed in seeds[:count]:
        records = []
        for kind, arms in (
            ("pooled", ["a", "b"]), ("keys", ["fixed-random"]), ("null", ["a"]),
            ("reference", ["b"]), ("delay", ["10", "20", "40"]),
        ):
            for arm in arms:
                delay = int(arm) if kind == "delay" else 0
                records.append(dict(
                    phase="phase", kind=kind, arm=arm, count=rounds, rtlf=bool(delay),
                    flagged_ns=delay, t=0, median_ns=delay, mean_ns=0,
                ))
        processes.append(dict(id=seed, seed=seed, reference="a", records=records))
    return dict(config=config, processes=processes)


def record(run, process, kind, arm):
    return next(
        item for item in run["processes"][process]["records"]
        if (item["kind"], item["arm"]) == (kind, arm)
    )


def mark(run, processes, kind, arm, **changes):
    for index in processes:
        record(run, index, kind, arm).update(changes)
    return run


def over_welch(sign):
    """One Welch flag of the given sign. The median carries the sign."""
    return dict(t=sign * math.nextafter(WELCH_MAGNITUDE, math.inf), median_ns=sign)


def both_over(sign):
    """Both tests, of the given sign, past the decile tick floor."""
    return dict(rtlf=True, flagged_ns=DECILE_TICKS * FIXTURE_TICK_NS, **over_welch(sign))


def verdict_of(report, kind, arm, phase="phase"):
    row = next(
        item for item in report["verdicts"][phase]
        if (item["kind"], item["arm"]) == (kind, arm)
    )
    return row["verdict"]


def judged(first_flags, again_flags, again_resolution=1, largest=1):
    opening = classify(first_flags, 1, largest)
    follow = classify(again_flags, again_resolution, largest)
    return follow_up(opening, again_flags, follow)


def selftest():
    named = 0

    def case(name, actual, expected):
        nonlocal named
        equal(actual, expected, name)
        named += 1
        print(name)

    def refused(name, run, rerun=None):
        try:
            evaluate(run, rerun)
        except Refused:
            case(name, "refused", "refused")
        else:
            raise AssertionError(name + ": accepted invalid run")

    def outcome(run, rerun=None, kind="pooled", arm="a"):
        report, status = evaluate(run, rerun)
        return verdict_of(report, kind, arm), status, report

    none = Flag(False, False, 0)
    plus = Flag(True, False, SIGN_POSITIVE)
    minus = Flag(False, True, SIGN_NEGATIVE)
    both = Flag(True, True, SIGN_POSITIVE)

    for name, flags, resolution, expected in (
        ("row 1, two both-tests processes", [both, both, none], None, LEAK),
        ("row 2, the two tests split across processes", [plus, Flag(False, True, SIGN_POSITIVE), none], None, CANDIDATE),
        ("row 3, opposed signs", [plus, minus, none], 1, NOISE),
        ("row 3, one flag", [plus, none, none], None, NOISE),
        ("row 4, equality at the largest resolution", [none] * 3, 1, PASS),
        ("row 5, resolution above the largest", [none] * 3, 1.01, INCONCLUSIVE),
        ("row 5, undefined resolution", [none] * 3, None, INCONCLUSIVE),
        ("regression: one both-tests process is a candidate", [both, none, none], 1, CANDIDATE),
        ("regression: two both-tests processes are a leak", [both, both, none], 1, LEAK),
        ("regression: a disagreeing third process stays a candidate", [plus, plus, minus], 1, CANDIDATE),
    ):
        case(name, classify(flags, resolution, 1).verdict, expected)

    threshold = dict(rtlf=True, flagged_ns=DECILE_TICKS * FIXTURE_TICK_NS, median_ns=0, mean_ns=1, t=-WELCH_MAGNITUDE)
    case("three ticks included and the Welch boundary excluded", flag(threshold, FIXTURE_TICK_NS), plus)
    case("the tick comes from the plan", flag(threshold, DECILE_TICKS).decile, False)
    below_ticks = dict(threshold, flagged_ns=math.nextafter(DECILE_TICKS * FIXTURE_TICK_NS, 0))
    case("just below three ticks", flag(below_ticks, FIXTURE_TICK_NS).decile, False)
    just_over = math.nextafter(WELCH_MAGNITUDE, math.inf)
    case("Welch magnitude just outside, negative", flag(dict(threshold, t=-just_over), FIXTURE_TICK_NS).welch, True)
    case("Welch magnitude at the boundary, positive", flag(dict(threshold, t=WELCH_MAGNITUDE), FIXTURE_TICK_NS).welch, False)
    case("the median takes precedence over the mean", flag(dict(threshold, median_ns=SIGN_NEGATIVE), FIXTURE_TICK_NS).sign, SIGN_NEGATIVE)
    case("regression: a zero median takes the mean's sign",
         classify([flag(threshold, FIXTURE_TICK_NS), plus, none], 1, 1).verdict, CANDIDATE)
    silent = flag(dict(threshold, mean_ns=0), FIXTURE_TICK_NS)
    case("both differences zero agree with a negative flag",
         classify([silent, minus, none], 1, 1).verdict, CANDIDATE)
    zero_both = classify([Flag(True, True, 0), none, none], 1, 1)
    case("a zero-sign both-tests process qualifies both directions",
         zero_both.signs, frozenset({SIGN_NEGATIVE, SIGN_POSITIVE}))
    paired = classify([plus, plus, Flag(True, True, SIGN_NEGATIVE)], 1, 1)
    case("an agreeing pair owns the candidate's directions", paired.signs, frozenset({SIGN_POSITIVE}))

    refused("row 0, fewer than three valid processes", run_fixture(2))
    refused("a plan of five is outside the tested populations", run_fixture(5, planned=5))
    clean_four, status, _report = outcome(run_fixture(4, planned=4))
    case("a plan of four is a tested population", (clean_four, status), (PASS, EXIT_COMPLETE))
    for kind, arm in (("pooled", "a"), ("keys", "fixed-random"), ("null", "a"),
                      ("delay", "20"), ("reference", "b")):
        run = run_fixture()
        run["processes"][1]["records"].remove(record(run, 1, kind, arm))
        refused("a missing " + kind + " record is a refusal", run)

    run = mark(run_fixture(), (0, 2), "pooled", "a", **over_welch(SIGN_POSITIVE))
    found, status, _report = outcome(run)
    case("a pooled contrast is judged", (found, status), (CANDIDATE, EXIT_OPEN))
    run = mark(run_fixture(), (0, 1, 2), "reference", "b", **both_over(SIGN_POSITIVE))
    found, status, _report = outcome(run)
    case("a flagged reference carries no verdict", (found, status), (PASS, EXIT_COMPLETE))

    for label, change in (("Welch", over_welch(SIGN_POSITIVE)), ("decile", dict(rtlf=True, flagged_ns=DECILE_TICKS * FIXTURE_TICK_NS))):
        run = mark(run_fixture(4, cap=1), (0,), "null", "a", **change)
        found, status, report = outcome(run)
        case("a void launch at the replacement cap, " + label, (found, status), (PASS, EXIT_COMPLETE))
        case("the void launch is named, " + label, report["processes"][0]["status"], "void")
        mark(run, (1,), "null", "a", **change)
        refused("the replacement cap is exceeded, " + label, run)

    refused("an extra launch after the plan is full", run_fixture(4, cap=1))
    run = run_fixture()
    record(run, 1, "pooled", "b")["count"] -= 1
    refused("a short record", run)
    run = run_fixture()
    run["processes"][0]["records"].append(record(run, 0, "null", "a"))
    refused("a duplicate record", run)
    run = run_fixture()
    run["config"]["rounds"] -= 1
    refused("below the planned rounds floor", run)

    run = mark(run_fixture(), (1,), "delay", "20", rtlf=False, flagged_ns=0, **over_welch(SIGN_POSITIVE))
    found, status, _report = outcome(run)
    case("a hole in the delays has no resolution", (found, status), (INCONCLUSIVE, EXIT_OPEN))
    run = mark(run_fixture(), (2,), "delay", "10", rtlf=False, flagged_ns=0)
    run["config"]["phases"]["phase"]["largest_ns"] = 20
    found, _status, _report = outcome(run)
    case("the first delay detected in every process", found, PASS)
    record(run, 2, "delay", "20")["median_ns"] = 21
    found, status, _report = outcome(run)
    case("the coarsest measured shift decides the resolution", (found, status), (INCONCLUSIVE, EXIT_OPEN))
    run = run_fixture()
    record(run, 1, "delay", "10")["median_ns"] = SIGN_NEGATIVE
    found, status, _report = outcome(run)
    case("a nonpositive measured delay has no resolution", (found, status), (INCONCLUSIVE, EXIT_OPEN))

    def candidate(prefix, sign, rounds=MINIMUM_ROUNDS):
        return mark(run_fixture(rounds=rounds, prefix=prefix), (0, 1), "pooled", "a", **over_welch(sign))

    first = candidate("a", SIGN_POSITIVE)
    opposite = candidate("b", SIGN_NEGATIVE, rounds=RERUN_ROUNDS)
    found, status, _report = outcome(first, opposite)
    case("an opposite-direction re-run stays a candidate", (found, status), (CANDIDATE, EXIT_OPEN))
    row_one = mark(run_fixture(rounds=RERUN_ROUNDS, prefix="b"), (0, 1), "pooled", "a", **both_over(SIGN_NEGATIVE))
    found, status, _report = outcome(first, row_one)
    case("a re-run meeting row 1 is a leak in its own right", (found, status), (LEAK, EXIT_OPEN))
    same_row_one = mark(run_fixture(rounds=RERUN_ROUNDS, prefix="b"), (0, 1), "pooled", "a", **both_over(SIGN_POSITIVE))
    found, _status, _report = outcome(first, same_row_one)
    case("a same-direction row-1 re-run is a leak", found, LEAK)

    case("two re-run processes on a qualified direction persist",
         judged([plus, plus, none], [plus, plus, minus]), PERSISTENT_LEAK)
    case("a zero sign on the re-run supports the qualified direction",
         judged([both, none, none], [plus, Flag(False, True, 0), none]), PERSISTENT_LEAK)
    case("one opposite flag does not use the extra both-tests sign",
         judged([plus, plus, Flag(True, True, SIGN_NEGATIVE)], [minus, none, none]), NOT_REPRODUCED)
    case("two opposite flags are a new candidate",
         judged([plus, plus, Flag(True, True, SIGN_NEGATIVE)], [minus, minus, none]), CANDIDATE)
    case("one both-tests re-run, even on the qualified direction, stays a candidate",
         judged([plus, plus, none], [both, none, none]), CANDIDATE)
    case("an inconclusive re-run is not reproduced",
         judged([plus, plus, none], [none, none, none], again_resolution=None), NOT_REPRODUCED)

    refused("a re-run needs four times the rounds", first, run_fixture(prefix="b"))
    refused("a re-run needs fresh seeds", first, run_fixture(rounds=RERUN_ROUNDS))
    refused("a re-run of four processes is refused", first, run_fixture(4, planned=4, rounds=RERUN_ROUNDS, prefix="b"))

    run = run_fixture(4, cap=1)
    run["config"]["phases"]["second"] = dict(run["config"]["phases"]["phase"])
    for process in run["processes"]:
        process["records"].extend(
            dict(item, phase="second") for item in tuple(process["records"])
        )
    next(item for item in run["processes"][1]["records"]
         if item["phase"] == "second" and item["kind"] == "null").update(over_welch(SIGN_POSITIVE))
    _found, status, report = outcome(run)
    case("a null in one phase voids the whole process", status, EXIT_COMPLETE)
    case("the flagged phase is the one recorded", report["processes"][1]["void_phases"], ["second"])
    run["processes"][1]["records"].pop()
    refused("a void launch still needs every record", run)

    run = run_fixture()
    run["processes"][0]["records"] = []
    refused("a launch with no records", run)
    run = run_fixture()
    run["processes"][1]["id"] = run["processes"][0]["id"]
    refused("a duplicate launch id", run)
    run = run_fixture()
    record(run, 0, "pooled", "a")["t"] = float("nan")
    refused("a non-finite statistic", run)
    run = run_fixture()
    record(run, 0, "pooled", "a")["rtlf"] = 1
    refused("a non-boolean decile flag", run)

    again = candidate("b", SIGN_POSITIVE, rounds=RERUN_ROUNDS)
    found, status, _report = outcome(first, again)
    case("a persistent candidate fails the gate", (found, status), (PERSISTENT_LEAK, EXIT_OPEN))
    clear = run_fixture(rounds=RERUN_ROUNDS, prefix="b")
    found, status, _report = outcome(first, clear)
    case("a passing re-run is not reproduced", (found, status), (NOT_REPRODUCED, EXIT_COMPLETE))
    noise = mark(run_fixture(rounds=RERUN_ROUNDS, prefix="b"), (0,), "pooled", "a", **over_welch(SIGN_NEGATIVE))
    found, status, _report = outcome(first, noise)
    case("a one-flag re-run is not reproduced", (found, status), (NOT_REPRODUCED, EXIT_COMPLETE))
    blind = mark(run_fixture(rounds=RERUN_ROUNDS, prefix="b"), (1,), "delay", "20", rtlf=False, flagged_ns=0)
    found, status, _report = outcome(first, blind)
    case("an inconclusive re-run retires the candidate", (found, status), (NOT_REPRODUCED, EXIT_COMPLETE))

    leaked = mark(run_fixture(rounds=RERUN_ROUNDS, prefix="b"), (0, 1), "pooled", "b", **both_over(SIGN_POSITIVE))
    # The original contrast stays not reproduced; the other fixture is a new leak.
    mark(leaked, (0, 1), "pooled", "a", t=0, median_ns=0)
    _found, status, report = outcome(first, leaked)
    case("a new re-run leak fails the gate", status, EXIT_OPEN)
    case("the new leak is the other fixture", verdict_of(report, "pooled", "b"), LEAK)
    fresh = mark(run_fixture(rounds=RERUN_ROUNDS, prefix="b"), (0,), "pooled", "b", **both_over(SIGN_POSITIVE))
    _found, status, report = outcome(first, fresh)
    case("a new re-run candidate stays open", status, EXIT_OPEN)
    case("the new candidate is the other fixture", verdict_of(report, "pooled", "b"), CANDIDATE)

    config = first["config"]
    case("the plan digest ignores object key order",
         config_hash(dict(reversed(list(config.items())))), config_hash(config))
    case("the replacement cap is part of the digest",
         config_hash(config) != config_hash(dict(config, replacement_cap=1)), True)
    command_line(case)
    print(f"{named} named cases")
    exhaustive(3)
    exhaustive(4)


def command_line(case):
    """Exit 2 through main, with a body that has no verdicts."""
    run = run_fixture()
    run["processes"][1]["records"].remove(record(run, 1, "pooled", "a"))
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as handle:
        json.dump(run, handle)
        path = handle.name
    try:
        buffer = io.StringIO()
        with redirect_stdout(buffer):
            code = main([path])
        body = json.loads(buffer.getvalue())
    finally:
        Path(path).unlink()
    case("a refused run exits 2", code, EXIT_REFUSED)
    case("a refused run prints no verdicts", set(body), {"refused"})


if __name__ == "__main__":
    selftest()
