#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""What a consumer of this workspace builds, and what a package holds.

Two rules, both about what leaves the repository.

1. **A default build of the front crate is the verifier alone.** A node
   that only verifies must not compile signer code or the signer's
   dependencies. So the front crate `shekyl-prism` has one feature,
   `signer`, off by default, and:

   - its feature table is exactly `default = []` and
     `signer = ["dep:prism-int"]`, and `prism-int` is an optional
     dependency;
   - the default graph of the front crate, over normal and build edges,
     holds none of the signer's crates: `prism-int`, `zeroize`, `prism-ct-barrier`;
   - the graph with `--features signer` holds every one of them. Without
     this the second rule would pass on a graph reader that sees nothing.

2. **The integer crate's package holds no vectors.** The committed vector
   file of `prism-int` is 3.3 MB. `cargo package --list` must not name
   it, nor the test that cannot be built without it, and must name the
   crate's sources: a list that named nothing would pass the first half.

`--selftest` plants a breach of each clause in the values the checks read,
and requires each to be refused and the real values to pass.
"""
import copy
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
FRONT = "shekyl-prism"
INTEGERS = "prism-int"
FEATURES = {"default": [], "signer": [f"dep:{INTEGERS}"]}
# The crates only the signer needs. A default build of the front crate
# holds none, and a build with `signer` holds all.
SIGNER_ONLY = (INTEGERS, "zeroize", "prism-ct-barrier")
# In the repository, and not in the package.
NOT_PACKAGED = ("tests/data/vectors.tsv", "tests/vectors.rs")
# In the package: the subject of the list is there to be read.
PACKAGED = ("Cargo.toml", "src/lib.rs", "src/secret.rs", "tests/oracle.rs")


def run(*command):
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    if result.returncode:
        raise SystemExit(" ".join(command) + " failed:\n" + result.stderr)
    return result.stdout


def graph(*extra):
    """The crates of the front crate's normal and build graph, by name."""
    printed = run(
        "cargo", "tree", "--locked", "-p", FRONT, "-e", "normal,build",
        "--prefix", "none", "-f", "{p}", *extra,
    )
    return {line.split()[0] for line in printed.splitlines() if line.split()}


def package_list():
    """The paths `cargo package` would put in the integer crate's package."""
    # `--allow-dirty` so that the list can be read before a commit; the
    # list is of the working tree either way.
    printed = run("cargo", "package", "--list", "--locked", "--allow-dirty", "-p", INTEGERS)
    return {line.strip() for line in printed.splitlines() if line.strip()}


def closure_fault(manifest, default, signer):
    """The first breach of rule 1, or None."""
    if manifest.get("features") != FEATURES:
        return f"{FRONT}: [features] must be exactly {FEATURES}"
    dependency = manifest.get("dependencies", {}).get(INTEGERS)
    if not isinstance(dependency, dict) or dependency.get("optional") is not True:
        return f"{FRONT}: {INTEGERS} must be an optional dependency"
    if FRONT not in default or FRONT not in signer:
        return f"{FRONT} is not in its own build graph"
    for name in SIGNER_ONLY:
        if name in default:
            return f"the default build of {FRONT} holds {name}"
        if name not in signer:
            return f"the build of {FRONT} with `signer` does not hold {name}"
    return None


def package_fault(listed):
    """The first breach of rule 2, or None."""
    for path in PACKAGED:
        if path not in listed:
            return f"the package of {INTEGERS} does not hold {path}"
    for path in NOT_PACKAGED:
        if path in listed:
            return f"the package of {INTEGERS} holds {path}"
        if not (ROOT / INTEGERS / path).is_file():
            return f"{INTEGERS}/{path} is not in the repository"
    return None


def selftest(manifest, default, signer, listed):
    for fault in (closure_fault(manifest, default, signer), package_fault(listed)):
        if fault:
            raise SystemExit("selftest: the tree itself fails: " + fault)

    def planted(change):
        changed = copy.deepcopy(manifest)
        change(changed)
        return changed

    closures = {
        "the feature is on by default":
            (planted(lambda m: m["features"].__setitem__("default", ["signer"])),
             default, signer),
        "there is no feature table":
            (planted(lambda m: m.pop("features")), default, signer),
        "the feature turns on a feature of a crate":
            (planted(lambda m: m["features"]["signer"].append("prism-verify/op-count")),
             default, signer),
        "there is a second feature":
            (planted(lambda m: m["features"].__setitem__("extra", [])), default, signer),
        "the dependency is not optional":
            (planted(lambda m: m["dependencies"].__setitem__(INTEGERS, {"workspace": True})),
             default, signer),
        "the default build holds the integer crate":
            (manifest, default | {INTEGERS}, signer),
        "the default build holds zeroize": (manifest, default | {"zeroize"}, signer),
        "the default build holds prism-ct-barrier": (manifest, default | {"prism-ct-barrier"}, signer),
        "the signer build lacks the integer crate": (manifest, default, signer - {INTEGERS}),
        "the signer build lacks prism-ct-barrier": (manifest, default, signer - {"prism-ct-barrier"}),
        "the graph reader printed nothing": (manifest, set(), set()),
    }
    for name, arguments in closures.items():
        if closure_fault(*arguments) is None:
            raise SystemExit(f"selftest: not refused: {name}")
    packages = {
        "the package holds the vectors": listed | {NOT_PACKAGED[0]},
        "the package holds the vectors' test": listed | {NOT_PACKAGED[1]},
        "the package list is empty": set(),
        "the package lacks a source file": listed - {"src/secret.rs"},
    }
    for name, planted_list in packages.items():
        if package_fault(planted_list) is None:
            raise SystemExit(f"selftest: not refused: {name}")
    print(f"selftest: {len(closures) + len(packages)} planted breaches refused")


def main():
    manifest = tomllib.loads((ROOT / FRONT / "Cargo.toml").read_text())
    default = graph()
    signer = graph("--features", "signer")
    listed = package_list()
    if sys.argv[1:] == ["--selftest"]:
        selftest(manifest, default, signer, listed)
        return
    if sys.argv[1:]:
        raise SystemExit("usage: check_consumer_build.py [--selftest]")
    for fault in (closure_fault(manifest, default, signer), package_fault(listed)):
        if fault:
            raise SystemExit(fault)
    print(
        f"{FRONT}: the default build holds {len(default)} crates and none of "
        + ", ".join(SIGNER_ONLY)
        + f"; with `signer`, {len(signer)} and all of them. "
        + f"The package of {INTEGERS} holds {len(listed)} files and not its vectors."
    )


if __name__ == "__main__":
    main()
