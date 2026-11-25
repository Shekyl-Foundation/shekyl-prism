#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Variable-time oracle for SIGNER_SPEC 2.3.

Public fixtures only. The exact reducer is the contract. exp64 is the
production candidate. A schedule is derived from one class's reduced-norm
bounds: the ordered norms A0 and C0, and the Hermitian determinant Delta.
Phi_1 is at most twice A0, so the contraction ratio is 4*A0^2/Delta.
"""

import itertools
import json
import math
import unittest
from fractions import Fraction
from pathlib import Path
from typing import NamedTuple


ROOT = Path(__file__).resolve().parents[1]

# The certificate's contraction. rho(S) = RHO_PAIR^floor((S-1)/2)
# * RHO_STEP^((S-1) mod 2). The main phase runs while rho(S)^2 * Phi_1^2/Delta
# is at least 1/POTENTIAL_SPLIT.
RHO_PAIR = Fraction(87, 1000)
RHO_STEP = Fraction(31, 100)
POTENTIAL_SPLIT = Fraction(1, 1000)
# After the main phase: four tail swaps, then the size reduction that does not swap.
TAIL_SWAPS = 4
FINAL_SIZE_REDUCTION = 1

# exp64. The divisor is the top QUOTIENT_TOP_BITS of the norm. An exponent at
# or below QUOTIENT_WINDOW is one exact half-up step. A big step drops the
# exponent by QUOTIENT_WINDOW minus QUOTIENT_EXPONENT_LOSS (Lemma U). Lemma
# B2's count of those sub-steps uses that same loss as its additive slack.
QUOTIENT_TOP_BITS = 64
QUOTIENT_WINDOW = 60
QUOTIENT_EXPONENT_LOSS = 2
QUOTIENT_PROGRESS_BITS = QUOTIENT_WINDOW - QUOTIENT_EXPONENT_LOSS

# Q4: B = ceil(L/2) + Q4_SLACK_BITS, and a secret norm is below 2^B.
Q4_SLACK_BITS = 32
# A stored pre-reduced basis uses the guarded schedule only when
# nrd(v_s) <= PREREDUCED_NORM_GUARD * sqrt(p) * N.
PREREDUCED_NORM_GUARD = 256


class ClassBounds(NamedTuple):
    """Reduced-norm bounds of one admitted input class.

    a0_squared is the square of the upper bound on the ordered first norm.
    delta is the Hermitian determinant of a full basis of that class norm.
    c0 is the upper bound on the ordered second norm.
    """

    a0_squared: int
    delta: Fraction
    c0: int

    def contraction_ratio(self):
        """Phi_1 <= 2*A0, so Phi_1^2/Delta <= 4*A0^2/Delta."""
        return Fraction(4*self.a0_squared, 1)/self.delta


def challenge_degree_bound(exponent):
    """Exclusive upper bound on the challenge degree: q < 2^(2e-5)."""
    return 1 << (2*exponent - 5)


def q4_norm_bound(prime):
    """2^B, B = ceil(L/2) + Q4_SLACK_BITS. L is the bit length of prime."""
    return 1 << ((prime.bit_length() + 1)//2 + Q4_SLACK_BITS)


def ceil_sqrt(value):
    """Least integer whose square is at least value. value >= 0."""
    root = math.isqrt(value)
    if root*root == value:
        return root
    return root + 1


def generic_bounds(norm, prime):
    """A0 <= norm^2, Delta = p*norm^2/4, C0 <= 2*norm^2 + p."""
    return ClassBounds(norm**4, Fraction(prime*norm*norm, 4), 2*norm*norm + prime)


def guarded_signing_bounds(guard, degree_bound, secret_bound, prime):
    """Pre-reduced signing, in reduced-norm units.

    A0 <= guard*sqrt(p)*N*q^2, so A0^2 <= guard^2*p*N^2*q^4.
    Delta = p*(N*q)^2/4. C0 <= (q+1)^2*guard*ceil(sqrt(p))*N.
    """
    a0_squared = guard*guard*prime*secret_bound*secret_bound*degree_bound**4
    delta = Fraction(prime*(secret_bound*degree_bound)**2, 4)
    c0 = (degree_bound + 1)**2*guard*ceil_sqrt(prime)*secret_bound
    return ClassBounds(a0_squared, delta, c0)


def times_i(vector):
    a, b, c, d = vector
    return -b, a, -d, c


def columns(u, v):
    return u, times_i(u), v, times_i(v)


def inner(p, x, y):
    return x[0]*y[0] + x[1]*y[1] + p*(x[2]*y[2] + x[3]*y[3])


def combine(basis, coefficients):
    return tuple(sum(a*b for a, b in zip(row, coefficients)) for row in zip(*basis))


def half_up_quotient(dividend, divisor):
    """floor(dividend/divisor + 1/2). Ties go toward positive infinity. divisor > 0."""
    return (2*dividend + divisor)//(2*divisor)


def correct_half_up(estimate, dividend, divisor):
    """Move an estimate by at most one onto floor(dividend/divisor + 1/2).

    The in-window exp64 estimate is at most one away from that integer.
    X = 2*dividend + divisor - 2*estimate*divisor moves it: +1 when X >= 2*divisor,
    -1 when X < 0.
    """
    twice_residual = 2*(dividend - estimate*divisor)
    if twice_residual >= divisor:
        return estimate + 1
    if twice_residual < -divisor:
        return estimate - 1
    return estimate


def size_reduce(p, u, v, a):
    ju = times_i(u)
    real = half_up_quotient(inner(p, v, u), a)
    imag = half_up_quotient(inner(p, v, ju), a)
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


def exp64_quotients(h0, h1, norm):
    """One exp64 step on public fixtures. The production port masks both paths."""
    if norm <= 0:
        raise ValueError("the Gaussian generators must be independent")
    shift = max(0, norm.bit_length() - QUOTIENT_TOP_BITS)
    top = norm >> shift
    exponent = max(h0.bit_length(), h1.bit_length()) - norm.bit_length()
    if exponent <= QUOTIENT_WINDOW:
        return tuple(
            correct_half_up(half_up_quotient(h >> shift, top), h, norm)
            for h in (h0, h1))
    scale = exponent - QUOTIENT_WINDOW

    def big(component):
        # One unit below the kept top, then shifted back. A top of 0 or 1
        # contributes nothing, which is the undershoot.
        qtop = (abs(component) >> (shift + scale))//top
        magnitude = max(0, qtop - 1) << scale
        return -magnitude if component < 0 else magnitude

    return big(h0), big(h1)


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
            entry = inner(p, x, basis[j])*(1 if i == j else 2)
            form[i][j], remainder = divmod(entry, 4*ideal_norm)
            if remainder:
                raise ValueError("the class norm form must be integral")
    return form


def signer_parameters(name):
    path = ROOT/"prism-vectors"/"data"/"signer"/(name + ".txt")
    values = dict(line.split() for line in path.read_text().splitlines()
                  if line.startswith(("e ", "f ", "secret_norm ")))
    exponent = int(values["e"])
    prime = (int(values["f"]) << exponent) - 1
    secret_norm = int.from_bytes(bytes.fromhex(values["secret_norm"]), "little")
    return exponent, prime, secret_norm


def main_phase_reaches(steps, ratio):
    """Whether rho(steps)^2 * (Phi_1^2/Delta) is still at least 1/POTENTIAL_SPLIT."""
    pairs, single = divmod(steps - 1, 2)
    rho = RHO_PAIR**pairs*RHO_STEP**single
    return rho*rho*ratio >= 1/POTENTIAL_SPLIT


def big_steps_fit(count, max_norm):
    """Lemma B2: `count` big sub-steps fit when 4^(progress*count + loss) < C0.

    lambda_1 >= 1, so the comparison is against the reduced-norm bound C0.
    """
    bits = QUOTIENT_PROGRESS_BITS*count + QUOTIENT_EXPONENT_LOSS
    return 4**bits < max_norm


def schedule(bounds):
    """Exact steps, then exp64 steps, after the initial strict ordering.

    The exact count keeps at least one main-phase step, then adds the tail
    swaps and the final size reduction. Exp64 appends the big sub-steps that
    Lemma B2 allows under the class's C0.
    """
    ratio = bounds.contraction_ratio()
    main = 1
    while main_phase_reaches(main + 1, ratio):
        main += 1
    exact = main + TAIL_SWAPS + FINAL_SIZE_REDUCTION
    extra = 0
    while big_steps_fit(extra + 1, bounds.c0):
        extra += 1
    return exact, exact + extra


def native_schedules(name):
    """The five public rows, each as (exact steps, exp64 steps)."""
    exponent, prime, secret_norm = signer_parameters(name)
    secret_bound = q4_norm_bound(prime)
    degree_bound = challenge_degree_bound(exponent)
    rows = {
        "Signing, generic HNF": generic_bounds(secret_bound*degree_bound, prime),
        f"Signing, pre-reduced, guard {PREREDUCED_NORM_GUARD}": guarded_signing_bounds(
            PREREDUCED_NORM_GUARD, degree_bound, secret_bound, prime),
        "Key generation, norm D": generic_bounds(secret_norm, prime),
        "Secret ideal pre-reduction, Q4": generic_bounds(secret_bound, prime),
        "Challenge alone": generic_bounds(degree_bound, prime),
    }
    return {label: schedule(bounds) for label, bounds in rows.items()}


class Stage2cOracle(unittest.TestCase):
    def assert_window_exact(self, h0, h1, norm):
        exponent = max(h0.bit_length(), h1.bit_length()) - norm.bit_length()
        self.assertLessEqual(exponent, QUOTIENT_WINDOW, (h0, h1, norm))
        self.assertEqual(exp64_quotients(h0, h1, norm),
                         (half_up_quotient(h0, norm), half_up_quotient(h1, norm)),
                         (h0, h1, norm))

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
            self.assertEqual(value*4*ideal_norm, a*a + b*b + p*(k*k + d*d))

    def test_class_bounds_are_the_reduced_norm_inputs(self):
        # The cancelled numerals are the test's, not the schedule's.
        prime, norm = 19, 5
        generic = generic_bounds(norm, prime)
        self.assertEqual(generic.a0_squared, norm**4)
        self.assertEqual(generic.delta, Fraction(prime*norm*norm, 4))
        self.assertEqual(generic.c0, 2*norm*norm + prime)
        self.assertEqual(generic.contraction_ratio(), Fraction(16*norm*norm, prime))

        guard, degree, secret = 4, 8, 16
        guarded = guarded_signing_bounds(guard, degree, secret, prime)
        self.assertEqual(guarded.a0_squared, guard*guard*prime*secret*secret*degree**4)
        self.assertEqual(guarded.delta, Fraction(prime*(secret*degree)**2, 4))
        self.assertEqual(guarded.c0, (degree + 1)**2*guard*ceil_sqrt(prime)*secret)
        self.assertEqual(guarded.contraction_ratio(), 16*guard*guard*degree*degree)

    def test_exp64_window_is_half_up(self):
        # No truncation: the correction must not move an already exact quotient.
        bits = 6
        for norm in range(1, 1 << bits):
            for h0 in range(1 - (1 << bits), 1 << bits):
                for h1 in range(1 - (1 << bits), 1 << bits):
                    self.assert_window_exact(h0, h1, norm)
        # The top 64 bits are kept. The low bits are discarded and then corrected.
        tops = (1 << (QUOTIENT_TOP_BITS - 1), (1 << QUOTIENT_TOP_BITS) - 1)
        for dropped in (1, 2, 7):
            for top in tops:
                for low in (0, 1, 1 << (dropped - 1), (1 << dropped) - 1):
                    norm = (top << dropped) + low
                    self.assertEqual(norm.bit_length() - QUOTIENT_TOP_BITS, dropped)
                    for quotient in (-2, -1, 0, 1, 2):
                        for offset in (0, 1, -1, norm//2, -(norm//2), norm//2 - 1):
                            dividend = quotient*norm + offset
                            self.assert_window_exact(dividend, dividend//3, norm)
                            self.assert_window_exact(-dividend, 0, norm)

    def test_exp64_large_quotient_progress(self):
        norm = (1 << 130) + 15
        for offset in (QUOTIENT_WINDOW, QUOTIENT_WINDOW + 1, 119, 640):
            for h0, h1 in ((norm << offset, -norm//3), (1 - norm, -(norm << offset))):
                quotients = exp64_quotients(h0, h1, norm)
                if offset == QUOTIENT_WINDOW:
                    self.assertEqual(quotients, (half_up_quotient(h0, norm),
                                                 half_up_quotient(h1, norm)))
                    continue
                r0, r1 = h0 - quotients[0]*norm, h1 - quotients[1]*norm
                self.assertGreater(max(abs(r0), abs(r1)), norm)
                self.assertLess(r0*r0 + r1*r1, h0*h0 + h1*h1)
                new_exponent = max(r0.bit_length(), r1.bit_length()) - norm.bit_length()
                self.assertLessEqual(new_exponent, offset - QUOTIENT_PROGRESS_BITS)

    def test_native_schedule_table(self):
        sets = ("p248", "p324", "p376", "p500", "p664")
        schedules = [native_schedules(name) for name in sets]
        spec = (ROOT/"docs/SIGNER_SPEC.md").read_text()
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
        with self.assertRaises(ValueError):
            exp64_quotients(1, 1, 0)

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
        fixture = json.loads((ROOT/"scripts/fixtures/stage2c_p500.json").read_text())
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
