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
        # This bites against a group law that returns infinity on a partial
        # collision. It does NOT cover a Z that is zero on one factor only.
        with self.assertRaisesRegex(ValueError, "mixed addition"):
            proofs.ec_add((0, 1, 1), (0, 6, 1), 1, 35)
        with self.assertRaisesRegex(proofs.ImproperResidue, "proper factor"):
            proofs.ec_double((1, 5, 1), 1, 35)

    def test_group_law_rejects_a_partial_zero_coordinate(self):
        # Z is 0 mod 5 and a unit mod 7. Doubling must not treat the point as finite.
        # This bites against `z % n == 0` as the only infinity test. It does NOT
        # cover the sum of two finite points.
        with self.assertRaisesRegex(proofs.ImproperResidue, "proper factor"):
            proofs.ec_double((1, 1, 7), 1, 35)
        self.assertEqual(proofs.ec_double((1, 1, 0), 1, 35), proofs.POINT_AT_INFINITY)

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

    def test_trial_band_decides_without_a_pratt_search(self):
        # 2047 = 23 · 89 sits below the trial bound, so a Fermat test is not the proof.
        # This bites against sending every odd integer to Pratt. It does NOT cover
        # an integer at or above LEAF_TRIAL_LIMIT.
        self.assertIs(proofs.classify_leaf(97), proofs.LeafKind.PRIME)
        self.assertIs(proofs.classify_leaf(2047), proofs.LeafKind.COMPOSITE)

    def test_pratt_band_names_a_composite_pseudoprime(self):
        # 1373653 = 829 · 1657 is a base-2 Fermat pseudoprime above the trial bound.
        # This bites against treating a passed base-2 test as prime. It does NOT
        # cover a search that finds no witness.
        self.assertIs(proofs.classify_leaf(1_373_653), proofs.LeafKind.COMPOSITE)
        with self.assertRaisesRegex(ValueError, "is composite"):
            proofs.require_proven_prime(1_373_653)

    def test_unfinished_pratt_search_is_not_split(self):
        # cofactor-p324 leaf. Its smallest Pratt witness is 17, so a limit of 17
        # stops before that base. Pollard on this prime would raise "no proper
        # divisor"; the search must raise LeafSearchIncomplete instead.
        # This bites against splitting an undecided integer. It does NOT cover
        # a composite that fails a Fermat test.
        prime = 72_368_493_649
        self.assertIs(proofs.classify_leaf(prime), proofs.LeafKind.PRIME)
        self.assertIs(proofs.classify_leaf(prime, witness_limit=18), proofs.LeafKind.PRIME)
        with self.assertRaises(proofs.LeafSearchIncomplete):
            proofs.classify_leaf(prime, witness_limit=17)

    def test_pratt_quotient_refuses_an_inexact_division(self):
        # This bites against floor division in the order check. It does NOT
        # cover a factorisation that merely omits a prime.
        self.assertEqual(proofs.pratt_quotient(15, 5), 3)
        with self.assertRaisesRegex(ValueError, "does not divide"):
            proofs.pratt_quotient(15, 4)


if __name__ == "__main__":
    unittest.main()
