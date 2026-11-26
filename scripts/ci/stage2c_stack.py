#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
# All rights reserved.
# BSD-3-Clause
"""Measure stack writes and search the returned frames for public fixture words."""
import argparse
import os
import re
import subprocess
import tempfile
from pathlib import Path

from check_ct_shift import ARCH, HERE, SET_WIDTHS, VECTORS, build_command, release_libraries, run


def inspect(binary, markers, mode, encoded, cls):
    cmd = ["gdb", "-nx", "-q", "-batch"]
    for label, address in markers.items():
        cmd.extend(["-ex", f"set ${label} = 0x{address}"])
    cmd.extend(["-x", HERE / "stack.gdb", "--args", binary, mode, encoded, cls])
    result = subprocess.run(cmd, env={**os.environ, "PRISM_STACK_FIXTURE": str(encoded)},
                                     text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    output = result.stdout
    if result.returncode:
        raise RuntimeError(output)
    match = re.search(r"^stack-result (\d+) (\d+) (\d+)$", output, re.M)
    if not match:
        raise RuntimeError(output)
    return tuple(map(int, match.groups()))


def inspect_instructions(binary, encoded, cls, profile, listing):
    subprocess.run(["valgrind", "--quiet", "--tool=callgrind", "--collect-atstart=no",
                    "--toggle-collect=*reducer_probe*", "--dump-instr=yes", "--compress-pos=no",
                    "--compress-strings=no", f"--callgrind-out-file={profile}",
                    binary, "reduce", encoded, cls], check=True)
    pcs = set()
    functions = set()
    in_binary = False
    call_cost = False
    for line in profile.read_text().splitlines():
        if line.startswith("ob="):
            in_binary = line[3:] == str(binary)
        elif line.startswith("fn="):
            function = line[3:]
        elif line.startswith("calls="):
            call_cost = True
        elif line.startswith("0x"):
            if in_binary and not call_cost:
                pcs.add(int(line.split()[0], 16))
                functions.add(function)
            call_cost = False
    if not any("reducer_probe" in f for f in functions) or len(pcs) < 100:
        raise ValueError("missing reducer instruction profile")
    instructions = {}
    for pc, opcode in re.findall(r"^\s*([0-9a-f]+):\s+(\w+)", listing, re.M):
        instructions[int(pc, 16)] = opcode
    if pcs - instructions.keys():
        raise ValueError("profile and release executable disagree")
    divides = sorted(pc for pc in pcs if re.fullmatch(r"(?:i?div[bwlq]?|[us]div)", instructions[pc]))
    if divides:
        raise ValueError(f"division in the measured reducer at {list(map(hex, divides))}")
    return pcs


def main(out):
    out.mkdir(parents=True, exist_ok=True)
    libs = release_libraries()
    with tempfile.TemporaryDirectory() as directory:
        tmp = Path(directory)
        for name, width in SET_WIDTHS.items():
            binary = tmp / "probe"
            run(*build_command(libs, name, binary))
            assembly = run("objdump", "-d", "-C", "--no-show-raw-insn", binary)
            (out / f"{ARCH}-{name}.asm").write_text(assembly)
            symbols = run("nm", "-C", binary)
            markers = {label: re.search(rf"^([0-9a-f]+) t probe::{symbol}$", symbols, re.M)[1]
                       for label, symbol in (("start", "stack_start"), ("end", "probe_end"))}
            encoded = tmp / "basis.bin"
            encoded.write_bytes(bytes(range(64)) + bytes(9 * (width*8+1)-64))
            _, found, _ = inspect(binary, markers, "residue-control", encoded, "sign-generic")
            if found == 0:
                raise ValueError("stack scan missed the deliberately retained words")
            counts = {}
            visited = set()
            for line in (VECTORS / f"{name}.txt").read_text().splitlines():
                cls, *fields = line.split()
                values = [int(v, 16) for v in fields[:9]]
                encoded.write_bytes(b"".join(abs(v).to_bytes(width*8, "little") + bytes([v < 0])
                                             for v in values))
                depth, found, total = inspect(binary, markers, "stack", encoded, cls)
                if cls not in counts:
                    visited.update(inspect_instructions(binary, encoded, cls, tmp / "callgrind", assembly))
                counts[cls] = counts.get(cls, 0) + 1
                print(f"{ARCH} {name} {cls} {counts[cls]} stack_bytes={depth} "
                      f"residual_words={found}/{total}", flush=True)
            (out / f"{ARCH}-{name}.pcs").write_text("".join(f"{pc:x}\n" for pc in sorted(visited)))
            print(f"{ARCH} {name} full reducer: {len(visited)} linked instructions checked")
            if counts != {"prered-guard256": 6, "sign-generic": 5}:
                raise ValueError(f"incomplete {name} stack corpus")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    main(parser.parse_args().output)
