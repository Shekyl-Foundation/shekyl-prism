#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Inspect the linked release shift consumer on the native CI architecture."""
import argparse
import hashlib
import select
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from stage2c_fixtures import SET_WIDTHS, corpus, encode_basis
from stage2c_isa import hardware_divide, instructions, self_test as isa_self_test
from stage2c_probe import ARCH, build_command, release_libraries, run


def shift_cases(width):
    bits = width * 64
    return [(0, 0), (1, 1), (2**63, 63), (2**64-1, 64), (7, 65),
            (0x7351f87ca912de34, 255), (1, bits-1), (0, bits), (17, 2**64-1)]


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


def memcheck(binary, mode, args, expected_bytes, tmp):
    prefix = f"--vgdb-prefix={tmp}/vgdb"
    resume = tmp / "resume"
    cmd = ["valgrind", "--tool=memcheck", "--error-exitcode=93", "--quiet", "--vgdb-poll=1000",
           prefix, binary, mode, *args, resume]
    with subprocess.Popen(cmd, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, text=True) as proc:
        try:
            if not select.select([proc.stdout], [], [], 30)[0]:
                raise ValueError("probe did not announce its input buffer")
            ready = proc.stdout.readline().strip()
            match = re.fullmatch(r"(0x[0-9a-f]+) (\d+)", ready)
            if not match or int(match[2]) != expected_bytes:
                raise ValueError(f"unexpected {mode} input buffer: {ready!r}")
            addr = match[1]
            monitor = ["vgdb", prefix, f"--pid={proc.pid}", "--cmd-time-out=10", "--max-invoke-ms=0"]
            marked = subprocess.run([*monitor, "-c", f"make_memory undefined {addr} {expected_bytes}",
                                     "-c", f"get_vbits {addr} {expected_bytes}"],
                                    capture_output=True, text=True, timeout=20)
            if marked.returncode:
                raise ValueError(f"vgdb failed on {mode}: {marked.stderr}")
            if "".join(marked.stdout.split()).lower() != "ff" * expected_bytes:
                raise ValueError(f"Memcheck did not mark every byte of the {mode} input")
            resume.touch(exist_ok=False)
            _, errors = proc.communicate(timeout=90)
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.communicate()
            resume.unlink(missing_ok=True)
    if mode == "control":
        if proc.returncode != 93 or "Conditional jump" not in errors:
            raise ValueError(f"Memcheck missed known secret branch: {errors}")
    elif proc.returncode:
        raise ValueError(f"Memcheck failed on {mode}: {errors}")


def main(tmp, listing):
    if ARCH not in ("x86_64", "aarch64"):
        raise ValueError(f"no audited probe for {ARCH}")
    libs = release_libraries()
    listings = []
    for name, width in SET_WIDTHS.items():
        listings.append(check_set(tmp, libs, name, width))
    if listing:
        listing.write_text("\n".join(listings))


def check_set(tmp, libs, name, width):
    cases = shift_cases(width)
    binary = tmp / "probe"
    rustc = build_command(libs, name, binary)
    # Static executable addresses for QEMU. Memcheck uses its normal dynamic
    # runtime interposition; both builds compile the very same Rust consumer.
    run(*rustc, "-Ctarget-feature=+crt-static")
    symbols = run("nm", "--defined-only", "-C", binary)
    names = {symbol: int(addr, 16) for addr, symbol in
             re.findall(r"^([a-f0-9]+) [tT] probe::(shift_probe|leaky_control|probe_end)$", symbols, re.M)}
    log = tmp / "qemu.log"

    def pcs(mode, x, k):
        start = names["shift_probe" if mode == "probe" else "leaky_control"]
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
        parsed = list(instructions(section))
        if not any(address in visited for address, _, _ in parsed):
            continue
        inspected.append(section)
        for address, op, operands in parsed:
            if hardware_divide(op):
                raise ValueError(f"{name} hardware division at {address:#x}: {op} {operands}")
    if not inspected:
        raise ValueError("assembly scan inspected no functions")
    print(f"{ARCH} {name}: {len(reference)} equal blocks, {len(cases)} cases, {len(inspected)} functions inspected")
    print("trace digest", hashlib.sha256(str(reference).encode()).hexdigest())
    if ARCH == "x86_64":
        run(*rustc)
        memcheck(binary, "control", ["0", "1"], 16, tmp)
        for x, k in cases:
            memcheck(binary, "probe", [f"{x:016x}", f"{k:016x}"], 16, tmp)
        rows = corpus(name)
        for cls, values in rows:
            encoded = tmp / "basis.bin"
            encoded.write_bytes(encode_basis(values[:9], width))
            memcheck(binary, "reduce", [encoded, cls], encoded.stat().st_size, tmp)
        print(f"Memcheck accepted all {len(rows)} {name} constructors and reductions")
    return f"\n# {ARCH} {name}\n" + "\n".join(inspected)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assembly", type=Path, help="write disassembly of the visited functions")
    args = parser.parse_args()
    isa_self_test()
    with tempfile.TemporaryDirectory() as temp:
        main(Path(temp), args.assembly)
