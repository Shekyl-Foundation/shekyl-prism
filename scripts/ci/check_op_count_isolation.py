#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Hold the operation counter out of every default build.

`op-count` is a feature of prism-field. Cargo unifies features across a
build, so one member naming it in a manifest would turn the counter on for
every crate built with that member. This check requires that:

1. prism-field declares `op-count`, off by default (the subject exists);
2. no manifest in the repository names it anywhere else;
3. the resolved feature graph of prism-verify does not contain it by
   default, and does contain it when given on the command line (so the
   test can see the feature it says is absent).
"""
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
FEATURE = "op-count"


def manifests():
    found = [ROOT / "Cargo.toml", ROOT / "fuzz" / "Cargo.toml"]
    found += sorted(ROOT.glob("prism-*/Cargo.toml"))
    missing = [p for p in found if not p.is_file()]
    if missing:
        raise SystemExit(f"expected manifest is absent: {missing[0]}")
    return found


def field_features(*extra):
    """The features prism-field is built with under prism-verify."""
    result = subprocess.run(
        ["cargo", "tree", "--locked", "-p", "prism-verify", "-e", "normal",
         "--prefix", "none", "-f", "{p}\t{f}", *extra],
        cwd=ROOT, text=True, capture_output=True,
    )
    if result.returncode:
        raise SystemExit("cargo tree failed:\n" + result.stderr)
    rows = [line.split("\t")[1] for line in result.stdout.splitlines()
            if line.startswith("prism-field ") and "\t" in line]
    if not rows:
        raise SystemExit("prism-field is not in the graph of prism-verify")
    # A repeated node is printed with a trailing " (*)".
    return {name for row in rows for name in row.replace(" (*)", "").split(",") if name}


def main():
    field = tomllib.loads((ROOT / "prism-field/Cargo.toml").read_text())
    features = field.get("features", {})
    if features.get(FEATURE) != []:
        raise SystemExit("prism-field must declare op-count with no dependencies")
    if features.get("default") != []:
        raise SystemExit("prism-field default features must be empty")
    for path in manifests():
        text = path.read_text()
        allowed = 1 if path == ROOT / "prism-field/Cargo.toml" else 0
        # Comments may explain the rule; only manifest content is counted.
        content = "\n".join(line.split("#", 1)[0] for line in text.splitlines())
        if content.count(FEATURE) != allowed:
            raise SystemExit(f"{path.relative_to(ROOT)} names {FEATURE}; it is command-line only")
    if FEATURE in field_features():
        raise SystemExit("the default feature graph of prism-verify contains op-count")
    if FEATURE not in field_features("--features", f"prism-field/{FEATURE}"):
        raise SystemExit("op-count did not appear when requested; this check cannot see it")
    print("op-count is absent from the default feature graph and from every manifest but its own")


if __name__ == "__main__":
    main()
