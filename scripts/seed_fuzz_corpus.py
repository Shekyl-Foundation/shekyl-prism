#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Seed every fuzz target with inputs that reach the successful verifier."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
base = ROOT / "prism-verify/testdata"
paths = sorted((base / "accept").glob("*.txt")) + sorted(base.glob("s5_lvl*.txt"))
assert len(paths) == 113, "review the corpus when fixtures change"
for path in paths:
    rows = dict(line.split(" ", 1) for line in path.read_text().splitlines() if " " in line)
    key = bytes.fromhex(rows["A"])
    signature = b"".join(bytes.fromhex(rows[name]) for name in ("E", "Px", "Py", "Qx", "Qy")) + bytes.fromhex(rows["salt"])
    message = rows["message"].encode()
    level = {"NIST-I": 1, "NIST-III": 3, "NIST-V": 5}[rows["level"]]
    wire = len(key).to_bytes(2, "little") + len(signature).to_bytes(2, "little") + key + signature + message
    # A signature is field encodings end to end, which is what field_square reads.
    for target, data in [(f"verify_level_{level}", wire), ("decode_public_key", key), ("decode_signature", signature), ("field_square", signature)]:
        directory = ROOT / "fuzz/corpus" / target
        directory.mkdir(parents=True, exist_ok=True)
        (directory / path.stem).write_bytes(data)
print(f"seeded {len(paths)} honest signatures")
