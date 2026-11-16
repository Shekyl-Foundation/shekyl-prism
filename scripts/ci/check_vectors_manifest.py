#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The committed test data is the data the manifest lists.

`prism-vectors/data.sha256` has one line for every file under
`prism-vectors/data`: its SHA-256 and its path, in `sha256sum` form,
sorted by path. A data file cannot change, appear or disappear without
that file changing in the same commit, so a change to the regression
anchor is always a visible line in a diff.

    check_vectors_manifest.py            what CI runs
    check_vectors_manifest.py --write    rewrite the manifest from the files
    check_vectors_manifest.py --moved    the move of 2026-11-16 changed no byte
    check_vectors_manifest.py --selftest

The default check reads the files git tracks, so it does not depend on
what else is in the directory, and a manifest with no lines fails: a
comparison of nothing is not a pass.

`--moved` is the record of one event. On 2026-11-16 every committed test
file moved from the `testdata` directory of three crates to
`prism-vectors/data`. `prism-vectors/manifest/at_ddbcaf9.sha256` is the
same list taken from the git objects of the commit before the move.
`--moved` maps each of its paths by directory, with no other change, and
requires the result to equal the manifest as it was at the move, line
for line: the same files, under the same names, with the same digests.
The manifest at the move is `at_the_move.sha256` beside it, so this
keeps passing when a later change regenerates a vector.
"""
import hashlib
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = "prism-vectors/data"
MANIFEST = ROOT / "prism-vectors" / "data.sha256"
BEFORE = ROOT / "prism-vectors" / "manifest" / "at_ddbcaf9.sha256"
AT_THE_MOVE = ROOT / "prism-vectors" / "manifest" / "at_the_move.sha256"
# Where each directory of the commit before the move went.
MOVED = (
    ("prism-verify/testdata/", "prism-vectors/data/verify/"),
    ("prism-isogeny/testdata/", "prism-vectors/data/isogeny/"),
    ("prism-compare/testdata/", "prism-vectors/data/compare/"),
)


def tracked():
    """Every file git tracks under the data directory, sorted by path."""
    listed = subprocess.run(
        ["git", "ls-files", "-z", "--", DATA], cwd=ROOT, check=True, capture_output=True
    ).stdout
    return sorted(name.decode() for name in listed.split(b"\0") if name)


def rendered(paths):
    """The manifest of `paths` as they are in the working tree."""
    lines = []
    for path in paths:
        digest = hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
        lines.append(f"{digest}  {path}\n")
    return "".join(lines)


def parsed(text, name):
    """`(digest, path)` for every line; a line in another form is a fault."""
    rows = []
    for number, line in enumerate(text.splitlines(), 1):
        digest, separator, path = line.partition("  ")
        if not separator or len(digest) != 64 or not path:
            raise SystemExit(f"{name}:{number}: not `<sha256>  <path>`")
        rows.append((digest, path))
    if not rows:
        raise SystemExit(f"{name} lists no file")
    return rows


def fault(manifest_text, paths, render):
    """Why the manifest is not the data, or None."""
    rows = parsed(manifest_text, MANIFEST.name)
    listed = [path for _, path in rows]
    if listed != sorted(listed):
        return f"{MANIFEST.name} is not sorted by path"
    if len(set(listed)) != len(listed):
        return f"{MANIFEST.name} lists a path twice"
    missing = sorted(set(paths) - set(listed))
    if missing:
        return f"{missing[0]} is committed and not in {MANIFEST.name} ({len(missing)} such)"
    extra = sorted(set(listed) - set(paths))
    if extra:
        return f"{MANIFEST.name} lists {extra[0]}, which is not committed ({len(extra)} such)"
    if render(paths) != manifest_text:
        actual = dict((path, digest) for digest, path in parsed(render(paths), "the data"))
        changed = [path for digest, path in rows if actual[path] != digest]
        return f"{changed[0]} is not the bytes {MANIFEST.name} records ({len(changed)} such)"
    return None


def mapped(path):
    """Where a path of the commit before the move is now."""
    for old, new in MOVED:
        if path.startswith(old):
            return new + path[len(old):]
    raise SystemExit(f"{BEFORE.name}: {path} is under no directory that moved")


def moved_fault(before_text, after_text):
    """Why the move was not a move, or None."""
    before = parsed(before_text, BEFORE.name)
    after = parsed(after_text, AT_THE_MOVE.name)
    expected = sorted((mapped(path), digest) for digest, path in before)
    actual = sorted((path, digest) for digest, path in after)
    if len(expected) != len(actual):
        return f"{len(before)} files before the move and {len(after)} after"
    for (old_path, old_digest), (new_path, new_digest) in zip(expected, actual):
        if old_path != new_path:
            return f"{old_path} was expected and {new_path} is there"
        if old_digest != new_digest:
            return f"{new_path} changed in the move"
    return None


def selftest():
    paths = tracked()
    good = rendered(paths)
    if fault(good, paths, rendered) is not None:
        raise SystemExit("selftest: a manifest written from the files fails")
    lines = good.splitlines(keepends=True)
    flipped = ("0" if lines[0][0] != "0" else "1") + lines[0][1:]
    planted = {
        "a file's bytes changed": "".join([flipped] + lines[1:]),
        "a committed file is not listed": "".join(lines[1:]),
        "a listed file is not committed": good + f"{'0' * 64}  {DATA}/zz/absent.txt\n",
        "the list is out of order": "".join([lines[1], lines[0]] + lines[2:]),
        "a path is listed twice": "".join([lines[0]] + lines),
    }
    for case, text in planted.items():
        if fault(text, paths, rendered) is None:
            raise SystemExit(f"selftest: not caught: {case}")
    before = BEFORE.read_text()
    at_the_move = AT_THE_MOVE.read_text()
    if moved_fault(before, at_the_move) is not None:
        raise SystemExit("selftest: the recorded move fails")
    rows = at_the_move.splitlines(keepends=True)
    changed = ("0" if rows[0][0] != "0" else "1") + rows[0][1:]
    renamed = rows[0].rstrip("\n") + ".renamed\n"
    moves = {
        "a digest changed in the move": "".join([changed] + rows[1:]),
        "a file was dropped in the move": "".join(rows[1:]),
        "a file was renamed in the move": "".join([renamed] + rows[1:]),
    }
    for case, text in moves.items():
        if moved_fault(before, text) is None:
            raise SystemExit(f"selftest: not caught: {case}")
    print(
        f"selftest: {len(planted)} planted manifests and {len(moves)} planted moves refused"
    )


def main():
    arguments = sys.argv[1:]
    if arguments == ["--selftest"]:
        selftest()
        return
    if arguments == ["--write"]:
        paths = tracked()
        if not paths:
            raise SystemExit(f"no tracked file under {DATA}")
        MANIFEST.write_text(rendered(paths))
        print(f"wrote {MANIFEST.relative_to(ROOT)}: {len(paths)} files")
        return
    if arguments == ["--moved"]:
        problem = moved_fault(BEFORE.read_text(), AT_THE_MOVE.read_text())
        if problem:
            raise SystemExit(problem)
        count = len(parsed(BEFORE.read_text(), BEFORE.name))
        print(
            f"{count} files of {BEFORE.name} are the {count} of {AT_THE_MOVE.name}: "
            "each under its mapped directory, with the same name and the same digest"
        )
        return
    if arguments:
        raise SystemExit("usage: check_vectors_manifest.py [--write | --moved | --selftest]")
    paths = tracked()
    if not paths:
        raise SystemExit(f"no tracked file under {DATA}")
    problem = fault(MANIFEST.read_text(), paths, rendered)
    if problem:
        raise SystemExit(problem)
    print(f"{len(paths)} files under {DATA} are the bytes {MANIFEST.relative_to(ROOT)} records")


if __name__ == "__main__":
    main()
