#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Variable-time oracle for SIGNER_SPEC 2.3. All inputs here are public fixtures."""

import itertools
import json
import math
from fractions import Fraction
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


def size_reduce(p, u, v, a):
    ju = times_i(u)
    real = (2*inner(p, v, u) + a) // (2*a)
    imag = (2*inner(p, v, ju) + a) // (2*a)
    return combine((v, u, ju), (1, -real, -imag))


def reduce_exact(p, u, v):
    """Variable-time reference for the logical exact steps."""
    if inner(p, v, v) < inner(p, u, u):
        u, v = v, u
    while True:
        a = inner(p, u, u)
        if a == 0:
            raise ValueError("the Gaussian generators must be independent")
        v = size_reduce(p, u, v, a)
        if inner(p, v, v) >= a:
            return u, v
        u, v = v, u


def exp64_quotients(h0, h1, a):
    """Integer model of one exp64 step on public fixtures."""
    shift = max(0, a.bit_length() - 64)
    top = a >> shift
    exponent = max(h0.bit_length(), h1.bit_length()) - a.bit_length()
    result = []
    for h in (h0, h1):
        if exponent <= 60:
            estimate = (2*(h >> shift) + top) // (2*top)
            twice_residual = 2*(h - estimate*a)
            result.append(estimate + (twice_residual >= a) - (twice_residual < -a))
        else:
            s = exponent - 60
            qtop = (abs(h) >> (shift + s)) // top
            magnitude = max(0, qtop - 1) << s
            result.append(-magnitude if h < 0 else magnitude)
    return tuple(result)


def reduce_exp64(p, u, v, steps):
    if inner(p, v, v) < inner(p, u, u):
        u, v = v, u
    for _ in range(steps):
        a = inner(p, u, u)
        ju = times_i(u)
        real, imag = exp64_quotients(inner(p, v, u), inner(p, v, ju), a)
        v = combine((v, u, ju), (1, -real, -imag))
        if inner(p, v, v) < a:
            u, v = v, u
    return u, v


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


def signer_parameters(name):
    path = ROOT / "prism-vectors" / "data" / "signer" / (name + ".txt")
    values = dict(line.split() for line in path.read_text().splitlines()
                  if line.startswith(("e ", "f ", "secret_norm ")))
    e = int(values["e"])
    p = (int(values["f"]) << e) - 1
    d = int.from_bytes(bytes.fromhex(values["secret_norm"]), "little")
    return e, p, d


def schedule(phi_squared_over_delta, max_norm):
    # Largest S for which rho(S)^2 Phi_1^2 / Delta >= 1000, starting
    # with rho(1) = 1. Pairs contract by 87/1000, one step by 31/100.
    def reaches_main_phase(s):
        pairs, single = divmod(s - 1, 2)
        rho = Fraction(87, 1000)**pairs * Fraction(31, 100)**single
        return rho*rho*phi_squared_over_delta >= 1000

    s = 1
    while reaches_main_phase(s + 1):
        s += 1
    exact = s + 5
    extra = 0
    while 4**(58*(extra + 1) + 2) < max_norm:
        extra += 1
    return exact, exact + extra


