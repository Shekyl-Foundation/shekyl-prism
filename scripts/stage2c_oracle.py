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
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
PARAMETER_SETS = ("p248", "p324", "p376", "p500", "p664")
SCHEDULE_HEADER = "| Native input class |"

# Independent review, split at lambda=2^-9. Bands are ordered from
# the tail toward the main phase; each pair is (transition cap, upper
# ratio in billionths). All remaining transitions use the main ratio.
BANDS = ((1, 289625053), (1, 275566686), (1, 273153948), (1, 272906433),
         (1, 268713095), (2, 268605695), (3, 268525146))
MAIN_RATIO = Fraction(537, 2000)
MAX_POTENTIAL = Fraction(7998092957, 1 << 32)
TAIL_THRESHOLD = 1 << 9
# One boundary transition, up to four tail swaps, then final size reduction.
TAIL_SLOTS = 1 + 4 + 1

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
    c0: Fraction

    def first_ratio(self):
        return self.a0_squared / self.delta

    def second_ratio(self):
        return self.c0**2 / self.delta


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


def generic_bounds(ideal_norm, prime):
    """A0 <= ideal_norm^2, Delta = p*ideal_norm^2/4, C0 <= 2*ideal_norm^2 + p/4.

    ideal_norm is the class bound on the ideal norm T: Nmax*qmax, D, Nmax or qmax.
    """
    return ClassBounds(ideal_norm**4, Fraction(prime*ideal_norm*ideal_norm, 4),
                       2*ideal_norm*ideal_norm + Fraction(prime, 4))


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

    Section 2.3: X = 2*dividend + divisor - 2*estimate*divisor.
    Decrement when X < 0. Increment when X >= 2*divisor.
    The in-window exp64 estimate is at most one away from the half-up integer.
    """
    x = 2*dividend + divisor - 2*estimate*divisor
    if x < 0:
        return estimate - 1
    if x >= 2*divisor:
        return estimate + 1
    return estimate


def half_up_quotients(h0, h1, norm):
    """The two exact half-up components of one Gaussian step."""
    return half_up_quotient(h0, norm), half_up_quotient(h1, norm)


def order_generators(p, u, v):
    """Initial strict order: swap precisely when N4(V) < N4(U)."""
    if inner(p, v, v) < inner(p, u, u):
        return v, u
    return u, v


def gaussian_update(p, u, v, quotients):
    """Replace V by V - m0*U - m1*J(U). Does not swap.

    quotients(h0, h1, norm) returns the two integers m0, m1.
    """
    norm = inner(p, u, u)
    if norm <= 0:
        raise ValueError("the Gaussian generators must be independent")
    ju = times_i(u)
    real, imag = quotients(inner(p, v, u), inner(p, v, ju), norm)
    return combine((v, u, ju), (1, -real, -imag))


def gaussian_step(p, u, v, quotients):
    """One size reduction and a strict swap. Returns (u, v, swapped)."""
    norm = inner(p, u, u)
    reduced = gaussian_update(p, u, v, quotients)
    if inner(p, reduced, reduced) < norm:
        return reduced, u, True
    return u, reduced, False


def reduce_steps(p, u, v, quotients, steps):
    """Initial strict order, then exactly `steps` Gaussian steps."""
    u, v = order_generators(p, u, v)
    for _ in range(steps):
        u, v, _swapped = gaussian_step(p, u, v, quotients)
    return u, v


def reduce_exact(p, u, v):
    """At most A0 steps: each swap lowers the positive integral first norm."""
    u, v = order_generators(p, u, v)
    first_norm = inner(p, u, u)
    while first_norm > 0:
        u, v, swapped = gaussian_step(p, u, v, half_up_quotients)
        if not swapped:
            return u, v
        next_norm = inner(p, u, u)
        if next_norm >= first_norm:
            raise ArithmeticError("exact Gaussian swap must decrease the first norm")
        first_norm = next_norm
    raise ValueError("the Gaussian generators must be independent")


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
    """e, the prime, and the secret ideal norm. One row of each, or a fault."""
    path = ROOT/"prism-vectors"/"data"/"signer"/(name + ".txt")
    wanted = ("e", "f", "secret_norm")
    found = {key: [] for key in wanted}
    for line in path.read_text().splitlines():
        key, _, value = line.partition(" ")
        if key in found:
            found[key].append(value.split()[0] if value else "")
    values = {}
    for key, rows in found.items():
        if len(rows) != 1 or rows[0] == "":
            raise ValueError(f"{name}: {len(rows)} rows `{key}`")
        values[key] = rows[0]
    exponent = int(values["e"])
    prime = (int(values["f"]) << exponent) - 1
    secret_norm = int.from_bytes(bytes.fromhex(values["secret_norm"]), "little")
    return exponent, prime, secret_norm


def schedule_rows(spec):
    """The one native schedule table. A second header is a fault."""
    lines = spec.splitlines()
    starts = [index for index, line in enumerate(lines) if line.startswith(SCHEDULE_HEADER)]
    if len(starts) != 1:
        raise ValueError(f"SIGNER_SPEC has {len(starts)} native schedule tables")
    header = [cell.strip() for cell in lines[starts[0]].strip().strip("|").split("|")]
    sets = header[1:]
    rows = []
    for line in lines[starts[0] + 2:]:
        if not line.startswith("|"):
            break
        rows.append([cell.strip() for cell in line.strip().strip("|").split("|")])
    return sets, rows


def main_steps(ratio):
    """Largest main-phase transition count still allowed by the potential."""
    remaining = MAX_POTENTIAL**2 * ratio
    count = 0
    for cap, numerator in BANDS:
        squared = Fraction(numerator, 10**9)**2
        for _ in range(cap):
            remaining *= squared
            if remaining < TAIL_THRESHOLD:
                return count
            count += 1
    # The ratio is now constant. Find the last admissible power exactly.
    lo, hi = 0, 1
    while remaining * MAIN_RATIO**(2*hi) >= TAIL_THRESHOLD:
        lo, hi = hi, 2*hi
    while hi-lo > 1:
        mid = (lo+hi)//2
        if remaining * MAIN_RATIO**(2*mid) >= TAIL_THRESHOLD:
            lo = mid
        else:
            hi = mid
    return count + lo


def schedule(bounds, minimum_norm_bounds=None):
    """Account for each feasible number of initial big substeps, even on a tail-only run."""
    r0 = bounds.first_ratio()
    rc = bounds.second_ratio()
    if minimum_norm_bounds is not None:
        rc = max(rc, minimum_norm_bounds.second_ratio())
    exact = main_steps(r0) + TAIL_SLOTS
    exp64 = exact
    k = 1
    while 4**(QUOTIENT_PROGRESS_BITS*k + 3) < rc:
        head_ratio = rc / 16**(QUOTIENT_PROGRESS_BITS*k + 2)
        exp64 = max(exp64, k + main_steps(min(r0, head_ratio)) + TAIL_SLOTS)
        k += 1
    return exact, exp64


def native_schedules(name):
    """The five public rows, each as (exact steps, exp64 steps)."""
    exponent, prime, secret_norm = signer_parameters(name)
    secret_bound = q4_norm_bound(prime)
    degree_bound = challenge_degree_bound(exponent)
    low = generic_bounds(1, prime)
    return {
        "Signing, generic HNF": schedule(generic_bounds(secret_bound*degree_bound, prime), low),
        f"Signing, pre-reduced, guard {PREREDUCED_NORM_GUARD}": schedule(guarded_signing_bounds(
            PREREDUCED_NORM_GUARD, degree_bound, secret_bound, prime)),
        "Key generation, norm D": schedule(generic_bounds(secret_norm, prime)),
        "Secret ideal pre-reduction, Q4": schedule(generic_bounds(secret_bound, prime), low),
        "Challenge alone": schedule(generic_bounds(degree_bound, prime), low),
    }


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
        # The expected ratios are the cancelled forms of A0^2/Delta.
        prime, ideal_norm = 19, 5
        generic = generic_bounds(ideal_norm, prime)
        self.assertEqual(generic.a0_squared, ideal_norm**4)
        self.assertEqual(generic.delta, Fraction(prime*ideal_norm*ideal_norm, 4))
        self.assertEqual(generic.c0, 2*ideal_norm*ideal_norm + Fraction(prime, 4))
        self.assertEqual(generic.first_ratio(), Fraction(4*ideal_norm*ideal_norm, prime))

        guard, degree, secret = 4, 8, 16
        guarded = guarded_signing_bounds(guard, degree, secret, prime)
        self.assertEqual(guarded.a0_squared, guard*guard*prime*secret*secret*degree**4)
        self.assertEqual(guarded.delta, Fraction(prime*(secret*degree)**2, 4))
        self.assertEqual(guarded.c0, (degree + 1)**2*guard*ceil_sqrt(prime)*secret)
        self.assertEqual(guarded.first_ratio(), 4*guard*guard*degree*degree)

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

    def test_big_first_step_can_go_straight_to_the_tail(self):
        # At equality a big head is excluded. Immediately above it, the
        # remaining chain can be empty and the head still takes one slot.
        at_cutoff = ClassBounds(1, Fraction(1), Fraction(2**61))
        above = at_cutoff._replace(c0=at_cutoff.c0 + 1)
        self.assertEqual(schedule(at_cutoff), (TAIL_SLOTS, TAIL_SLOTS))
        self.assertEqual(schedule(above), (TAIL_SLOTS, TAIL_SLOTS + 1))

    def test_native_schedule_table(self):
        schedules = [native_schedules(name) for name in PARAMETER_SETS]
        sets, rows = schedule_rows((ROOT/"docs/SIGNER_SPEC.md").read_text())
        self.assertEqual(sets, [name.upper() for name in PARAMETER_SETS])
        self.assertCountEqual([row[0] for row in rows], schedules[0])
        for label, *counts in rows:
            self.assertEqual(len(counts), len(PARAMETER_SETS), label)
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
        first = gaussian_update(3, u, v, half_up_quotients)
        self.assertEqual(first, (4, -1, 1, 0))
        a = inner(3, first, first)
        self.assertLess(a, inner(3, u, u))
        # After swapping, the first vector is already shortest. The imaginary
        # projection of the second is exactly +1/2, outside the terminal range.
        self.assertEqual(2*inner(3, u, times_i(first)), a)
        last = gaussian_update(3, first, u, half_up_quotients)
        self.assertEqual(last, (1, -2, -2, -1))
        self.assertEqual(inner(3, last, last), a)
        self.assertEqual(self.check_reduction(3, u, v), (first, last))

    def test_strict_descent_allows_slow_swaps(self):
        # Three consecutive swaps. Two can leave 96% of the earlier norm.
        p = 3
        start = ((25, -5, 4, 0), (60, -10, 8, 0))
        u, v = start
        for expected in (100, 98, 96):
            u, v, swapped = gaussian_step(p, u, v, half_up_quotients)
            self.assertTrue(swapped)
            self.assertEqual(inner(p, u, u), expected)
        self.assertEqual(self.check_reduction(p, *start),
                         ((0, 0, 4, -4), (-5, -5, -4, 0)))

        # A faulty step reporting a swap without any decrease must fail promptly.
        with patch(__name__ + ".gaussian_step", return_value=(*start, True)):
            with self.assertRaisesRegex(ArithmeticError, "must decrease"):
                reduce_exact(p, *start)

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
        for name in PARAMETER_SETS:
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
                start = ((scale, 0, 0, 0), (a, b, 1, 0))
                pair = self.check_reduction(p, *start)
                self.assertEqual(inner(p, pair[0], pair[0]), int(fixture[key], 16))
                exact_steps, exp64_steps = native_schedules("p500")["Signing, generic HNF"]
                self.assertEqual(reduce_steps(p, *start, half_up_quotients, exact_steps), pair)
                self.assertEqual(reduce_steps(p, *start, exp64_quotients, exp64_steps), pair)
                if key == "o0_numerator_minimum":
                    self.check_form(p, pair, t)


if __name__ == "__main__":
    unittest.main()
