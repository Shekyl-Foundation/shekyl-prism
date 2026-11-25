#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Variable-time oracle for SIGNER_SPEC 2.3. All inputs here are public fixtures."""

import itertools
import json
import math
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]


def times_i(vector):
    a, b, c, d = vector
    return -b, a, -d, c


def columns(u, v):
    return u, times_i(u), v, times_i(v)


def inner(p, x, y):
    return x[0]*y[0] + x[1]*y[1] + p*(x[2]*y[2] + x[3]*y[3])


def combine(basis, coefficients):
    return tuple(sum(a*b for a, b in zip(row, coefficients)) for row in zip(*basis))


def reduce_exact(p, u, v):
    """Public-fixture reference. The production schedule needs a separate proof."""
    if inner(p, v, v) < inner(p, u, u):
        u, v = v, u
    while True:
        a = inner(p, u, u)
        if a == 0:
            raise ValueError("the Gaussian generators must be independent")
        ju = times_i(u)
        real = (2*inner(p, v, u) + a) // (2*a)
        imag = (2*inner(p, v, ju) + a) // (2*a)
        v = combine((v, u, ju), (1, -real, -imag))
        if inner(p, v, v) >= a:
            return u, v
        u, v = v, u


def determinant(basis):
    # Each tuple is a column. The determinant of the transpose is the same.
    total = 0
    for order in itertools.permutations(range(4)):
        inversions = sum(x > y for i, x in enumerate(order) for y in order[i+1:])
        term = math.prod(column[row] for column, row in zip(basis, order))
        total += -term if inversions % 2 else term
    return total


def in_lattice(basis, det, vector):
    # Cramer's rule tests membership independently of Gaussian reduction.
    return all(determinant(basis[:i] + (vector,) + basis[i+1:]) % det == 0
               for i in range(4))


def norm_form(p, basis, ideal_norm):
    form = [[0]*4 for _ in basis]
    for i, x in enumerate(basis):
        for j in range(i + 1):
            entry = inner(p, x, basis[j]) * (1 if i == j else 2)
            form[i][j], remainder = divmod(entry, 4*ideal_norm)
            if remainder:
                raise ValueError("the class norm form must be integral")
    return form


def field_prime(name):
    path = ROOT / "prism-vectors" / "data" / "signer" / (name + ".txt")
    values = dict(line.split() for line in path.read_text().splitlines()
                  if line.startswith(("e ", "f ")))
    return (int(values["f"]) << int(values["e"])) - 1


