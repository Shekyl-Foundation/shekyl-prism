#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Every place a secret integer leaves has a row in the treatment table.

`prism-int` lets a secret out by three functions and no other:
`SecretInt::declassify`, `SecretBit::declassify` and
`SecretInt::export_le_bytes`. `docs/SIGNER_SPEC.md` (2.1.3) has a table
with one row for each call site outside that crate: what leaves, why, and
how the last stage treats it. This check holds the two together.

1. The three functions are defined in `prism-int/src/secret.rs`, each
   once (the subject exists). A rename would otherwise leave this check
   counting nothing.
2. The table is found, and each of its rows names one of the two method
   names in its `Exit` column. The placeholder row, whose first cell is a
   dash, counts as no row.
3. The call sites of each method name in the Rust sources of the
   workspace, outside `prism-int`, are as many as the table's rows for
   that name. A call site is the name after `.` or after `::`, called or
   passed as a function (`.map(SecretInt::declassify)`), on a line that is
   not a comment.

The count is by method name. `declassify` is one name for two functions,
on the integer and on the bit; a row says which in its own words.

`--selftest` plants each breach in a copy.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CRATE = "prism-int"
DEFINITIONS = "prism-int/src/secret.rs"
SPEC = "docs/SIGNER_SPEC.md"
# The method name, and how many functions of the crate have it.
EXITS = {"declassify": 2, "export_le_bytes": 1}
HEADER = "| # | Stage | Call site | Exit |"
SKIP = {"target", "external", ".git", CRATE}


def sources(root):
    """Every Rust source of the workspace outside the integer crate, as text."""
    found = {}
    for path in sorted(root.rglob("*.rs")):
        relative = path.relative_to(root)
        if SKIP.intersection(relative.parts):
            continue
        found[str(relative)] = path.read_text()
    return found


def definitions_fault(text):
    """Rule 1."""
    for name, expected in EXITS.items():
        found = len(re.findall(rf"^\s*pub fn {name}\(", text, flags=re.M))
        if found != expected:
            return f"{DEFINITIONS} defines `{name}` {found} times, expected {expected}"
    return None


def table_rows(spec):
    """The `Exit` cell of each row of the table, or a fault as a string."""
    lines = spec.splitlines()
    starts = [index for index, line in enumerate(lines) if line.startswith(HEADER)]
    if len(starts) != 1:
        return f"{SPEC} has {len(starts)} secret-treatment tables, expected one"
    rows = []
    for line in lines[starts[0] + 2:]:
        if not line.startswith("|"):
            break
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if cells[0] in ("—", "-", ""):
            continue
        if len(cells) < 4:
            return f"{SPEC}: a row of the secret-treatment table is short: {line}"
        named = [name for name in EXITS if name in cells[3]]
        if len(named) != 1:
            return f"{SPEC}: the Exit cell `{cells[3]}` does not name one of {', '.join(EXITS)}"
        rows.append(named[0])
    return rows


def call_sites(texts):
    """`(file, line number, name)` for every call of an exit."""
    found = []
    for name_of_file, text in texts.items():
        for number, line in enumerate(text.splitlines(), start=1):
            code = line.split("//", 1)[0]
            for name in EXITS:
                for _ in re.finditer(rf"(\.|::){name}\b", code):
                    found.append((name_of_file, number, name))
    return found


def fault(definitions, spec, texts):
    """The first breach, or None."""
    problem = definitions_fault(definitions)
    if problem:
        return problem
    rows = table_rows(spec)
    if isinstance(rows, str):
        return rows
    sites = call_sites(texts)
    for name in EXITS:
        calls = [site for site in sites if site[2] == name]
        listed = rows.count(name)
        if len(calls) != listed:
            where = "; ".join(f"{file}:{number}" for file, number, _ in calls) or "none"
            return (f"`{name}` has {len(calls)} call sites outside {CRATE} and {listed} rows in the "
                    f"secret-treatment table of {SPEC}. Call sites: {where}")
    return None


def selftest():
    definitions = (ROOT / DEFINITIONS).read_text()
    spec = (ROOT / SPEC).read_text()
    # The plants start from an empty table, whatever the real one holds.
    empty = spec[: spec.index(HEADER)] + HEADER + " What leaves |\n| --- | --- | --- | --- | --- |\n| — | — | none yet | | |\n"
    row = "| 1 | 2b | `a.rs`, `f` | `SecretInt::declassify` | a norm |\n"
    call = {"prism-quat/src/a.rs": "fn f(x: &SecretInt<4>) { let y = x.declassify(); }\n"}
    cases = (
        ("an empty table and no call site", definitions, empty, {}, None),
        ("a call site without a row", definitions, empty, call, "call sites"),
        ("a row without a call site", definitions, empty + row, {}, "call sites"),
        ("a row and its call site", definitions, empty + row, call, None),
        ("a call in a comment", definitions, empty, {"a.rs": "// x.declassify()\n"}, None),
        ("a path call", definitions, empty, {"a.rs": "let y = SecretBit::declassify(&b);\n"}, "call sites"),
        ("an exit passed as a function", definitions, empty, {"a.rs": "let ys = xs.iter().map(SecretInt::declassify);\n"}, "call sites"),
        ("a longer name", definitions, empty, {"a.rs": "x.declassify_later();\n"}, None),
        ("an export without a row", definitions, empty, {"a.rs": "x.export_le_bytes(&mut b, &mut s)?;\n"}, "call sites"),
        ("a row that names no exit", definitions, empty + row.replace("declassify", "reveal"), {}, "does not name"),
        ("a renamed definition", definitions.replace("pub fn export_le_bytes(", "pub fn export_bytes("), empty, {}, "defines"),
        ("a second definition", definitions + "\n    pub fn declassify(&self) {}\n", empty, {}, "defines"),
        ("no table", definitions, "nothing\n", {}, "tables"),
    )
    for name, source, document, texts, expected in cases:
        got = fault(source, document, texts)
        if (got is None) != (expected is None) or (expected and expected not in got):
            print(f"selftest: {name}: got {got!r}", file=sys.stderr)
            return 1
    print(f"selftest: {len(cases)} cases")
    return 0


def main():
    if sys.argv[1:] == ["--selftest"]:
        return selftest()
    if sys.argv[1:]:
        print("usage: check_secret_exits.py [--selftest]", file=sys.stderr)
        return 2
    texts = sources(ROOT)
    if not texts:
        print("no Rust source found outside the integer crate", file=sys.stderr)
        return 1
    problem = fault((ROOT / DEFINITIONS).read_text(), (ROOT / SPEC).read_text(), texts)
    if problem:
        print(problem, file=sys.stderr)
        return 1
    rows = table_rows((ROOT / SPEC).read_text())
    print(f"{len(rows)} call sites of the secret exits outside {CRATE}, and {len(rows)} rows; "
          f"{len(texts)} source files read")
    return 0


if __name__ == "__main__":
    sys.exit(main())
