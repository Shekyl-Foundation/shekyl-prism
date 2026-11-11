#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Compare Rust's order-2^e basis with the pinned C tree's.

The C basis stores ``P``, ``Q = difference_point(P, R)`` and ``PmQ = R``,
where ``x(R) = -x(P) - A``. Rust's ``even_basis`` stores ``P`` and ``R``,
and stores ``[f]`` of the difference of the points before the cofactor.
This script reports which of Rust's two differences equals the C ``Q``.
"""

import pathlib
import platform
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
TREE = ROOT / "external" / "PRISM_v2"
BUILD = pathlib.Path("/tmp/prism-v2-ref")
EXPORTER = ROOT / "scripts" / "c" / "export_even_basis.c"

LEVELS = (("I", "lvl1", "248"), ("III", "lvl3", "376"), ("V", "lvl5", "500"))
TARGET = {
    "x86_64": "TARGET_AMD64",
    "aarch64": "TARGET_ARM64",
}.get(platform.machine(), "TARGET_OTHER")


def run(cmd):
    return subprocess.run(cmd, check=True, text=True, capture_output=True)


def compile_level(variant):
    binary = BUILD / f"export_even_basis_{variant}"
    libs = [
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
            "-DRADIX_64",
            "-DHAVE_UINT128",
            f"-D{TARGET}",
            "-DTARGET_OS_UNIX",
            "-DMINI_GMP",
            f"-I{TREE / 'include'}",
            f"-I{TREE / 'src' / 'common'}",
            f"-I{TREE / 'src' / 'common' / 'generic' / 'include'}",
            f"-I{TREE / 'src' / 'gf' / 'ref' / 'include'}",
            f"-I{TREE / 'src' / 'gf' / 'ref' / variant / 'include'}",
            f"-I{TREE / 'src' / 'ec' / 'ref' / 'include'}",
            f"-I{TREE / 'src' / 'precomp' / 'ref' / variant / 'include'}",
            f"-I{TREE / 'src' / 'mini-gmp'}",
            str(EXPORTER),
            "-o",
            str(binary),
            "-Wl,--start-group",
            *[str(lib) for lib in libs],
            "-Wl,--end-group",
            "-lm",
        ]
    )
    return binary


def rust_basis(level, coefficient):
    proc = run(
        [
            "cargo",
            "run",
            "--quiet",
            "--release",
            "--manifest-path",
            str(ROOT / "Cargo.toml"),
            "--bin",
            "even_basis",
            "--",
            level,
            coefficient,
        ]
    )
    lines = proc.stdout.split()
    if len(lines) != 4:
        raise SystemExit(f"{level} rust basis printed {len(lines)} lines")
    return lines


def c_basis(binary, coefficient, exponent):
    proc = run([str(binary), coefficient, exponent])
    lines = proc.stdout.split()
    if len(lines) != 3:
        raise SystemExit(f"c basis printed {len(lines)} lines")
    return lines


def compare(level, coefficient, binary, exponent):
    rust_p, rust_r, rust_cleared, rust_cofactor = rust_basis(level, coefficient)
    c_p, c_q, c_pmq = c_basis(binary, coefficient, exponent)
    return {
        "p": rust_p == c_p,
        "r": rust_r == c_pmq,
        "cleared": rust_cleared == c_q,
        "cofactor": rust_cofactor == c_q,
    }


def main():
    count = int(sys.argv[1]) if len(sys.argv) > 1 else 100
    failures = 0
    for level, variant, exponent in LEVELS:
        binary = compile_level(variant)
        coefficients = []
        vector = ROOT / "prism-verify" / "testdata" / f"s5_lvl{variant[-1]}.txt"
        for line in vector.read_text().splitlines():
            if line.startswith("A "):
                coefficients.append(line.split()[1])
        sampled = run(
            [
                "cargo",
                "run",
                "--quiet",
                "--release",
                "--manifest-path",
                str(ROOT / "Cargo.toml"),
                "--bin",
                "even_basis",
                "--",
                "sample",
                level,
                str(count),
            ]
        ).stdout.split()
        coefficients.extend(sampled)
        tallies = {"p": 0, "r": 0, "cleared": 0, "cofactor": 0}
        for coefficient in coefficients:
            row = compare(level, coefficient, binary, exponent)
            for key, ok in row.items():
                tallies[key] += int(ok)
            if not (row["p"] and row["r"] and (row["cleared"] or row["cofactor"])):
                failures += 1
                print(f"{level} mismatch {coefficient[:16]}… {row}", flush=True)
        total = len(coefficients)
        print(
            f"{level} {total} keys  P {tallies['p']}  R {tallies['r']}  "
            f"diff(cleared) {tallies['cleared']}  [f]diff {tallies['cofactor']}",
            flush=True,
        )
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
