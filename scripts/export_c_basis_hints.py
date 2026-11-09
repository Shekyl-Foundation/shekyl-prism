#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Build the pinned PRISM tree and record one key-generation hint per level.

The Rust crates do not link this tree. The output is
`prism-verify/testdata/c_basis_hints.txt`: the canonical encoding of A and
the byte `ec_curve_to_basis_2f_to_hint` returned. A is the public key.

The exporter supplies `randombytes` from a frozen seed, so a second run
writes the same file. CI diffs that file. A big-endian host is refused:
the reference's own salt path is native-endian, and this script does not
claim a result it has not shown is independent of byte order.
"""

import pathlib
import platform
import subprocess
import sys

from check_c_reference import PIN

ROOT = pathlib.Path(__file__).resolve().parents[1]
TREE = ROOT / "external" / "PRISM_v2"
BUILD = pathlib.Path("/tmp/prism-v2-ref")
OUT = ROOT / "prism-verify" / "testdata" / "c_basis_hints.txt"
EXPORTER = ROOT / "scripts" / "c" / "export_basis_hint.c"

LEVELS = (
    ("NIST-I", "lvl1", 64),
    ("NIST-III", "lvl3", 96),
    ("NIST-V", "lvl5", 128),
)

# First seed tried. The next integers are the retries. The file records
# which one keygen accepted, and a later run tries them in the same order.
SEED_BASE = 0x5E5E7A11
SEED_TRIES = 32

TARGET = {
    "x86_64": "TARGET_AMD64",
    "aarch64": "TARGET_ARM64",
}.get(platform.machine(), "TARGET_OTHER")


def run(cmd: list[str]) -> None:
    print("+", " ".join(cmd), flush=True)
    subprocess.run(cmd, check=True)


def one_level(name: str, variant: str, key_len: int) -> tuple[str, int]:
    binary = BUILD / f"export_basis_hint_{variant}"
    libs = [
        BUILD / "src" / f"libprism_{variant}.a",
        BUILD / "src" / "signature" / "ref" / variant / f"libprism_signature_{variant}.a",
        BUILD / "src" / "verification" / "ref" / variant / f"libprism_verification_{variant}.a",
        BUILD / "src" / "id2iso" / "ref" / variant / f"libprism_id2iso_{variant}.a",
        BUILD / "src" / "quaternion" / "ref" / "generic" / "libprism_quaternion_generic.a",
        BUILD / "src" / "hd" / "ref" / variant / f"libprism_hd_{variant}.a",
        BUILD / "src" / "ec" / "ref" / variant / f"libprism_ec_{variant}.a",
        BUILD / "src" / "gf" / "ref" / variant / f"libprism_gf_{variant}.a",
        BUILD / "src" / "mp" / "ref" / "generic" / "libprism_mp_generic.a",
        BUILD / "src" / "precomp" / "ref" / variant / f"libprism_precomp_{variant}.a",
        BUILD / "libGMP.a",
        BUILD / "src" / "common" / "generic" / "libprism_common_sys.a",
    ]
    run(
        [
            "cc",
            "-O2",
            "-DPRISM_BUILD_TYPE_REF",
            f"-DPRISM_VARIANT={variant}",
            "-DENABLE_SIGN",
            "-DRADIX_64",
            "-DHAVE_UINT128",
            f"-D{TARGET}",
            "-DTARGET_OS_UNIX",
            "-DMINI_GMP",
            f"-I{TREE / 'src' / 'nistapi' / variant}",
            f"-I{TREE / 'include'}",
            f"-I{TREE / 'src' / 'common'}",
            str(EXPORTER),
            "-o",
            str(binary),
            "-Wl,--start-group",
            *[str(lib) for lib in libs],
            "-Wl,--end-group",
            "-lm",
        ]
    )
    printed = None
    used = None
    for offset in range(SEED_TRIES):
        seed = SEED_BASE + offset
        proc = subprocess.run(
            [str(binary), f"{seed:#x}"], text=True, capture_output=True
        )
        if proc.returncode == 0 and proc.stdout.strip():
            printed = proc.stdout.strip()
            used = seed
            break
        print(
            f"{name} seed {seed:#x} rejected: {proc.stderr.strip()}",
            file=sys.stderr,
        )
    if printed is None or used is None:
        print(f"{name} keygen failed for {SEED_TRIES} seeds", file=sys.stderr)
        raise SystemExit(1)
    key, hint = printed.split()
    if len(key) != key_len * 2:
        print(f"{name} key is {len(key) // 2} bytes, expected {key_len}", file=sys.stderr)
        raise SystemExit(1)
    if int(hint) > 255:
        print(f"{name} hint {hint} does not fit a byte", file=sys.stderr)
        raise SystemExit(1)
    print(f"{name} seed {used:#x}", flush=True)
    return f"{name} {key} {int(hint)}", used


def main() -> int:
    if sys.byteorder != "little":
        print("refusing to export on a big-endian host", file=sys.stderr)
        return 1
    if not (TREE / "CMakeLists.txt").exists():
        print(f"missing submodule at {TREE}", file=sys.stderr)
        return 1
    BUILD.mkdir(parents=True, exist_ok=True)
    run(
        [
            "cmake",
            "-S",
            str(TREE),
            "-B",
            str(BUILD),
            "-DPRISM_BUILD_TYPE=ref",
            "-DCMAKE_BUILD_TYPE=Release",
            "-DGMP_LIBRARY=MINI",
        ]
    )
    run(
        [
            "cmake",
            "--build",
            str(BUILD),
            "--parallel",
            "--target",
            *[f"prism_{variant}" for _, variant, _ in LEVELS],
        ]
    )
    rows = []
    seeds = []
    for name, variant, key_len in LEVELS:
        row, seed = one_level(name, variant, key_len)
        rows.append(row)
        seeds.append(f"# {name} seed {seed:#x}")
    header = [
        f"# pin {PIN}",
        f"# seed-base {SEED_BASE:#x}",
        *seeds,
        "# A and the hint byte from ec_curve_to_basis_2f_to_hint.",
    ]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(header + rows) + "\n")
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
