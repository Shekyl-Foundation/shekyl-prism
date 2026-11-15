#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""`prism-fixtures` is in no build but a test's, a reporter's or a fuzzer's.

The crate reads committed test files. Nothing that ships may depend on
it, so it may appear under `[dev-dependencies]` of a workspace member and
never under `[dependencies]` or `[build-dependencies]`. The fuzz crate is
its own workspace and is not shipped; it depends on it normally.

1. the crate is a workspace member (the subject exists);
2. the tests that read fixtures name it as a dev-dependency (the subject
   is in use, so an empty scan is not a clean scan);
3. no member names it as a normal or build dependency;
4. it depends on `prism-field` and on nothing else of this workspace,
   which is what lets the unit tests of `prism-isogeny` use it.
"""
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = "prism-fixtures"
USERS = ("prism-isogeny", "prism-verify")
ONLY_DEPENDENCY = "prism-field"


def main() -> int:
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
    members = workspace["workspace"]["members"]
    if FIXTURES not in members:
        print(f"{FIXTURES} is not a workspace member", file=sys.stderr)
        return 1
    errors = []
    for member in members:
        manifest = tomllib.loads((ROOT / member / "Cargo.toml").read_text())
        for table in ("dependencies", "build-dependencies"):
            if FIXTURES in manifest.get(table, {}):
                errors.append(f"{member} names {FIXTURES} under [{table}]")
        if member in USERS and FIXTURES not in manifest.get("dev-dependencies", {}):
            errors.append(f"{member} does not name {FIXTURES} under [dev-dependencies]")
    own = tomllib.loads((ROOT / FIXTURES / "Cargo.toml").read_text())
    local = sorted(name for name in own.get("dependencies", {}) if name.startswith("prism-"))
    if local != [ONLY_DEPENDENCY]:
        errors.append(f"{FIXTURES} depends on {local}, not on {ONLY_DEPENDENCY} alone")
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    print(f"{FIXTURES}: a dev-dependency of {', '.join(USERS)}; in no normal build; "
          f"depends on {ONLY_DEPENDENCY} alone")
    return 0


if __name__ == "__main__":
    sys.exit(main())
