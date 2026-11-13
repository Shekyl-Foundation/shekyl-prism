#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""E3 at projection widths 16 and 20, as a regression on the real backend.

    check_replay_regression.py <replay-binary> <backend-dir>

Trial 0 is deterministic for the pinned backend. At both widths the
collision replay must be accepted by the archived derivation, the
precomputed collision must be refused by the salted one, the second
preimage onto the signed scalar must be accepted at the reduced width, and
the default full-width verifier must refuse every forgery.
"""
import json
import subprocess
import sys

EXPECTED = {
    "unsalted_accepted": True,
    "salted_precomputed_replay_accepted": False,
    "second_preimage_accepted": True,
    "default_verifier_accepted": False,
}


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    run = subprocess.run(
        [sys.argv[1], sys.argv[2], "0", "ci"], text=True, capture_output=True
    )
    if run.returncode:
        raise SystemExit(f"replay exited {run.returncode}:\n{run.stderr}")
    records = [
        json.loads(line[len("RESULT "):])
        for line in run.stdout.splitlines()
        if line.startswith("RESULT ")
    ]
    widths = [record.get("projection_bits") for record in records]
    if widths != [16, 20]:
        raise SystemExit(f"expected one record at widths 16 and 20, found {widths}")
    for record in records:
        width = record["projection_bits"]
        for field, expected in EXPECTED.items():
            if record.get(field) is not expected:
                raise SystemExit(
                    f"width {width}: {field} is {record.get(field)!r}, expected {expected!r}"
                )
        if record["actual_salt"] == record["guessed_salt"]:
            raise SystemExit(f"width {width}: the signer used the guessed salt")
        print(
            f"width {width}: unsalted replay accepted after "
            f"{record['unsalted_queries']} hashes; precomputed salted collision "
            f"refused; second preimage accepted after "
            f"{record['second_preimage_queries']} hashes"
        )


if __name__ == "__main__":
    main()
