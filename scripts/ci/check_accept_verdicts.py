#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Every file in ``testdata/accept`` has an ``Ok`` row in the verdict list.

The Sage model does not run in CI. The committed list is what this job
reads, and it reads the directory rather than a fixed set of names: a
file with no row, a row with no file, or a verdict other than ``Ok``
fails. The list sits next to the model report.
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ACCEPT = ROOT / "prism-verify" / "testdata" / "accept"
VERDICTS = ROOT / "prism-isogeny" / "testdata" / "s5_model" / "accept_verdicts.tsv"


def load_rows(text: str) -> list[tuple[str, str, str]]:
    lines = [line for line in text.splitlines() if line]
    if not lines or lines[0] != "file\tlevel\tverdict":
        raise SystemExit("verdict list has no header")
    rows = []
    for line in lines[1:]:
        parts = line.split("\t")
        if len(parts) != 3:
            raise SystemExit(f"verdict row is not three fields: {line}")
        rows.append((parts[0], parts[1], parts[2]))
    return rows


def level_of(text: str) -> str | None:
    for line in text.splitlines():
        if line.startswith("level "):
            return line.split(" ", 1)[1]
    return None


def check(files: list[str], rows: list[tuple[str, str, str]]) -> list[str]:
    errors = []
    if not files:
        errors.append("accept directory has no vectors")
    if not rows:
        errors.append("verdict list has no rows")
    by_name = {}
    for name, level, verdict in rows:
        if name in by_name:
            errors.append(f"duplicate verdict {name}")
        by_name[name] = (level, verdict)
        if verdict != "Ok":
            errors.append(f"{name} verdict {verdict}")
    for name in files:
        if name not in by_name:
            errors.append(f"no verdict for {name}")
    for name in by_name:
        if name not in files:
            errors.append(f"verdict for a missing file {name}")
    return errors


def selftest() -> int:
    errors = check(["a.txt"], [("a.txt", "NIST-I", "Ok")])
    if errors:
        print(f"selftest: a matching row failed: {errors}", file=sys.stderr)
        return 1
    if not check(["a.txt"], [("a.txt", "NIST-I", "Kernel")]):
        print("selftest: a non-Ok verdict passed", file=sys.stderr)
        return 1
    if not check(["a.txt", "b.txt"], [("a.txt", "NIST-I", "Ok")]):
        print("selftest: a missing row passed", file=sys.stderr)
        return 1
    print("accept verdict selftest passed")
    return 0


def main() -> int:
    if len(sys.argv) > 1 and sys.argv[1] == "--selftest":
        return selftest()
    files = sorted(path.name for path in ACCEPT.glob("*.txt"))
    rows = load_rows(VERDICTS.read_text())
    errors = check(files, rows)
    by_name = {name: level for name, level, _verdict in rows}
    for path in ACCEPT.glob("*.txt"):
        got = level_of(path.read_text())
        expected = by_name.get(path.name)
        if expected is not None and got != expected:
            errors.append(f"{path.name} level {got} disagrees with {expected}")
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print(f"accept {len(files)} files, each Ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
