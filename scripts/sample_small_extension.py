#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Orders of `-p` modulo the `e′` challenge primes.

`sample_e_prime` draws the first 1,000 prime degrees at exponent `e − 2`
for each level, from the verifier's own challenge derivation. This script
checks each degree against ePrint 2025/1602: `q` meets their condition
when `(-p)^k ≡ 1 (mod q)` for a positive `k` small enough that Lemma 3's
cost `Õ(max{k², k√q})` is at most `2^λ`. For a prime in the `e′` window
that cost is at least `√q`, and `√q > 2^λ` at every level, so no positive
order qualifies. The script still proves each order exceeds `2^24`, which
is the check that the special shape `p = f·2^e − 1` did not make one small.

The three committed §5 degrees are checked the same way and are not part
of the thousand.
"""

import argparse
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
VECTORS = ROOT / "prism-verify" / "testdata"
# A bound the order check can finish, far above the empty Lemma 3 threshold.
ORDER_FLOOR = 1 << 24

# Table 2. The exponent here is `e`, and the sample uses `e′ = e − 2`.
LEVELS = (
    ("NIST-I", 248, 128, 5),
    ("NIST-III", 376, 192, 65),
    ("NIST-V", 500, 256, 27),
)


def prime(level: str) -> int:
    for name, exponent, _lambda, cofactor in LEVELS:
        if name == level:
            return cofactor * (1 << exponent) - 1
    raise SystemExit(f"unknown level {level}")


def order_at_most(base: int, modulus: int, bound: int) -> int | None:
    """The order of `base` modulo `modulus` when it is at most `bound`."""
    if modulus <= 1 or bound < 1:
        return None
    base %= modulus
    if base == 0:
        return None
    baby_count = 1
    while baby_count * baby_count < bound:
        baby_count += 1
    baby: dict[int, int] = {}
    value = 1
    for step in range(baby_count):
        baby.setdefault(value, step)
        value = (value * base) % modulus
    stride = pow(base, baby_count, modulus)
    giant = 1
    found: int | None = None
    for block in range(1, baby_count + 1):
        giant = (giant * stride) % modulus
        step = baby.get(giant)
        if step is None:
            continue
        candidate = block * baby_count - step
        if 1 <= candidate <= bound and (found is None or candidate < found):
            found = candidate
    if found is not None and pow(base, found, modulus) != 1:
        raise SystemExit("the order search named a non-order")
    return found


def selftest() -> None:
    # 2 has order 10 modulo 11.
    if order_at_most(2, 11, 9) is not None:
        raise SystemExit("order 10 was reported at or below 9")
    if order_at_most(2, 11, 10) != 10:
        raise SystemExit("order 10 modulo 11 was not found")
    if order_at_most(1, 11, 1) != 1:
        raise SystemExit("1 does not have order 1")


def vector_degrees() -> list[tuple[str, int, int]]:
    """The three committed §5 degrees, checked against their own scalars."""
    found = []
    for name, exponent, _lambda, _cofactor in LEVELS:
        digit = {"NIST-I": "1", "NIST-III": "3", "NIST-V": "5"}[name]
        path = VECTORS / f"s5_lvl{digit}.txt"
        fields = {}
        for line in path.read_text().splitlines():
            key, _, value = line.partition(" ")
            fields[key] = value
        width = exponent - 2
        scalar = int(fields["s"])
        degree = int(fields["q"])
        expect = (1 << (2 * width - 1)) - scalar * scalar
        if degree != expect:
            raise SystemExit(f"{path.name}: q is not 2^{{2e′−1}} − s^2")
        if scalar % 2 == 0 or scalar >= 1 << (width - 1):
            raise SystemExit(f"{path.name}: s is outside the e′ domain")
        found.append((name, width, degree))
    return found


def read_samples(lines: list[str]) -> list[tuple[str, int, int, int]]:
    samples = []
    for number, line in enumerate(lines, 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) != 5:
            raise SystemExit(f"sample line {number} does not have 5 fields")
        level, exponent, index, scalar_hex, degree_hex = parts
        width = int(exponent)
        scalar = int.from_bytes(bytes.fromhex(scalar_hex), "little")
        degree = int.from_bytes(bytes.fromhex(degree_hex), "little")
        expect = (1 << (2 * width - 1)) - scalar * scalar
        if degree != expect:
            raise SystemExit(f"sample line {number}: q is not 2^{{2e′−1}} − s^2")
        samples.append((level, width, int(index), degree))
    return samples


def report_one(level: str, degree: int, label: str) -> str:
    modulus_prime = prime(level)
    base = (-modulus_prime) % degree
    small = order_at_most(base, degree, ORDER_FLOOR)
    if small is None:
        return f"{label} order>{ORDER_FLOOR} q_bits={degree.bit_length()}"
    return f"{label} ORDER {small} q_bits={degree.bit_length()}"


def summarize(samples: list[tuple[str, int, int, int]]) -> int:
    failed = 0
    for name, exponent, _lambda, _cofactor in LEVELS:
        rows = [row for row in samples if row[0] == name]
        if not rows:
            print(f"{name}: no samples", file=sys.stderr)
            failed += 1
            continue
        small = 0
        indexes = []
        bits = []
        for _level, width, index, degree in rows:
            if width != exponent - 2:
                print(f"{name}: sample exponent {width}", file=sys.stderr)
                failed += 1
            base = (-prime(name)) % degree
            if order_at_most(base, degree, ORDER_FLOOR) is not None:
                small += 1
            indexes.append(index)
            bits.append(degree.bit_length())
        indexes.sort()
        threshold = "none: sqrt(q) > 2^λ"
        print(
            f"{name} samples={len(rows)} q_bits={min(bits)}..{max(bits)} "
            f"j_min={indexes[0]} j_median={indexes[len(indexes) // 2]} "
            f"j_max={indexes[-1]} orders_at_most_{ORDER_FLOOR}={small} "
            f"lemma3_threshold={threshold}"
        )
        failed += small
    return failed


def main() -> int:
    selftest()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--count",
        type=int,
        default=1000,
        help="prime degrees per level (default 1000)",
    )
    parser.add_argument(
        "--samples",
        type=pathlib.Path,
        help="read a saved sample_e_prime transcript instead of running it",
    )
    args = parser.parse_args()
    if args.count < 1:
        print("the count is a positive integer", file=sys.stderr)
        return 1

    print("committed §5 degrees")
    for level, _width, degree in vector_degrees():
        print(report_one(level, degree, level))

    if args.samples is not None:
        lines = args.samples.read_text().splitlines()
    else:
        binary = [
            "cargo",
            "run",
            "--quiet",
            "--release",
            "-p",
            "prism-verify",
            "--bin",
            "sample_e_prime",
            "--",
            str(args.count),
        ]
        completed = subprocess.run(
            binary, cwd=ROOT, check=False, capture_output=True, text=True
        )
        if completed.returncode != 0:
            sys.stderr.write(completed.stderr)
            return completed.returncode
        sys.stderr.write(completed.stderr)
        lines = completed.stdout.splitlines()

    samples = read_samples(lines)
    print("random (A, message) at e′")
    return 1 if summarize(samples) else 0


if __name__ == "__main__":
    sys.exit(main())
