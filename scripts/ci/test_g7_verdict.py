# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Run via python3 scripts/g7_verdict.py --selftest. No measurements are made."""

from copy import deepcopy
from itertools import combinations, permutations, product

from g7_verdict import Flag, Refused, classify, config_hash, evaluate, flag, repeated


def definition(states, resolution, largest):
    """Independent oracle: enumerate pairs of launches, without production helpers."""
    pairs = list(combinations(states, 2))
    for (d, t, sign), (e, u, other) in pairs:
        if d and t and e and u and (sign * other >= 0):
            return "Leak"
    for (d, t, sign), (e, u, other) in pairs:
        if (d or t) and (e or u) and (sign * other >= 0):
            return "Candidate"
    if any(d and t for d, t, _ in states):
        return "Candidate"
    if any(d or t for d, t, _ in states):
        return "Noise"
    if resolution is None or resolution > largest:
        return "Inconclusive"
    return "Pass"


def equal(actual, expected, context):
    if actual != expected:
        raise AssertionError(f"{context}: got {actual}, expected {expected}")


def exhaustive():
    states = list(product((False, True), (False, True), (-1, 0, 1)))
    order = ["Pass", "Noise", "Candidate", "Leak"]
    count = 0
    for triple in product(states, repeat=3):
        fs = [Flag(*s) for s in triple]
        result = classify(fs, 1, 1)
        equal(result, definition(triple, 1, 1), triple)
        for perm in permutations(fs):
            equal(classify(perm, 1, 1), result, triple)
        equal(classify([Flag(d, t, -s) for d, t, s in triple], 1, 1), result, triple)
        for i, (d, t, s) in enumerate(triple):
            for new in (Flag(True, t, s), Flag(d, True, s)):
                augmented = fs.copy()
                augmented[i] = new
                equal(order.index(classify(augmented, 1, 1)) >= order.index(result), True, triple)
        count += 1
    equal(count, 1728, "exhaustive domain")
    print(f"{count} three-process combinations: totality/oracle, permutations, reversal, adding flags")


def run_fixture(count=3, cap=0, rounds=4096, prefix="a"):
    seeds = [prefix + str(i) for i in range(max(3, count) + cap)]
    config = dict(tick_ns=2, planned=3, rounds=rounds, replacement_cap=cap,
                  seeds=seeds[:3 + cap], fixtures=["a", "b"],
                  phases={"phase": dict(levels_ns=[10, 20, 40], largest_ns=10)})
    processes = []
    for seed in seeds[:count]:
        records = []
        for kind, arms in (("pooled", ["a", "b"]), ("keys", ["fixed-random"]),
                           ("null", ["a"]), ("reference", ["b"]), ("delay", ["10", "20", "40"])):
            for arm in arms:
                delay = int(arm) if kind == "delay" else 0
                records.append(dict(phase="phase", kind=kind, arm=arm, count=rounds,
                                    rtlf=bool(delay), flagged_ns=delay, t=0, median_ns=delay, mean_ns=0))
        processes.append(dict(id=seed, seed=seed, reference="a", records=records))
    return dict(config=config, processes=processes)


def record(run, process, kind, arm):
    return next(r for r in run["processes"][process]["records"] if (r["kind"], r["arm"]) == (kind, arm))


