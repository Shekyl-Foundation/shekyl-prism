#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Install one CI tool from a hashed release tarball.

The checksum is the pin. A host architecture with no row fails rather than
building the tool from a floating crates.io tip. `--selftest` checks the
table and that every workflow install names a row; it does not download.
"""

import argparse
import hashlib
import os
import stat
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path

# (tool, arch) -> (url, sha256)
TOOLS = {
    ("cargo-deny", "x86_64"): (
        "https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz",
        "9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f",
    ),
    ("cargo-deny", "aarch64"): (
        "https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-aarch64-unknown-linux-musl.tar.gz",
        "995c82be0defc7a025cae49a2aa2644ce8245c9a3318fc4103907c6a285e8c7d",
    ),
    ("cargo-fuzz", "x86_64"): (
        "https://github.com/rust-fuzz/cargo-fuzz/releases/download/0.13.2/cargo-fuzz-0.13.2-x86_64-unknown-linux-musl.tar.gz",
        "b5b704018b63e0f151c17a057ac53b5111e1db545d1b9f72fee79f08a545931c",
    ),
    ("cargo-llvm-cov", "x86_64"): (
        "https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.9.1/cargo-llvm-cov-x86_64-unknown-linux-gnu.tar.gz",
        "b3f68e625481fed9b16444174f3fa5ebcdbde4a1878803a35eabe2dcefcdc41a",
    ),
    ("cargo-llvm-cov", "aarch64"): (
        "https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.9.1/cargo-llvm-cov-aarch64-unknown-linux-gnu.tar.gz",
        "abf5f13c1520f8756d2192bfaaeadb0208f5b7eaa8cadc15bf847befa42b7360",
    ),
    ("cargo-mutants", "x86_64"): (
        "https://github.com/sourcefrog/cargo-mutants/releases/download/v27.1.0/cargo-mutants-x86_64-unknown-linux-gnu.tar.gz",
        "dfe6dc37d0342c891d2829b5a695aa57c2d0edecef7e7d0399a30cc6e206411e",
    ),
    ("cross", "x86_64"): (
        "https://github.com/cross-rs/cross/releases/download/v0.2.5/cross-x86_64-unknown-linux-gnu.tar.gz",
        "642375d1bcf3bd88272c32ba90e999f3d983050adf45e66bd2d3887e8e838bad",
    ),
}


def arch() -> str:
    machine = os.uname().machine
    if machine not in {"x86_64", "aarch64"}:
        raise SystemExit(f"no tool binaries for {machine}")
    return machine


def dest_dir() -> Path:
    cargo_home = os.environ.get("CARGO_HOME", str(Path.home() / ".cargo"))
    path = Path(cargo_home) / "bin"
    path.mkdir(parents=True, exist_ok=True)
    return path


def install(tool: str) -> None:
    row = TOOLS.get((tool, arch()))
    if row is None:
        raise SystemExit(f"no {tool} binary for {arch()}")
    url, expect = row
    with tempfile.TemporaryDirectory() as tmp:
        archive = Path(tmp) / "tool.tar.gz"
        urllib.request.urlretrieve(url, archive)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        if digest != expect:
            raise SystemExit(f"{tool}: sha256 {digest} != {expect}")
        with tarfile.open(archive) as tar:
            members = [
                member
                for member in tar.getmembers()
                if member.isfile() and Path(member.name).name == tool
            ]
            if len(members) != 1:
                names = [member.name for member in members]
                raise SystemExit(f"{tool}: expected one binary, found {names}")
            payload = tar.extractfile(members[0])
            if payload is None:
                raise SystemExit(f"{tool}: empty archive member")
            target = dest_dir() / tool
            target.write_bytes(payload.read())
            target.chmod(target.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    print(f"installed {tool} to {dest_dir() / tool}")


def selftest() -> int:
    if len(TOOLS) < 5:
        print("selftest: the tool table is implausibly small", file=sys.stderr)
        return 1
    for (tool, _machine), (url, digest) in TOOLS.items():
        if not url.startswith("https://"):
            print(f"selftest: {tool} url is not https", file=sys.stderr)
            return 1
        if len(digest) != 64 or any(ch not in "0123456789abcdef" for ch in digest):
            print(f"selftest: {tool} checksum is not 64 hex chars", file=sys.stderr)
            return 1
    root = Path(__file__).resolve().parents[2]
    mentioned = []
    for path in (root / ".github" / "workflows").glob("*.yml"):
        for line in path.read_text().splitlines():
            marker = "install_pinned_tool.py "
            if marker in line and not line.lstrip().startswith("#"):
                tool = line.split(marker, 1)[1].split()[0]
                if not tool.startswith("-"):
                    mentioned.append(tool)
    if not mentioned:
        print("selftest: no workflow installs a pinned tool", file=sys.stderr)
        return 1
    known = {tool for tool, _machine in TOOLS}
    for tool in mentioned:
        if tool not in known:
            print(f"selftest: workflow installs unknown tool {tool}", file=sys.stderr)
            return 1
    print("pinned-tool selftest passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true")
    parser.add_argument("tool", nargs="?")
    args = parser.parse_args()
    if args.selftest:
        return selftest()
    if not args.tool:
        print("name a tool, or pass --selftest", file=sys.stderr)
        return 1
    install(args.tool)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
