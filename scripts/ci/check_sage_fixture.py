#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The committed Sage files name the committed Sage script as their source.

`scripts/sage/no_corpus_fixture.sage` writes two files for each set that
has no vectors and records its own SHA-256 in the header of each. CI has
no Sage, so it cannot run the script again. What it holds is narrower:
each file's header records the digest of the script as it was when it
ran, the pinned image, and the command for its own set. An edit to the
script without a new run of every set fails here. An edit to a file's
data lines does not: only a run of the script in the image shows that,
and the command is in the script.

The script as it was when it ran. On 2026-11-16 the committed files moved
to `prism-vectors/data/verify` and the script's output directory moved
with them, in three lines. The files were not written again: a move
changes no content, and a header's digest is content. So the digest they
record is of the script with those three lines as they were, and
`SINCE_THE_RUN` lists them. This check puts each back, requires each to
be in the script exactly once, and compares the digest of the result. Any
other edit to the script still fails. The next run of the script rewrites
the headers, and the list is emptied in the same change.

`--selftest` shows that: an edit outside the listed lines is refused, and
so is a listed line that is absent or there twice.

The Rust tests compare the verifier with the files. This check is about
where the files came from.
"""
import hashlib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "sage" / "no_corpus_fixture.sage"
DATA = ROOT / "prism-vectors" / "data" / "verify"
# Every set the script writes files for. A set the script names and this
# does not, or the other way round, fails below.
SETS = ("p324", "p664")
KINDS = ("field", "curves")
IMAGE = "sagemath/sagemath@sha256:4f5589eb6c565949a006f8665de2876b8414410daf5ac554f4434a15d4f3d528"
# Lines of the script that changed after the committed files were
# written, as (the line now, the line when the script ran). Empty when the
# files were written by the script as it is.
SINCE_THE_RUN = (
    (
        "#   prism-vectors/data/verify/<set>_field.tsv    field arithmetic in F_p, F_p^2\n",
        "#   prism-verify/testdata/<set>_field.tsv    field arithmetic in F_p, F_p^2\n",
    ),
    (
        "#   prism-vectors/data/verify/<set>_curves.tsv   supersingular curves, points of\n",
        "#   prism-verify/testdata/<set>_curves.tsv   supersingular curves, points of\n",
    ),
    (
        '    out = ROOT / "prism-vectors" / "data" / "verify"\n',
        '    out = ROOT / "prism-verify" / "testdata"\n',
    ),
)


def digest_when_run(source: str) -> str:
    """The SHA-256 of the script with every line of `SINCE_THE_RUN` put back."""
    for now, then in SINCE_THE_RUN:
        if source.count(now) != 1:
            raise SystemExit(
                f"the script has {source.count(now)} lines {now.strip()!r}; "
                "SINCE_THE_RUN needs exactly one to put back"
            )
        source = source.replace(now, then)
    return hashlib.sha256(source.encode()).hexdigest()


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
    digest = digest_when_run(source)
    if IMAGE not in source:
        raise SystemExit("the script no longer names the pinned Sage image")
    if sets_of_the_script(source) != SETS:
        raise SystemExit(f"the script writes {sets_of_the_script(source)} and this check holds {SETS}")
    for name in SETS:
        command = f"# Command: sage scripts/sage/{SCRIPT.name} {name} (from the repository root)"
        for kind in KINDS:
            path = DATA / f"{name}_{kind}.tsv"
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
    since = f", {len(SINCE_THE_RUN)} listed lines put back" if SINCE_THE_RUN else ""
    print(
        f"{files} Sage files of {', '.join(SETS)} record {SCRIPT.name} at {digest[:16]}…"
        f"{since} and the pinned image"
    )


def refused(source: str) -> bool:
    """Whether `digest_when_run` stops on `source`."""
    try:
        digest_when_run(source)
    except SystemExit:
        return True
    return False


def selftest():
    source = SCRIPT.read_text()
    held = digest_when_run(source)
    if digest_when_run(source + "# an edit\n") == held:
        raise SystemExit("selftest: an edit outside the listed lines kept the digest")
    for now, then in SINCE_THE_RUN:
        if not refused(source.replace(now, "", 1)):
            raise SystemExit(f"selftest: a listed line was absent and accepted: {now.strip()}")
        if not refused(source + now):
            raise SystemExit(f"selftest: a listed line was there twice and accepted: {now.strip()}")
        if now == then:
            raise SystemExit(f"selftest: a listed line maps to itself: {now.strip()}")
    print(
        f"selftest: an edit outside the {len(SINCE_THE_RUN)} listed lines changes the digest; "
        "a listed line absent or doubled is refused"
    )


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        selftest()
    elif sys.argv[1:]:
        raise SystemExit("usage: check_sage_fixture.py [--selftest]")
    else:
        main()
