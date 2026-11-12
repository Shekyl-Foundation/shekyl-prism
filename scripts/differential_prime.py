#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Compare challenge-degree primality with sympy.

Draws odd scalars below `2^{e'-1}` for each NIST level, asks
`prism-verify`'s `degree_is_prime` (`src/bin/degree_check.rs`), and requires
the same answer as `sympy.isprime` on `q = 2^{2e'-1} - s^2`. A short table of
degrees whose primality is known without either implementation comes first,
so a driver and a sympy that fail the same way still go red.

The seed is printed. Pass `--seed` to repeat a run.
"""

import argparse
import random
import secrets
import subprocess
import sys

# (exponent, scalar, q is prime). q = 2^{2e'-1} - s^2, and s is odd and
# below 2^{e'-1}, which is the verifier's domain.
KNOWN = (
    (4, 1, True),  # 127
    (4, 3, False),  # 119
    (4, 5, True),  # 103
    (4, 7, True),  # 79
)

LEVELS = (
    ("NIST-I", 246),
    ("NIST-III", 374),
    ("NIST-V", 498),
)

COUNT = 10_000


def degree(exponent: int, scalar: int) -> int:
    return (1 << (2 * exponent - 1)) - scalar * scalar


def scalar_bytes(rng: random.Random, exponent: int) -> bytes:
    bits = exponent - 1
    width = (bits + 7) // 8
    raw = bytearray(rng.randbytes(width))
    raw[0] |= 1
    raw[-1] &= (1 << (((bits - 1) % 8) + 1)) - 1
    return bytes(raw)


def trial_prime(n: int) -> bool:
    if n < 2:
        return False
    if n % 2 == 0:
        return n == 2
    d = 3
    while d * d <= n:
        if n % d == 0:
            return False
        d += 2
    return True


def selftest() -> int:
    saw_prime = False
    saw_composite = False
    for exponent, scalar, prime in KNOWN:
        q = degree(exponent, scalar)
        if trial_prime(q) != prime:
            print(f"known table disagrees with trial division: e={exponent} s={scalar}", file=sys.stderr)
            return 1
        saw_prime = saw_prime or prime
        saw_composite = saw_composite or not prime
        if scalar % 2 == 0 or scalar >= 2 ** (exponent - 1):
            print(f"known scalar is outside the domain: {scalar}", file=sys.stderr)
            return 1
    if not (saw_prime and saw_composite):
        print("known table has only one verdict", file=sys.stderr)
        return 1
    sample = scalar_bytes(random.Random(1), 248)
    value = int.from_bytes(sample, "little")
    if value % 2 == 0 or value >= 2**247 or len(sample) != 31:
        print("scalar draw left the domain", file=sys.stderr)
        return 1
    print("differential-prime selftest passed")
    return 0


def rust_verdicts(lines: list[str]) -> list[str]:
    proc = subprocess.run(
        ["cargo", "run", "--release", "--locked", "-p", "prism-verify", "--bin", "degree_check"],
        input="\n".join(lines) + "\n",
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        print(proc.stderr, file=sys.stderr)
        raise SystemExit(f"degree-check exited {proc.returncode}")
    out = [line for line in proc.stdout.splitlines() if line.strip()]
    if len(out) != len(lines):
        raise SystemExit(f"degree-check returned {len(out)} lines, expected {len(lines)}")
    return out


def verdict_word(prime: bool) -> str:
    return "prime" if prime else "composite"


def check_known(sympy_isprime) -> list[str]:
    lines = []
    for exponent, scalar, prime in KNOWN:
        q = degree(exponent, scalar)
        if sympy_isprime(q) != prime:
            raise SystemExit(f"sympy disagrees with the known table at e={exponent} s={scalar}")
        lines.append(f"{exponent} {scalar.to_bytes(1, 'little').hex()}")
    rust = rust_verdicts(lines)
    for line, (_, _, prime) in zip(rust, KNOWN, strict=True):
        if not line.endswith(" " + verdict_word(prime)):
            raise SystemExit(f"degree-check disagrees with the known table: {line}")
    return rust


def check_random(sympy_isprime, count: int, seed: bytes) -> None:
    rng = random.Random(seed)
    lines = []
    degrees = []
    for _name, exponent in LEVELS:
        for _ in range(count):
            raw = scalar_bytes(rng, exponent)
            scalar = int.from_bytes(raw, "little")
            lines.append(f"{exponent} {raw.hex()}")
            degrees.append(degree(exponent, scalar))
    rust = rust_verdicts(lines)
    mismatches = 0
    per_level = {name: {"prime": 0, "composite": 0} for name, _ in LEVELS}
    shown = 0
    for (name, _exponent), q, line in zip(
        (level for level in LEVELS for _ in range(count)),
        degrees,
        rust,
        strict=True,
    ):
        expected = sympy_isprime(q)
        got = line.endswith(" prime")
        per_level[name][verdict_word(expected)] += 1
        if got != expected:
            mismatches += 1
            if shown < 20:
                print(f"mismatch {name}: {line} sympy={'prime' if expected else 'composite'}", file=sys.stderr)
                shown += 1
    # At 10_000 draws a level with no primes is a stuck generator. A short
    # debugging run can honestly be all composite; don't fail that.
    for name, counts in per_level.items():
        print(f"{name} prime={counts['prime']} composite={counts['composite']}")
        if count >= 1000 and (counts["prime"] == 0 or counts["composite"] == 0):
            raise SystemExit(f"{name} produced only one verdict")
    if mismatches:
        raise SystemExit(f"{mismatches} disagreements")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true")
    parser.add_argument("--count", type=int, default=COUNT)
    parser.add_argument("--seed", default="")
    args = parser.parse_args()
    if args.selftest:
        return selftest()
    if args.count < 1:
        print("count must be positive", file=sys.stderr)
        return 1
    import sympy

    seed = bytes.fromhex(args.seed) if args.seed else secrets.token_bytes(16)
    print(f"seed {seed.hex()}")
    print(f"sympy {sympy.__version__}")
    check_known(sympy.isprime)
    check_random(sympy.isprime, args.count, seed)
    print(f"no disagreements over {args.count} degrees at each level")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