def selftest():
    named = 0

    def case(name, actual, expected):
        nonlocal named
        equal(actual, expected, name)
        named += 1
        print(name)

    def refused(name, run, rerun=None):
        try:
            evaluate(run, rerun, {})
        except Refused:
            case(name, "refused", "refused")
        else:
            raise AssertionError(name + ": accepted invalid run")

    def outcome(run, rerun=None):
        report = {}
        evaluate(run, rerun, report)
        return report["verdicts"]["phase"][0]["verdict"]

    none = Flag(False, False, 0)
    plus = Flag(True, False, 1)
    minus = Flag(False, True, -1)
    both = Flag(True, True, 1)

    for name, fs, r, expected in (
        ("row 1, two both", [both, both, none], None, "Leak"),
        ("row 2, split tests", [plus, Flag(False, True, 1), none], None, "Candidate"),
        ("row 3, opposed", [plus, minus, none], 1, "Noise"),
        ("row 3, singleton", [plus, none, none], None, "Noise"),
        ("row 4, equality at largest", [none]*3, 1, "Pass"),
        ("row 5, above largest", [none]*3, 1.01, "Inconclusive"),
        ("row 5, undefined", [none]*3, None, "Inconclusive"),
        ("regression 1: noise -> candidate", [both, none, none], 1, "Candidate"),
        ("regression 2: candidate -> leak", [both, both, none], 1, "Leak"),
        ("regression 3: noise -> candidate (third disagrees)", [plus, plus, minus], 1, "Candidate"),
    ):
        case(name, classify(fs, r, 1), expected)
    r = dict(rtlf=True, flagged_ns=6, median_ns=0, mean_ns=1, t=-4.5)
    case("three ticks inclusive and Welch equality excluded", flag(r, 2), plus)
    case("tick comes from config", flag(r, 3).decile, False)
    case("just below three ticks", flag(dict(r, flagged_ns=5.999), 2).decile, False)
    case("Welch negative just outside", flag(dict(r, t=-4.501), 2).welch, True)
    case("positive Welch boundary", flag(dict(r, t=4.5), 2).welch, False)
    case("median takes precedence", flag(dict(r, median_ns=-1), 2).sign, -1)
    case("regression 4: noise -> candidate (zero median, positive mean)",
         classify([flag(r, 2), plus, none], 1, 1), "Candidate")
    case("both zero agree with negative", classify([flag(dict(r, mean_ns=0), 2), minus, none], 1, 1),
         "Candidate")
    refused("row 0, fewer than three", run_fixture(2))
    for kind, arm in (("pooled", "a"), ("keys", "fixed-random"), ("null", "a"),
                      ("delay", "20"), ("reference", "b")):
        run = run_fixture()
        run["processes"][1]["records"].remove(record(run, 1, kind, arm))
        refused("regression 5: no flag -> refused, missing " + kind, run)
    run = run_fixture()
    record(run, 0, "pooled", "a").update(t=5, median_ns=1)
    record(run, 2, "pooled", "a").update(t=6, median_ns=1)
    case("regression 6: printed without verdict -> pooled candidate", outcome(run), "Candidate")
    run = run_fixture()
    for i in range(3):
        record(run, i, "reference", "b").update(t=5, rtlf=True, flagged_ns=10)
    case("flagged reference is informational", outcome(run), "Pass")
    for label, change in (("Welch", dict(t=5)), ("decile", dict(rtlf=True, flagged_ns=6))):
        run = run_fixture(4, cap=1)
        record(run, 0, "null", "a").update(change)
        report = {}
        case("replacement at cap, " + label, evaluate(run, None, report), 0)
        case("void process printed, " + label, report["processes"][0]["status"], "void")
        record(run, 1, "null", "a").update(change)
        refused("replacement cap exceeded, " + label, run)
    run = run_fixture(4, cap=1)
    refused("extra launch after all valid", run)
    run = run_fixture()
    record(run, 1, "pooled", "b")["count"] -= 1
    refused("short record", run)
    run = run_fixture()
    run["processes"][0]["records"].append(record(run, 0, "null", "a"))
    refused("duplicate record", run)
    run = run_fixture()
    run["config"]["rounds"] -= 1
    refused("below planned rounds floor", run)
    run = run_fixture()
    record(run, 1, "delay", "20").update(rtlf=False, flagged_ns=0, t=6)
    case("hole in calibration, even when Welch flags", outcome(run), "Inconclusive")
    run = run_fixture()
    record(run, 2, "delay", "10").update(rtlf=False, flagged_ns=0)
    run["config"]["phases"]["phase"]["largest_ns"] = 20
    case("first common detected control", outcome(run), "Pass")
    record(run, 2, "delay", "20")["median_ns"] = 21
    case("coarsest measured shift across valid processes", outcome(run), "Inconclusive")
    run = run_fixture()
    record(run, 1, "delay", "10")["median_ns"] = -1
    case("nonpositive measured delay", outcome(run), "Inconclusive")

    first = run_fixture()
    for i in (0, 1):
        record(first, i, "pooled", "a").update(t=5, median_ns=1)
    again = run_fixture(rounds=16384, prefix="b")
    for i in (0, 1):
        record(again, i, "pooled", "a").update(t=-5, median_ns=-1)
    case("regression 7: persistent leak -> not reproduced (opposite signs)",
         outcome(first, again), "Not reproduced")
    for i in (0, 1):
        record(again, i, "pooled", "a").update(rtlf=True, flagged_ns=6)
    case("opposite-direction rerun row 1", outcome(first, again), "Leak")
    case("matching signs persist", repeated([plus, plus, none], [plus, plus, minus], "Candidate"),
         "Persistent leak")
    case("single both persists via wildcard", repeated([both, none, none], [plus, Flag(False, True, 0), none],
         "Candidate"), "Persistent leak")
    case("a lone both does not override first-run agreement", repeated([plus, plus, Flag(True, True, -1)],
         [minus, minus, none], "Candidate"), "Not reproduced")
    refused("rerun needs four times the rounds", first, run_fixture(prefix="b"))
    refused("rerun needs fresh seeds", first, run_fixture(rounds=16384))
    run = run_fixture(4, cap=1)
    run["config"]["phases"]["second"] = deepcopy(run["config"]["phases"]["phase"])
    for process in run["processes"]:
        process["records"].extend([dict(r, phase="second") for r in process["records"]])
    next(r for r in run["processes"][1]["records"] if r["phase"] == "second" and r["kind"] == "null")["t"] = 5
    report = {}
    case("null in one phase voids entire process", evaluate(run, None, report), 0)
    case("flagged phase recorded", report["processes"][1]["void_phases"], ["second"])
    run["processes"][1]["records"].pop()
    refused("missing control in void process", run)
    run = run_fixture()
    run["processes"][0]["records"] = []
    refused("missing all arms", run)
    run = run_fixture()
    run["processes"][1]["id"] = run["processes"][0]["id"]
    refused("duplicate launch", run)
    run = run_fixture()
    record(run, 0, "pooled", "a")["t"] = float("nan")
    refused("non-finite statistic", run)
    run = run_fixture()
    record(run, 0, "pooled", "a")["rtlf"] = 1
    refused("non-boolean decile flag", run)
    for i in (0, 1):
        record(again, i, "pooled", "a").update(rtlf=False, flagged_ns=0, t=5, median_ns=1)
    report = {}
    case("persistent candidate fails gate", evaluate(first, again, report), 1)
    case("persistent verdict printed", report["verdicts"]["phase"][0]["verdict"], "Persistent leak")
    for i in (0, 1):
        record(again, i, "pooled", "a")["median_ns"] = -1
    case("not reproduced clears candidate", evaluate(first, again, {}), 0)
    record(again, 0, "pooled", "b").update(rtlf=True, flagged_ns=6, t=6)
    record(again, 1, "pooled", "b").update(rtlf=True, flagged_ns=6, t=6)
    case("new independent rerun leak fails gate", evaluate(first, again, {}), 1)
    record(again, 1, "pooled", "b").update(rtlf=False, flagged_ns=0, t=0)
    report = {}
    case("new independent rerun candidate needs follow-up", evaluate(first, again, report), 1)
    case("new candidate reported on the correct contrast", report["verdicts"]["phase"][1]["verdict"],
         "Candidate")
    config = first["config"]
    case("config digest independent of object key order", config_hash(dict(reversed(list(config.items())))),
         config_hash(config))
    case("replacement cap in digest", config_hash(config) != config_hash(dict(config, replacement_cap=1)), True)
    print(f"{named} named cases including seven historical regressions")
    exhaustive()
