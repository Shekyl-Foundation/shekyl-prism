#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Every first-party .rs, .py, and .c file carries the Foundation copyright.

`external/` is the pinned C tree and is not ours. A walk that sees no
first-party file fails: an empty scan is not a clean scan.
"""

import argparse
import pathlib
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
MARKER = "The Shekyl Foundation"
EXTENSIONS = {".rs", ".py", ".c"}
SKIP_DIRS = {"external", "target", ".git", "corpus", "artifacts"}
HEADER_LINES = 12

# Files the walk is not allowed to miss. Deleting one fails the gate even
# when every other file still has a header.
REQUIRED = (
    "prism-field/src/lib.rs",
    "prism-isogeny/src/lib.rs",
    "prism-verify/src/lib.rs",
    "prism-verify/src/bin/degree_check.rs",
    "prism-bench/src/main.rs",
    "scripts/check_c_reference.py",
    "scripts/export_c_basis_hints.py",
    "scripts/differential_prime.py",
    "scripts/c/export_basis_hint.c",
    "fuzz/fuzz_targets/verify_level_1.rs",
    "fuzz/fuzz_targets/verify_level_3.rs",
    "fuzz/fuzz_targets/verify_level_5.rs",
    "fuzz/fuzz_targets/decode_public_key.rs",
    "fuzz/fuzz_targets/decode_signature.rs",
)


def has_header(path: pathlib.Path) -> bool:
    lines = path.read_text(errors="replace").splitlines()[:HEADER_LINES]
    return any(MARKER in line for line in lines)


def first_party(root: pathlib.Path) -> list[pathlib.Path]:
    found = []
    for dirpath, dirnames, filenames in root.walk():
        dirnames[:] = [name for name in dirnames if name not in SKIP_DIRS]
        for name in filenames:
            path = dirpath / name
            if path.suffix in EXTENSIONS:
                found.append(path)
    return found


def scan(root: pathlib.Path) -> list[str]:
    errors = []
    files = first_party(root)
    if not files:
        return ["no first-party .rs, .py, or .c file"]
    seen = {path.relative_to(root).as_posix() for path in files}
    for required in REQUIRED:
        if required not in seen:
            errors.append(f"missing {required}")
    for path in files:
        if not has_header(path):
            errors.append(f"no copyright header: {path.relative_to(root)}")
    return errors


def selftest() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = pathlib.Path(tmp)
        good = root / "ok.rs"
        good.write_text("// Copyright (c) 2025-2026, The Shekyl Foundation\nfn main() {}\n")
        bad = root / "bad.py"
        bad.write_text("print('hello')\n")
        errors = scan(root)
        if not any("bad.py" in error for error in errors):
            print("selftest: a file without the header passed", file=sys.stderr)
            return 1
        if not any("missing " in error for error in errors):
            print("selftest: a missing required file passed", file=sys.stderr)
            return 1
        if any("ok.rs" in error for error in errors):
            print("selftest: a headed file was refused", file=sys.stderr)
            return 1
    print("copyright selftest passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest:
        if selftest() != 0:
            return 1
    errors = scan(ROOT)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print(f"copyright headers present on {len(first_party(ROOT))} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
