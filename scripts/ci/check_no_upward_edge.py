#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""No dependency edge of the library points up, dev edges included.

The crates of the library and of its tests are in one order:

    prism-field < prism-vectors < prism-isogeny < prism-verify
        < prism-testkit < shekyl-prism

A crate of that list may name, in any dependency table, only crates
earlier in it. `[dev-dependencies]` count: Cargo allows a crate's tests to
depend on a crate above it, and that is still a loop. A layer crate whose
tests need the front crate cannot be tested without it, and the loop has
to be undone before anything is published. So:

- none of `prism-field`, `prism-vectors`, `prism-isogeny` and
  `prism-verify` names `prism-testkit` or `shekyl-prism`;
- `prism-testkit` does not name `shekyl-prism`;
- none of the six names a tool of this workspace (the bench, the
  comparison, the signer), which all sit above the verifier.

A test lives in the lowest crate that contains everything it exercises.
A test that needs the builders of `prism-testkit`, or both entries of the
verifier, lives in `shekyl-prism/tests/` or in the testkit's own tests.

`check_vectors_isolation.py` holds the narrower rule that the vectors
crate names `prism-field` alone; this check does not repeat it.

1. every crate of the order is a workspace member with a manifest (the
   subject exists);
2. no manifest of the order names a workspace crate that is not earlier
   in the order, in any table, including a renamed package, a rename
   inherited from `[workspace.dependencies]` and a target-specific table;
3. `cargo tree` agrees: the graph of each crate of the order, over normal,
   build and dev edges and for every target, holds no workspace crate that
   is not earlier.

