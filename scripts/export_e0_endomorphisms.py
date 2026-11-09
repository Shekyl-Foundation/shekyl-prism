#!/usr/bin/env python3
"""Find one endomorphism of E0 of norm q = 2^{2e-1} - s^2 for each level.

q is prime, s is a small odd positive integer, and
q - p(c^2 + d^2) = a^2 + b^2 with p = f·2^e - 1 and c, d odd and small.
The graph of this endomorphism does not split in theta_chain_compute_and_eval.
The kernel that does split is the fixed-degree one from scripts/c/export_e0_vector.c.
"""

import pathlib
import sys

LEVELS = (
    ("NIST-I", 248, 5),
    ("NIST-III", 376, 65),
    ("NIST-V", 500, 27),
)

OUT = pathlib.Path(__file__).resolve().parents[1] / "prism-verify" / "testdata" / "e0_endomorphisms.txt"


def miller_rabin(n: int, rounds: int = 16) -> bool:
    if n < 2:
        return False
    small = (2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31)
    for prime in small:
        if n == prime:
            return True
        if n % prime == 0:
            return False
    d = n - 1
    s = 0
    while d % 2 == 0:
        d //= 2
        s += 1
    for a in small[:rounds]:
        x = pow(a, d, n)
        if x == 1 or x == n - 1:
            continue
        for _ in range(s - 1):
            x = pow(x, 2, n)
            if x == n - 1:
                break
        else:
            return False
    return True


def sqrt_minus_one(prime: int) -> int:
    for base in range(2, 64):
        if pow(base, (prime - 1) // 2, prime) == prime - 1:
            return pow(base, (prime - 1) // 4, prime)
    raise RuntimeError("no nonresidue")


def cornacchia(prime: int) -> tuple[int, int]:
    root = sqrt_minus_one(prime)
    if pow(root, 2, prime) != prime - 1:
        raise RuntimeError("sqrt(-1)")
    first, second = prime, root
    limit = int(prime**0.5) if prime < 2**100 else isqrt(prime)
    while first > limit:
        if second == 0:
            raise RuntimeError("cornacchia")
        first, second = second, first % second
    square = prime - first * first
    other = isqrt(square)
    if other * other != square:
        raise RuntimeError("cornacchia")
    return first, other


def isqrt(n: int) -> int:
    return int(n**0.5) if n < 2**100 else int(__import__("math").isqrt(n))


def search(name: str, e: int, f: int) -> str:
    prime = f * 2**e - 1
    half = 2 ** (2 * e - 1)
    for s in range(1, 20000, 2):
        degree = half - s * s
        if not miller_rabin(degree):
            continue
        for c in range(1, 200, 2):
            for d in range(1, c + 1, 2):
                norm = degree - prime * (c * c + d * d)
                if norm % 4 != 1 or norm <= 1 or not miller_rabin(norm):
                    continue
                a, b = cornacchia(norm)
                modulus = 2**e
                t = pow(s, -1, modulus)
                print(f"{name} s={s} c={c} d={d}", file=sys.stderr, flush=True)
                return f"{name}\ns {s}\na {a}\nb {b}\nc {c}\nd {d}\nt {t}\n"
    raise RuntimeError(name)


def main() -> int:
    lines = [search(*level) for level in LEVELS]
    OUT.write_text("".join(lines))
    print(OUT)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
