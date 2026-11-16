#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The committed Sage files name the committed Sage script as their source.

`scripts/sage/no_corpus_fixture.sage` writes two files for each set that
has no vectors and records its own SHA-256 in the header of each. CI has
no Sage, so it cannot run the script again. What it holds is narrower:
each file's header records the digest of the script in the tree, the
pinned image, and the command for its own set. An edit to the script
without a new run of every set fails here. An edit to a file's data lines
does not: only a run of the script in the image shows that, and the
command is in the script.

The Rust tests compare the verifier with the files. This check is about
where the files came from.
"""
import hashlib
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "sage" / "no_corpus_fixture.sage"
TESTDATA = ROOT / "prism-verify" / "testdata"
# Every set the script writes files for. A set the script names and this
# does not, or the other way round, fails below.
SETS = ("p324", "p664")
KINDS = ("field", "curves")
IMAGE = "sagemath/sagemath@sha256:4f5589eb6c565949a006f8665de2876b8414410daf5ac554f4434a15d4f3d528"


def sets_of_the_script(source: str) -> tuple[str, ...]:
    """The names in the script's `SETS` table."""
    table = re.search(r"^SETS = \{\n(.*?)^\}", source, re.M | re.S)
    if table is None:
        raise SystemExit("the script has no SETS table")
    return tuple(re.findall(r'^\s*"(\w+)":', table.group(1), re.M))


def main():
    if not SCRIPT.is_file():
        raise SystemExit(f"the Sage script is absent: {SCRIPT.relative_to(ROOT)}")
    source = SCRIPT.read_text()
    digest = hashlib.sha256(SCRIPT.read_bytes()).hexdigest()
    if IMAGE not in source:
        raise SystemExit("the script no longer names the pinned Sage image")
    if sets_of_the_script(source) != SETS:
        raise SystemExit(f"the script writes {sets_of_the_script(source)} and this check holds {SETS}")
    for name in SETS:
        command = f"# Command: sage scripts/sage/{SCRIPT.name} {name} (from the repository root)"
        for kind in KINDS:
            path = TESTDATA / f"{name}_{kind}.tsv"
            if not path.is_file():
                raise SystemExit(f"a Sage file is absent: {path.relative_to(ROOT)}")
            header = [line for line in path.read_text().splitlines() if line.startswith("#")]
            recorded = [
                m.group(1)
                for line in header
                if (m := re.fullmatch(r"# Script sha256 when run: ([0-9a-f]{64})", line))
            ]
            if len(recorded) != 1:
                raise SystemExit(f"{path.name} does not record the script's digest exactly once")
            if recorded[0] != digest:
                raise SystemExit(
                    f"{path.name} records another version of {SCRIPT.name}; run the script again for every set"
                )
            if f"# Image: {IMAGE}" not in header:
                raise SystemExit(f"{path.name} does not name the pinned Sage image")
            if command not in header:
                raise SystemExit(f"{path.name} does not record the command for {name}")
    files = len(SETS) * len(KINDS)
    print(f"{files} Sage files of {', '.join(SETS)} record {SCRIPT.name} at {digest[:16]}… and the pinned image")


if __name__ == "__main__":
    main()
