#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Decide whether the C-vector rebuild has to run.

It runs when the range touches export scripts, reference sources, testdata,
Rust field/isogeny/verifier inputs, dependency pins or the compiler/CI setup. Anything else — a missing base, a git error, an unnamed range —
is `true`. Skipping the rebuild on a guess is the failure; running it is
the cost.
"""

import argparse
import os
import pathlib
import subprocess
import sys
import tempfile


def wanted(path: str) -> bool:
    if path in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".github/workflows/ci.yml"):
        return True
    if path.startswith(("scripts/", "external/", "prism-field/", "prism-isogeny/", "prism-verify/", "prism-compare/")):
        return True
    return "testdata" in path.split("/")


def paths_from_name_status(raw: bytes) -> list[str] | None:
    """Paths from `git diff --name-status --no-renames -z`.

    Rename detection would report only the destination, so a move out of
    `scripts/` or `testdata/` would look out of scope. NUL-delimited names
    are not quoted, so a non-ASCII path keeps its prefix.
    """
    parts = raw.split(b"\0")
    if parts and parts[-1] == b"":
        parts.pop()
    paths = []
    index = 0
    while index < len(parts):
        try:
            status = parts[index].decode()
        except UnicodeDecodeError:
            return None
        index += 1
        if index >= len(parts):
            return None
        try:
            paths.append(parts[index].decode())
        except UnicodeDecodeError:
            return None
        index += 1
        # A rename or copy record carries the destination after the source.
        # `--no-renames` should not emit one; accept it if git does.
        if status[:1] in {"R", "C"}:
            if index >= len(parts):
                return None
            try:
                paths.append(parts[index].decode())
            except UnicodeDecodeError:
                return None
            index += 1
    return paths


def names(base: str, head: str, cwd: pathlib.Path | None = None) -> list[str] | None:
    if not base or set(base) <= {"0"}:
        print("no base; rebuilding the vectors", file=sys.stderr)
        return None
    merge_base = subprocess.run(
        ["git", "merge-base", base, head],
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )
    if merge_base.returncode != 0:
        print(merge_base.stderr.strip() or "merge-base failed", file=sys.stderr)
        return None
    diff = subprocess.run(
        [
            "git",
            "diff",
            "--name-status",
            "--no-renames",
            "-z",
            merge_base.stdout.strip(),
            head,
        ],
        cwd=cwd,
        check=False,
        capture_output=True,
    )
    if diff.returncode != 0:
        print(diff.stderr.decode(errors="replace").strip() or "git diff failed", file=sys.stderr)
        return None
    return paths_from_name_status(diff.stdout)


def emit(needed: bool) -> None:
    line = f"c_vectors={'true' if needed else 'false'}"
    print(line)
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as handle:
            handle.write(line + "\n")


def commit(root: pathlib.Path, message: str) -> str:
    subprocess.run(
        ["git", "-c", "user.email=t@example.com", "-c", "user.name=t", "-c", "commit.gpgsign=false", "commit", "-qm", message],
        cwd=root,
        check=True,
    )
    return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()


def selftest() -> int:
    cases = {
        "scripts/export_c_basis_hints.py": True,
        "external/PRISM_v2/src/prism.c": True,
        "prism-verify/testdata/c_basis_hints.txt": True,
        "prism-verify/src/lib.rs": True,
        "prism-isogeny/src/torsion.rs": True,
        "prism-compare/src/lib.rs": True,
        "prism-isogeny/src/jac.rs": True,
        "prism-field/src/lib.rs": True,
        "prism-field/Cargo.toml": True,
        "Cargo.toml": True,
        "Cargo.lock": True,
        "rust-toolchain.toml": True,
        ".github/workflows/ci.yml": True,
        "prism-bench/src/main.rs": False,
        "docs/C_REFERENCE.md": False,
    }
    for path, expect in cases.items():
        if wanted(path) != expect:
            print(f"selftest: {path} wanted={wanted(path)}", file=sys.stderr)
            return 1
    renamed = paths_from_name_status(b"R100\0scripts/old.py\0docs/moved.py\0")
    if renamed is None or not any(wanted(path) for path in renamed):
        print(f"selftest: a rename out of scripts/ was missed: {renamed}", file=sys.stderr)
        return 1
    with tempfile.TemporaryDirectory() as tmp:
        root = pathlib.Path(tmp)
        subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
        subprocess.run(["git", "config", "diff.renames", "true"], cwd=root, check=True)
        scripts = root / "scripts"
        scripts.mkdir()
        source = scripts / "old.py"
        source.write_text("a\n")
        subprocess.run(["git", "add", "scripts/old.py"], cwd=root, check=True)
        base = commit(root, "add")
        docs = root / "docs"
        docs.mkdir()
        source.rename(docs / "old.py")
        subprocess.run(["git", "add", "-A"], cwd=root, check=True)
        head = commit(root, "move")
        moved = names(base, head, cwd=root)
        if moved is None or not any(wanted(path) for path in moved):
            print(f"selftest: git hid the scripts/ side of a rename: {moved}", file=sys.stderr)
            return 1
        accented = scripts / "é.py"
        accented.write_text("b\n")
        subprocess.run(["git", "add", "-A"], cwd=root, check=True)
        later = commit(root, "accent")
        named = names(head, later, cwd=root)
        if named is None or "scripts/é.py" not in named or not wanted("scripts/é.py"):
            print(f"selftest: a non-ASCII scripts path was quoted away: {named}", file=sys.stderr)
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
