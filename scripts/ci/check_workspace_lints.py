#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Library crates inherit `unsafe_code = "forbid"` from the workspace.

`prism-bench` does not inherit. Cargo rejects a member that both inherits
workspace lints and overrides one, and the userspace counter (`cntvct_el0`
/ `rdtsc`) has no safe wrapper. That crate sets `deny` itself, and the
allow sits on `counter` only. The signer barrier has a separate override
limited to its audited register identity.
"""

import argparse
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BENCH = "prism-bench"
BARRIER = "prism-ct-barrier"


def load(path: Path) -> dict:
    return tomllib.loads(path.read_text())


def allow_is_on_counter(source: str) -> bool:
    lines = source.splitlines()
    for index, line in enumerate(lines):
        if re.match(r"fn counter\b", line.strip()) is None:
            continue
        previous = []
        for earlier in lines[:index]:
            stripped = earlier.strip()
            if not stripped or stripped.startswith("//"):
                continue
            previous.append(stripped)
        return bool(previous) and previous[-1] == "#[allow(unsafe_code)]"
    return False


def judge(package: str, own: str | None, source: str) -> str | None:
    if package == BARRIER:
        if own != "deny":
            return f"{package} must deny unsafe_code"
        code = re.sub(r"//[^\n]*", "", source)
        audited = """
            fn value_barrier_u64(mut x: u64) -> u64 {
                #[allow(unsafe_code)]
                unsafe {
                    core::arch::asm!("/* {0} */", inout(reg) x, options(pure, nomem, nostack, preserves_flags));
                }
                x
            }
        """
        if ("".join(audited.split()) not in "".join(code.split())
                or len(re.findall(r"\bunsafe\b", code)) != 1
                or code.count("allow(unsafe_code)") != 1):
            return f"{package} must contain only the audited register identity"
        return None
    if package == BENCH:
        if own != "deny":
            return f'{package} must set unsafe_code = "deny" (the counter override)'
        if source.count("allow(unsafe_code)") != 1:
            return f"{package} must allow unsafe_code in exactly one place"
        if not allow_is_on_counter(source):
            return f"{package} allows unsafe_code somewhere other than counter"
        if "#![forbid(unsafe_code)]" in source:
            return f"{package} forbids unsafe_code and cannot compile the counter"
        return None
    if own not in (None, "forbid"):
        return f"{package} sets unsafe_code = {own!r}"
    if "#![forbid(unsafe_code)]" not in source:
        return f"{package} is missing #![forbid(unsafe_code)]"
    if "allow(unsafe_code)" in source:
        return f"{package} allows unsafe_code"
    return None


def scan() -> list[str]:
    workspace = load(ROOT / "Cargo.toml")
    lint = workspace["workspace"]["lints"]["rust"]["unsafe_code"]
    if lint != "forbid":
        return [f"workspace unsafe_code is {lint!r}"]
    members = workspace["workspace"]["members"]
    if len(members) < 4:
        return [f"workspace has {len(members)} members"]
    errors = []
    names = set()
    for member in members:
        manifest = load(ROOT / member / "Cargo.toml")
        package = manifest["package"]["name"]
        names.add(package)
        inherits = manifest.get("lints", {}).get("workspace") is True
        own = manifest.get("lints", {}).get("rust", {}).get("unsafe_code")
        if package in (BENCH, BARRIER):
            if inherits:
                errors.append(f"{member} inherits forbid, which the audited override cannot satisfy")
        elif not inherits:
            errors.append(f"{member} does not inherit workspace lints")
            continue
        lib = ROOT / member / "src" / ("main.rs" if package == BENCH else "lib.rs")
        errors_or_none = judge(package, own, lib.read_text())
        if errors_or_none:
            errors.append(errors_or_none)
    barrier_dir = ROOT / BARRIER
    barrier_manifest = load(barrier_dir / "Cargo.toml")
    if ({p.relative_to(barrier_dir).as_posix() for p in barrier_dir.rglob("*.rs")} != {"src/lib.rs"}
            or any("dependencies" in key for key in barrier_manifest)
            or "target" in barrier_manifest):
        errors.append("the barrier must be a single dependency-free source file")
    if BARRIER not in names:
        errors.append(f"no {BARRIER} package")
    if BENCH not in names:
        errors.append(f"no {BENCH} package")
    return errors


def selftest() -> int:
    cases = (
        ("prism-field", None, "#![forbid(unsafe_code)]\n", None),
        ("prism-field", "allow", "#![forbid(unsafe_code)]\n", "sets"),
        ("prism-field", None, "fn main() {}\n", "missing"),
        (
            BENCH,
            "deny",
            "#[allow(unsafe_code)]\nfn counter() -> u64 { unsafe { 0 } }\n",
            None,
        ),
        (BENCH, "forbid", "fn counter() {}\n", "deny"),
        (
            BENCH,
            "deny",
            "#[allow(unsafe_code)]\nfn other() {}\nfn counter() {}\n",
            "other than counter",
        ),
    )
    barrier = (ROOT / BARRIER / "src/lib.rs").read_text()
    cases += (
        (BARRIER, "deny", barrier, None),
        (BARRIER, "deny", barrier + "\n#[allow(unsafe_code)] unsafe fn extra() {}", "identity"),
        (BARRIER, "deny", barrier.replace("preserves_flags", "readonly"), "identity"),
    )
    for package, own, source, needle in cases:
        result = judge(package, own, source)
        if needle is None and result is not None:
            print(f"selftest refused a valid {package}: {result}", file=sys.stderr)
            return 1
        if needle is not None and (result is None or needle not in result):
            print(f"selftest accepted {package} ({result})", file=sys.stderr)
            return 1
    print("workspace-lint selftest passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest and selftest() != 0:
        return 1
    errors = scan()
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print("workspace lints: unsafe_code is forbid, inherited except the bench counter and signer barrier")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
