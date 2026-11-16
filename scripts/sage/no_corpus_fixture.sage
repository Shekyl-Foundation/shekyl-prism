# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Sage values for a parameter set that has no signer and so no vectors.
# The set is named on the command line. Two files are written for it:
#
#   prism-vectors/data/verify/<set>_field.tsv    field arithmetic in F_p, F_p^2
#   prism-vectors/data/verify/<set>_curves.tsv   supersingular curves, points of
#                                            known order, a 2^e basis, pairings
#
# Run from the repository root in the pinned image, with the Theta_dim4
# submodule checked out (the pairing is that library's weil_pairing_pari,
# the function scripts/sage/weil_pairing.sage uses):
#
#   docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp -v "$PWD:/workspace" -w /workspace \
#     sagemath/sagemath@sha256:4f5589eb6c565949a006f8665de2876b8414410daf5ac554f4434a15d4f3d528 \
#     sage scripts/sage/no_corpus_fixture.sage p324
#
# Every set goes through the same code. What a set states is in SETS below:
# its cofactor and exponent. Everything else is computed from those: the
# prime, the search bound, the lengths, the edge values, the seed. Nothing
# is read from a table keyed by a label.
#
# The header of each file records the SHA-256 of this script, and
# scripts/ci/check_sage_fixture.py holds the committed files to it. An edit
# here therefore means running every set again. A run is deterministic:
# both generators are seeded from the set's name.
#
# What is independent of the Rust code and what mirrors it:
#   - Field results: Sage's own arithmetic. Independent.
#   - Supersingularity and point orders: Sage's own. Independent.
#   - The basis index (`basis_index`, `abscissa`): the same search the Rust
#     verifier runs, written again here. It mirrors the search; what is
#     independent is that Sage then confirms the points it names have order
#     2^e and pair to a primitive 2^e-th root of unity.
#   - Pairing values: weil_pairing_pari of the Theta_dim4 library.

import hashlib
import os
import sys
from pathlib import Path

ROOT = Path(os.environ.get("PRISM_ROOT", Path.cwd()))
if not (ROOT / "external" / "Theta_dim4" / "Theta_dim4_sage").is_dir():
    raise SystemExit("run from the repository root with external/Theta_dim4 checked out")
sys.path.insert(0, str(ROOT / "external" / "Theta_dim4" / "Theta_dim4_sage"))

from sage.all import EllipticCurve, GF, ZZ, set_random_seed
from sage.groups.generic import order_from_multiple
from pkg.utilities.discrete_log import weil_pairing_pari

# name: (f, e), for p = f * 2^e - 1.
SETS = {
    "p324": (3, 324),
    "p664": (17, 664),
}

if len(sys.argv) != 2 or sys.argv[1] not in SETS:
    raise SystemExit("usage: sage scripts/sage/no_corpus_fixture.sage <" + "|".join(SETS) + ">")
NAME = sys.argv[1]
F_COFACTOR, E_EXPONENT = SETS[NAME]
assert NAME == "p%d" % E_EXPONENT

# The curve walk: this many 2-isogeny steps from A = 0, and every
# WALK_STRIDE-th curve of it is written. Which of those curves are on
# which branch of the basis search, and which are a permitted response
# model, is an accident of each set's walk; prism-vectors records it
# for each set and the tests hold it.
WALK_STEPS = 40
WALK_STRIDE = 5

