#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Proposal only: wrapper for the pinned-certificate verifier.

This file is not wired into CI. If the prime-certificate experiment is
adopted on `dev`, it would live at `scripts/ci/check_signer_prime_certificates.py`
and would call the verifier after the certificate files have been moved
to a production path. Today it runs the experiment's verifier in place,
so the proposed nightly command can be tried from this branch:

    python3 research/prime-certificates/proposed-ci/check_signer_prime_certificates.py
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
VERIFY_DIR = HERE.parent
sys.path.insert(0, str(VERIFY_DIR))
import verify  # noqa: E402


def main() -> int:
    return verify.main()


if __name__ == "__main__":
    sys.exit(main())
