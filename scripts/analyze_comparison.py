#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Tables of docs/SALT_COMPARISON.md from the raw records in docs/comparison.

    python3 scripts/analyze_comparison.py            # print every table
    python3 scripts/analyze_comparison.py --write    # also rewrite e3/plot_data.tsv

Reads only committed raw output: e1/density.txt, e3/trial-*.txt,
e5/index_leak.txt, e6/level*.txt and e6/key-*.txt. Nothing here is measured; every
figure printed is arithmetic on those records. A missing or short record
is an error, never a silently smaller table.
"""
import json
import math
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "docs" / "comparison"
LEVELS = {"NIST-I": ("I", 128, 32), "NIST-III": ("III", 192, 48), "NIST-V": ("V", 256, 64)}
WIDTHS = [16, 20, 24, 28, 32]
TRIALS = 20
SIGNATURES_LOG2 = 64


def fields(line):
    return dict(part.split("=", 1) for part in line.split()[1:])


def density():
    rows = {}
    for line in (ROOT / "e1/density.txt").read_text().splitlines():
        if line.startswith("DENSITY "):
            row = fields(line)
            rows[(row["level"], row["hash"])] = row
    if len(rows) != 6:
        raise SystemExit(f"e1: expected six density rows, found {len(rows)}")
    print("E1 density")
    print("| Row | Hash | Samples | Prime degrees | Density | 95% interval |")
    print("| --- | --- | ---: | ---: | ---: | --- |")
    pooled = {}
    for (level, version), row in sorted(rows.items(), key=lambda kv: (LEVELS[kv[0][0]][1], kv[0][1])):
        n, k = int(row["samples"]), int(row["primes"])
        low, high = wilson(k, n)
        print(f"| {LEVELS[level][0]} | {version} | {n} | {k} | {100 * k / n:.3f}% | {100 * low:.3f}% to {100 * high:.3f}% |")
        total = pooled.setdefault(level, [0, 0, int(row["free_bits"])])
        total[0] += k
        total[1] += n
    print()
    print("E1 agreement of the two hash versions (two-proportion z)")
    for level in LEVELS:
        a, b = rows[(level, "challenge-v1-unsalted")], rows[(level, "challenge-v2-salted")]
        ka, na, kb, nb = int(a["primes"]), int(a["samples"]), int(b["primes"]), int(b["samples"])
        p = (ka + kb) / (na + nb)
        z = (ka / na - kb / nb) / math.sqrt(p * (1 - p) * (1 / na + 1 / nb))
        print(f"  {LEVELS[level][0]}: z = {z:+.2f}")
    print()
    print("E2 full-width cost (derived from E1; hash queries, log2)")
    print("| Row | Free bits b | Pooled density | Unsalted collision replay | Salted, one signed target | Salted, 2^64 signed targets | Salt bits k | Bound per unit cost beyond Problem 3 |")
    print("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    for level, (k, n, b) in pooled.items():
        name, _, salt_len = LEVELS[level]
        rho = k / n
        unsalted = (b - math.log2(rho)) / 2
        salt_bits = 8 * salt_len
        first = SIGNATURES_LOG2 - b
        second = SIGNATURES_LOG2 - math.log2(rho) - (salt_bits - 1)
        bound = math.log2(2.0 ** (first - first) + 2.0 ** (second - first)) + first
        print(
            f"| {name} | {b} | {100 * rho:.3f}% | 2^{unsalted:.1f} | 2^{b} | 2^{b - SIGNATURES_LOG2} "
            f"| {salt_bits} | 2^{first} + 2^{second:.1f} = 2^{bound:.2f} |"
        )
    print()


def wilson(k, n, z=1.96):
    p = k / n
    centre = (p + z * z / (2 * n)) / (1 + z * z / n)
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / (1 + z * z / n)
    return centre - half, centre + half


def fit(points):
    n = len(points)
    mx = sum(x for x, _ in points) / n
    my = sum(y for _, y in points) / n
    sxx = sum((x - mx) ** 2 for x, _ in points)
    slope = sum((x - mx) * (y - my) for x, y in points) / sxx
    intercept = my - slope * mx
    residual = sum((y - intercept - slope * x) ** 2 for x, y in points)
    return slope, intercept, math.sqrt(residual / (n - 2) / sxx)


def replay(write):
    records = {}
    for trial in range(TRIALS):
        path = ROOT / f"e3/trial-{trial:02d}.txt"
        for line in path.read_text().splitlines():
            if line.startswith("RESULT "):
                record = json.loads(line[len("RESULT "):])
                if record["trial"] != trial:
                    raise SystemExit(f"{path.name} holds trial {record['trial']}")
                records[(record["projection_bits"], trial)] = record
    missing = [(w, t) for w in WIDTHS for t in range(TRIALS) if (w, t) not in records]
    if missing or len(records) != len(WIDTHS) * TRIALS:
        raise SystemExit(f"e3: missing or extra records, first missing {missing[:3]}")
    print("E3 reduced-width forgery, level I, real signer and verifier")
    print("| Width w | Free bits | Unsalted replays accepted | Unsalted hash queries, geometric mean (min to max) | Precomputed collisions accepted, salted | Second-preimage hash queries, geometric mean (min to max) | Second preimages accepted | Default verifier accepted |")
    print("| ---: | ---: | ---: | --- | ---: | --- | ---: | ---: |")
    lines = ["width\tfree_bits\ttrial\tunsalted_queries\tunsalted_accepted\tsalted_precomputed_replay_accepted\tsecond_preimage_queries\tsecond_preimage_accepted\tsalted_backend_errors\tsalted_rejections"]
    unsalted_points, salted_points = [], []
    backend = rejections = unsignable = 0
    for width in WIDTHS:
        rows = [records[(width, t)] for t in range(TRIALS)]
        old = [r["unsalted_queries"] for r in rows]
        new = [r["second_preimage_queries"] for r in rows]
        geo = lambda xs: 2 ** (sum(math.log2(x) for x in xs) / len(xs))
        print(
            f"| {width} | {width - 1} | {sum(r['unsalted_accepted'] for r in rows)}/{TRIALS} "
            f"| {geo(old):.0f} ({min(old)} to {max(old)}) "
            f"| {sum(r['salted_precomputed_replay_accepted'] for r in rows)}/{TRIALS} "
            f"| {geo(new):.0f} ({min(new)} to {max(new)}) "
            f"| {sum(r['second_preimage_accepted'] for r in rows)}/{TRIALS} "
            f"| {sum(r['default_verifier_accepted'] for r in rows)}/{TRIALS} |"
        )
        for r in rows:
            unsalted_points.append((width, math.log2(r["unsalted_queries"])))
            salted_points.append((width, math.log2(r["second_preimage_queries"])))
            backend += r["salted_backend_errors"] + r["salted_malformed"]
            rejections += r["salted_rejections"]
            unsignable += r["unsalted_unsignable_collisions"]
            lines.append("\t".join(str(x) for x in (
                width, width - 1, r["trial"], r["unsalted_queries"], int(r["unsalted_accepted"]),
                int(r["salted_precomputed_replay_accepted"]), r["second_preimage_queries"],
                int(r["second_preimage_accepted"]), r["salted_backend_errors"] + r["salted_malformed"],
                r["salted_rejections"])))
    print()
    for name, points in (("unsalted collision replay", unsalted_points), ("salted second preimage", salted_points)):
        slope, intercept, error = fit(points)
        print(f"  fit of log2(queries) on w, {name}: slope {slope:.3f} (standard error {error:.3f}), intercept {intercept:.2f}, {len(points)} points")
    print(f"  salted signing calls: {len(records)}; backend or malformed results that advanced the counter: {backend}; refused responses: {rejections}")
    print(f"  unsalted collisions the archived signer could not sign as planned: {unsignable}")
    print()
    if write:
        (ROOT / "e3/plot_data.tsv").write_text("\n".join(lines) + "\n")


def index_leak():
    counts, total = {}, None
    carried = []
    for line in (ROOT / "e5/index_leak.txt").read_text().splitlines():
        if line.startswith("SKIPPED "):
            row = fields(line)
            counts[int(row["earlier_prime"])] = int(row["vectors"])
        elif line.startswith("TOTAL "):
            total = int(fields(line)["vectors"])
        elif line.startswith("INDEX "):
            carried.append(int(fields(line)["carried_index"]))
    if total != 113 or sum(counts.values()) != total or len(carried) != total:
        raise SystemExit("e5: the record does not cover the 113 archived vectors")
    carried.sort()
    print("E5 archived index disclosure")
    for skipped, vectors in sorted(counts.items()):
        print(f"  earlier prime indices = {skipped}: {vectors} of {total} vectors")
    print(f"  carried index: min {carried[0]}, median {carried[len(carried) // 2]}, max {carried[-1]}")
    print()


def unsignable():
    print("E6 and signer half of E4: both signers on the same messages, full width")
    print("| Row | Messages | Unsalted: signed | Unsalted: backend error | Unsalted: exhausted or malformed | Unsalted: skipped primes | Unsalted mean hashes | Salted: signed | Salted mean hashes | Salted mean primes | Salted backend errors | Salted refused responses |")
    print("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    for level, (name, _, _) in LEVELS.items():
        digit = {"I": 1, "III": 3, "V": 5}[name]
        tried = 0
        old = Counter()
        old_hashes = skipped = 0
        new_signed = new_hashes = new_primes = new_backend = new_refused = 0
        seen = set()
        lines = []
        for path in sorted(ROOT.glob(f"e6/level{digit}-*.txt")):
            text = path.read_text().splitlines()
            lines += text
            done = [fields(line) for line in text if line.startswith("TRIED ")]
            if len(done) != 1:
                raise SystemExit(f"{path.name}: the run did not finish")
            tried += int(done[0]["count"])
        for line in lines:
            if not line.startswith("SIGN "):
                continue
            head, salted = line.split(" salted=", 1)
            row = fields(head)
            if row["n"] in seen:
                raise SystemExit(f"e6 level {name}: message {row['n']} recorded twice")
            seen.add(row["n"])
            outcome = dict(p.split("=", 1) for p in row["unsalted"].split(";")[1:])
            kind = row["unsalted"].split(";")[0]
            old[kind] += 1
            if kind == "signed":
                old_hashes += int(outcome["hashes"])
                skipped += int(outcome["skipped_primes"])
            if salted.startswith("signed "):
                new = dict(p.split("=", 1) for p in salted.split()[1:])
                new_signed += 1
                new_hashes += int(new["hashes"])
                new_primes += int(new["primes"])
                new_backend += int(new["backend_errors"]) + int(new["malformed"])
                new_refused += int(new["rejections"])
        if len(seen) != tried or tried == 0:
            raise SystemExit(f"e6 level {name}: {len(seen)} records for {tried} messages")
        print(
            f"| {name} | {tried} | {old['signed']} | {old['backend-error']} | {old['exhausted'] + old['malformed']} "
            f"| {skipped} | {old_hashes / max(old['signed'], 1):.1f} | {new_signed} "
            f"| {new_hashes / max(new_signed, 1):.1f} | {new_primes / max(new_signed, 1):.4f} | {new_backend} | {new_refused} |"
        )
        for line in lines:
            if line.startswith("UNSIGNABLE "):
                print("  " + line)
    print()


KEYS = 12
KEY_MESSAGES = 2000


def keys():
    print("E6 over distinct keys, row I, full width")
    print("| Key | Seed | Public key (first 8 bytes) | Messages | Unsalted: signed at the first prime index | Unsalted: backend error, exhausted or malformed | Salted: signed at the first prime candidate | Salted: backend errors | Salted: refused responses |")
    print("| ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |")
    public = set()
    totals = Counter()
    for key in range(KEYS):
        path = ROOT / f"e6/key-{key:02d}.txt"
        text = path.read_text().splitlines()
        head = [fields(line) for line in text if line.startswith("KEY ")]
        done = [fields(line) for line in text if line.startswith("TRIED ")]
        if len(head) != 1 or int(head[0]["key"]) != key or head[0]["level"] != "NIST-I":
            raise SystemExit(f"{path.name}: not the record of key {key} at row I")
        if len(done) != 1 or int(done[0]["count"]) != KEY_MESSAGES:
            raise SystemExit(f"{path.name}: the run did not finish its {KEY_MESSAGES} messages")
        public.add(head[0]["public"])
        seen = set()
        row = Counter()
        for line in text:
            if not line.startswith("SIGN "):
                continue
            first, salted = line.split(" salted=", 1)
            old = fields(first)
            seen.add(old["n"])
            kind = old["unsalted"].split(";")[0]
            outcome = dict(p.split("=", 1) for p in old["unsalted"].split(";")[1:])
            if kind == "signed" and outcome["skipped_primes"] == "0":
                row["old_first"] += 1
            elif kind != "signed":
                row["old_failed"] += 1
            if salted.startswith("signed "):
                new = dict(p.split("=", 1) for p in salted.split()[1:])
                row["new_backend"] += int(new["backend_errors"]) + int(new["malformed"])
                row["new_refused"] += int(new["rejections"])
                if new["primes"] == "1":
                    row["new_first"] += 1
        if len(seen) != KEY_MESSAGES:
            raise SystemExit(f"{path.name}: {len(seen)} records for {KEY_MESSAGES} messages")
        row["unsignable"] = int(done[0]["unsignable"])
        totals.update(row)
        totals["messages"] += KEY_MESSAGES
        print(
            f"| {key} | `{head[0]['seed']}` | `{head[0]['public'][:16]}` | {KEY_MESSAGES} | {row['old_first']} "
            f"| {row['old_failed']} | {row['new_first']} | {row['new_backend']} | {row['new_refused']} |"
        )
    if len(public) != KEYS:
        raise SystemExit("e6: two key records share a public key")
    print(
        f"  {KEYS} distinct public keys, {totals['messages']} messages: unsalted signed at the first prime index "
        f"{totals['old_first']}, unsalted failures {totals['old_failed']}, UNSIGNABLE {totals['unsignable']}; "
        f"salted at the first prime candidate {totals['new_first']}, backend errors {totals['new_backend']}, "
        f"refused {totals['new_refused']}"
    )
    print()


def main():
    write = sys.argv[1:] == ["--write"]
    if sys.argv[1:] and not write:
        raise SystemExit(__doc__)
    density()
    replay(write)
    index_leak()
    unsignable()
    keys()


if __name__ == "__main__":
    main()
