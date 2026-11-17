#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Hold the operation counter, and its API, out of every default build.

The counter is measuring equipment. `prism-field` has it under its feature
`op-count`, and `prism-verify` has a feature of the same name that marks
the verifier's stages and turns the field crate's on. Cargo unifies
features across a build, so any manifest that enabled either would turn
the counter on for every crate built with it. The count run asks for it on
the command line, `--features prism-verify/op-count`, and nothing else
does. This check requires that:

1. `prism-field` declares `op-count` as `[]`, and `prism-verify` declares
   `op-count` as exactly `["prism-field/op-count"]` (the subject exists,
   and the verifier's feature cannot be on with the counter off);
2. no other manifest declares a feature of that name, and no manifest
   enables either one: not in a default set, not from another feature, not
   in the `features` of a dependency, in any table including
   `[workspace.dependencies]` and target-specific tables;
3. the resolved graph of every workspace member, over every edge kind,
   has the feature on neither crate by default, and has it on both when it
   is requested (so the check can see the feature it says is absent);
4. a consumer of a default build cannot name `prism_field::ops`,
   `prism_verify::stage` or `prism_verify::Stage`, and can name all three
   when it enables `prism-verify/op-count` and nothing else; and the two
   private modules the crates mark through (`prism_field::tally`,
   `prism_verify::mark`) cannot be named in either build. The same holds
   through the front crate, whose `hazmat` module re-exports both crates:
   `shekyl_prism::hazmat::field::ops` and `shekyl_prism::hazmat::verify::stage`
   are absent by default and present when the feature is asked for. This
   compiles an isolated crate: it tests that the API is absent, and does
   not search the source.

5. the count reporter, the example `op_counts` of `prism-testkit`, has
   `required-features = ["prism-verify/op-count"]`. That line asks for
   the feature and does not turn it on: a default build skips the
   reporter, and a build of it without the feature is refused by Cargo
   instead of printing zeros. The testkit declares no feature of its own.

`--selftest` plants each breach of 1, 2 and 5 in a copy of the manifests.

The consumer is built with `--offline` from a copy of the tree's lock file,
so the locked dependencies must be in the registry first
(`cargo fetch --locked`).
"""
import copy
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FEATURE = "op-count"
FIELD = "prism-field"
VERIFY = "prism-verify"
FRONT = "shekyl-prism"
# The crate whose example `REPORTER` writes the count records.
TESTKIT = "prism-testkit"
REPORTER = "op_counts"
# What each of the two crates must declare the feature as.
DECLARED = {FIELD: [], VERIFY: [f"{FIELD}/{FEATURE}"]}
DEPENDENCY_KINDS = ("dependencies", "dev-dependencies", "build-dependencies")


def manifest_of(crate):
    return f"{crate}/Cargo.toml"


def manifests():
    """Every manifest this gate judges, parsed. A missing one fails."""
    root = tomllib.loads((ROOT / "Cargo.toml").read_text())
    paths = [ROOT / "Cargo.toml", ROOT / "fuzz" / "Cargo.toml"]
    paths += [ROOT / member / "Cargo.toml" for member in root["workspace"]["members"]]
    for extra in sorted(ROOT.glob("prism-*/Cargo.toml")):
        if extra not in paths:
            paths.append(extra)
    missing = [path for path in paths if not path.is_file()]
    if missing:
        raise SystemExit(f"expected manifest is absent: {missing[0]}")
    return {
        str(path.relative_to(ROOT)): tomllib.loads(path.read_text()) for path in paths
    }


def names_feature(entry):
    """Whether one entry of a feature list is the feature, on any crate.

    `op-count`, `prism-field/op-count` and `prism-field?/op-count` all are.
    """
    return isinstance(entry, str) and entry.rsplit("/", 1)[-1] == FEATURE


def string_list(value):
    """Whether a value is what a feature list must be.

    A string in place of a list would be read letter by letter, and no
    letter names the feature.
    """
    return isinstance(value, list) and all(isinstance(item, str) for item in value)


def dependency_tables(manifest):
    """`(where, table)` for every dependency table of a manifest."""
    for kind in DEPENDENCY_KINDS:
        table = manifest.get(kind)
        if isinstance(table, dict):
            yield kind, table
    workspace = manifest.get("workspace")
    if isinstance(workspace, dict) and isinstance(workspace.get("dependencies"), dict):
        yield "workspace.dependencies", workspace["dependencies"]
    targets = manifest.get("target")
    if isinstance(targets, dict):
        for name, target in targets.items():
            if not isinstance(target, dict):
                continue
            for kind in DEPENDENCY_KINDS:
                table = target.get(kind)
                if isinstance(table, dict):
                    yield f"target.{name}.{kind}", table


def manifest_fault(parsed):
    """The first breach of rules 1 and 2, or None."""
    for crate in (FIELD, VERIFY):
        if manifest_of(crate) not in parsed:
            return f"{manifest_of(crate)} must be in the census"
    for name, manifest in parsed.items():
        features = manifest.get("features", {})
        if not isinstance(features, dict):
            return f"{name}: [features] is not a table"
        crate = name.removesuffix("/Cargo.toml")
        expected = DECLARED.get(crate)
        if expected is None:
            if FEATURE in features:
                return (
                    f"{name} declares {FEATURE}; only {FIELD} and {VERIFY} may"
                )
        elif features.get(FEATURE) != expected:
            return (
                f"{name} must declare {FEATURE} = {json.dumps(expected)}, "
                f"and has {json.dumps(features.get(FEATURE))}"
            )
        if expected is not None and features.get("default") != []:
            return f"{name}: default features must be declared, and empty"
        for feature, entries in features.items():
            if not string_list(entries):
                return f"{name}: feature `{feature}` is not a list of strings"
            if feature == FEATURE and expected is not None:
                continue
            if any(names_feature(entry) for entry in entries):
                return f"{name}: feature `{feature}` turns {FEATURE} on"
        for where, table in dependency_tables(manifest):
            for key, value in table.items():
                if not isinstance(value, dict):
                    continue
                enabled = value.get("features", [])
                if not string_list(enabled):
                    return f"{name}: [{where}] {key}: features is not a list of strings"
                if any(names_feature(entry) for entry in enabled):
                    return f"{name}: [{where}] {key} enables {FEATURE}"
    return reporter_fault(parsed)


def reporter_fault(parsed):
    """The first breach of rule 5, or None."""
    name = manifest_of(TESTKIT)
    if name not in parsed:
        return f"{name} must be in the census"
    manifest = parsed[name]
    if "features" in manifest:
        return f"{name} declares features; the testkit must declare none"
    examples = manifest.get("example", [])
    found = [
        example for example in examples
        if isinstance(example, dict) and example.get("name") == REPORTER
    ] if isinstance(examples, list) else []
    if len(found) != 1:
        return f"{name} must declare the example `{REPORTER}` once"
    required = [f"{VERIFY}/{FEATURE}"]
    if found[0].get("required-features") != required:
        return (
            f"{name}: the example `{REPORTER}` must have required-features = "
            f"{json.dumps(required)}, and has "
            f"{json.dumps(found[0].get('required-features'))}"
        )
    return None


def members():
    root = tomllib.loads((ROOT / "Cargo.toml").read_text())
    return root["workspace"]["members"]


def node_features(package, *extra):
    """The features `prism-field` and `prism-verify` have in one graph."""
    result = subprocess.run(
        ["cargo", "tree", "--locked", "-p", package, "-e", "normal,build,dev",
         "--prefix", "none", "-f", "{p}\t{f}", *extra],
        cwd=ROOT, text=True, capture_output=True,
    )
    if result.returncode:
        raise SystemExit(f"cargo tree failed for {package}:\n" + result.stderr)
    found = {}
    for line in result.stdout.splitlines():
        if "\t" not in line:
            continue
        node, listed = line.split("\t", 1)
        crate = node.split()[0]
        if crate in (FIELD, VERIFY):
            # A repeated node is printed with a trailing " (*)".
            names = listed.replace(" (*)", "").split(",")
            found.setdefault(crate, set()).update(n for n in names if n)
    if not result.stdout.strip():
        raise SystemExit(f"cargo tree printed nothing for {package}")
    return found


def graph_fault():
    """Where a resolved graph breaks rule 3, or None."""
    seen_field = False
    for member in members():
        found = node_features(member)
        seen_field = seen_field or FIELD in found
        for crate, features in found.items():
            if FEATURE in features:
                return f"the default graph of {member} has {FEATURE} on {crate}"
    if not seen_field:
        return f"{FIELD} is in no member's graph; this check sees nothing"
    requested = node_features(VERIFY, "--features", f"{VERIFY}/{FEATURE}")
    for crate in (FIELD, VERIFY):
        if FEATURE not in requested.get(crate, set()):
            return (
                f"{FEATURE} is not on {crate} when {VERIFY}/{FEATURE} is "
                "requested; this check cannot see it"
            )
    return None


# What a default consumer must be refused, with the compiler's error code
# for the refusal. Each source is one line, so the code says why that line
# failed, and a code is steadier across toolchains than a message's words.
# The two public modules are not compiled in a default build, so the
# compiler finds no such name. The two modules the crates mark through are
# compiled and private; the feature must not open those either.
ABSENT = "E0432"   # unresolved import
PRIVATE = "E0603"  # the item is private
REFUSED = (
    ("pub use prism_field::ops;\n", ABSENT),
    ("pub use prism_field::ops::take;\n", ABSENT),
    ("pub use prism_verify::stage;\n", ABSENT),
    ("pub use prism_verify::Stage;\n", ABSENT),
    ("pub use shekyl_prism::hazmat::field::ops;\n", ABSENT),
    ("pub use shekyl_prism::hazmat::verify::stage;\n", ABSENT),
    ("pub use prism_field::tally::tally;\n", PRIVATE),
    ("pub use prism_verify::mark::Stage;\n", PRIVATE),
)
# The same two, with the feature on.
STILL_PRIVATE = REFUSED[6:]
ALLOWED = (
    "pub use prism_field::ops::{enter, take, tally, Counts, Kind, Op, Slot, ENABLED, SLOTS};\n"
    "pub use prism_verify::stage;\n"
    "pub use prism_verify::Stage;\n"
    "pub const NAMES: usize = Stage::ALL.len();\n"
    "pub use shekyl_prism::hazmat::field::ops::take as front_take;\n"
    "pub use shekyl_prism::hazmat::verify::stage::Stage as FrontStage;\n"
)


def error_codes(stdout):
    """The codes of the compiler's errors in `--message-format=json` output."""
    codes = set()
    for line in stdout.splitlines():
        if not line.startswith("{"):
            continue
        record = json.loads(line)
        message = record.get("message")
        if record.get("reason") != "compiler-message" or not isinstance(message, dict):
            continue
        if message.get("level") == "error" and message.get("code"):
            codes.add(message["code"]["code"])
    return codes


def rendered(result):
    """What the compiler said, for a failure message."""
    said = []
    for line in result.stdout.splitlines():
        if line.startswith("{"):
            message = json.loads(line).get("message")
            if isinstance(message, dict) and message.get("rendered"):
                said.append(message["rendered"])
    return "".join(said) + result.stderr


def api_fault():
    """Where the compiled API breaks rule 4, or None."""
    env = dict(os.environ)
    env.pop("CARGO_TARGET_DIR", None)
    env["CARGO_BUILD_JOBS"] = "2"
    with tempfile.TemporaryDirectory(prefix="prism-op-count-api-") as directory:
        tmp = Path(directory)
        (tmp / "src").mkdir()
        field = json.dumps(str(ROOT / FIELD))
        verify = json.dumps(str(ROOT / VERIFY))
        front = json.dumps(str(ROOT / FRONT))
        (tmp / "Cargo.toml").write_text(
            '[package]\nname="prism-op-count-api"\nversion="0.0.0"\nedition="2021"\n'
            f'[workspace]\n[features]\ncount=["{VERIFY}/{FEATURE}"]\n'
            f"[dependencies]\n{FIELD}={{path={field}}}\n{VERIFY}={{path={verify}}}\n"
            f"{FRONT}={{path={front}}}\n"
        )
        # The consumer is its own workspace. It starts from the tree's lock
        # file, so cargo keeps the versions the tree is locked to and only
        # drops the packages this crate does not use.
        shutil.copyfile(ROOT / "Cargo.lock", tmp / "Cargo.lock")
        command = ["cargo", "check", "--offline", "--message-format=json",
                   "--manifest-path", str(tmp / "Cargo.toml")]
        source = tmp / "src/lib.rs"

        def build(text, *extra):
            source.write_text(text)
            return subprocess.run(command + list(extra), env=env, text=True, capture_output=True)

        default = build(
            "pub use prism_field::Field;\npub use prism_verify::verify;\n"
            "pub use shekyl_prism::hazmat::verify::verify as front_verify;\n"
        )
        if default.returncode:
            return "a default consumer of both crates failed to build:\n" + rendered(default)
        for text, code in REFUSED:
            refused = build(text)
            if refused.returncode == 0:
                return f"a default build compiles `{text.strip()}`"
            if error_codes(refused.stdout) != {code}:
                return (
                    f"`{text.strip()}` failed, but not with {code} alone:\n"
                    + rendered(refused)
                )
        for text, code in STILL_PRIVATE:
            refused = build(text, "--features", "count")
            if refused.returncode == 0 or error_codes(refused.stdout) != {code}:
                return (
                    f"with the feature on, `{text.strip()}` is not refused "
                    f"with {code} alone:\n" + rendered(refused)
                )
        counted = build(ALLOWED, "--features", "count")
        if counted.returncode:
            return (
                f"a consumer that enables {VERIFY}/{FEATURE} cannot name the "
                "counter's API:\n" + rendered(counted)
            )
    return None


def selftest():
    real = manifests()
    fault = manifest_fault(real)
    if fault is not None:
        raise SystemExit("selftest: the tree itself fails: " + fault)
    field, verify = manifest_of(FIELD), manifest_of(VERIFY)
    bench, isogeny = manifest_of("prism-bench"), manifest_of("prism-isogeny")
    front = manifest_of(FRONT)
    testkit = manifest_of(TESTKIT)

    def plant(name, change):
        manifest = copy.deepcopy(real[name])
        change(manifest)
        return {name: manifest}

    def feature(key, value):
        return lambda m: m.setdefault("features", {}).__setitem__(key, value)

    def dependency(table, key, value):
        def change(m):
            node = m
            for part in table:
                node = node.setdefault(part, {})
            node[key] = value
        return change

    on = {"workspace": True, "features": [FEATURE]}
    planted = {
        "a third crate declares the feature":
            plant(isogeny, feature(FEATURE, [])),
        "a tool has a feature that turns the verifier's on":
            plant(bench, feature("count", [f"{VERIFY}/{FEATURE}"])),
        "a tool has a feature that turns the field crate's on, weakly":
            plant(bench, feature("count", [f"{FIELD}?/{FEATURE}"])),
        "a tool enables it on its dependency":
            plant(bench, dependency(("dependencies",), VERIFY, on)),
        "a dev-dependency enables it":
            plant(isogeny, dependency(("dev-dependencies",), FIELD, on)),
        "a target-specific build-dependency of the fuzz crate enables it":
            plant("fuzz/Cargo.toml", dependency(
                ("target", "cfg(unix)", "build-dependencies"), FIELD,
                {"path": "../prism-field", "features": [FEATURE]})),
        "the workspace's dependency table enables it for every member":
            plant("Cargo.toml", dependency(
                ("workspace", "dependencies"), FIELD,
                {"version": "0.1.0", "path": "prism-field", "features": [FEATURE]})),
        "the verifier enables it on its own dependency":
            plant(verify, dependency(("dependencies",), FIELD, on)),
        "the field crate has it by default":
            plant(field, feature("default", [FEATURE])),
        "the verifier has it by default":
            plant(verify, feature("default", [FEATURE])),
        "the verifier's default set is not declared":
            plant(verify, lambda m: m["features"].pop("default")),
        "the verifier's feature does not turn the counter on":
            plant(verify, feature(FEATURE, [])),
        "the verifier's feature turns something else on as well":
            plant(verify, feature(FEATURE, [f"{FIELD}/{FEATURE}", "research-reduced-width"])),
        "another feature of the verifier turns it on":
            plant(verify, feature("research-reduced-width", [FEATURE])),
        "the field crate's feature is not empty":
            plant(field, feature(FEATURE, ["fp2/std"])),
        "the field crate drops the feature":
            plant(field, lambda m: m["features"].pop(FEATURE)),
        "the verifier drops the feature":
            plant(verify, lambda m: m["features"].pop(FEATURE)),
        "the front crate declares the feature":
            plant(front, feature(FEATURE, [f"{VERIFY}/{FEATURE}"])),
        "the front crate enables it on the verifier":
            plant(front, dependency(("dependencies",), VERIFY, on)),
        "the count reporter stops requiring the feature":
            plant(testkit, lambda m: m["example"][0].pop("required-features")),
        "the count reporter requires the field crate's feature alone":
            plant(testkit, lambda m: m["example"][0].__setitem__(
                "required-features", [f"{FIELD}/{FEATURE}"])),
        "the count reporter is no longer declared":
            plant(testkit, lambda m: m.pop("example")),
        "the testkit declares a feature":
            plant(testkit, feature("count", [])),
        "the testkit enables it on the verifier":
            plant(testkit, dependency(("dependencies",), VERIFY, on)),
        "a tool's feature is a string, not a list":
            plant(bench, feature("count", f"{VERIFY}/{FEATURE}")),
        "a dependency's features are a string, not a list":
            plant(bench, dependency(("dependencies",), VERIFY,
                                    {"workspace": True, "features": FEATURE})),
    }
    for case, change in planted.items():
        for name, manifest in change.items():
            if manifest == real[name]:
                raise SystemExit(f"selftest: could not plant '{case}' in {name}")
        if manifest_fault({**real, **change}) is None:
            raise SystemExit(f"selftest: not caught: {case}")
    print(f"selftest: {len(planted)} planted manifests refused")


def main():
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        return
    if sys.argv[1:]:
        raise SystemExit("usage: check_op_count_isolation.py [--selftest]")
    for fault in (manifest_fault(manifests()), graph_fault(), api_fault()):
        if fault:
            raise SystemExit(fault)
    print(
        f"{FEATURE}: declared by {FIELD} and {VERIFY} alone and enabled by no "
        "manifest; in no member's default graph, and visible when requested; "
        "a default consumer cannot name the counter or the stages, and one "
        f"that asks for it can; the reporter of {TESTKIT} requires it"
    )


if __name__ == "__main__":
    main()
