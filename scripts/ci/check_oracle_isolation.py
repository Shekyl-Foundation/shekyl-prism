#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Hold the replaced-routine oracles out of every build but the fuzz crate's.

`oracle` is a feature of prism-isogeny. It makes public the routines an
optimized routine replaced (the affine Miller loop), so a fuzz target can
compare the two. Cargo unifies features across a build, so a workspace
member naming it would put those routines in the verifier everyone links.
This check requires that:

1. prism-isogeny declares `oracle`, off by default (the subject exists);
2. fuzz/Cargo.toml asks for it exactly once, and no other manifest in the
   repository names it;
3. the resolved feature graph of prism-verify does not contain it by
   default, and does contain it when given on the command line (so the
   check can see the feature it says is absent).

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


def manifests():
    found = [ROOT / "Cargo.toml", ROOT / "fuzz" / "Cargo.toml"]
    found += sorted(ROOT.glob("prism-*/Cargo.toml"))
    missing = [p for p in found if not p.is_file()]
    if missing:
        raise SystemExit(f"expected manifest is absent: {missing[0]}")
    return {str(p.relative_to(ROOT)): p.read_text() for p in found}


def mentions(text):
    """How often manifest content, comments aside, names the feature."""
    content = "\n".join(line.split("#", 1)[0] for line in text.splitlines())
    return content.count(FEATURE)


def manifest_fault(texts):
    """The first breach of rules 1 and 2 in a set of manifests, or None."""
    if OWNER not in texts or USER not in texts:
        return f"{OWNER} and {USER} must both be checked"
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


def isogeny_features(*extra):
    """The features prism-isogeny is built with under prism-verify."""
    result = subprocess.run(
        ["cargo", "tree", "--locked", "-p", "prism-verify", "-e", "normal",
         "--prefix", "none", "-f", "{p}\t{f}", *extra],
        cwd=ROOT, text=True, capture_output=True,
    )
    if result.returncode:
        raise SystemExit("cargo tree failed:\n" + result.stderr)
    rows = [line.split("\t")[1] for line in result.stdout.splitlines()
            if line.startswith("prism-isogeny ") and "\t" in line]
    if not rows:
        raise SystemExit("prism-isogeny is not in the graph of prism-verify")
    # A repeated node is printed with a trailing " (*)".
    return {name for row in rows for name in row.replace(" (*)", "").split(",") if name}


def requested():
    return isogeny_features("--features", f"prism-isogeny/{FEATURE}")


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
    if FEATURE not in requested():
        raise SystemExit("selftest: the graph reader does not see a requested oracle")
    print(f"selftest: {len(planted)} planted manifests refused; "
          "the graph reader sees the feature when it is requested")


def main():
    if sys.argv[1:] == ["--selftest"]:
        return selftest()
    if sys.argv[1:]:
        raise SystemExit("usage: check_oracle_isolation.py [--selftest]")
    fault = manifest_fault(manifests())
    if fault:
        raise SystemExit(fault)
    if FEATURE in isogeny_features():
        raise SystemExit("the default feature graph of prism-verify contains oracle")
    if FEATURE not in requested():
        raise SystemExit("oracle did not appear when requested; this check cannot see it")
    print("oracle is absent from the default feature graph of prism-verify "
          "and named by no manifest but prism-isogeny's and the fuzz crate's")


if __name__ == "__main__":
    main()
