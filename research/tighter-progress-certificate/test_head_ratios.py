# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Regression for a large first quotient when A²/Delta is below one."""
from fractions import Fraction
import unittest

from counts import head_ratios
from variants import q_exp64


class HeadRatiosTest(unittest.TestCase):
    def test_first_large_quotient_before_a_tail_only_run(self):
        # Positive definite Gram at equality in C²/Delta >= 4|h/A|².
        a, h = 2, 2**63
        c = h**2
        delta = a*c - h**2
        r0, rc = Fraction(a*a, delta), Fraction(c*c, delta)
        self.assertLess(r0, 1)
        self.assertTrue(q_exp64(h, 0, a)[2])
        self.assertEqual(list(head_ratios(r0, rc)), [(1, r0)])

    def test_strict_b2_threshold(self):
        threshold = 4**61
        self.assertEqual(list(head_ratios(1, threshold)), [])
        self.assertEqual([k for k, _ in head_ratios(1, threshold + 1)], [1])


if __name__ == "__main__":
    unittest.main()