class Stage2cOracle(unittest.TestCase):
    def check_reduction(self, p, u, v):
        basis = columns(u, v)
        det = determinant(basis)
        self.assertNotEqual(det, 0)
        first, second = reduce_exact(p, u, v)
        result = columns(first, second)
        self.assertEqual(abs(determinant(result)), abs(det))
        for vector in result:
            self.assertTrue(in_lattice(basis, det, vector))
        a = inner(p, first, first)
        self.assertLessEqual(a, inner(p, second, second))
        for vector in (first, times_i(first)):
            twice = 2*inner(p, vector, second)
            self.assertLessEqual(-a, twice)
            self.assertLess(twice, a)
        self.assertEqual(reduce_exact(p, first, second), (first, second))
        return first, second

    def check_form(self, p, result, ideal_norm):
        basis = columns(*result)
        form = norm_form(p, basis, ideal_norm)
        # Singletons and pair sums determine all coefficients of a quadratic form.
        units = ((1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1))
        inputs = list(units) + [(-3, 1, 4, -2)]
        for x, y in itertools.combinations(units, 2):
            inputs.append(combine((x, y), (1, 1)))
        for i in range(4):
            self.assertTrue(all(value == 0 for value in form[i][i+1:]))
        for c in inputs:
            value = sum(c[i]*form[i][j]*c[j] for i in range(4) for j in range(4))
            a, b, k, d = combine(basis, c)
            self.assertEqual(value * 4*ideal_norm, a*a + b*b + p*(k*k + d*d))

    def test_restoring_division_rounding(self):
        # Exhaust all six-bit sign/magnitude dividends and positive divisors.
        bits = 6
        for a in range(1, 1 << bits):
            for h in range(1 - (1 << bits), 1 << bits):
                quotient = remainder = 0
                for i in reversed(range(bits)):
                    trial = 2*remainder + ((abs(h) >> i) & 1)
                    self.assertLess(trial, 2*a)
                    take = int(trial >= a)
                    remainder = trial - take*a
                    quotient = 2*quotient + take
                self.assertEqual((quotient, remainder), divmod(abs(h), a))
                if h < 0:
                    rounded = -(quotient + (remainder > a - remainder))
                else:
                    rounded = quotient + (remainder >= a - remainder)
                self.assertEqual(rounded, (2*h + a) // (2*a))

    def test_ties_and_order(self):
        u = (2, 0, 0, 0)
        # Equal norms before and after size reduction. The imaginary +1/2 moves.
        self.assertEqual(self.check_reduction(3, u, (0, 1, 1, 0)), (u, (0, -1, 1, 0)))
        # Positive real and negative imaginary halves in one step.
        self.assertEqual(self.check_reduction(7, (1, -1, 1, 1), u), (u, (-1, -1, 1, 1)))
        with self.assertRaises(ValueError):
            reduce_exact(7, u, times_i(u))

    def test_exhaustive_minima(self):
        for p, u, v in ((3, (4, 0, 0, 0), (2, 1, 1, 0)),
                        (7, (2, 2, 0, 0), (-2, 1, 1, 0)),
                        (19, (3, 0, 1, 0), (0, -1, 0, 1))):
            with self.subTest(p=p):
                basis = columns(u, v)
                det = determinant(basis)
                first, _ = self.check_reduction(p, u, v)
                shortest = inner(p, first, first)
                limits = [math.isqrt((shortest - 1)//w) for w in (1, 1, p, p)]
                for x in itertools.product(*(range(-r, r + 1) for r in limits)):
                    if any(x) and inner(p, x, x) < shortest:
                        self.assertFalse(in_lattice(basis, det, x), x)

    def test_field_weights_and_full_quotients(self):
        for name in ("p248", "p324", "p376", "p500", "p664"):
            with self.subTest(name=name):
                p = field_prime(name)
                t = 3
                x, y = next((x, y) for x in range(0, 2*t, 2) for y in range(1, 2*t, 2)
                            if (x*x + y*y + p) % (4*t) == 0)
                u, v = (2*t, 0, 0, 0), (x, y, 1, 0)
                pair = self.check_reduction(p, u, v)
                self.check_form(p, pair, t)
                sheared = combine((v, u, times_i(u)), (1, p + 1, -p))
                self.assertEqual(self.check_reduction(p, sheared, u), pair)

    def test_form_orientation_and_denominator(self):
        # Both real and imaginary Gram entries contribute with different signs.
        pair = ((2, 0, 0, 0), (1, 1, 1, 1))
        form = norm_form(7, columns(*pair), 1)
        self.assertEqual(form, [[1, 0, 0, 0], [0, 1, 0, 0],
                                [1, 1, 4, 0], [-1, 1, 0, 4]])
        self.check_form(7, pair, 1)
        with self.assertRaises(ValueError):
            norm_form(7, columns(*pair), 3)

    def test_p500_saturation_regression(self):
        fixture = json.loads((ROOT / "scripts/fixtures/stage2c_p500.json").read_text())
        t, x, y = (int(fixture[key], 16) for key in ("T", "x", "y"))
        p = field_prime("p500")
        self.assertEqual((x*x + y*y + p) % t, 0)
        # Lift the two residues mod T to their unique even and odd representatives.
        ox = x % t
        oy = y % t
        if ox % 2:
            ox += t
        if oy % 2 == 0:
            oy += t
        for key, scale, a, b in (("model_minimum", t, x, y),
                                 ("o0_numerator_minimum", 2*t, ox, oy)):
            with self.subTest(key=key):
                pair = self.check_reduction(p, (scale, 0, 0, 0), (a, b, 1, 0))
                self.assertEqual(inner(p, pair[0], pair[0]), int(fixture[key], 16))
                if key == "o0_numerator_minimum":
                    self.check_form(p, pair, t)


if __name__ == "__main__":
    unittest.main()