# `e` rounded up to a multiple of 128: the verifier's search bound.
SEARCH_BOUND = -(-E_EXPONENT // 128) * 128
P = F_COFACTOR * 2**E_EXPONENT - 1
P_BITS = int(P).bit_length()
FP_LEN = (P_BITS + 7) // 8
TOP_MASK = (1 << (P_BITS - 8 * (FP_LEN - 1))) - 1
RANDOM_ROWS = 10000
# "PRISM" and the exponent in ASCII, as one big-endian integer: for p324,
# 0x5052_4953_4D33_3234.
SEED = int.from_bytes(b"PRISM" + str(E_EXPONENT).encode(), "big")
assert SEED < 2**64

assert ZZ(P).is_prime()
assert P % 8 == 7
# The curve file has one column for the point missing the cofactor's
# prime. A cofactor with more than one prime would need a column for
# each, and no set without a corpus has one.
assert ZZ(F_COFACTOR).is_prime() and F_COFACTOR % 2 == 1

Fp = GF(P)
Fp2 = GF(P**2, name="i", modulus=ZZ["x"]("x^2+1"))
I = Fp2.gen()


class SplitMix64:
    """The generator the Rust test implements again. Sixty-four bits out."""

    def __init__(self, seed):
        self.state = int(seed) & (2**64 - 1)

    def next(self):
        mask = 2**64 - 1
        self.state = (self.state + 0x9E3779B97F4A7C15) & mask
        z = self.state
        z = ((z ^^ (z >> 30)) * 0xBF58476D1CE4E5B9) & mask
        z = ((z ^^ (z >> 27)) * 0x94D049BB133111EB) & mask
        return z ^^ (z >> 31)

    def element(self):
        """A uniform residue: FP_LEN bytes with the spare top bits cleared,
        drawn again while the value is not below p."""
        words = (FP_LEN + 7) // 8
        while True:
            raw = b"".join(int(self.next()).to_bytes(8, "little") for _ in range(words))
            raw = bytearray(raw[:FP_LEN])
            raw[-1] &= TOP_MASK
            value = int.from_bytes(bytes(raw), "little")
            if value < P:
                return value


def enc(value):
    return int(value).to_bytes(FP_LEN, "little")


def parts(z):
    coefficients = z.polynomial().list() + [0, 0]
    return int(coefficients[0]), int(coefficients[1])


def enc2(z):
    re, im = parts(z)
    return enc(re) + enc(im)


def canonical_root(value):
    """Of the two square roots of a base-field square, the smaller integer."""
    root = int(Fp(value).sqrt())
    return min(root, (P - root) % P)


def canonical_root2(value):
    """Of the two square roots in F_p^2, the one whose (re, im) is smaller,
    comparing re first."""
    root = Fp2(value).sqrt()
    return min(parts(root), parts(-root))


def base_row(a, b):
    a, b = Fp(a), Fp(b)
    inverse = enc(0) if a == 0 else enc(1 / a)
    square = a.is_square()
    return {
        "add": enc(a + b),
        "sub": enc(a - b),
        "mul": enc(a * b),
        "square": enc(a * a),
        "inv": inverse,
        "is_square": bytes([1 if square else 0]),
        "sqrt_of_square": enc(canonical_root(a * a)),
    }


def ext_row(x, y):
    inverse = enc2(Fp2(0)) if x == 0 else enc2(1 / x)
    re, im = canonical_root2(x * x)
    return {
        "add": enc2(x + y),
        "sub": enc2(x - y),
        "mul": enc2(x * y),
        "square": enc2(x * x),
        "inv": inverse,
        "is_square": bytes([1 if x.is_square() else 0]),
        "sqrt_of_square": enc(re) + enc(im),
    }


OPS = ["add", "sub", "mul", "square", "inv", "is_square", "sqrt_of_square"]


def field_file():
    lines = []
    # Small values, the top of the field, the two middles, a power of two
    # sixteen below 2^e, the last value below 2^e, and the top bit of p.
    edge = [
        0, 1, 2, 3, P - 1, P - 2, (P - 1) // 2, (P + 1) // 2,
        2 ** (E_EXPONENT - 4), 2**E_EXPONENT - 1, 2 ** (P_BITS - 1),
    ]
    for a in edge:
        for b in edge:
            row = base_row(a, b)
            lines.append(
                "BASE\t" + enc(a).hex() + "\t" + enc(b).hex() + "\t" + "\t".join(row[op].hex() for op in OPS)
            )
    small = [0, 1, P - 1, (P - 1) // 2, 2 ** (P_BITS - 1)]
    ext_edge = [Fp2(re) + I * Fp2(im) for re in small for im in small]
    for x in ext_edge[::3]:
        for y in ext_edge[::4]:
            row = ext_row(x, y)
            lines.append(
                "EXT\t" + enc2(x).hex() + "\t" + enc2(y).hex() + "\t" + "\t".join(row[op].hex() for op in OPS)
            )
    # Random values: the inputs come from the generator, which the Rust test
    # runs again, and each operation's results are hashed in row order.
    stream = SplitMix64(SEED)
    base_digest = {op: hashlib.sha3_256() for op in OPS}
    ext_digest = {op: hashlib.sha3_256() for op in OPS}
    squares = 0
    for _ in range(RANDOM_ROWS):
        a, b = stream.element(), stream.element()
        x = Fp2(stream.element()) + I * Fp2(stream.element())
        y = Fp2(stream.element()) + I * Fp2(stream.element())
        row = base_row(a, b)
        for op in OPS:
            base_digest[op].update(row[op])
        squares += row["is_square"][0]
        row = ext_row(x, y)
        for op in OPS:
            ext_digest[op].update(row[op])
    # About half of F_p is squares; a generator stuck on one value fails this.
    assert RANDOM_ROWS * 2 // 5 < squares < RANDOM_ROWS * 3 // 5
    for op in OPS:
        lines.append(f"RANDOM\tbase\t{op}\t{RANDOM_ROWS}\t{base_digest[op].hexdigest()}")
    for op in OPS:
        lines.append(f"RANDOM\text\t{op}\t{RANDOM_ROWS}\t{ext_digest[op].hexdigest()}")
    lines.append(f"STREAM\tnext_after_all_rows\t{stream.next():016x}")
    return lines


def rhs_square(A, x):
    return (x**3 + A * x**2 + x).is_square()


def basis_index(A):
    """The verifier's index search, with its bound. `1 + b^2` is tested in
    F_p."""
    if A.is_square():
        for b in range(1, SEARCH_BOUND + 1):
            if Fp(1 + b * b).is_square():
                continue
            z = 1 + I * ZZ(b)
            if not ((A * A) * (z - 1) - z * z).is_square():
                return ZZ(b), True
    else:
        x = A
        for n in range(1, SEARCH_BOUND + 1):
            if rhs_square(A, x):
                return ZZ(n), False
            x = x + A
    raise ValueError("basis index search failed inside the bound")


def abscissa(A, index, square):
    if square:
        return -A / (1 + I * ZZ(index))
    return A * ZZ(index)


def walk(stream, steps):
    """Montgomery coefficients on a 2-isogeny walk from A = 0. A kernel
    point (alpha, 0) with alpha a root of x^2 + A x + 1 gives the codomain
    A' = 2 (1 - 2 alpha^2)."""
    A = Fp2(0)
    found = []
    R = Fp2["x"]
    for _ in range(steps):
        roots = [root for root, _ in R([1, A, 1]).roots()]
        assert len(roots) == 2
        alpha = roots[stream.next() % 2]
        A = 2 * (1 - 2 * alpha**2)
        assert A**2 != 4
        found.append(A)
    return found


def curves_file():
    lines = []
    stream = SplitMix64(SEED ^^ 0xC0FFEE)
    # Sage draws the points of order p + 1 with its own generator. Seeded,
    # so that a second run writes the same file.
    set_random_seed(SEED)
    order = P + 1
    factorization = ZZ(order).factor()
    assert list(factorization) == [(2, E_EXPONENT), (F_COFACTOR, 1)]
    kept = 0
    for step, A in enumerate(walk(stream, WALK_STEPS), start=1):
        if step % WALK_STRIDE:
            continue
        E = EllipticCurve(Fp2, [0, A, 0, 1, 0])
        assert E.is_supersingular()
        # A point of order exactly p + 1. Sage's order, from the known
        # factorization of the multiple.
        while True:
            R = E.random_point()
            if R.is_zero():
                continue
            if order_from_multiple(R, order, factorization=factorization) == order:
                break
        no_f, no2 = F_COFACTOR * R, 2 * R
        assert order_from_multiple(no_f, order, factorization=factorization) == order // F_COFACTOR
        assert order_from_multiple(no2, order, factorization=factorization) == order // 2
        only_f = (2**E_EXPONENT) * R
        assert order_from_multiple(only_f, order, factorization=factorization) == F_COFACTOR
        index, square = basis_index(A)
        xp = abscissa(A, index, square)
        xq = -xp - A
        Pfull = E(xp, (xp**3 + A * xp**2 + xp).sqrt())
        Qfull = E(xq, (xq**3 + A * xq**2 + xq).sqrt())
        Pe, Qe = F_COFACTOR * Pfull, F_COFACTOR * Qfull
        n = 2**E_EXPONENT
        assert order_from_multiple(Pe, n, factorization=ZZ(n).factor()) == n
        assert order_from_multiple(Qe, n, factorization=ZZ(n).factor()) == n
        pairing = Fp2(weil_pairing_pari(Pe, Qe, n))
        assert pairing ** (n // 2) != 1 and pairing**n == 1
        kept += 1
        lines.append(
            "\t".join(
                [
                    "CURVE",
                    str(kept),
                    enc2(A).hex(),
                    str(index),
                    "1" if square else "0",
                    enc2(R[0]).hex(),
                    enc2(no_f[0]).hex(),
                    enc2(no2[0]).hex(),
                    enc2(only_f[0]).hex(),
                    enc2(Pe[0]).hex(),
                    enc2(Pe[1]).hex(),
                    enc2(Qe[0]).hex(),
                    enc2(Qe[1]).hex(),
                    enc2(pairing).hex(),
                ]
            )
        )
    assert kept == WALK_STEPS // WALK_STRIDE
    return lines


def header(what, columns):
    # Sage runs a preparsed copy of this file, so `__file__` is not it.
    script = ROOT / "scripts" / "sage" / "no_corpus_fixture.sage"
    digest = hashlib.sha256(script.read_bytes()).hexdigest()
    return [
        f"# {what} for {NAME}: p = {F_COFACTOR}*2^{E_EXPONENT} - 1.",
        "# Written by scripts/sage/no_corpus_fixture.sage. Do not edit.",
        "# Image: sagemath/sagemath@sha256:4f5589eb6c565949a006f8665de2876b8414410daf5ac554f4434a15d4f3d528",
        f"# Command: sage scripts/sage/no_corpus_fixture.sage {NAME} (from the repository root)",
        f"# Script sha256 when run: {digest}",
        f"# Generator: SplitMix64, seed 0x{SEED:016x}; field elements are little-endian, re then im.",
    ] + ["# " + line for line in columns]


def main():
    out = ROOT / "prism-vectors" / "data" / "verify"
    field = header(
        "Field arithmetic",
        [
            "BASE a b add sub mul square(a) inv(a) is_square(a) sqrt_of_square(a)",
            "EXT  x y add sub mul square(x) inv(x) is_square(x) sqrt_of_square(x)",
            "inv of zero is written as zero. sqrt_of_square is the canonical root of a^2:",
            "in F_p the smaller integer; in F_p^2 the smaller (re, im), re compared first.",
            "RANDOM field op rows sha3-256 of the op's results over the rows, in order.",
            "Each random row draws a, b, then re(x), im(x), re(y), im(y) from the generator.",
        ],
    ) + field_file()
    (out / f"{NAME}_field.tsv").write_text("\n".join(field) + "\n")
    curves = header(
        "Supersingular curves and points",
        [
            f"CURVE n A index square x(R) x([{F_COFACTOR}]R) x([2]R) x([2^e]R) x(P) y(P) x(Q) y(Q) e(P,Q)",
            "A is a Montgomery coefficient on a 2-isogeny walk from A = 0.",
            f"R has order p + 1, so [{F_COFACTOR}]R, [2]R and [2^e]R have orders (p+1)/{F_COFACTOR}, (p+1)/2 and {F_COFACTOR}.",
            f"index, square: the verifier's basis index search, bound {SEARCH_BOUND}, written again in Sage.",
            f"P, Q: [{F_COFACTOR}] of the points with abscissas x_index and -x_index - A; both of order 2^e.",
            "e(P,Q): weil_pairing_pari(P, Q, 2^e) of the Theta_dim4 library.",
        ],
    ) + curves_file()
    (out / f"{NAME}_curves.tsv").write_text("\n".join(curves) + "\n")
    print(f"wrote {NAME}_field.tsv ({len(field)} lines) and {NAME}_curves.tsv ({len(curves)} lines)")


main()
