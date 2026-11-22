#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The largest integer each function of the C reference reaches.

Runs the reference's key generation and our exporter's signing
(scripts/c/export_s5_vectors.c) in the build scripts/build_traced_reference.sh
makes, where every ibz_* function that writes an integer records the bit
length of what it wrote and where it was called from. Merges the records of
all runs, resolves each call site to the function it is in, and writes one
table: docs/benchmarks/integer-widths-reference.tsv.

    python3 scripts/measure_integer_widths.py --work <work-directory> \
        [--keys 32] [--signs 8] [--jobs 2] [--timeout 600] [--out <file>]

Standard library only. Every seed and every challenge scalar is derived from
a fixed label, so a run is reproducible given the same build.
"""

import argparse
import hashlib
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

SETS = (("p248", "lvl1", 248), ("p376", "lvl3", 376), ("p500", "lvl5", 500))
LABEL = b"shekyl-prism integer widths 2026-11-22"


def seed(*parts):
    """48 bytes, the exporter's seed length."""
    return hashlib.sha384(LABEL + b"|" + "|".join(str(p) for p in parts).encode()).digest()


BASES = (2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71,
         73, 79, 83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151)


def probable_prime(n):
    """Miller-Rabin to 36 fixed bases: enough to choose inputs with."""
    if n < 2:
        return False
    for base in BASES:
        if n % base == 0:
            return n == base
    odd, twos = n - 1, 0
    while odd % 2 == 0:
        odd //= 2
        twos += 1
    for base in BASES:
        x = pow(base, odd, n)
        if x in (1, n - 1):
            continue
        for _ in range(twos - 1):
            x = x * x % n
            if x == n - 1:
                break
        else:
            return False
    return True


