#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Regenerate the verifier corpus with public fixture RNG streams.

Oracle: pinned independent C isogeny implementation, checked by Rust.
These are algebraic-verifier vectors, not production master-seed KATs.
The original fixture RNG streams are preserved where provenance exists.
The explicit decision flag records the user's approved salt brief.
"""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PARAMS = {"NIST-I": (1, 248, 5, 32), "NIST-III": (3, 376, 65, 48), "NIST-V": (5, 500, 27, 64)}
FIELDS = ("E", "Px", "Py", "Qx", "Qy")


def rows(text):
    return dict(line.split(" ", 1) for line in text.splitlines() if " " in line)


def normalized(data, level):
    _, e, odd, width = PARAMS[level]
    p = (odd << e) - 1
    def parts(s):
        raw = bytes.fromhex(s)
        return tuple(int.from_bytes(raw[i:i+width], "little") for i in (0, width))
    y = parts(data["Py"])
    if y > tuple((-v) % p for v in y):
        for name in ("Py", "Qy"):
            data[name] = b"".join(((-v) % p).to_bytes(width, "little") for v in parts(data[name])).hex()
    return data


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--decision", required=True, choices=["salt-brief-2026-11-12"])
    ap.add_argument("--backend-dir", type=Path, default=ROOT / "target/native")
    args = ap.parse_args()
    manifest = json.loads((ROOT / "scripts/fixtures/salt_seed_manifest.json").read_text())
    if len(manifest) != 113:
        raise SystemExit("manifest must cover all 113 archived vectors")
    report = []
    for entry in manifest:
        if entry["keygen_stream_seed"] is None:
            raise SystemExit(f"missing original seed recipe: {entry['file']}")
        archived = ROOT / "prism-compare/testdata/unsalted" / entry["file"]
        old = rows(archived.read_text())
        level = entry["level"]
        n, _, _, _ = PARAMS[level]
        binary = args.backend_dir / f"export_s5_lvl{n}"
        seed = bytes.fromhex(entry["keygen_stream_seed"])
        sign_seed = bytes.fromhex(entry["sign_stream_seed"])
        with tempfile.TemporaryDirectory(prefix="prism-public-fixture-") as scratch:
            keyfile = Path(scratch) / "secret"
            key = subprocess.run([str(binary), "keygen", str(keyfile)], input=seed, capture_output=True, check=True, timeout=60)
            key_rows = rows(key.stdout.decode())
            if key_rows["A"] != old["A"] and not entry.get("replacement", False):
                raise SystemExit(f"recovered seed does not reproduce archived key: {entry['file']}")
            pk = key_rows["A"]
            if not entry.get("replacement", False):
                replay = subprocess.run([str(binary), "sign", str(keyfile), old["s"]], input=sign_seed, capture_output=True, check=True, timeout=60)
                original = normalized(rows(replay.stdout.decode()), level)
                if any(original[k] != old[k] for k in FIELDS):
                    raise SystemExit(f"recovered signing stream does not reproduce archived response: {entry['file']}")
            primes = errors = rejected = 0
            accepted = None
            for block in range(0, 65536, 512):
                candidates = subprocess.run([str(ROOT / "target/release/fixture_challenges"), level, pk, seed.hex(), str(block)], text=True, capture_output=True, check=True).stdout
                for line in candidates.splitlines():
                    counter, salt, scalar_hex, prime = line.split()
                    if prime != "true":
                        continue
                    if primes == 16:
                        raise SystemExit(f"prime bound exhausted: {entry['file']}")
                    primes += 1
                    scalar = int.from_bytes(bytes.fromhex(scalar_hex), "little")
                    answer = subprocess.run([str(binary), "sign", str(keyfile), str(scalar)], input=sign_seed, capture_output=True, timeout=60)
                    if answer.returncode:
                        errors += 1
                        continue
                    data = normalized(rows(answer.stdout.decode()), level)
                    signature = b"".join(bytes.fromhex(data[k]) for k in FIELDS) + bytes.fromhex(salt)
                    record = f"{level} {pk} {old['message'].encode().hex()} {signature.hex()}\n"
                    verdict = subprocess.run([str(ROOT / "target/release/verify_bytes")], input=record, text=True, capture_output=True, check=True).stdout.strip()
                    if verdict != "Ok":
                        rejected += 1
                        continue
                    q = (1 << (2 * (PARAMS[level][1] - 2) - 1)) - scalar * scalar
                    output = {"level": level, "message": old["message"], "salt": salt, "s": scalar, "q": q, "A": pk}
                    output.update({k: data[k] for k in FIELDS + ("VkPx", "VkPy", "VkQx", "VkQy", "VkDx")})
                    output.update(oracle="pinned-C-fixture-stream-checked-by-Rust", decision=args.decision)
                    accepted = "".join(f"{k} {v}\n" for k, v in output.items())
                    report.append(dict(file=entry["file"], salt_counter=int(counter), prime_candidates=primes, backend_errors=errors, verification_rejections=rejected))
                    break
                if accepted is not None:
                    break
            if accepted is None:
                raise SystemExit(f"counter bound exhausted: {entry['file']}")
            target = ROOT / "prism-verify/testdata" / entry["file"]
            target.write_text(accepted)
            print(f"accepted {entry['file']} counter={counter} primes={primes} backend_errors={errors} rejected={rejected}", flush=True)
    (ROOT / "docs/comparison/salted-vector-generation.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"regenerated {len(report)}/113 vectors", flush=True)


if __name__ == "__main__":
    main()
