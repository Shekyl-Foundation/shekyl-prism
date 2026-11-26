#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Build the release Stage 2c probe, one binary per parameter set.

The probe source owns the stack-window and canary lengths. Callers read
them from `<probe> constants` so the numbers exist in one place.
"""
import json
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))
from stage2c_fixtures import LIMB_BYTES

PROBE_SOURCE = Path(__file__).resolve().parent / "stage2c" / "probe.rs"
ARCH = platform.machine()
INSTRUMENT_KEYS = ("paint", "guard", "canary", "stride")


def run(*args):
    """Run a command at the repository root and return its text."""
    return subprocess.check_output(args, cwd=ROOT, text=True)


def release_libraries():
    """Release rlibs of the two crates the probe links."""
    messages = run(
        "cargo", "build", "--locked", "--release",
        "-p", "prism-int", "-p", "prism-field",
        "--message-format=json",
    )
    libraries = {}
    for line in messages.splitlines():
        message = json.loads(line)
        name = message.get("target", {}).get("name")
        if name not in ("prism_int", "prism_field"):
            continue
        matches = [Path(filename) for filename in message["filenames"] if filename.endswith(".rlib")]
        if len(matches) != 1:
            raise RuntimeError(f"expected one {name} rlib, found {matches}")
        libraries[name] = matches[0]
    if set(libraries) != {"prism_int", "prism_field"}:
        raise RuntimeError(f"release libraries missing: {sorted(libraries)}")
    return libraries


def build_command(libraries, name, binary):
    """The rustc line for one parameter set. Callers may append target features."""
    return [
        "rustc", "--edition=2021", "-Copt-level=3", "-Cpanic=abort",
        "-Crelocation-model=static", "--cfg", f'stage2c_set="{name}"',
        "--extern", f"prism_int={libraries['prism_int']}",
        "--extern", f"prism_field={libraries['prism_field']}",
        "-L", f"dependency={libraries['prism_int'].parent}/deps",
        PROBE_SOURCE, "-o", binary,
    ]


def instrument_constants(binary):
    """Paint window, guard page, canary length, and basis stride from the probe.

    A missing key, a guard that is not a proper suffix of the paint, or a
    canary that is not a whole number of limbs is a broken instrument.
    """
    text = run(str(binary), "constants")
    values = {}
    for line in text.splitlines():
        key, raw = line.split()
        values[key] = int(raw)
    if set(values) != set(INSTRUMENT_KEYS) or len(values) != len(INSTRUMENT_KEYS):
        raise ValueError(f"probe constants: {text!r}")
    if not 0 < values["guard"] < values["paint"]:
        raise ValueError("guard page is not a proper suffix of the paint window")
    if values["canary"] % LIMB_BYTES != 0 or values["canary"] <= 0:
        raise ValueError(f"canary length {values['canary']} is not a run of limbs")
    if values["stride"] <= values["canary"]:
        raise ValueError("canary does not fit in one basis input")
    return values
