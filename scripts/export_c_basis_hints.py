#!/usr/bin/env python3
"""Build the pinned PRISM tree and record one key-generation hint per level.

The Rust crates do not link this tree. The output is
`prism-verify/testdata/c_basis_hints.txt`: the canonical encoding of A and
the byte `ec_curve_to_basis_2f_to_hint` returned. A is the public key.
"""

import pathlib
import platform
import subprocess
import sys

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

TARGET = {
    "x86_64": "TARGET_AMD64",
    "aarch64": "TARGET_ARM64",
}.get(platform.machine(), "TARGET_OTHER")


def run(cmd: list[str]) -> None:
    print("+", " ".join(cmd), flush=True)
    subprocess.run(cmd, check=True)


def main() -> int:
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
    lines = []
    for name, variant, key_len in LEVELS:
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
        for _ in range(8):
            proc = subprocess.run([str(binary)], text=True, capture_output=True)
            if proc.returncode == 0 and proc.stdout.strip():
                printed = proc.stdout.strip()
                break
            print(f"{name} keygen retry: {proc.stderr.strip()}", file=sys.stderr)
        if printed is None:
            print(f"{name} keygen failed", file=sys.stderr)
            return 1
        key, hint = printed.split()
        if len(key) != key_len * 2:
            print(f"{name} key is {len(key) // 2} bytes, expected {key_len}", file=sys.stderr)
            return 1
        if int(hint) > 255:
            print(f"{name} hint {hint} does not fit a byte", file=sys.stderr)
            return 1
        lines.append(f"{name} {key} {int(hint)}")
        print(lines[-1][:80], "...", flush=True)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines) + "\n")
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