def native_schedules(name):
    e, p, d = signer_parameters(name)
    n = 1 << ((p.bit_length() + 1)//2 + 32)
    q = 1 << (2*e - 5)

    # All ratios and maximum norms here are in reduced-norm units.
    # For a full norm-T O0 basis Delta = p*T^2/4 and Phi_1 <= 2*T^2.
    def generic(t):
        return schedule(Fraction(16*t*t, p), 2*t*t + p)

    guard = 256
    ceil_sqrt_p = 1 + math.isqrt(p)
    guarded = schedule(16*guard**2*q*q, (q + 1)**2 * guard * ceil_sqrt_p * n)
    return {
        "Signing, generic HNF": generic(n*q),
        "Signing, pre-reduced, guard 256": guarded,
        "Key generation, norm D": generic(d),
        "Secret ideal pre-reduction, Q4": generic(n),
        "Challenge alone": generic(q),
    }


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

    def test_exp64_large_quotient_progress(self):
        a = (1 << 130) + 15
        for offset in (60, 61, 119, 640):
            for h0, h1 in ((a << offset, -a//3), (1 - a, -(a << offset))):
                q = exp64_quotients(h0, h1, a)
                if offset == 60:
                    self.assertEqual(q, ((2*h0 + a)//(2*a), (2*h1 + a)//(2*a)))
                    continue
                r0, r1 = h0 - q[0]*a, h1 - q[1]*a
                self.assertGreater(max(abs(r0), abs(r1)), a)
                self.assertLess(r0*r0 + r1*r1, h0*h0 + h1*h1)
                new_exponent = max(r0.bit_length(), r1.bit_length()) - a.bit_length()
                self.assertLessEqual(new_exponent, offset - 58)

    def test_native_schedule_table(self):
        sets = ("p248", "p324", "p376", "p500", "p664")
        schedules = [native_schedules(name) for name in sets]
        spec = (ROOT / "docs/SIGNER_SPEC.md").read_text()
        table = spec.split("| Native input class |", 1)[1].splitlines()
        rows = []
        for line in table[2:]:
            if not line.startswith("|"):
                break
            rows.append([cell.strip() for cell in line.strip("|").split("|")])
        self.assertCountEqual([row[0] for row in rows], schedules[0])
        self.assertEqual([cell.strip() for cell in table[0].strip("| ").split("|")],
                         [name.upper() for name in sets])
        for label, *counts in rows:
            self.assertEqual(counts, [f"{s[label][0]} / {s[label][1]}" for s in schedules])

    def test_exp64_window_correction(self):
        # Discarded low bits change the uncorrected rounding on either sign.
        a = (1 << 80) + 2
        for offset in (-1, 0, 1):
            h = (1 << 55)*a + a//2 + offset
            self.assertEqual(exp64_quotients(h, -h, a),
                             ((2*h + a)//(2*a), (-2*h + a)//(2*a)))

    def test_native_hermitian_determinant(self):
        # Model and O0 lifts generate different lattices even for the same T.
        p, t = 3, 5
        for u, v, expected in (((t, 0, 0, 0), (4, 4, 1, 0), p*t*t),
                               ((2*t, 0, 0, 0), (4, 9, 1, 0), 4*p*t*t)):
            for x, y in ((u, v), reduce_exact(p, u, v)):
                delta = inner(p, x, x)*inner(p, y, y)
                delta -= inner(p, x, y)**2 + inner(p, times_i(x), y)**2
                self.assertEqual(delta, expected)

    def test_ties_and_order(self):
        u = (2, 0, 0, 0)
        # Equal norms before and after size reduction. The imaginary +1/2 moves.
        self.assertEqual(self.check_reduction(3, u, (0, 1, 1, 0)), (u, (0, -1, 1, 0)))
        # Positive real and negative imaginary halves in one step.
        self.assertEqual(self.check_reduction(7, (1, -1, 1, 1), u), (u, (-1, -1, 1, 1)))
        with self.assertRaises(ValueError):
            reduce_exact(7, u, times_i(u))

    def test_final_size_reduction_after_last_swap(self):
        # A norm-five ideal at toy prime 3. Exactly one swap, two active steps.
        u, v = (10, 0, 0, 0), (4, 9, 1, 0)
        first = size_reduce(3, u, v, inner(3, u, u))
        self.assertEqual(first, (4, -1, 1, 0))
        a = inner(3, first, first)
        self.assertLess(a, inner(3, u, u))
        # After swapping, the first vector is already shortest. The imaginary
        # projection of the second is exactly +1/2, outside the terminal range.
        self.assertEqual(2*inner(3, u, times_i(first)), a)
        last = size_reduce(3, first, u, a)
        self.assertEqual(last, (1, -2, -2, -1))
        self.assertEqual(inner(3, last, last), a)
        self.assertEqual(self.check_reduction(3, u, v), (first, last))

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
                _, p, _ = signer_parameters(name)
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
        _, p, _ = signer_parameters("p500")
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
                steps = native_schedules("p500")["Signing, generic HNF"][1]
                self.assertEqual(reduce_exp64(p, (scale, 0, 0, 0), (a, b, 1, 0), steps), pair)
                if key == "o0_numerator_minimum":
                    self.check_form(p, pair, t)


if __name__ == "__main__":
    unittest.main()
