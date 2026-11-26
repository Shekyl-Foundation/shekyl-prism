#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Measure stack retention and the instructions of every reviewed reduction.

The probe owns the paint window and the canary length. This script reads
those constants, plants a canary the word filter must recover in full, then
records residual input words for every fixture. Callgrind runs once per
class over that class's whole file, and integer hardware division in the
visited instructions fails the run.
"""
import argparse
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage2c_fixtures import (CLASS_COUNTS, LIMB_BYTES, SET_WIDTHS, corpus, encode_basis,
                              nontrivial_words, retention_canary, self_test)
from stage2c_isa import hardware_divide, mnemonics, self_test as isa_self_test
from stage2c_probe import ARCH, PROBE_SOURCE, build_command, instrument_constants, release_libraries, run

HERE = Path(__file__).resolve().parent / "stage2c"
# A profile that missed the reducer is a green scan of nothing.
MIN_VISITED_INSTRUCTIONS = 100
MARKERS = (("start", "stack_start"), ("end", "probe_end"))


def marker_addresses(symbols):
    """Linked addresses of the retention brackets. A missing marker is a failed build."""
    found = {}
    for label, symbol in MARKERS:
        match = re.search(rf"^([0-9a-f]+) t probe::{symbol}$", symbols, re.M)
        if not match:
            raise RuntimeError(f"missing probe::{symbol} in\n{symbols}")
        found[label] = match[1]
    return found


def write_words(path, words):
    path.write_text("".join(f"{word.hex()}\n" for word in words))


def inspect(binary, markers, mode, encoded, cls, words, limits):
    """Paint the probe's window and return `(depth, found, total)`."""
    if not words:
        raise ValueError("retention scan was given no words")
    word_file = encoded.with_suffix(".words")
    write_words(word_file, words)
    command = ["gdb", "-nx", "-q", "-batch"]
    for label, address in markers.items():
        command.extend(["-ex", f"set ${label} = 0x{address}"])
    command.extend([
        "-ex", f"set $paint = {limits['paint']}",
        "-ex", f"set $guard = {limits['guard']}",
        "-x", str(HERE / "stack.gdb"),
        "--args", str(binary), mode, str(encoded), cls,
    ])
    result = subprocess.run(
        command,
        env={**os.environ, "PRISM_STACK_WORDS": str(word_file)},
        text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    if result.returncode:
        raise RuntimeError(result.stdout)
    match = re.search(r"^stack-result (\d+) (\d+) (\d+)$", result.stdout, re.M)
    if not match:
        raise RuntimeError(result.stdout)
    depth, found, total = (int(group) for group in match.groups())
    if total != len(words):
        raise ValueError(f"scanner reported {total} words for a list of {len(words)}")
    return depth, found, total


def parse_profile(path, binary, opcodes):
    """Program counters and function names Callgrind attributed to `binary`.

    `opcodes` is the whole release listing. A counter missing from it means
    the profile and the executable are not the same build.
    """
    pcs = set()
    functions = set()
    in_binary = False
    call_cost = False
    function = None
    binary_path = str(Path(binary).resolve())
    for line in path.read_text().splitlines():
        if line.startswith("ob="):
            in_binary = str(Path(line[3:]).resolve()) == binary_path
            function = None
        elif line.startswith("fn="):
            function = line[3:]
        elif line.startswith("calls="):
            call_cost = True
        elif line.startswith("0x") and in_binary and function is not None:
            if not call_cost:
                pcs.add(int(line.split()[0], 16))
                functions.add(function)
            call_cost = False
    if not any("reducer_probe" in name for name in functions):
        raise ValueError("profile missed reducer_probe")
    if len(pcs) < MIN_VISITED_INSTRUCTIONS:
        raise ValueError(f"profile has {len(pcs)} instructions")
    missing = pcs - opcodes.keys()
    if missing:
        sample = ", ".join(hex(pc) for pc in sorted(missing)[:4])
        raise ValueError(f"profile and release executable disagree at {sample}")
    return pcs, functions


def profile_class(binary, encoded, cls, count, opcodes, profile_path):
    subprocess.run([
        "valgrind", "--quiet", "--tool=callgrind", "--collect-atstart=no",
        "--toggle-collect=*reducer_probe*", "--dump-instr=yes",
        "--compress-pos=no", "--compress-strings=no",
        f"--callgrind-out-file={profile_path}",
        str(binary), "profile", str(encoded), cls, str(count),
    ], check=True)
    return parse_profile(profile_path, binary, opcodes)


def main(out):
    self_test()
    isa_self_test()
    if ARCH not in ("x86_64", "aarch64"):
        raise ValueError(f"no audited probe for {ARCH}")
    if not PROBE_SOURCE.is_file():
        raise FileNotFoundError(PROBE_SOURCE)
    out.mkdir(parents=True, exist_ok=True)
    libraries = release_libraries()
    with tempfile.TemporaryDirectory() as directory:
        tmp = Path(directory)
        for name, width in SET_WIDTHS.items():
            binary = tmp / "probe"
            run(*build_command(libraries, name, binary))
            limits = instrument_constants(binary)
            assembly = run("objdump", "-d", "-C", "--no-show-raw-insn", str(binary))
            (out / f"{ARCH}-{name}.asm").write_text(assembly)
            markers = marker_addresses(run("nm", "-C", str(binary)))
            canary = retention_canary(width, limits["canary"])
            if len(canary) != limits["stride"]:
                raise ValueError("canary length and probe stride disagree")
            planted = nontrivial_words(canary, width)
            if len(planted) != limits["canary"] // LIMB_BYTES:
                raise ValueError("canary did not plant one word per limb")
            encoded = tmp / "basis.bin"
            encoded.write_bytes(canary)
            _, found, total = inspect(binary, markers, "canary", encoded, "sign-generic", planted, limits)
            if (found, total) != (len(planted), len(planted)):
                raise ValueError(f"retention scan missed planted words ({found}/{total})")
            counts = Counter()
            blobs = {cls: bytearray() for cls in CLASS_COUNTS}
            for cls, values in corpus(name):
                basis = encode_basis(values[:9], width)
                if len(basis) != limits["stride"]:
                    raise ValueError(f"{name} encoder and probe disagree on the basis layout")
                words = nontrivial_words(basis, width)
                if not words:
                    raise ValueError(f"{name} {cls} fixture has no nontrivial input word")
                encoded.write_bytes(basis)
                depth, found, total = inspect(binary, markers, "stack", encoded, cls, words, limits)
                counts[cls] += 1
                blobs[cls] += basis
                print(
                    f"{ARCH} {name} {cls} {counts[cls]} stack_bytes={depth} "
                    f"residual_words={found}/{total}",
                    flush=True,
                )
            if dict(counts) != CLASS_COUNTS:
                raise ValueError(f"incomplete {name} stack corpus: {dict(counts)}")
            opcodes = mnemonics(assembly)
            visited = set()
            functions = set()
            for cls, blob in blobs.items():
                blob_path = tmp / f"{cls}.bin"
                blob_path.write_bytes(blob)
                pcs, names = profile_class(
                    binary, blob_path, cls, counts[cls], opcodes, tmp / f"{cls}.callgrind",
                )
                visited |= pcs
                functions |= names
            divides = sorted(pc for pc in visited if hardware_divide(opcodes[pc]))
            if divides:
                raise ValueError(f"division in the measured reducer at {list(map(hex, divides))}")
            census = Counter(opcodes[pc] for pc in visited)
            print(f"{ARCH} {name} instructions {len(visited)} functions {len(functions)}")
            print(
                f"{ARCH} {name} opcodes "
                + " ".join(f"{opcode}={count}" for opcode, count in sorted(census.items()))
            )
            (out / f"{ARCH}-{name}.pcs").write_text("".join(f"{pc:x}\n" for pc in sorted(visited)))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    main(parser.parse_args().output)
