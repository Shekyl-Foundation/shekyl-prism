#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The committed Sage files name the committed Sage script as their source.

`scripts/sage/p324_fixture.sage` writes two files and records its own
SHA-256 in the header of each. CI has no Sage, so it cannot run the script
again. What it holds is narrower: each file's header records the digest
of the script in the tree and the pinned image. An edit to the script
without a new run fails here. An edit to a file's data lines does not:
only a run of the script in the image shows that, and the command is in
the script.

The Rust tests compare the verifier with the files. This check is about
where the files came from.
"""
import hashlib
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "sage" / "p324_fixture.sage"
FILES = [
    ROOT / "prism-verify" / "testdata" / "p324_field.tsv",
    ROOT / "prism-verify" / "testdata" / "p324_curves.tsv",
]
IMAGE = "sagemath/sagemath@sha256:4f5589eb6c565949a006f8665de2876b8414410daf5ac554f4434a15d4f3d528"


def main():
    if not SCRIPT.is_file():
        raise SystemExit(f"the Sage script is absent: {SCRIPT.relative_to(ROOT)}")
    digest = hashlib.sha256(SCRIPT.read_bytes()).hexdigest()
    if IMAGE not in SCRIPT.read_text():
        raise SystemExit("the script no longer names the pinned Sage image")
    for path in FILES:
        if not path.is_file():
            raise SystemExit(f"a Sage file is absent: {path.relative_to(ROOT)}")
        header = [line for line in path.read_text().splitlines() if line.startswith("#")]
        recorded = [m.group(1) for line in header if (m := re.fullmatch(r"# Script sha256 when run: ([0-9a-f]{64})", line))]
        if len(recorded) != 1:
            raise SystemExit(f"{path.name} does not record the script's digest exactly once")
        if recorded[0] != digest:
            raise SystemExit(
                f"{path.name} records another version of {SCRIPT.name}; run the script again and commit both files"
            )
        if f"# Image: {IMAGE}" not in header:
            raise SystemExit(f"{path.name} does not name the pinned Sage image")
    print(f"both p324 Sage files record {SCRIPT.name} at {digest[:16]}… and the pinned image")


if __name__ == "__main__":
    main()
