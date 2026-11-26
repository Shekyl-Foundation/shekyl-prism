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


# The reducer calls ideal geometry through hazmat::ideal. The probe links
# shekyl-prism instead of compiling geometry.rs a second time.
WANTED_LIBRARIES = ("prism_int", "prism_field", "shekyl_prism")


def release_libraries():
    """Release rlibs the probe links, including the signer ideal layer."""
    messages = run(
        "cargo", "build", "--locked", "--release",
        "-p", "shekyl-prism", "--features", "signer",
        "--message-format=json",
    )
    libraries = {}
    for line in messages.splitlines():
        message = json.loads(line)
        name = message.get("target", {}).get("name")
        if name not in WANTED_LIBRARIES:
            continue
        matches = [
            Path(filename)
            for filename in message.get("filenames", [])
            if filename.endswith(".rlib")
        ]
        # An rmeta-only artifact is not the rlib. A later message may carry it.
        if len(matches) == 1:
            libraries[name] = matches[0]
    missing = [name for name in WANTED_LIBRARIES if name not in libraries]
    if missing:
        raise RuntimeError(f"release libraries missing: {missing}")
    return libraries


def dependency_dirs(libraries):
    """Directories that hold these rlibs and the crates they depend on.

    Cargo writes a directly built package under ``target/<profile>`` and its
    dependencies under ``deps``. Search both when the directory exists.
    """
    found = []
    for library in libraries.values():
        for candidate in (library.parent, library.parent / "deps"):
            if candidate.is_dir() and candidate not in found:
                found.append(candidate)
    return found


def build_command(libraries, name, binary):
    """The rustc line for one parameter set. Callers may append target features."""
    search = [arg for path in dependency_dirs(libraries) for arg in ("-L", f"dependency={path}")]
    return [
        "rustc", "--edition=2021", "-Copt-level=3", "-Cpanic=abort",
        "-Crelocation-model=static", "--cfg", f'stage2c_set="{name}"',
        "--extern", f"prism_int={libraries['prism_int']}",
        "--extern", f"prism_field={libraries['prism_field']}",
        "--extern", f"shekyl_prism={libraries['shekyl_prism']}",
        *search,
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