def scalar(e, key, index):
    """An odd s of at most e - 3 bits for which q = 2^(2e-5) - s^2 is prime.

    The protocol's challenge is such an s, and its signer is only ever asked
    for a prime q (VERIFIER_SPEC.md). Index 0 is the smallest such s, so the
    largest degree; the others are the first that passes at or after a
    derived starting point.
    """
    if index == 0:
        s = 1
    else:
        digest = hashlib.shake_256(LABEL + b"|scalar|%d|%d|%d" % (e, key, index)).digest(e // 8 + 8)
        s = (int.from_bytes(digest, "big") % (1 << (e - 3))) | 1
    while not probable_prime((1 << (2 * e - 5)) - s * s):
        s += 2
        if s >> (e - 3):
            s = 1
    return s


def run(binary, arguments, seed_bytes, trace, limit):
    """The exit status, or "timeout". A run that is stopped still writes its record."""
    env = dict(os.environ, IBZ_TRACE_OUT=trace)
    process = subprocess.Popen([binary] + arguments, stdin=subprocess.PIPE, env=env,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        process.communicate(seed_bytes, timeout=limit)
        return process.returncode
    except subprocess.TimeoutExpired:
        process.terminate()
        try:
            process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        return "timeout"


def read_trace(path, table):
    if not os.path.exists(path):
        return
    with open(path) as handle:
        for line in handle:
            op, site, bits, calls = line.split()
            key = (op, int(site, 16))
            old = table.get(key, (0, 0))
            table[key] = (max(old[0], int(bits)), old[1] + int(calls))


def resolve(binary, sites):
    """The innermost function at each return address, inlined callers included."""
    sites = sorted(sites)
    done = subprocess.run(
        ["addr2line", "-f", "-i", "-e", binary] + ["0x%x" % (site - 1) for site in sites],
        capture_output=True, text=True, check=True)
    lines = done.stdout.split("\n")
    # addr2line -i prints, for one address, a (function, file:line) pair for
    # each inlining level, innermost first, with nothing between addresses.
    # Asking for one address at a time keeps the pairing certain.
    if len(lines) - 1 != 2 * len(sites):
        names = {}
        for site in sites:
            one = subprocess.run(
                ["addr2line", "-f", "-i", "-e", binary, "0x%x" % (site - 1)],
                capture_output=True, text=True, check=True)
            names[site] = one.stdout.split("\n")[0]
        return names
    return {site: lines[2 * i] for i, site in enumerate(sites)}


def plain(name):
    """Without the reference's namespace prefix and the compiler's clone suffix."""
    name = re.sub(r"\.(constprop|isra|part|lto_priv|cold)(\.\d+)?", "", name)
    return re.sub(r"^prism_(gen|lvl\d+_ref|lvl\d+)_", "", name)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--work", required=True)
    parser.add_argument("--keys", type=int, default=32)
    parser.add_argument("--signs", type=int, default=8)
    parser.add_argument("--jobs", type=int, default=2)
    parser.add_argument("--timeout", type=int, default=600, help="seconds a run may take")
    parser.add_argument("--out", default=None)
    args = parser.parse_args()
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out = args.out or os.path.join(root, "docs", "benchmarks", "integer-widths-reference.tsv")
    work = os.path.abspath(args.work)
    scratch = os.path.join(work, "runs")
    os.makedirs(scratch, exist_ok=True)

    rows = []
    summary = []
    for name, variant, e in SETS:
        binary = os.path.join(work, "traced_" + variant)

        def keygen(key):
            path = os.path.join(scratch, "%s-key-%d" % (name, key))
            trace = path + ".trace"
            for stale in (path, trace):
                if os.path.exists(stale):
                    os.remove(stale)
            return run(binary, ["keygen", path], seed(name, "keygen", key), trace, args.timeout)

        def sign(job):
            key, index = job
            path = os.path.join(scratch, "%s-key-%d" % (name, key))
            trace = "%s-sign-%d.trace" % (path, index)
            if os.path.exists(trace):
                os.remove(trace)
            return run(binary, ["sign", path, str(scalar(e, key, index))],
                       seed(name, "sign", key, index), trace, args.timeout)

        with ThreadPoolExecutor(args.jobs) as pool:
            key_codes = list(pool.map(keygen, range(args.keys)))
            good = [key for key, code in enumerate(key_codes) if code == 0]
            jobs = [(key, index) for key in good for index in range(args.signs)]
            sign_codes = list(pool.map(sign, jobs))

        tables = {"keygen": {}, "sign": {}}
        for key in range(args.keys):
            read_trace(os.path.join(scratch, "%s-key-%d.trace" % (name, key)), tables["keygen"])
        for key, index in jobs:
            read_trace(os.path.join(scratch, "%s-key-%d-sign-%d.trace" % (name, key, index)),
                       tables["sign"])
        sites = {site for table in tables.values() for (_, site) in table}
        names = resolve(binary, sites)
        for phase in ("keygen", "sign"):
            merged = {}
            for (op, site), (bits, calls) in tables[phase].items():
                key = (plain(names[site]), op)
                old = merged.get(key, (0, 0))
                merged[key] = (max(old[0], bits), old[1] + calls)
            for (function, op), (bits, calls) in merged.items():
                rows.append((name, phase, function, op, bits, calls))
            top = max((bits for bits, _ in merged.values()), default=0)
            summary.append((name, phase, top))
        for what, codes in (("keygen", key_codes), ("sign", sign_codes)):
            counts = {}
            for code in codes:
                counts[code] = counts.get(code, 0) + 1
            summary.append((name, what + " runs", "%d; by exit status %s" % (
                len(codes), ", ".join("%s: %d" % (code, counts[code])
                                      for code in sorted(counts, key=str)))))

    rows.sort(key=lambda row: (row[0], row[1], -row[4], row[2], row[3]))
    with open(out, "w") as handle:
        handle.write("# The largest bit length written by each ibz_* function of the C reference, by the\n")
        handle.write("# function that called it. Written by scripts/measure_integer_widths.py; see\n")
        handle.write("# docs/SIGNER_SPEC.md section 2.1. keys=%d signs-per-key=%d\n" % (args.keys, args.signs))
        for name, what, value in summary:
            handle.write("# %s %s: %s\n" % (name, what, value))
        handle.write("set\tphase\tfunction\toperation.output\tmax_bits\tcalls\n")
        for row in rows:
            handle.write("\t".join(str(field) for field in row) + "\n")
    for line in summary:
        print(*line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
