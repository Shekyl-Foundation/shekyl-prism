#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Hold the reference code of prism-isogeny out of every build but the fuzz crate's.

`oracle` is a feature of prism-isogeny. It makes public code that is kept
for reference and is no part of the verifier: the affine Miller loop the
pairing replaced, which a fuzz target compares with the pairing, and the
short-Weierstrass model of a split (2,2)-isogeny. Cargo unifies features
across a build, so a workspace member naming it would put that code in
the verifier everyone links. This check requires that:

1. prism-isogeny declares `oracle`, off by default (the subject exists);
2. fuzz/Cargo.toml asks for it exactly once, and no other manifest in the
   repository names it. The manifests are every `Cargo.toml` git tracks,
   and the set must hold the workspace's, the owner's, the fuzz crate's
   and the front crate's, so a manifest the listing lost is a failure and
   not a pass;
3. the resolved feature graph of prism-isogeny under prism-verify, and
   under the front crate shekyl-prism, does not contain it by default,
   and does contain it when given on the command line (so the check can
   see the feature it says is absent).

`--selftest` runs the manifest rule against planted manifests and the
graph reader against the requested feature, without touching the tree.
"""
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
FEATURE = "oracle"
OWNER = "prism-isogeny/Cargo.toml"
USER = "fuzz/Cargo.toml"
FRONT = "shekyl-prism/Cargo.toml"
# The manifests the rule cannot be said to hold without having read.
REQUIRED = ("Cargo.toml", OWNER, USER, FRONT)
# The packages whose build must not contain the feature: the verifier,
# and the front crate, which depends on prism-isogeny directly.
GRAPHS = ("prism-verify", "shekyl-prism")


def manifests():
    """Every Cargo.toml git tracks, by its path from the root."""
    result = subprocess.run(
        ["git", "ls-files", "-z", "--", "Cargo.toml", "*/Cargo.toml"],
        cwd=ROOT, text=True, capture_output=True,
    )
    if result.returncode:
        raise SystemExit("git ls-files failed:\n" + result.stderr)
    names = sorted(name for name in result.stdout.split("\0") if name)
    missing = [name for name in names if not (ROOT / name).is_file()]
    if missing:
        raise SystemExit(f"tracked manifest is absent from the tree: {missing[0]}")
    return {name: (ROOT / name).read_text() for name in names}


def mentions(text):
    """How often manifest content, comments aside, names the feature."""
    content = "\n".join(line.split("#", 1)[0] for line in text.splitlines())
    return content.count(FEATURE)


def manifest_fault(texts):
    """The first breach of rules 1 and 2 in a set of manifests, or None."""
    for name in REQUIRED:
        if name not in texts:
            return f"{name} is not among the manifests read"
    features = tomllib.loads(texts[OWNER]).get("features", {})
    if features.get(FEATURE) != []:
        return f"prism-isogeny must declare {FEATURE} with no dependencies"
    if features.get("default") != []:
        return "prism-isogeny default features must be empty"
    wanted = (
        tomllib.loads(texts[USER]).get("dependencies", {}).get("prism-isogeny", {})
    )
    if not isinstance(wanted, dict) or wanted.get("features") != [FEATURE]:
        return f"{USER} must ask prism-isogeny for exactly [{FEATURE!r}]"
    for name, text in texts.items():
        allowed = 1 if name in (OWNER, USER) else 0
        if mentions(text) != allowed:
            return f"{name} names {FEATURE} {mentions(text)} times, expected {allowed}"
    return None


def isogeny_features(package, *extra):
    """The features prism-isogeny is built with under `package`."""
    result = subprocess.run(
        ["cargo", "tree", "--locked", "-p", package, "-e", "normal",
         "--prefix", "none", "-f", "{p}\t{f}", *extra],
        cwd=ROOT, text=True, capture_output=True,
    )
    if result.returncode:
        raise SystemExit("cargo tree failed:\n" + result.stderr)
    rows = [line.split("\t")[1] for line in result.stdout.splitlines()
            if line.startswith("prism-isogeny ") and "\t" in line]
    if not rows:
        raise SystemExit(f"prism-isogeny is not in the graph of {package}")
    # A repeated node is printed with a trailing " (*)".
    return {name for row in rows for name in row.replace(" (*)", "").split(",") if name}


def requested(package):
    return isogeny_features(package, "--features", f"prism-isogeny/{FEATURE}")


def selftest():
    real = manifests()
    if manifest_fault(real) is not None:
        raise SystemExit("selftest: the tree itself fails: " + manifest_fault(real))
    planted = {
        "a workspace member asks for the feature": {
            "prism-verify/Cargo.toml": real["prism-verify/Cargo.toml"].replace(
                "prism-isogeny = { workspace = true }",
                'prism-isogeny = { workspace = true, features = ["oracle"] }'),
        },
        "the front crate asks for the feature": {
            FRONT: real[FRONT].replace(
                "prism-isogeny = { workspace = true }",
                'prism-isogeny = { workspace = true, features = ["oracle"] }'),
        },
        "the feature is on by default": {
            OWNER: real[OWNER].replace("default = []", 'default = ["oracle"]'),
        },
        "the fuzz crate no longer asks for it": {
            USER: real[USER].replace(', features = ["oracle"]', ""),
        },
        "the feature is no longer declared": {
            OWNER: real[OWNER].replace("oracle = []\n", ""),
        },
    }
    for case, change in planted.items():
        for name, text in change.items():
            if text == real[name]:
                raise SystemExit(f"selftest: could not plant '{case}' in {name}")
        if manifest_fault({**real, **change}) is None:
            raise SystemExit(f"selftest: not caught: {case}")
    for name in REQUIRED:
        lost = {other: text for other, text in real.items() if other != name}
        if manifest_fault(lost) is None:
            raise SystemExit(f"selftest: not caught: {name} was not read")
    for package in GRAPHS:
        if FEATURE not in requested(package):
            raise SystemExit(
                f"selftest: the graph reader does not see a requested oracle under {package}")
    print(f"selftest: {len(planted)} planted manifests refused, and a set missing "
          f"any of the {len(REQUIRED)} required manifests; the graph reader sees "
          f"the feature when it is requested under each of {len(GRAPHS)} packages")


def main():
    if sys.argv[1:] == ["--selftest"]:
        return selftest()
    if sys.argv[1:]:
        raise SystemExit("usage: check_oracle_isolation.py [--selftest]")
    texts = manifests()
    fault = manifest_fault(texts)
    if fault:
        raise SystemExit(fault)
    for package in GRAPHS:
        if FEATURE in isogeny_features(package):
            raise SystemExit(f"the default feature graph of {package} contains oracle")
        if FEATURE not in requested(package):
            raise SystemExit(
                f"oracle did not appear when requested under {package}; "
                "this check cannot see it")
    print(f"oracle is absent from the default feature graphs of {' and '.join(GRAPHS)} "
          f"and named by none of the {len(texts)} tracked manifests but "
          "prism-isogeny's and the fuzz crate's")


if __name__ == "__main__":
    main()
