#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Where `prism-vectors` may and may not appear in a dependency graph.

`prism-vectors` holds the committed test data and its readers. Two rules
keep it what it is:

- **No build of a library crate contains test data.** The library crates
  are `prism-field`, `prism-prime`, `prism-ideal`, `prism-isogeny`,
  `prism-verify` and the front crate `shekyl-prism`. Each may name
  `prism-vectors` as a dev-dependency and in no other kind.
- **The vectors crate does not depend on the code it tests.** It has
  none of `prism-verify`, `prism-isogeny` and `shekyl-prism` in its
  graph, in any kind, dev-dependencies included. A dev-dependency back onto the verifier
  would give the verifier's tests two copies of the verifier's types. Its
  only dependency on this workspace is `prism-field`.

Tools (the bench, the comparison, the signer, the fuzz crate) are not
constrained: they may depend on the vectors crate normally.

1. the crate is a workspace member, and every member manifest is in the
   census (the subject exists);
2. no library manifest names it under a normal or build table, including
   a renamed package, a rename inherited from `[workspace.dependencies]`
   and a target-specific table;
3. its own manifest names no workspace crate but `prism-field`, in any
   table, again including renames and target-specific tables;
4. `cargo tree` agrees: the crate is in no normal or build graph of a
   library crate, and its own graph over every edge kind holds no
   workspace crate but `prism-field`.

