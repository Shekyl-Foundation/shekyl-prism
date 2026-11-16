#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""`prism-vectors` is in no shipped build.

The crate reads committed test files. A shipped crate may name it only as
a dev-dependency, and only the crates whose tests read it do. The fuzz
crate is its own workspace and is not shipped; it depends on it normally.
The fixtures crate itself depends on `prism-field` and on nothing else of
this workspace, which is what lets the unit tests of `prism-isogeny` use it.

1. the crate is a workspace member, and every member manifest is in the
   census (the subject exists);
2. `prism-isogeny` and `prism-verify` name it as a dev-dependency and
   nowhere else, and the fuzz crate names it as a normal dependency;
3. no other manifest names it under any dependency table, including a
   renamed package and a target-specific table;
4. its own normal and build dependencies on this workspace are
   `prism-field` alone, again including renames and target-specific tables;
5. `cargo tree` agrees: the crate is on the dev edge of those two members
   and the normal edge of the fuzz crate, and on no normal or build edge
   of a shipped member.

`--selftest` plants each breach of 1–4 in a copy of the manifests, and
checks that the graph reader can tell a dev edge from a normal one.
"""
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = "prism-vectors"
ONLY_DEPENDENCY = "prism-field"
OWN = "prism-vectors/Cargo.toml"
DEV_USERS = ("prism-isogeny/Cargo.toml", "prism-verify/Cargo.toml")
NORMAL_USER = "fuzz/Cargo.toml"
# Kinds that put the crate into a build of the package itself.
SHIPPED_KINDS = ("dependencies", "build-dependencies")
EDGE_KINDS = ("dependencies", "dev-dependencies", "build-dependencies")


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


def package_name(key, value):
    """The package a dependency names, which is not always its table key."""
    if isinstance(value, dict):
        named = value.get("package")
        if isinstance(named, str):
            return named
    return key


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


def edges(text):
    """`(kind, package)` for every dependency `text` declares."""
    manifest = tomllib.loads(text)
    found = []
    for kind, table in dependency_tables(manifest):
        for key, value in table.items():
            found.append((kind, package_name(key, value)))
    return found


def kinds_of(text, package):
    return {kind for kind, name in edges(text) if name == package}


def workspace_dependencies(text):
    """Workspace crates `text` links for a normal or build of itself."""
    manifest = tomllib.loads(text)
    found = set()
    for kind, table in dependency_tables(manifest):
        if kind not in SHIPPED_KINDS:
            continue
        for key, value in table.items():
            name = package_name(key, value)
            if name.startswith("prism-"):
                found.add(name)
    return sorted(found)


def manifest_fault(texts):
    """The first breach of the manifest rules, or None."""
    for name in (OWN, NORMAL_USER, *DEV_USERS, "Cargo.toml"):
        if name not in texts:
            return f"{name} must be in the census"
    members = tomllib.loads(texts["Cargo.toml"])["workspace"]["members"]
    if PACKAGE not in members:
        return f"{PACKAGE} is not a workspace member"
    if workspace_dependencies(texts[OWN]) != [ONLY_DEPENDENCY]:
        return (
            f"{PACKAGE} depends on {workspace_dependencies(texts[OWN])}, "
            f"not on {ONLY_DEPENDENCY} alone"
        )
    for name, text in texts.items():
        kinds = kinds_of(text, PACKAGE)
        if name == OWN:
            if kinds:
                return f"{PACKAGE} depends on itself"
            continue
        if name == NORMAL_USER:
            if kinds != {"dependencies"}:
                return f"{NORMAL_USER} must depend on {PACKAGE} normally and nowhere else"
            continue
        if name in DEV_USERS:
            if kinds != {"dev-dependencies"}:
                return f"{name} must name {PACKAGE} under [dev-dependencies] and nowhere else"
            continue
        if kinds:
            return f"{name} names {PACKAGE} as a dependency ({', '.join(sorted(kinds))})"
    return None


def tree(edges, package=None, manifest=None):
    """Package names in one resolved graph. An empty print is a blind reader."""
    command = [
        "cargo", "tree", "--locked", "--prefix", "none", "-f", "{p}", "-e", edges,
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
    members = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["members"]
    for member in members:
        if member == PACKAGE:
            continue
        if PACKAGE in tree("normal,build", package=member):
            return f"the normal build of {member} contains {PACKAGE}"
    for member in ("prism-isogeny", "prism-verify"):
        if PACKAGE not in tree("dev", package=member):
            return f"the dev graph of {member} does not contain {PACKAGE}"
    if PACKAGE not in tree("normal", manifest=ROOT / "fuzz" / "Cargo.toml"):
        return f"the fuzz crate's normal graph does not contain {PACKAGE}"
    own = sorted(
        name for name in tree("normal,build", package=PACKAGE)
        if name.startswith("prism-") and name != PACKAGE
    )
    if own != [ONLY_DEPENDENCY]:
        return f"the resolved graph of {PACKAGE} is {own}, not {ONLY_DEPENDENCY} alone"
    return None


def selftest():
    real = manifests()
    fault = manifest_fault(real)
    if fault is not None:
        raise SystemExit("selftest: the tree itself fails: " + fault)
    planted = {
        "a shipped member renames the crate into its normal dependencies": {
            "prism-verify/Cargo.toml": real["prism-verify/Cargo.toml"].replace(
                "[dependencies]\n",
                '[dependencies]\n'
                'fixtures = { package = "prism-vectors", workspace = true }\n',
                1,
            ),
        },
        "a target table pulls the crate into a normal build": {
            "prism-signer/Cargo.toml": real["prism-signer/Cargo.toml"]
            + "\n[target.'cfg(unix)'.dependencies]\n"
            + 'prism-vectors = { workspace = true }\n',
        },
        "a user drops the dev-dependency": {
            "prism-isogeny/Cargo.toml": real["prism-isogeny/Cargo.toml"].replace(
                "prism-vectors = { workspace = true }\n",
                "",
                1,
            ),
        },
        "the fixtures crate gains another workspace crate through a renamed "
        "target build-dependency": {
            OWN: real[OWN]
            + "\n[target.'cfg(all())'.build-dependencies]\n"
            + 'isogeny = { package = "prism-isogeny", path = "../prism-isogeny" }\n',
        },
        "the fuzz crate drops the crate": {
            NORMAL_USER: real[NORMAL_USER].replace(
                'prism-vectors = { version = "0.1.0", path = "../prism-vectors" }\n',
                "",
                1,
            ),
        },
        "the crate leaves the workspace": {
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
        raise SystemExit("selftest: the normal graph reader sees the crate where it is absent")
    if PACKAGE not in tree("dev", package="prism-verify"):
        raise SystemExit("selftest: the dev graph reader does not see the crate")
    print(
        f"selftest: {len(planted)} planted manifests refused; "
        "the graph reader tells a dev edge from a normal one"
    )


def main():
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        return
    if sys.argv[1:]:
        raise SystemExit("usage: check_fixtures_isolation.py [--selftest]")
    fault = manifest_fault(manifests())
    if fault:
        raise SystemExit(fault)
    fault = graph_fault()
    if fault:
        raise SystemExit(fault)
    print(
        f"{PACKAGE}: a dev-dependency of prism-isogeny and prism-verify; "
        f"a normal dependency of the fuzz crate; in no shipped build; "
        f"depends on {ONLY_DEPENDENCY} alone"
    )


if __name__ == "__main__":
    main()
