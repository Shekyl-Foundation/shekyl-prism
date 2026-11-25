#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The committed vectors of the integer crate, prism-int.

    python3 scripts/int_vectors.py            writes prism-int/tests/data/vectors.tsv
    python3 scripts/int_vectors.py --check    fails if the committed file differs

Every operation of the crate, at seven widths: one and two limbs, where the
edges are cheap and are taken in every combination, and the five widths of
the parameter sets (26, 33, 38, 49 and 65 limbs, docs/SIGNER_SPEC.md 2.1.2),
where the cases are those that reach the top of the limbs.

The expected results are Python's own integers, by the conventions of
docs/SIGNER_SPEC.md 2.1.4. Nothing here is taken from the Rust code but one
thing: `knuth_events` follows the crate's division step by step, only to
find operands that reach each of its corrections. The quotients themselves
are Python's.

Three readers: this script, the crate's test `tests/vectors.rs`, and
`scripts/c/int_conventions.c`, which gives the same operands to the C
reference's `ibz_*` functions.

A line is `operation`, the number of limbs, the operands, `=`, the results,
separated by tabs. An integer is `0x…` in hexadecimal, after `-` when
negative. A result that is refused is the refusal's name. One line carries
every result its operands have: `lines_for` says which, operation by
operation.

When to write the file again: when an operation or a width is added, and
not otherwise. It is 3.3 MB, and every new copy is 3.3 MB more in the
repository's history. A change of the crate that changes no result needs
no new file, and `--check`, which CI runs, writes nothing. The file is
left out of the crate's package (`exclude` in prism-int/Cargo.toml).

Standard library only.
"""

import argparse
import hashlib
import math
import os
import sys

WIDTHS = (1, 2, 26, 33, 38, 49, 65)
LABEL = b"shekyl-prism prism-int vectors"
LIMB = 1 << 64
# The counts of low bits each value is asked for.
LOW_COUNTS = (0, 1, 2, 3, 63, 64)


class Stream:
    """A fixed stream of integers, from SHA-256 in counter mode."""

    def __init__(self, *parts):
        self.key = LABEL + b"|" + "|".join(str(part) for part in parts).encode()
        self.counter = 0

    def bits(self, count):
        """An integer below 2^count."""
        out = b""
        while 8 * len(out) < count:
            out += hashlib.sha256(self.key + b"|%d" % self.counter).digest()
            self.counter += 1
        return int.from_bytes(out, "big") >> (8 * len(out) - count) if count else 0

    def below(self, bound):
        """An integer in [0, bound)."""
        return self.bits(bound.bit_length() + 64) % bound

    def exact(self, count):
        """An integer of exactly `count` bits."""
        return 0 if count == 0 else (1 << (count - 1)) | self.bits(count - 1)

    def pick(self, items):
        return items[self.below(len(items))]


def show(value):
    return ("-" if value < 0 else "") + hex(abs(value))


def sign(value):
    return (value > 0) - (value < 0)


# The conventions, in Python.


def div_trunc(a, b):
    quotient = sign(a) * sign(b) * (abs(a) // abs(b))
    return quotient, a - quotient * b


def div_round(a, b):
    quotient, remainder = div_trunc(a, b)
    if 2 * abs(remainder) > abs(b):
        quotient += sign(a) * sign(b)
    return quotient


def centred(a, m):
    residue = a % m
    return residue - m if residue > m // 2 else residue


def xgcd(a, b):
    """Euclid on the magnitudes in the order given, then the operands' signs."""
    r0, r1 = abs(a), abs(b)
    s0, s1 = (0 if r0 == 0 and r1 == 0 else 1), 0
    t0, t1 = 0, 1
    while r1:
        quotient = r0 // r1
        r0, r1 = r1, r0 - quotient * r1
        s0, s1 = s1, s0 - quotient * s1
        t0, t1 = t1, t0 - quotient * t1
    u = -s0 if a < 0 else s0
    v = -t0 if b < 0 else t0
    assert r0 == u * a + v * b == math.gcd(a, b)
    if a and b:
        assert abs(u) <= abs(b) // r0 and abs(v) <= abs(a) // r0
    return r0, u, v