`--selftest` plants each breach of 1–3 in a copy of the manifests, and
checks that the graph reader can tell a dev edge from a normal one.
"""
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = "prism-vectors"
OWN = "prism-vectors/Cargo.toml"
ONLY_DEPENDENCY = "prism-field"
# The crates whose builds must not contain test data.
LIBRARY = (
    "prism-field",
    "prism-prime",
    "prism-ideal",
    "prism-isogeny",
    "prism-verify",
    "shekyl-prism",
)
# A crate of this workspace, by its name: every member is `prism-*` but the
# front crate.
FRONT = "shekyl-prism"
# Kinds that put a crate into a build of the package itself.
BUILD_KINDS = ("dependencies", "build-dependencies")
EDGE_KINDS = ("dependencies", "dev-dependencies", "build-dependencies")


def library_manifest(crate):
    return f"{crate}/Cargo.toml"


def manifests():
    """Every manifest this gate judges. A missing one is a failed census."""
    root_text = (ROOT / "Cargo.toml").read_text()
    members = tomllib.loads(root_text)["workspace"]["members"]
    paths = [ROOT / "Cargo.toml", ROOT / "fuzz" / "Cargo.toml"]
    paths += [ROOT / member / "Cargo.toml" for member in members]
    for extra in sorted(ROOT.glob("prism-*/Cargo.toml")):
        if extra not in paths:
            paths.append(extra)
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
    manifest = tomllib.loads(text)
    found = []
    for kind, table in dependency_tables(manifest):
        for key, value in table.items():
            found.append((kind, package_name(key, value, inherited)))
    return found


def kinds_of(text, package, inherited):
    return {kind for kind, name in edges(text, inherited) if name == package}


def workspace_dependencies(text, inherited):
    """Workspace crates `text` names in any dependency table, by kind."""
    return sorted(
        (name, kind) for kind, name in edges(text, inherited) if of_workspace(name)
    )


def of_workspace(name):
    return name.startswith("prism-") or name == FRONT


def manifest_fault(texts):
    """The first breach of the manifest rules, or None."""
    for name in (OWN, "Cargo.toml", *map(library_manifest, LIBRARY)):
        if name not in texts:
            return f"{name} must be in the census"
    members = tomllib.loads(texts["Cargo.toml"])["workspace"]["members"]
    if PACKAGE not in members:
        return f"{PACKAGE} is not a workspace member"
    inherited = workspace_table(texts["Cargo.toml"])
    own = workspace_dependencies(texts[OWN], inherited)
    if own != [(ONLY_DEPENDENCY, "dependencies")]:
        return (
            f"{PACKAGE} names {own} of this workspace; it may name "
            f"{ONLY_DEPENDENCY} as a normal dependency and nothing else"
        )
    for crate in LIBRARY:
        kinds = kinds_of(texts[library_manifest(crate)], PACKAGE, inherited)
        shipped = sorted(kinds & set(BUILD_KINDS))
        if shipped:
            return (
                f"{crate} names {PACKAGE} under {', '.join(shipped)}; "
                "a library crate may name it under [dev-dependencies] only"
            )
    return None


def tree(edges, package=None, manifest=None):
    """Package names in one resolved graph. An empty print is a blind reader."""
    command = [
        "cargo", "tree", "--locked", "--prefix", "none", "-f", "{p}", "-e", edges,
        "--target", "all",
    ]
    if manifest is not None:
        command += ["--manifest-path", str(manifest)]
    else:
        command += ["-p", package]
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    label = package if package is not None else manifest
    if result.returncode:
        raise SystemExit(f"cargo tree failed for {label} ({edges}):\n" + result.stderr)
    names = [line.split()[0] for line in result.stdout.splitlines() if line.split()]
    if not names:
        raise SystemExit(f"cargo tree printed nothing for {label} ({edges})")
    return names


def graph_fault():
    """Where the resolver disagrees with the manifest rules, or None."""
    for crate in LIBRARY:
        if PACKAGE in tree("normal,build", package=crate):
            return f"a build of {crate} contains {PACKAGE}"
    own = sorted(
        name for name in set(tree("normal,build,dev", package=PACKAGE))
        if of_workspace(name) and name != PACKAGE
    )
    if own != [ONLY_DEPENDENCY]:
        return (
            f"the graph of {PACKAGE} over every edge kind holds {own}, "
            f"not {ONLY_DEPENDENCY} alone"
        )
    return None


def selftest():
    real = manifests()
    fault = manifest_fault(real)
    if fault is not None:
        raise SystemExit("selftest: the tree itself fails: " + fault)
    verify = library_manifest("prism-verify")
    isogeny = library_manifest("prism-isogeny")
    field = library_manifest("prism-field")
    front = library_manifest(FRONT)
    planted = {
        "a library crate renames the vectors crate into its normal dependencies": {
            verify: real[verify].replace(
                "[dependencies]\n",
                '[dependencies]\n'
                'vectors = { package = "prism-vectors", workspace = true }\n',
                1,
            ),
        },
        "a target table pulls the vectors crate into a library build": {
            isogeny: real[isogeny]
            + "\n[target.'cfg(unix)'.build-dependencies]\n"
            + 'prism-vectors = { workspace = true }\n',
        },
        "the field crate takes the vectors crate normally": {
            field: real[field].replace(
                "[dependencies]\n",
                '[dependencies]\nprism-vectors = { workspace = true }\n',
                1,
            ),
        },
        "the front crate takes the vectors crate normally": {
            front: real[front].replace(
                "[dependencies]\n",
                '[dependencies]\nprism-vectors = { workspace = true }\n',
                1,
            ),
        },
        "the vectors crate takes the front crate as a dev-dependency": {
            OWN: real[OWN]
            + "\n[dev-dependencies]\n"
            + 'shekyl-prism = { workspace = true }\n',
        },
        "the vectors crate takes the verifier as a dev-dependency": {
            OWN: real[OWN]
            + "\n[dev-dependencies]\n"
            + 'prism-verify = { workspace = true }\n',
        },
        "the vectors crate takes the isogeny crate through a renamed target "
        "build-dependency": {
            OWN: real[OWN]
            + "\n[target.'cfg(all())'.build-dependencies]\n"
            + 'isogeny = { package = "prism-isogeny", path = "../prism-isogeny" }\n',
        },
        "a library crate takes the vectors crate normally on one platform, "
        "under a name the workspace table gives it": {
            "Cargo.toml": real["Cargo.toml"].replace(
                "[workspace.dependencies]\n",
                '[workspace.dependencies]\n'
                'data = { package = "prism-vectors", version = "0.1.0", path = "prism-vectors" }\n',
                1,
            ),
            verify: real[verify]
            + "\n[target.'cfg(windows)'.dependencies]\n"
            + 'data = { workspace = true }\n',
        },
        "the vectors crate drops the field crate": {
            OWN: real[OWN].replace("prism-field = { workspace = true }\n", "", 1),
        },
        "the vectors crate leaves the workspace": {
            "Cargo.toml": real["Cargo.toml"].replace('    "prism-vectors",\n', "", 1),
        },
    }
    for case, change in planted.items():
        for name, text in change.items():
            if text == real[name]:
                raise SystemExit(f"selftest: could not plant '{case}' in {name}")
        if manifest_fault({**real, **change}) is None:
            raise SystemExit(f"selftest: not caught: {case}")
    if PACKAGE in tree("normal,build", package="prism-verify"):
        raise SystemExit("selftest: the build graph reader sees the crate where it is absent")
    if PACKAGE not in tree("dev", package="prism-verify"):
        raise SystemExit("selftest: the dev graph reader does not see the crate")
    print(
        f"selftest: {len(planted)} planted manifests refused; "
        "the graph reader tells a dev edge from a build edge"
    )


def main():
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        return
    if sys.argv[1:]:
        raise SystemExit("usage: check_vectors_isolation.py [--selftest]")
    fault = manifest_fault(manifests())
    if fault:
        raise SystemExit(fault)
    fault = graph_fault()
    if fault:
        raise SystemExit(fault)
    print(
        f"{PACKAGE}: in no build of {', '.join(LIBRARY)}; "
        f"its own graph, dev edges included, holds {ONLY_DEPENDENCY} alone"
    )


if __name__ == "__main__":
    main()
