#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Inspect the linked release shift consumer on the native CI architecture."""
import argparse
import hashlib
import json
import platform
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent / "ct_shift"
ROOT = HERE.parents[2]
ARCH = platform.machine()
sys.path.insert(0, str(ROOT / "scripts"))
from stage2c_fixtures import SET_WIDTHS, VECTORS


def shift_cases(width):
    bits = width * 64
    return [(0, 0), (1, 1), (2**63, 63), (2**64-1, 64), (7, 65),
            (0x7351f87ca912de34, 255), (1, bits-1), (0, bits), (17, 2**64-1)]


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True)


def trace(binary, start, end, mode, x, k, log):
    # argv occupies guest stack space. Keep its size fixed across inputs so
    # an alignment-dependent libc copy sees the same public addresses.
    run(f"qemu-{ARCH}", "-d", "nochain,exec", "-D", log, binary, mode, f"0x{x:016x}", f"0x{k:016x}")
    active = False
    blocks = []
    for match in re.finditer(r"^Trace \d+: \S+ \[[\da-f]+/([\da-f]+)/", log.read_text(), re.M):
        pc = int(match[1], 16)
        if pc == end and active:
            return blocks
        active |= pc == start
        if active:
            blocks.append(pc)
    raise ValueError("missing probe entry or exit in QEMU trace")


def main(tmp, listing):
    if ARCH not in ("x86_64", "aarch64"):
        raise ValueError(f"no audited probe for {ARCH}")
    msgs = run("cargo", "build", "--locked", "--release", "-p", "prism-int", "-p", "prism-field",
               "--message-format=json")
    libs = {}
    for line in msgs.splitlines():
        msg = json.loads(line)
        name = msg.get("target", {}).get("name")
        if name in ("prism_int", "prism_field"):
            [libs[name]] = [Path(f) for f in msg["filenames"] if f.endswith(".rlib")]
    listings = []
    for name, width in SET_WIDTHS.items():
        listings.append(check_set(tmp, libs, name, width))
    if listing:
        listing.write_text("\n".join(listings))


def check_set(tmp, libs, name, width):
    cases = shift_cases(width)
    archive = tmp / "probe.a"
    run("rustc", "--edition=2021", "--crate-type=staticlib", "-Copt-level=3", "-Cpanic=abort",
        "-Crelocation-model=static", "--cfg", f'stage2c_set="{name}"', "--extern", f"prism_int={libs['prism_int']}",
        "--extern", f"prism_field={libs['prism_field']}", "-L", f"dependency={libs['prism_int'].parent}/deps",
        HERE / "probe.rs", "-o", archive)
    binary = tmp / "check"
    cc = ["cc", "-g", "-O1", "-no-pie", "-Wl,--gc-sections", f"-DSTAGE2C_LIMBS={width}", HERE / "driver.c", archive, "-ldl", "-lpthread", "-lm", "-o", binary]
    run(*cc, "-static")
    names = {symbol: int(addr, 16) for addr, symbol in re.findall(r"^([\da-f]+) [Tt] (\w+)$", run("nm", binary), re.M)}
    log = tmp / "qemu.log"

    def pcs(mode, x, k):
        start = names["prism_shift_probe" if mode == "probe" else "leaky_control"]
        result = trace(binary, start, names["probe_end"], mode, x, k, log)
        if not result:
            raise ValueError("empty trace")
        return result

    if pcs("control", 1, 0) == pcs("control", 1, 1):
        raise ValueError("trace missed known secret branch")
    reference = pcs("probe", *cases[0])
    for x, k in cases[1:]:
        if pcs("probe", x, k) != reference:
            raise ValueError(f"{name} shift trace changed on public case {(x, k)}")
    # Inspect every function actually visited, including out-of-line arithmetic.
    assembly = run("objdump", "-d", "--no-show-raw-insn", binary)
    sections = re.split(r"(?m)^(?=[0-9a-f]+ <[^>]+>:$)", assembly)
    visited = set(reference)
    inspected = []
    for section in sections:
        instructions = re.findall(r"(?m)^[ \t]*([\da-f]+):[ \t]+(\w+)[ \t]*(.*)$", section)
        if not any(int(pc, 16) in visited for pc, _, _ in instructions):
            continue
        # The public driver contains both the comparator and the consumer.
        if " <main>:" in section.splitlines()[0]:
            continue
        inspected.append(section)
        for pc, op, operands in instructions:
            if re.fullmatch(r"(?:i?div[bwlq]?|[us]div)", op):
                raise ValueError(f"{name} hardware division at {pc}: {op} {operands}")
    if not inspected:
        raise ValueError("assembly scan inspected no functions")
    print(f"{ARCH} {name}: {len(reference)} equal blocks, {len(cases)} cases, {len(inspected)} functions inspected")
    print("trace digest", hashlib.sha256(str(reference).encode()).hexdigest())
    if ARCH == "x86_64":
        run(*cc, "-DMEMCHECK")
        vg = ["valgrind", "--tool=memcheck", "--error-exitcode=93", "--quiet", binary]
        control = subprocess.run([*vg, "control", "7", "1"], capture_output=True, text=True)
        if control.returncode != 93 or "Conditional jump" not in control.stderr:
            raise ValueError("Memcheck missed known secret branch")
        for x, k in cases:
            run(*vg, "probe", str(x), str(k))
        fixtures = VECTORS / f"{name}.txt"
        inputs = fixtures.read_text().splitlines()
        if not inputs:
            raise ValueError(f"empty {name} reducer corpus")
        for line in inputs:
            cls, *fields = line.split()
            values = [int(s, 16) for s in fields[:9]]
            encoded = tmp / "basis.bin"
            encoded.write_bytes(b"".join(abs(v).to_bytes(width * 8, "little") + bytes([v < 0]) for v in values))
            run(*vg, "reduce", encoded, str(int(cls == "prered-guard256")))
        print(f"Memcheck accepted all {len(inputs)} {name} constructors and reductions")
    return f"\n# {ARCH} {name}\n" + "\n".join(inspected)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assembly", type=Path, help="write disassembly of the visited functions")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as temp:
        main(Path(temp), args.assembly)
