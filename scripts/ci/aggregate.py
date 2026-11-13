#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The `ci-pass` decision.

Every job must be `success`. `c-vectors` may be `skipped`, and only when
the path filter said the rebuild was not needed. A skip for any other
reason fails: a required check that treats every skip as a pass is a
check that can be turned off by renaming a job.
"""

import argparse
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"

# Jobs other than the aggregator and the one path-scoped rebuild.
REQUIRED = (
    "changes",
    "lint",
    "test-x86_64",
    "test-aarch64",
    "bench-smoke",
    "op-counts",
    "deny",
    "hygiene",
    "c-reference-pin",
    "accept-list",
    "native-signer",
    "codeql",
)
SKIPPABLE = "c-vectors"
AGGREGATOR = "ci-pass"
JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*(?:#.*)?$")
RESULT = re.compile(r"^([A-Za-z0-9_-]+)=(.*)$")


def workflow_jobs(text: str) -> list[str]:
    jobs = []
    in_jobs = False
    for line in text.splitlines():
        if line.startswith("jobs:"):
            in_jobs = True
            continue
        if in_jobs and line and not line.startswith(" ") and not line.startswith("#"):
            break
        if not in_jobs:
            continue
        match = JOB.match(line)
        if match:
            jobs.append(match.group(1))
    return jobs


def judge(results: dict[str, str], needed: str) -> list[str]:
    errors = []
    for name in REQUIRED:
        result = results.get(name, "missing")
        if result != "success":
            errors.append(f"{name}={result}")
    present = results.get(SKIPPABLE, "missing")
    if present == "success":
        return errors
    if present == "skipped" and needed == "false":
        return errors
    errors.append(f"{SKIPPABLE}={present} needed={needed!r}")
    return errors


def selftest() -> int:
    ok = {name: "success" for name in REQUIRED}
    ok[SKIPPABLE] = "skipped"
    if judge(ok, "false"):
        print("selftest: a filtered skip was refused", file=sys.stderr)
        return 1
    if not judge(ok, "true"):
        print("selftest: a skip of a needed rebuild passed", file=sys.stderr)
        return 1
    failed = dict(ok)
    failed["lint"] = "failure"
    failed[SKIPPABLE] = "success"
    if not judge(failed, "false"):
        print("selftest: a failed lint passed", file=sys.stderr)
        return 1
    jobs = workflow_jobs(WORKFLOW.read_text())
    expect = set(REQUIRED) | {SKIPPABLE, AGGREGATOR}
    if set(jobs) != expect:
        print(f"selftest: ci.yml jobs {jobs} != {sorted(expect)}", file=sys.stderr)
        return 1
    if len(jobs) != len(expect):
        print("selftest: a job id is repeated", file=sys.stderr)
        return 1
    print("aggregate selftest passed")
    return 0


def parse_results(text: str) -> dict[str, str]:
    results = {}
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        match = RESULT.match(line)
        if not match:
            raise SystemExit(f"bad result line: {line}")
        results[match.group(1)] = match.group(2)
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest:
        return selftest()
    results = parse_results(os.environ.get("JOBS", ""))
    needed = os.environ.get("C_VECTORS_NEEDED", "")
    errors = judge(results, needed)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print("ci-pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
