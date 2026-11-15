#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The challenge scalar, computed without any code of this workspace.

The verifier derives `s` as cSHAKE256(pk || salt || message) with the
customization string `shekyl-prism-challenge-v2`, takes `e - 3` bits
little-endian, and forces the lowest bit to one
(`prism-verify/src/challenge.rs`). This script does the same from the
text of FIPS 202 and SP 800-185: its own Keccak-f[1600], its own cSHAKE
padding, no library hash for the result.

    challenge_scalar.py --check    what CI runs
    challenge_scalar.py --write    rewrite the p324 file

`--check` does three things, in this order, and stops at the first that
fails:

1. the Keccak here gives SHAKE256 where Python's hashlib gives it, on a
   spread of lengths, so the permutation and the sponge are right;
2. the scalar here equals the `s` of every committed vector, at the three
   sets that have vectors, so the cSHAKE framing and the projection are
   the verifier's;
3. `prism-verify/testdata/p324_challenge.tsv` is what this script writes.

Only then does the file mean anything at `p324`, which has no vectors: a
Rust test compares the verifier's scalar with each row of it.
"""
import hashlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TESTDATA = ROOT / "prism-verify" / "testdata"
OUT = TESTDATA / "p324_challenge.tsv"
CURVES = TESTDATA / "p324_curves.tsv"
DOMAIN = b"shekyl-prism-challenge-v2"
RATE = 136  # bytes, for a 256-bit capacity strength
MASK = (1 << 64) - 1

ROUND_CONSTANTS = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808A, 0x8000000080008000,
    0x000000000000808B, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008A, 0x0000000000000088, 0x0000000080008009, 0x000000008000000A,
    0x000000008000808B, 0x800000000000008B, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800A, 0x800000008000000A,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
]
ROTATIONS = [
    [0, 36, 3, 41, 18],
    [1, 44, 10, 45, 2],
    [62, 6, 43, 15, 61],
    [28, 55, 25, 21, 56],
    [27, 20, 39, 8, 14],
]


def rotate(value, count):
    count %= 64
    return ((value << count) | (value >> (64 - count))) & MASK


def keccak_f(lanes):
    """Keccak-f[1600] on a 5 by 5 array of 64-bit lanes, `lanes[x][y]`."""
    for constant in ROUND_CONSTANTS:
        column = [lanes[x][0] ^ lanes[x][1] ^ lanes[x][2] ^ lanes[x][3] ^ lanes[x][4] for x in range(5)]
        for x in range(5):
            mix = column[(x - 1) % 5] ^ rotate(column[(x + 1) % 5], 1)
            for y in range(5):
                lanes[x][y] ^= mix
        moved = [[0] * 5 for _ in range(5)]
        for x in range(5):
            for y in range(5):
                moved[y][(2 * x + 3 * y) % 5] = rotate(lanes[x][y], ROTATIONS[x][y])
        for x in range(5):
            for y in range(5):
                lanes[x][y] = moved[x][y] ^ ((~moved[(x + 1) % 5][y]) & moved[(x + 2) % 5][y])
        lanes[0][0] ^= constant
    return lanes


def sponge(message, suffix, length):
    """Absorb `message` and the domain `suffix` byte, squeeze `length` bytes."""
    padded = bytearray(message)
    padded.append(suffix)
    while len(padded) % RATE:
        padded.append(0)
    padded[-1] |= 0x80
    lanes = [[0] * 5 for _ in range(5)]
    for start in range(0, len(padded), RATE):
        block = padded[start : start + RATE]
        for index in range(RATE // 8):
            lane = int.from_bytes(block[8 * index : 8 * index + 8], "little")
            lanes[index % 5][index // 5] ^= lane
        keccak_f(lanes)
    out = bytearray()
    while len(out) < length:
        for index in range(RATE // 8):
            out += lanes[index % 5][index // 5].to_bytes(8, "little")
        if len(out) < length:
            keccak_f(lanes)
    return bytes(out[:length])


def shake256(message, length):
    return sponge(message, 0x1F, length)


def left_encode(value):
    width = max(1, (value.bit_length() + 7) // 8)
    return bytes([width]) + value.to_bytes(width, "big")


def encode_string(text):
    return left_encode(8 * len(text)) + text


def bytepad(text, width):
    padded = left_encode(width) + text
    return padded + bytes(-len(padded) % width)


def cshake256(message, length, customization):
    """SP 800-185 cSHAKE256 with an empty function name."""
    if not customization:
        return shake256(message, length)
    prefix = bytepad(encode_string(b"") + encode_string(customization), RATE)
    return sponge(prefix + message, 0x04, length)


def challenge(e, public_key, salt, message):
    """The verifier's scalar at exponent `e`, as little-endian bytes."""
    bits = e - 3
    raw = bytearray(cshake256(public_key + salt + message, (bits + 7) // 8, DOMAIN))
    raw[0] |= 1
    raw[-1] &= 0xFF >> (7 - ((bits - 1) % 8))
    return bytes(raw)


def check_keccak():
    for size in (0, 1, 55, 135, 136, 137, 272, 1000):
        message = bytes((7 * i + size) % 256 for i in range(size))
        for length in (1, 32, 136, 137, 400):
            if shake256(message, length) != hashlib.shake_256(message).digest(length):
                raise SystemExit(f"this Keccak is not SHAKE256 at input {size}, output {length}")


# The three sets that have committed vectors, by the string those vectors
# carry in their `level` field, with each exponent.
COMMITTED = {"NIST-I": 248, "NIST-III": 376, "NIST-V": 500}


def check_vectors():
    files = sorted(TESTDATA.glob("s5_lvl*.txt")) + sorted((TESTDATA / "accept").glob("*.txt"))
    if len(files) != 113:
        raise SystemExit(f"expected 113 committed vectors, found {len(files)}")
    seen = {label: 0 for label in COMMITTED}
    for path in files:
        rows = dict(line.split(" ", 1) for line in path.read_text().splitlines() if " " in line)
        e = COMMITTED[rows["level"]]
        scalar = challenge(e, bytes.fromhex(rows["A"]), bytes.fromhex(rows["salt"]), rows["message"].encode())
        if int.from_bytes(scalar, "little") != int(rows["s"]):
            raise SystemExit(f"{path.name}: this script's scalar is not the vector's s")
        seen[rows["level"]] += 1
    if min(seen.values()) == 0:
        raise SystemExit("a set with committed vectors was not checked")
    return seen


E_324 = 324
SALT_324 = 48


def p324_rows():
    """Fixed inputs at p324: each curve of the Sage file as the key, with a
    spread of salts and messages."""
    keys = [line.split("\t")[2] for line in CURVES.read_text().splitlines() if line.startswith("CURVE\t")]
    if len(keys) != 8:
        raise SystemExit("expected eight curves in p324_curves.tsv")
    salts = [
        bytes(SALT_324),
        bytes([0xFF]) * SALT_324,
        bytes(range(SALT_324)),
        (1).to_bytes(8, "little") + bytes(SALT_324 - 8),
    ]
    messages = [b"", b"shekyl-prism-p324", b"binary\x00message", bytes(range(256)) * 2]
    lines = []
    for index, key in enumerate(keys):
        salt = salts[index % len(salts)]
        for message in (messages[index % len(messages)], messages[(index + 1) % len(messages)]):
            scalar = challenge(E_324, bytes.fromhex(key), salt, message)
            if len(scalar) != 41 or scalar[0] & 1 != 1 or scalar[-1] >> 1:
                raise SystemExit("a p324 scalar is not odd and 321 bits wide")
            lines.append("\t".join([key, salt.hex(), message.hex() or "-", scalar.hex()]))
    return lines


def p324_text():
    header = [
        "# Challenge scalars at p324, from scripts/challenge_scalar.py. Do not edit.",
        "# cSHAKE256(pk || salt || message), customization shekyl-prism-challenge-v2,",
        "# 321 bits little-endian with the lowest bit forced to one.",
        "# The script is anchored first on the s of all 113 committed vectors.",
        "# key\tsalt\tmessage (hex, '-' for empty)\tscalar (41 bytes, little-endian)",
    ]
    return "\n".join(header + p324_rows()) + "\n"


def main():
    if sys.argv[1:] not in (["--check"], ["--write"]):
        raise SystemExit("usage: challenge_scalar.py --check | --write")
    check_keccak()
    seen = check_vectors()
    text = p324_text()
    if sys.argv[1] == "--write":
        OUT.write_text(text)
        print(f"wrote {OUT.name}")
        return
    if not OUT.is_file() or OUT.read_text() != text:
        raise SystemExit(f"{OUT.name} is not what this script writes; run --write and commit it")
    counts = ", ".join(f"{label} {count}" for label, count in seen.items())
    print(f"SHAKE256 agrees with hashlib; the scalar agrees with all 113 vectors ({counts}); "
          f"{OUT.name} is current ({len(text.splitlines()) - 5} rows)")


if __name__ == "__main__":
    main()
