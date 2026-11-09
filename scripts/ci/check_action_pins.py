#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Every `uses:` in a workflow is pinned to a 40-character commit SHA.

A floating tag is not a pin. A workflow with no `uses:` line is not a
workflow this gate can see, so that fails too.
"""

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
USE = re.compile(r"^\s*-?\s*uses:\s*(\S+)")
PINNED = re.compile(r"^[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)+@[0-9a-f]{40}$")


def problems(text: str) -> list[str]:
    uses = []
    errors = []
    for number, line in enumerate(text.splitlines(), start=1):
        stripped = line.lstrip()
        if stripped.startswith("#"):
            continue
        match = USE.search(line)
        if not match:
            continue
        action = match.group(1)
        uses.append(action)
        if not PINNED.match(action):
            errors.append(f"line {number}: {action} is not pinned to a commit")
    if not uses:
        errors.append("no uses: line")
    return errors


def selftest() -> int:
    good = "    - uses: actions/checkout@0123456789abcdef0123456789abcdef01234567 # v5\n"
    bad = "    - uses: actions/checkout@v5\n"
    if problems(good):
        print("selftest: a pinned action was refused", file=sys.stderr)
        return 1
    if not problems(bad):
        print("selftest: a tag pin passed", file=sys.stderr)
        return 1
    if not problems("jobs: {}\n"):
        print("selftest: an empty workflow passed", file=sys.stderr)
        return 1
    print("action-pin selftest passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest and selftest() != 0:
        return 1
    files = sorted(WORKFLOWS.glob("*.yml"))
    if len(files) < 2:
        print(f"{WORKFLOWS} has {len(files)} workflows", file=sys.stderr)
        return 1
    failed = False
    for path in files:
        for error in problems(path.read_text()):
            print(f"{path.relative_to(ROOT)}: {error}", file=sys.stderr)
            failed = True
    if failed:
        return 1
    print(f"action pins hold in {len(files)} workflows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
