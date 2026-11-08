#!/usr/bin/env python3
"""Confirm the pinned PRISM_v2 tree still has the verifier quirks the spec records.

The Rust crates do not link this tree. A hit here is salt-PRISM, not
conformance to docs/VERIFIER_SPEC.md.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
TREE = ROOT / "external" / "PRISM_v2"
PIN = "87fad7da5f12f79112521677f4f5530672afe9ba"

QUIRKS = {
    "src/verification/ref/lvlx/common.c": [
        "*salt == 0xFFFFFFFF",
        "sizeof(uint32_t)",
        "ibz_probab_prime(challenge_degree, 20)",
    ],
    "src/verification/ref/lvlx/verify.c": [
        "theta_chain_compute_and_eval_verify",
    ],
}


def main() -> int:
    if not (TREE / ".git").exists() and not (TREE / "src").exists():
        print(f"missing submodule at {TREE}", file=sys.stderr)
        print("git submodule update --init external/PRISM_v2", file=sys.stderr)
        return 1
    head = subprocess.check_output(
        ["git", "-C", str(TREE), "rev-parse", "HEAD"], text=True
    ).strip()
    if head != PIN:
        print(f"PRISM_v2 is {head}, the spec pins {PIN}", file=sys.stderr)
        return 1
    missing = 0
    for path, needles in QUIRKS.items():
        text = (TREE / path).read_text()
        for needle in needles:
            if needle not in text:
                print(f"{path} no longer contains {needle!r}", file=sys.stderr)
                missing += 1
    if missing:
        return 1
    print(f"PRISM_v2 {PIN} still resamples a uint32 salt and uses ibz_probab_prime")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