`--selftest` plants each breach of 1 and 2 in a copy of the manifests, and
checks that the graph reader sees a dev edge.
"""
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
# Lowest first. A crate may name only crates before it.
ORDER = (
    "prism-field",
    "prism-vectors",
    "prism-isogeny",
    "prism-verify",
    "prism-testkit",
    "shekyl-prism",
)
FRONT = "shekyl-prism"
EDGE_KINDS = ("dependencies", "dev-dependencies", "build-dependencies")


def manifest_of(crate):
    return f"{crate}/Cargo.toml"


def of_workspace(name):
    """Every member is `prism-*` but the front crate."""
    return name.startswith("prism-") or name == FRONT


def manifests():
    """The root manifest and that of each crate of the order, as text."""
    paths = [ROOT / "Cargo.toml"] + [ROOT / manifest_of(crate) for crate in ORDER]
    missing = [path for path in paths if not path.is_file()]
    if missing:
        raise SystemExit(f"expected manifest is absent: {missing[0]}")
    return {str(path.relative_to(ROOT)): path.read_text() for path in paths}


def package_name(key, value, inherited=None):
    """The package a dependency names, which is not always its table key.

    A dependency written `key = { workspace = true }` is whatever the root
    manifest's `[workspace.dependencies]` says `key` is, and that entry may
    rename a package too.
    """
    if isinstance(value, dict):
        named = value.get("package")
        if isinstance(named, str):
            return named
        if value.get("workspace") is True and inherited is not None:
            return package_name(key, inherited.get(key))
    return key


def workspace_table(root_text):
    """`[workspace.dependencies]` of the root manifest."""
    table = tomllib.loads(root_text).get("workspace", {}).get("dependencies", {})
    return table if isinstance(table, dict) else {}


def dependency_tables(manifest):
    """Every dependency table, including one scoped to a target."""
    for kind in EDGE_KINDS:
        table = manifest.get(kind)
        if isinstance(table, dict):
            yield kind, table
    targets = manifest.get("target")
    if not isinstance(targets, dict):
        return
    for target in targets.values():
        if not isinstance(target, dict):
            continue
        for kind in EDGE_KINDS:
            table = target.get(kind)
            if isinstance(table, dict):
                yield kind, table


def edges(text, inherited):
    """`(kind, package)` for every dependency `text` declares."""
    found = []
    for kind, table in dependency_tables(tomllib.loads(text)):
        for key, value in table.items():
            found.append((kind, package_name(key, value, inherited)))
    return found


def allowed(crate):
    """The crates `crate` may name: those before it in the order."""
    return set(ORDER[: ORDER.index(crate)])


def manifest_fault(texts):
    """The first breach of rules 1 and 2, or None."""
    for name in ("Cargo.toml", *map(manifest_of, ORDER)):
        if name not in texts:
            return f"{name} must be in the census"
    members = tomllib.loads(texts["Cargo.toml"])["workspace"]["members"]
    for crate in ORDER:
        if crate not in members:
            return f"{crate} is not a workspace member"
        declared = tomllib.loads(texts[manifest_of(crate)])
        if declared.get("package", {}).get("name") != crate:
            return f"{manifest_of(crate)} is not the manifest of {crate}"
    inherited = workspace_table(texts["Cargo.toml"])
    for crate in ORDER:
        below = allowed(crate)
        for kind, name in edges(texts[manifest_of(crate)], inherited):
            if of_workspace(name) and name not in below:
                return (
                    f"{crate} names {name} under [{kind}]: that edge points "
                    f"up. It may name only {sorted(below) or 'no workspace crate'}"
                )
    return None


def tree(package, kinds):
    """Package names in one resolved graph. An empty print is a blind reader."""
    result = subprocess.run(
        ["cargo", "tree", "--locked", "--prefix", "none", "-f", "{p}", "-e", kinds,
         "--target", "all", "-p", package],
        cwd=ROOT, text=True, capture_output=True,
    )
    if result.returncode:
        raise SystemExit(f"cargo tree failed for {package} ({kinds}):\n" + result.stderr)
    names = [line.split()[0] for line in result.stdout.splitlines() if line.split()]
    if not names:
        raise SystemExit(f"cargo tree printed nothing for {package} ({kinds})")
    return names


def graph_fault():
    """Where the resolver disagrees with the manifest rule, or None."""
    for crate in ORDER:
        above = sorted(
            name for name in set(tree(crate, "normal,build,dev"))
            if of_workspace(name) and name != crate and name not in allowed(crate)
        )
        if above:
            return f"the graph of {crate} over every edge kind holds {above}"
    return None


def selftest():
    real = manifests()
    fault = manifest_fault(real)
    if fault is not None:
        raise SystemExit("selftest: the tree itself fails: " + fault)

    def dev(crate, line):
        """`line` in the dev-dependencies of `crate`, whether it has any."""
        text = real[manifest_of(crate)]
        if "[dev-dependencies]\n" in text:
            return text.replace("[dev-dependencies]\n", "[dev-dependencies]\n" + line, 1)
        return text + "\n[dev-dependencies]\n" + line

    testkit = manifest_of("prism-testkit")
    planted = {
        "the verifier's tests take the front crate": {
            manifest_of("prism-verify"): dev(
                "prism-verify", 'shekyl-prism = { workspace = true }\n'
            ),
        },
        "the verifier's tests take the testkit": {
            manifest_of("prism-verify"): dev(
                "prism-verify", 'prism-testkit = { workspace = true }\n'
            ),
        },
        "the field crate's tests take the testkit": {
            manifest_of("prism-field"): dev(
                "prism-field", 'prism-testkit = { workspace = true }\n'
            ),
        },
        "the isogeny crate takes the front crate, renamed, in a target table": {
            manifest_of("prism-isogeny"): real[manifest_of("prism-isogeny")]
            + "\n[target.'cfg(all())'.dev-dependencies]\n"
            + 'front = { package = "shekyl-prism", path = "../shekyl-prism" }\n',
        },
        "the isogeny crate's tests take the verifier": {
            manifest_of("prism-isogeny"): dev(
                "prism-isogeny", 'prism-verify = { workspace = true }\n'
            ),
        },
        "the vectors crate takes the testkit as a build-dependency": {
            manifest_of("prism-vectors"): real[manifest_of("prism-vectors")]
            + "\n[build-dependencies]\n"
            + 'prism-testkit = { workspace = true }\n',
        },
        "the testkit's tests take the front crate": {
            testkit: dev("prism-testkit", 'shekyl-prism = { workspace = true }\n'),
        },
        "the testkit takes the front crate normally": {
            testkit: real[testkit].replace(
                "[dependencies]\n",
                '[dependencies]\nshekyl-prism = { workspace = true }\n',
                1,
            ),
        },
        "the verifier's tests take a tool": {
            manifest_of("prism-verify"): dev(
                "prism-verify", 'prism-signer = { workspace = true }\n'
            ),
        },
        "the front crate's tests take a tool": {
            manifest_of(FRONT): dev(FRONT, 'prism-bench = { path = "../prism-bench" }\n'),
        },
        "the verifier's tests take the front crate on one platform, under "
        "a name the workspace table gives it": {
            "Cargo.toml": real["Cargo.toml"].replace(
                "[workspace.dependencies]\n",
                '[workspace.dependencies]\n'
                'front = { package = "shekyl-prism", version = "0.1.0", path = "shekyl-prism" }\n',
                1,
            ),
            manifest_of("prism-verify"): real[manifest_of("prism-verify")]
            + "\n[target.'cfg(windows)'.dev-dependencies]\n"
            + 'front = { workspace = true }\n',
        },
        "the testkit leaves the workspace": {
            "Cargo.toml": real["Cargo.toml"].replace('    "prism-testkit",\n', "", 1),
        },
    }
    for case, change in planted.items():
        for name, text in change.items():
            if text == real[name]:
                raise SystemExit(f"selftest: could not plant '{case}' in {name}")
        if manifest_fault({**real, **change}) is None:
            raise SystemExit(f"selftest: not caught: {case}")
    # The front crate's tests take the testkit, which is a dev edge that
    # points down. A reader that did not follow dev edges would not see it.
    if "prism-testkit" in tree(FRONT, "normal,build"):
        raise SystemExit("selftest: the build graph of the front crate holds the testkit")
    if "prism-testkit" not in tree(FRONT, "normal,build,dev"):
        raise SystemExit("selftest: the graph reader does not see a dev edge")
    print(
        f"selftest: {len(planted)} planted manifests refused; "
        "the graph reader sees a dev edge"
    )


def main():
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        return
    if sys.argv[1:]:
        raise SystemExit("usage: check_no_upward_edge.py [--selftest]")
    texts = manifests()
    fault = manifest_fault(texts)
    if fault:
        raise SystemExit(fault)
    fault = graph_fault()
    if fault:
        raise SystemExit(fault)
    for crate in ORDER:
        named = sorted({
            name
            for _, name in edges(texts[manifest_of(crate)], workspace_table(texts["Cargo.toml"]))
            if of_workspace(name)
        })
        print(f"{crate}: names {', '.join(named) if named else 'no workspace crate'}")
    print(
        "no edge points up: each of " + ", ".join(ORDER)
        + " names only crates before it, dev edges included, and cargo tree agrees"
    )


if __name__ == "__main__":
    main()
