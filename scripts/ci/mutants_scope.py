#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Select compiled library sources and check cargo-mutants' actual inventory."""

import argparse
import json
import re
import shlex
import subprocess
from pathlib import Path

CONTROL_FILE = "shekyl-prism/src/ideal/mod.rs"
CONTROL = "replace * with + in Modulus<W, D>::mul"


def compiled_sources(packages, features):
    """Read dep-info only for the library artifacts Cargo selected in this build."""
    args = ["cargo", "check", "--lib", "--locked", "--message-format=json-render-diagnostics"]
    for package in packages:
        args += ["-p", package]
    if features:
        args += ["--features", features]
    artifacts = subprocess.check_output(args, text=True)
    sources = set()
    names = {name.replace("-", "_") for name in packages}
    seen = set()
    for line in artifacts.splitlines():
        message = json.loads(line)
        if message["reason"] != "compiler-artifact":
            continue
        target = message["target"]
        if target["name"] not in names or "lib" not in target["kind"]:
            continue
        meta, = (Path(f) for f in message["filenames"] if f.endswith(".rmeta"))
        deps = meta.with_name(meta.stem.removeprefix("lib") + ".d")
        _, dependencies = deps.read_text().splitlines()[0].split(": ", 1)
        for name in shlex.split(dependencies):
            if name.endswith(".rs"):
                path = Path(name).resolve()
                if path.is_relative_to(Path.cwd()):
                    sources.add(str(path.relative_to(Path.cwd())))
        seen.add(target["name"])
    if seen != names or not sources:
        raise ValueError(f"incomplete compiler inventory for {packages}: {seen}")
    return sources


def require_compiled(mutants, sources):
    if not mutants:
        raise ValueError("empty mutant inventory")
    missing = {m["file"] for m in mutants} - sources
    if missing:
        raise ValueError(f"mutants outside compiled sources: {sorted(missing)}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", required=True)
    parser.add_argument("--features", default="")
    parser.add_argument("--scope", default="")
    parser.add_argument("--inventory", type=Path, required=True)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--verify", type=Path)
    mode.add_argument("--control", action="store_true")
    args = parser.parse_args()
    if args.verify:
        require_compiled(json.loads(args.verify.read_text()), set(json.loads(args.inventory.read_text())))
        return
    packages = args.packages.split()
    sources = compiled_sources(packages, args.features)
    args.inventory.write_text(json.dumps(sorted(sources)))
    if args.control:
        mutants = ["cargo", "mutants", "-p", "shekyl-prism", "--features", args.features,
                   "--file", CONTROL_FILE, "--re", re.escape(CONTROL) + "$",
                   "--cargo-test-arg=--test=signer_layers", "--output", str(args.inventory.parent / "control")]
        planned = json.loads(subprocess.check_output([*mutants, "--list", "--json"]))
        require_compiled(planned, sources)
        if len(planned) != 1:
            raise ValueError(f"expected one positive control, found {len(planned)}")
        subprocess.run(mutants, check=True)
        caught = (args.inventory.parent / "control/mutants.out/caught.txt").read_text().splitlines()
        if caught != [planned[0]["name"]]:
            raise ValueError(f"positive control was not caught: {caught}")
        print("Caught positive control:", caught[0])
        return
    selected = sorted(p for p in sources if not args.scope or p.startswith(args.scope + "/"))
    if not selected:
        raise ValueError(f"no compiled source under {args.scope}")
    print("\n".join(selected))


if __name__ == "__main__":
    main()
