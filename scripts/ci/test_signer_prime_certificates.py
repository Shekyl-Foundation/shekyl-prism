#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Regression cases for the fixed-prime certificate gate."""

import unittest

import check_signer_prime_certificates as proofs


class CertificateTests(unittest.TestCase):
    def test_ecpp_requires_the_certified_point_order(self):
        # PARI manual, primecert(nextprime(2^64)).
        n = 18446744073709551629
        step = dict(N=n, t=-8423788454, s=160388, a=1,
                    P=[10598342506117936052, 2225259013356795550])
        proofs.verify_ecpp(n, [step])
        with self.assertRaisesRegex(ValueError, "order q"):
            proofs.verify_ecpp(n, [dict(step, P=[0, 0])])

    def test_group_law_rejects_mixed_reductions(self):
        # On y^2 = x^3 + x + 1 mod 35 these points coincide mod 5
        # and are opposites mod 7. Their sum cannot be declared infinity.
        with self.assertRaisesRegex(ValueError, "mixed addition"):
            proofs.ec_add((0, 1, 1), (0, 6, 1), 1, 35)
        with self.assertRaisesRegex(ValueError, "noninvertible doubling"):
            proofs.ec_double((1, 5, 1), 1, 35)

    def test_lucas_requires_the_complete_factorization(self):
        cert = proofs.certificate("field-p248", "np1")
        n = int(cert["N"])
        proofs.verify_lucas(n, cert)
        del cert["np1_factors"]["5"]
        with self.assertRaisesRegex(ValueError, "factorization"):
            proofs.verify_lucas(n, cert)

    def test_gap_rejects_a_later_prime(self):
        proofs.verify_gap(37, 5)
        with self.assertRaisesRegex(ValueError, "no composite witness"):
            proofs.verify_gap(41, 5)

    def test_atkin_bound_is_strict(self):
        self.assertFalse(proofs.exceeds_atkin_bound(9, 16))
        self.assertTrue(proofs.exceeds_atkin_bound(10, 16))


if __name__ == "__main__":
    unittest.main()
