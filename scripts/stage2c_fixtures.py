#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The five native reducer corpora, their byte layout, and the exact oracle.

Each vectors/<set>.txt retains the first two honest HNFs, all three Q5 saturation
cases and all six guarded intersections from research commit 35c59c6,
research/normalized-gram/rust/vectors_ng/<set>.txt.

A basis input is nine signed integers: the norm and the eight coordinates.
The release probe and every measurement script encode an integer as its
little-endian magnitude at the set's limb width, followed by one sign byte.
The first row of a class is the timing reference. Later rows of that class
are the contrasts against it.
"""
import argparse
from collections import Counter
from pathlib import Path
from stage2c_oracle import columns, native_schedules, norm_form, reduce_exact, signer_parameters

VECTORS = Path(__file__).resolve().parents[1] / "shekyl-prism/examples/stage2c/vectors"
SET_WIDTHS = {"p248": 11, "p324": 14, "p376": 16, "p500": 21, "p664": 27}

LIMB_BYTES = 8
SIGN_BYTES = 1
# Norm, then the four coordinates of U, then the four of V.
BASIS_INPUTS = 9
BASIS_OUTPUTS = 12
# Padding and small integers have fewer distinct octets. The retention scan
# ignores those limbs, so a match is a copied input word.
MIN_DISTINCT_OCTETS = 5
CLASS_COUNTS = {"sign-generic": 5, "prered-guard256": 6}
# The first row of each class in corpus order. Timing contrasts the others with it.
REFERENCE_INDEX = 0


def basis_stride(width):
    """Bytes of one signed integer at `width` limbs."""
    return width * LIMB_BYTES + SIGN_BYTES


def parse_row(line):
    """Return `(class token, integers)` for one corpus line.

    The integers are the nine inputs followed by the twelve oracle outputs.
    """
    cls, *fields = line.split()
    if cls not in CLASS_COUNTS:
        raise ValueError(f"unknown class {cls}")
    try:
        values = [int(token, 16) for token in fields]
    except ValueError as error:
        raise ValueError(f"non-hex fixture field in {cls}") from error
    if len(values) != BASIS_INPUTS + BASIS_OUTPUTS:
        raise ValueError(f"{cls} row has {len(values)} fields")
    return cls, values


def corpus(name):
    """Every row of one set, in file order, after the class census matches."""
    path = VECTORS / f"{name}.txt"
    rows = [parse_row(line) for line in path.read_text().splitlines() if line.strip()]
    counts = Counter(cls for cls, _ in rows)
    if dict(counts) != CLASS_COUNTS:
        raise ValueError(f"missing {name} fixtures: {dict(counts)}")
    return rows


def encode_basis(values, width):
    """Encode nine signed inputs the way `scripts/ci/stage2c/probe.rs` reads them."""
    if len(values) != BASIS_INPUTS:
        raise ValueError(f"a basis input has {BASIS_INPUTS} integers, not {len(values)}")
    limb_width = width * LIMB_BYTES
    encoded = bytearray()
    for value in values:
        try:
            magnitude = abs(value).to_bytes(limb_width, "little")
        except OverflowError as error:
            raise ValueError(f"value does not fit in {width} limbs") from error
        encoded += magnitude
        encoded.append(int(value < 0))
    return bytes(encoded)


def nontrivial_words(encoded, width):
    """Distinct input limbs with at least `MIN_DISTINCT_OCTETS` distinct bytes.

    Order is the order of the basis. The sign byte is not a limb.
    """
    stride = basis_stride(width)
    if len(encoded) != BASIS_INPUTS * stride:
        raise ValueError("encoded basis has the wrong length")
    limb_width = width * LIMB_BYTES
    words = []
    seen = set()
    for start in range(0, len(encoded), stride):
        magnitude = encoded[start:start + limb_width]
        for offset in range(0, limb_width, LIMB_BYTES):
            word = magnitude[offset:offset + LIMB_BYTES]
            if len(set(word)) < MIN_DISTINCT_OCTETS or word in seen:
                continue
            seen.add(word)
            words.append(word)
    return words


def retention_canary(width, canary_bytes):
    """A basis whose leading limbs are a planted copy the stack scan must find.

    `canary_bytes` comes from the probe's `constants` command. Each planted
    limb is a run of distinct octets, so it sits strictly above
    `MIN_DISTINCT_OCTETS`. The rest of the basis is zeros, which the word
    filter drops.
    """
    if canary_bytes <= 0 or canary_bytes % LIMB_BYTES != 0:
        raise ValueError(f"canary length {canary_bytes} is not a run of limbs")
    if canary_bytes > width * LIMB_BYTES:
        raise ValueError("canary does not fit in the first coordinate")
    planted = bytes(range(canary_bytes))
    total = BASIS_INPUTS * basis_stride(width)
    return planted + bytes(total - canary_bytes)


def self_test():
    """Fail if the encoder, the word filter, or the committed census drifts."""
    width = SET_WIDTHS["p248"]
    canary_bytes = 64
    canary = retention_canary(width, canary_bytes)
    words = nontrivial_words(canary, width)
    if len(words) != canary_bytes // LIMB_BYTES:
        raise AssertionError("a planted canary limb was filtered out")
    if nontrivial_words(bytes(len(canary)), width):
        raise AssertionError("an all-zero basis produced a word")
    encoded = encode_basis([1, -2, *([0] * (BASIS_INPUTS - 2))], width)
    stride = basis_stride(width)
    if encoded[width * LIMB_BYTES] != 0 or encoded[stride + width * LIMB_BYTES] != 1:
        raise AssertionError("sign byte does not follow the magnitude")
    if int.from_bytes(encoded[stride:stride + width * LIMB_BYTES], "little") != 2:
        raise AssertionError("negative magnitude was not the absolute value")
    try:
        encode_basis([1 << (width * 64), *([0] * (BASIS_INPUTS - 1))], width)
    except ValueError:
        pass
    else:
        raise AssertionError("an over-wide value encoded")
    counts = Counter(cls for cls, _ in corpus("p248"))
    if dict(counts) != CLASS_COUNTS:
        raise AssertionError(f"p248 census {dict(counts)}")


def verify_set(name, width):
    _, p, _ = signer_parameters(name)
    counts = Counter()
    for cls, values in corpus(name):
        counts[cls] += 1
        t = values[0]
        coordinates = values[1:BASIS_INPUTS]
        u, v = reduce_exact(p, tuple(coordinates[:4]), tuple(coordinates[4:8]))
        form = norm_form(p, columns(u, v), t)
        expected = [*u, *v, form[0][0], form[2][2], form[2][0], form[2][1]]
        if expected != values[BASIS_INPUTS:]:
            raise ValueError(f"incorrect basis or form on {name} {cls} fixture {counts[cls]}")
    print(f"Exact {name} oracle checked {dict(counts)}")
    schedules = native_schedules(name)
    guarded = schedules["Signing, pre-reduced, guard 256"][1]
    generic = schedules["Signing, generic HNF"][1]
    return f"stage2c {name} limbs={width} guarded={guarded} generic={generic} bases={sum(counts.values())}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native-log", type=Path, help="check actual native runs against the proof rows")
    args = parser.parse_args()
    self_test()
    expected = [verify_set(name, width) for name, width in SET_WIDTHS.items()]
    if args.native_log:
        actual = [line for line in args.native_log.read_text().splitlines() if line.startswith("stage2c ")]
        if Counter(actual) != Counter(expected):
            raise ValueError(f"native coverage differs: expected {expected}, got {actual}")


if __name__ == "__main__":
    main()