SMALL_PRIMES = (3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43)
# Primes with a short description: Mersenne primes, and the first norm of
# the secret ideal at four sets (docs/SIGNER_SPEC.md 1.7).
BIG_PRIMES = (
    (1 << 61) - 1,
    (1 << 89) - 1,
    (1 << 107) - 1,
    (1 << 127) - 1,
    (1 << 521) - 1,
    (1 << 607) - 1,
    (1 << 1279) - 1,
    (1 << 512) + 75,
    (1 << 768) + 183,
    (1 << 1024) + 643,
    (1 << 1536) + 75,
)


def legendre(a, prime):
    """Euler's criterion. `prime` is an odd prime."""
    power = pow(a % prime, (prime - 1) // 2, prime)
    return -1 if power == prime - 1 else power


def jacobi(a, factors):
    """The Jacobi symbol over the product of the odd primes `factors`."""
    symbol = 1
    for prime in factors:
        symbol *= legendre(a, prime)
    return symbol


# The crate's division, followed only to say which corrections a pair reaches.


def knuth_events(a, b, limbs):
    """The corrections `div_rem_magnitude` makes for |a| / |b| at `limbs` limbs."""
    events = set()
    a, b = abs(a), abs(b)
    n = (b.bit_length() + 63) // 64
    m = (a.bit_length() + 63) // 64
    if m < n:
        return {"short"}
    if n == 1:
        return {"single"}
    if m == limbs:
        events.add("full dividend")
    if n == limbs:
        events.add("full divisor")
    shift = 64 * n - b.bit_length()
    if shift == 0:
        events.add("no shift")
    v = b << shift
    u = a << shift
    if u >> (64 * limbs):
        events.add("top limb")
    top = (v >> (64 * (n - 1))) % LIMB
    nxt = (v >> (64 * (n - 2))) % LIMB
    for j in range(m - n, -1, -1):
        high = (u >> (64 * (j + n - 1))) % (LIMB * LIMB)
        third = (u >> (64 * (j + n - 2))) % LIMB
        estimate, rest = divmod(high, top)
        if estimate >> 64:
            events.add("estimate 2^64")
        lowered = 0
        for _ in range(2):
            if not (estimate >> 64 or estimate * nxt > ((rest << 64) | third)):
                break
            estimate -= 1
            rest += top
            lowered += 1
            if rest >> 64:
                events.add("rest leaves")
                break
        else:
            # Two lowerings without `rest` leaving its limb cannot happen.
            raise AssertionError("the estimate was lowered twice and the test still stands")
        if lowered:
            events.add("lowered %d" % lowered)
        window = (u >> (64 * j)) % (1 << (64 * (n + 1)))
        if window < estimate * v:
            events.add("add back")
            estimate -= 1
        u -= (estimate * v) << (64 * j)
    assert u >> shift == a % b
    return events


KNUTH_WANTED = ("add back", "lowered 1", "lowered 2", "rest leaves", "estimate 2^64",
                "top limb", "full dividend", "full divisor", "no shift")
SPECIAL_LIMBS = (0, 1, 2, 3, (1 << 63) - 1, 1 << 63, (1 << 63) + 1, LIMB - 2, LIMB - 1,
                 1 << 62, 1 << 61, (1 << 62) + 1)


def knuth_cases(limbs):
    """Pairs that between them reach every correction of the division."""
    if limbs < 2:
        return []
    stream = Stream("knuth", limbs)
    found = {}
    pairs = []

    def offer(a, b):
        if b == 0 or a >> (64 * limbs) or b >> (64 * limbs):
            return
        events = knuth_events(a, b, limbs)
        new = [event for event in events if event in KNUTH_WANTED and event not in found]
        # The add-back is wanted with a full dividend and a full divisor too.
        combined = []
        if "add back" in events:
            for other in ("top limb", "full divisor", "lowered 1"):
                key = "add back with " + other
                if other in events and key not in found:
                    combined.append(key)
        if new or combined:
            for event in new + combined:
                found[event] = True
            pairs.append((a, b))

    # The textbook pair that needs the add-back, at the top of the limbs.
    if limbs >= 3:
        offer((1 << (64 * limbs - 1)) + 3, (1 << (64 * limbs - 3)) + 1)
        offer((1 << (64 * limbs - 1)) + 3, (1 << (64 * (limbs - 1) - 3)) + 1)
    wanted = len(KNUTH_WANTED) + 3
    for _ in range(200000):
        if len(found) >= wanted:
            break
        n = 2 + stream.below(limbs - 1)
        m = n + stream.below(limbs - n + 1)
        b = sum(stream.pick(SPECIAL_LIMBS) << (64 * i) for i in range(n))
        a = sum(stream.pick(SPECIAL_LIMBS) << (64 * i) for i in range(m))
        # Often make the dividend a near-multiple of the divisor, which is
        # where an estimate is one too large.
        if stream.below(2) and b:
            q = sum(stream.pick(SPECIAL_LIMBS) << (64 * i) for i in range(max(1, m - n + 1)))
            a = q * b + stream.pick((0, 1, b - 1, b // 2)) - stream.below(2)
            if a < 0:
                continue
        offer(a, b)
    missing = [event for event in KNUTH_WANTED if event not in found]
    if limbs >= 3 and missing:
        raise SystemExit("no division at %d limbs reaches: %s" % (limbs, ", ".join(missing)))
    return pairs


# The cases.


def edges(limbs):
    """The values at the edges of `limbs` limbs, positive."""
    top = 64 * limbs
    half = top // 2
    values = {0, 1, 2, 3, LIMB - 1, (1 << half) - 1, 1 << half, (1 << half) + 1,
              (1 << (top - 1)) - 1, 1 << (top - 1), (1 << top) - 2, (1 << top) - 1,
              1 << 63, (1 << (top - 64)), ((LIMB - 1) << (top - 64))}
    if limbs >= 2:
        values |= {LIMB, LIMB + 1}
    return sorted(value for value in values if value >> top == 0)


def signed(values):
    return sorted({value for v in values for value in (v, -v)})


def few_edges(limbs):
    """The edges that matter at a wide width: zero, one, the half and the top."""
    top = 64 * limbs
    half = top // 2
    return [0, 1, (1 << half) - 1, 1 << half, 1 << (top - 1), (1 << top) - 1]


def randoms(stream, limbs, count):
    """Signed values of mixed lengths, up to the full width."""
    top = 64 * limbs
    out = []
    for index in range(count):
        length = stream.pick((top, top - 1, top // 2, top // 2 + 1, top // 3, 64, 65, 7))
        value = stream.exact(max(1, length))
        out.append(-value if stream.below(2) else value)
    return out


def fits(value, limbs):
    return abs(value) >> (64 * limbs) == 0


def lines_for(limbs):
    """Every line of one width, as lists of fields."""
    out = []
    top = 64 * limbs
    half = top // 2
    limit = 1 << top
    small = limbs <= 2
    stream = Stream("cases", limbs)

    def emit(op, operands, results):
        fields = [op, str(limbs)]
        fields += [item if isinstance(item, str) else show(item) for item in operands]
        fields.append("=")
        fields += [item if isinstance(item, str) else show(item) for item in results]
        out.append(fields)

    def value_or(value, refusal="overflow"):
        return [value] if fits(value, limbs) else [refusal]

    if small:
        pool = signed(edges(limbs)) + randoms(stream, limbs, 6)
        pairs = [(a, b) for a in pool for b in pool]
        singles = pool
    else:
        base = signed(few_edges(limbs))
        pairs = [(a, b) for a in (0, 1, -(1 << half), 1 << (top - 1), -(limit - 1), limit - 1)
                 for b in (1, -1, 1 << half, limit - 1)]
        pairs += [((1 << (top - 1)), (1 << (top - 1))), (limit - 1, -(limit - 1)),
                  (-(limit - 2), -1), (-(limit - 2), -2), (1, limit - 2), (LIMB - 1, 1),
                  (1 << (top - 64), LIMB), (1 << (top - 64), LIMB - 1),
                  ((1 << half) + 1, (1 << half) - 1), ((1 << half) + 1, (1 << half) + 1),
                  (limit - 1, 2), ((limit - 1) // 3, 3), ((limit - 1) // 3 + 1, 3)]
        rand = randoms(stream, limbs, 10)
        pairs += list(zip(rand[:5], rand[5:]))
        singles = base + rand[:4]
    if limbs >= 2:
        # A product that overflows by one limb product alone, whose two
        # halves are equal: 274177 · 67280421310721 = 2^64 + 1. And two
        # that overflow by nothing but the carry out of the top limb.
        pairs += [(274177 << (top - 64), 67280421310721 << 64),
                  (-(3 * 274177 << (top - 64)), 67280421310721 << 64),
                  ((LIMB - 1) << (top - 64), LIMB - 1), ((1 << (top - 1)) + 1, 2)]

    # The ring: the sum, the difference, the product and the order of a pair.
    for a, b in pairs:
        emit("ring", [a, b], value_or(a + b) + value_or(a - b) + value_or(a * b) + [str(sign(a - b))])
    # One value: its negation, sign, zero, odd, bit length, trailing zeros,
    # then for each count of LOW_COUNTS the low bits of the magnitude and
    # the residue modulo 2^count.
    for a in singles:
        results = [-a, str(int(a < 0)), str(int(a == 0)), str(abs(a) & 1), str(abs(a).bit_length())]
        results.append("none" if a == 0 else str((abs(a) & -abs(a)).bit_length() - 1))
        for count in LOW_COUNTS:
            results += [str(abs(a) % (1 << count)), str(a % (1 << count))]
        emit("unary", [a], results)
    shifts = (0, 1, 63, 64, 65, half, top - 1, top, top + 1, top + 64, 2 * top + 5)
    for a in (singles if small else [1, -3, (1 << half) + 1, -(1 << (top - 1)), limit - 1, singles[-1]]):
        for count in shifts:
            emit("shift", [a, str(count)], value_or(a << count) + [sign(a) * (abs(a) >> count)])
    for a in (singles if small else [0, 1, -(LIMB - 1), 1 << half, -(limit - 1), singles[-1]]):
        for length in sorted({0, 1, 8, 8 * limbs - 1, 8 * limbs, 8 * limbs + 3,
                              (abs(a).bit_length() + 7) // 8, max(0, (abs(a).bit_length() + 7) // 8 - 1)}):
            if abs(a) >> (8 * length):
                emit("bytes", [a, str(length)], ["overflow"])
            else:
                emit("bytes", [a, str(length)], [abs(a).to_bytes(length, "little").hex() or "-"])

    # Division, in its five roundings and the exact one: the truncated
    # quotient and remainder, the floored pair, the non-negative remainder,
    # the rounded quotient, the centred remainder, the exact quotient.
    if small:
        division = [(a, b) for a in pool for b in pool]
    else:
        division = [(a, b) for a in (0, limit - 1, -(limit - 1), 1 << (top - 1))
                    for b in (1, -1, 2, -3, LIMB - 1, (1 << half) + 1, -(limit - 1), limit - 2)]
        division += list(zip(rand[:5], rand[5:])) + list(zip(rand[5:], rand[:5]))
        division += [(rand[0], rand[0]), (rand[1], -rand[1]), (5, 0), (0, 0)]
        wide = [value for value in rand if abs(value).bit_length() >= top - 1][:2]
        division += [(value, stream.exact(length) * sign_) for value in wide
                     for length, sign_ in ((half, 1), (half + 1, -1), (65, 1), (top - 64, -1), (129, 1))]
    for a, b in knuth_cases(limbs):
        division += [(a, b), (-a, b)]
    # Ties, and one to each side of them, for the rounded quotient and the
    # centred remainder.
    for b in ((6, 7, LIMB - 1, LIMB - 2) if small else (7, (1 << half) + 2, (1 << half) + 1)):
        for k in ((0, 1, 5) if small else (0, 5)):
            for delta in (-1, 0, 1):
                for s in (1, -1):
                    a = s * (k * b + b // 2 + delta)
                    if fits(a, limbs) and fits(b, limbs):
                        division += [(a, b), (a, -b)]
    seen = set()
    for a, b in division:
        if (a, b) in seen:
            continue
        seen.add((a, b))
        if b == 0:
            emit("div", [a, b], ["divzero"])
            continue
        quotient, remainder = div_trunc(a, b)
        floor, floor_rest = divmod(a, b)
        results = [quotient, remainder, floor, floor_rest, a % abs(b), div_round(a, b)]
        results += ["modulus"] if b < 0 else [centred(a, b)]
        results += ["inexact"] if remainder else [quotient]
        emit("div", [a, b], results)

    # Greatest common divisors and inverses.
    if small:
        gcds = [(a, b) for a in pool[::3] for b in pool[::2]]
    else:
        gcds = [(0, 0), (0, -(limit - 1)), (limit - 1, 0), (limit - 1, limit - 1), (limit - 1, limit - 2),
                (-(1 << (top - 1)), 1 << half)]
        gcds += list(zip(rand[:3], rand[5:8]))
        shared = stream.exact(top // 3)
        gcds += [(shared * stream.exact(top // 3), -shared * stream.exact(top // 2))]
        # Consecutive Fibonacci numbers: the most steps Euclid can take.
        f0, f1 = 0, 1
        while f1 + f0 < limit:
            f0, f1 = f1, f0 + f1
        gcds += [(f1, f0), (-f0, f1)]
    for a, b in gcds:
        emit("xgcd", [a, b], list(xgcd(a, b)))
    moduli = [0, 1, -5, 2, 3, 8, LIMB - 1, 1 << (top - 1), limit - 1, (1 << half) + 1]
    for m in moduli:
        for a in (singles if small else [0, 1, -1, 2, 3, limit - 1, -(limit - 2), rand[0], rand[1]]):
            if m <= 1:
                emit("inv", [a, m], ["modulus"])
            elif math.gcd(a, m) != 1:
                emit("inv", [a, m], ["none"])
            else:
                emit("inv", [a, m], [pow(a, -1, m)])

    # Square roots.
    roots = {0, 1, 2, (1 << half) - 1, (1 << (half - 1)) + 1, LIMB - 1, stream.exact(half), stream.exact(half - 3)}
    squares = set()
    for root in roots:
        squares |= {root * root - 1, root * root, root * root + 1}
    squares |= {limit - 1, limit - 2, 1 << (top - 1), -1, -4, 5, 8}
    for a in sorted(value for value in squares if fits(value, limbs)):
        if a < 0:
            emit("sqrt", [a], ["negative", "none"])
            continue
        root = math.isqrt(a)
        emit("sqrt", [a], [root, root if root * root == a else "none"])

    # The Jacobi symbol, over products of known primes.
    emit("jacobi", [5, 0], ["modulus"])
    emit("jacobi", [5, 4], ["modulus"])
    emit("jacobi", [5, -7], ["modulus"])
    emit("jacobi", [0, 1], ["1"])
    emit("jacobi", [-(limit - 1), 1], ["1"])
    primes = [prime for prime in BIG_PRIMES if prime < limit]
    for index in range(40 if small else 14):
        factors = []
        product = 1
        for _ in range(1 + stream.below(4)):
            prime = stream.pick(SMALL_PRIMES + tuple(primes)) if index % 2 else stream.pick(SMALL_PRIMES)
            if product * prime < limit:
                factors.append(prime)
                product *= prime
        if not factors:
            factors, product = [3], 3
        for a in (0, 1, -1, 2, -2, factors[0], stream.below(limit), -stream.below(limit), stream.below(product)):
            emit("jacobi", [a, product], [str(jacobi(a, factors))])
    # The Legendre symbol, which the reference has: a prime below.
    for prime in list(SMALL_PRIMES[:4] if small else ()) + ([p for p in primes][-4:] or [SMALL_PRIMES[5]]):
        for a in (0, 1, -1, 2, 3, prime, prime - 1, stream.below(limit), -stream.below(limit), stream.below(prime)):
            emit("legendre", [a, prime], [str(legendre(a, prime))])

    # Arithmetic modulo an odd number of at most half the width.
    odd_half = stream.exact(half) | 1
    prime_moduli = [prime for prime in BIG_PRIMES if prime.bit_length() <= half] or [3]
    moduli = [1, 3, (1 << half) - 1, odd_half, prime_moduli[-1]]
    refused = [0, 2, -3, (1 << half) + 1, limit - 1]
    operands = [0, 1, -1, 2, limit - 1, -(limit - 1), stream.below(limit), -stream.below(limit)]
    for m in moduli:
        for a, b in zip(operands, operands[3:] + operands[:3]):
            emit("mulm", [a, b, m], [a * b % m])
        exponents = [0, 1, 2, 3, m - 1, m, limit - 1, stream.below(limit)][: (8 if small else 6)]
        for base, exponent in zip(operands, exponents):
            emit("powm", [base, exponent, m], [pow(base, exponent, m)])
    for m in refused:
        emit("mulm", [3, 5, m], ["modulus"])
        emit("powm", [3, 5, m], ["modulus"])
    emit("powm", [3, -1, 3], ["negative"])
    emit("powm", [3, -(limit - 1), moduli[2]], ["negative"])
    return out


def stage_lines(width):
    """Secret reducer operations at the coordinate and constructor widths."""
    bits = 64 * width
    modulus = 1 << bits
    stream = Stream("stage2c", width)
    for k in [0, 1, 62, 63, 64, 65, bits-1, bits, bits+1, LIMB-1, -1]:
        for a in [0, -1, -(modulus-1), stream.exact(bits), -stream.exact(bits//2)]:
            b = stream.exact(bits)
            word = -stream.exact(63)
            left = ("overflow" if k < 0 or a and abs(a).bit_length()+k > bits
                    else a << k if a else 0)
            right = ("overflow" if k < 0 else
                     (-1 if a < 0 else 1) * (abs(a) >> min(k, bits)))
            clipped = a if fits(a, 1) else "overflow"
            product = a * word
            values = [left, right, abs(a).bit_length(), abs(a*b) % modulus,
                      product if fits(product, width) else "overflow",
                      abs(a) % LIMB, clipped]
            fields = ["stage", str(width), show(a), show(b), show(word), show(k), "="]
            yield "\t".join(fields + [v if isinstance(v, str) else show(v) for v in values])


def text():
    lines = ["# Written by scripts/int_vectors.py. Do not edit: run it, or run it with --check."]
    for limbs in WIDTHS:
        lines += ["\t".join(fields) for fields in lines_for(limbs)]
    for width in (1, 2, 11, 14, 16, 21, 27, 22, 28, 32, 42, 54):
        lines.extend(stage_lines(width))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    path = os.path.join(root, "prism-int", "tests", "data", "vectors.tsv")
    made = text()
    if args.check:
        with open(path) as handle:
            if handle.read() != made:
                print("prism-int/tests/data/vectors.tsv is not what scripts/int_vectors.py writes", file=sys.stderr)
                return 1
        print("vectors.tsv: %d lines, as generated" % (made.count("\n") - 1))
        return 0
    with open(path, "w") as handle:
        handle.write(made)
    print("wrote %d lines" % (made.count("\n") - 1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
