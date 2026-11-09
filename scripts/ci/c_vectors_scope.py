#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Decide whether the C-vector rebuild has to run.

It runs when the range touches `scripts/`, `external/`, or a `testdata/`
directory. Anything else — a missing base, a git error, an unnamed range —
is `true`. Skipping the rebuild on a guess is the failure; running it is
the cost.
"""

import argparse
import os
import subprocess
import sys

def wanted(path: str) -> bool:
    if path.startswith("scripts/") or path.startswith("external/"):
        return True
    return "testdata" in path.split("/")


def names(base: str, head: str) -> list[str] | None:
    if not base or set(base) <= {"0"}:
        print("no base; rebuilding the vectors", file=sys.stderr)
        return None
    merge_base = subprocess.run(
        ["git", "merge-base", base, head],
        check=False,
        capture_output=True,
        text=True,
    )
    if merge_base.returncode != 0:
        print(merge_base.stderr.strip() or "merge-base failed", file=sys.stderr)
        return None
    diff = subprocess.run(
        ["git", "diff", "--name-only", merge_base.stdout.strip(), head],
        check=False,
        capture_output=True,
        text=True,
    )
    if diff.returncode != 0:
        print(diff.stderr.strip() or "git diff failed", file=sys.stderr)
        return None
    return [line for line in diff.stdout.splitlines() if line]


def emit(needed: bool) -> None:
    line = f"c_vectors={'true' if needed else 'false'}"
    print(line)
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as handle:
            handle.write(line + "\n")


def selftest() -> int:
    cases = {
        "scripts/export_c_basis_hints.py": True,
        "external/PRISM_v2/src/prism.c": True,
        "prism-verify/testdata/c_basis_hints.txt": True,
        "prism-verify/src/lib.rs": False,
        "docs/C_REFERENCE.md": False,
    }
    for path, expect in cases.items():
        if wanted(path) != expect:
            print(f"selftest: {path} wanted={wanted(path)}", file=sys.stderr)
            return 1
    print("c-vectors scope selftest passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true")
    parser.add_argument("--base", default="")
    parser.add_argument("--head", default="")
    args = parser.parse_args()
    if args.selftest:
        if selftest() != 0:
            return 1
        if not args.base:
            return 0
    changed = names(args.base, args.head)
    if changed is None:
        emit(True)
        return 0
    hits = [path for path in changed if wanted(path)]
    if hits:
        print("c-vectors inputs:", ", ".join(hits), file=sys.stderr)
    emit(bool(hits))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
