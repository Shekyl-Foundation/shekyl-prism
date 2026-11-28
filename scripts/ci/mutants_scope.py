#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The compiled-source inventory for one mutation build.

Discovery writes the library and binary sources rustc compiled for the
selected packages and features. The other modes only read that inventory:

- ``--verify`` requires every listed mutant to be in it.
- ``--reject-file`` lists one source under ``--reject-features`` and
  requires every mutant of that source to fall outside it.
- ``--control-file`` lists one mutant by its description, requires it to
  be in the inventory, runs it, and requires the caught list to be that
  mutant.

An empty mutant list is a failure in every mode. A check that examined
nothing is not a pass.

``--selftest`` also reads the two workflows. For each job the nightly
gives to the shard workflow it builds every command line the shard
would run this script with, from that job's inputs, and requires the
parser to take it. A value that begins with ``--`` reaches the parser
only when it is joined to its option (``--control-tests=--test=name``);
as a separate word the parser reads it as an option.
"""

import argparse
import contextlib
import io
import json
import re
import shlex
import subprocess
import sys
from pathlib import Path


def compiled_sources(packages, features):
    """Read the selected packages' library and binary dep-info from this build."""
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version=1", "--locked"], text=True))
    ids = {p["id"]: p["name"] for p in metadata["packages"] if p["name"] in packages}
    args = ["cargo", "check", "--lib", "--bins", "--locked", "--message-format=json-render-diagnostics"]
    for package in packages:
        args += ["-p", package]
    if features:
        args += ["--features", features]
    artifacts = subprocess.check_output(args, text=True)
    sources = set()
    built_roots = set()
    features_by_package = {}
    for line in artifacts.splitlines():
        message = json.loads(line)
        if message["reason"] != "compiler-artifact":
            continue
        target = message["target"]
        if message["package_id"] not in ids or not {"lib", "bin"}.intersection(target["kind"]):
            continue
        meta, = (Path(f) for f in message["filenames"] if f.endswith(".rmeta"))
        deps = meta.with_name(meta.stem.removeprefix("lib") + ".d")
        # rustc writes one make rule, then an empty rule per dependency.
        # Every dependency is on the first line.
        _, dependencies = deps.read_text().splitlines()[0].split(": ", 1)
        for name in shlex.split(dependencies):
            if name.endswith(".rs"):
                path = Path(name).resolve()
                if path.is_relative_to(Path.cwd()):
                    sources.add(str(path.relative_to(Path.cwd())))
        built_roots.add(target["src_path"])
        if str(Path(target["src_path"]).relative_to(Path.cwd())) not in sources:
            raise ValueError(f"missing target root in dep-info: {target}")
        if "lib" in target["kind"]:
            features_by_package[ids[message["package_id"]]] = set(message["features"])
    if set(features_by_package) != set(packages):
        raise ValueError(f"missing library inventory for {set(packages) - features_by_package.keys()}")
    for package in metadata["packages"]:
        if package["id"] not in ids:
            continue
        enabled = features_by_package[package["name"]]
        for target in package["targets"]:
            if ("bin" in target["kind"] and set(target.get("required-features", ())) <= enabled
                    and target["src_path"] not in built_roots):
                raise ValueError(f"missing compiler inventory for binary {target['name']}")
    return sources


def mutant_files(mutants):
    """The source files of a cargo-mutants list. An empty list has no subject."""
    if not mutants:
        raise ValueError("empty mutant inventory")
    return {mutant["file"] for mutant in mutants}


def uncompiled(mutants, sources):
    """Files named by `mutants` that `sources` does not contain."""
    return mutant_files(mutants) - set(sources)


def require_compiled(mutants, sources):
    """Refuse a list that names a source this build did not compile."""
    missing = uncompiled(mutants, sources)
    if missing:
        raise ValueError(f"mutants outside compiled sources: {sorted(missing)}")


def require_uncompiled(mutants, sources, source):
    """Refuse unless every listed mutant names `source` and this build did not compile it."""
    files = mutant_files(mutants)
    if files != {source}:
        raise ValueError(f"expected only mutants of {source}, found {sorted(files)}")
    if source in set(sources):
        raise ValueError(f"expected mutants of {source} to fall outside the compiled sources")


def list_mutants(packages, features, source, description):
    """The mutants cargo-mutants would generate for one source.

    `description` anchors an exact suffix of the mutant name. Empty lists
    every mutant of the source.
    """
    command = ["cargo", "mutants", "--list", "--json"]
    for package in packages:
        command += ["-p", package]
    if features:
        command += ["--features", features]
    command += ["--file", source]
    if description:
        command += ["--re", re.escape(description) + "$"]
    return json.loads(subprocess.check_output(command, text=True))


def catch_control(packages, features, source, description, test_args, sources, output):
    """Run the one named mutant and require the caught list to be that mutant."""
    planned = list_mutants(packages, features, source, description)
    require_compiled(planned, sources)
    if len(planned) != 1:
        raise ValueError(f"expected one positive control, found {len(planned)}")
    command = ["cargo", "mutants"]
    for package in packages:
        command += ["-p", package]
    if features:
        command += ["--features", features]
    command += ["--file", source, "--re", re.escape(description) + "$", "--output", str(output)]
    for arg in test_args:
        command += [f"--cargo-test-arg={arg}"]
    subprocess.run(command, check=True)
    caught = (output / "mutants.out" / "caught.txt").read_text().splitlines()
    if caught != [planned[0]["name"]]:
        raise ValueError(f"positive control was not caught: {caught}")
    print("Caught positive control:", caught[0])


def parse_inventory(text, path):
    """The source list discovery writes. A missing or empty list is not an inventory."""
    sources = json.loads(text)
    if not isinstance(sources, list) or not sources or not all(isinstance(item, str) for item in sources):
        raise ValueError(f"{path}: the inventory is a non-empty list of sources")
    return set(sources)


def load_inventory(path):
    """The inventory at `path`."""
    return parse_inventory(path.read_text(), path)


def selected_sources(sources, scope):
    """Compiled sources under `scope`, or every compiled source when `scope` is empty."""
    selected = sorted(path for path in sources if not scope or path.startswith(scope + "/"))
    if not selected:
        raise ValueError(f"no compiled source under {scope}")
    return selected


REPOSITORY = Path(__file__).resolve().parents[2]
NIGHTLY = REPOSITORY / ".github/workflows/nightly.yml"
SHARD_WORKFLOW = REPOSITORY / ".github/workflows/mutants-shard.yml"
SCRIPT = "scripts/ci/mutants_scope.py"

CALL = re.compile(r"^    uses: \./\.github/workflows/mutants-shard\.yml$")
WITH_VALUE = re.compile(r"^      (\w+): (.*)$")
INPUT_NAME = re.compile(r"^      (\w+):$")
INPUT_DEFAULT = re.compile(r"^        default: (.*)$")
STEP_ENV = re.compile(r"^          (\w+): \$\{\{ inputs\.(\w+) \}\}$")
STEP_NEEDS_INPUT = re.compile(r"inputs\.(\w+) != ''")
RUN_BODY_INDENT = " " * 10
SHELL_VARIABLE = re.compile(r"\$\{?(\w+)\}?")
# The shard workflow gives shard zero the checks that run once for a job.
SELFTEST_SHARD = "0"


def unquoted(value):
    """A YAML scalar as these workflows write one: bare, or in double quotes."""
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] == '"':
        return value[1:-1]
    return value


def nightly_jobs(text):
    """The inputs each nightly job gives the shard workflow, by job name."""
    jobs = {}
    lines = text.splitlines()
    for at, line in enumerate(lines):
        if not CALL.match(line) or lines[at + 1] != "    with:":
            continue
        given = {}
        for value in lines[at + 2:]:
            found = WITH_VALUE.match(value)
            if not found:
                break
            given[found.group(1)] = unquoted(found.group(2))
        jobs[given["job"]] = given
    return jobs


def shard_defaults(text):
    """The default of every input of the shard workflow that has one."""
    defaults = {}
    name = None
    for line in text.split("\njobs:\n", 1)[0].splitlines():
        found = INPUT_NAME.match(line)
        if found:
            name = found.group(1)
        found = INPUT_DEFAULT.match(line)
        if found and name:
            defaults[name] = unquoted(found.group(1))
    return defaults


def run_block(step):
    """The shell text of one step: the line after `run:`, or the block under `run: |`."""
    lines = step.splitlines()
    at = next(index for index, line in enumerate(lines) if line.lstrip().startswith("run:"))
    inline = lines[at].split("run:", 1)[1].strip()
    if inline != "|":
        return inline
    body = []
    for line in lines[at + 1:]:
        if line.strip() and not line.startswith(RUN_BODY_INDENT):
            break
        body.append(line)
    return "\n".join(body)


def shard_commands(text, inputs):
    """Every argument list the shard workflow gives this script under `inputs`.

    A step whose condition requires an input the job leaves empty does
    not run and has no command. A word of the command is expanded as the
    shell expands it inside double quotes: to one word, whatever it holds.
    """
    commands = []
    for step in text.split("\njobs:\n", 1)[1].split("\n      - ")[1:]:
        if SCRIPT not in step:
            continue
        condition = next((line for line in step.splitlines() if line.lstrip().startswith("if:")), "")
        if any(not inputs.get(name, "") for name in STEP_NEEDS_INPUT.findall(condition)):
            continue
        environment = {"RUNNER_TEMP": "runner-temp"}
        for line in step.splitlines():
            found = STEP_ENV.match(line)
            if found:
                environment[found.group(1)] = inputs.get(found.group(2), "")
        words = shlex.split(run_block(step).replace("\\\n", " "))
        words = words[words.index(SCRIPT) + 1:]
        if ">" in words:
            words = words[:words.index(">")]
        commands.append([SHELL_VARIABLE.sub(lambda name: environment[name.group(1)], word) for word in words])
    return commands


def workflow_failures(nightly, shard):
    """Why the parser would refuse a command line the workflows build. Empty when none."""
    failures = []
    jobs = nightly_jobs(nightly)
    if not jobs:
        return ["the nightly gives no job to the shard workflow"]
    defaults = shard_defaults(shard)
    modes = set()
    for job, given in sorted(jobs.items()):
        inputs = {**defaults, **given, "shard": SELFTEST_SHARD}
        commands = shard_commands(shard, inputs)
        if not commands:
            failures.append(f"{job}: the shard workflow never runs this script")
        for words in commands:
            try:
                with contextlib.redirect_stderr(io.StringIO()):
                    args = parse_arguments(words)
            except SystemExit:
                failures.append(f"{job}: the parser refuses {words}")
                continue
            except (ValueError, KeyError) as error:
                failures.append(f"{job}: {error} in {words}")
                continue
            for mode in ("verify", "reject_file", "control_file"):
                if getattr(args, mode):
                    modes.add(mode)
                    break
            else:
                modes.add("discovery")
            if args.control_file and args.control_tests != inputs["control_tests"]:
                failures.append(f"{job}: the control's tests arrived as {args.control_tests!r}")
    # The nightly has had no reject control since the ideal layer left
    # the front crate; the mode keeps its own cases below.
    for mode in ("discovery", "verify", "control_file"):
        if mode not in modes:
            failures.append(f"no job of the nightly reaches the {mode} mode")
    return failures


def selftest():
    """The membership decisions, with no cargo build."""
    sources = {"crate/src/lib.rs", "crate/src/bin/tool.rs"}
    inside = [{"file": "crate/src/lib.rs", "name": "one"}]
    outside = [{"file": "crate/src/gated.rs", "name": "gated"}]
    mixed = inside + outside
    failures = []

    def check(label, call):
        try:
            call()
        except ValueError as error:
            failures.append(f"{label}: {error}")

    def expect_error(label, call, text):
        try:
            call()
        except ValueError as error:
            if text not in str(error):
                failures.append(f"{label}: {error}")
            return
        failures.append(f"{label}: accepted")

    check("compiled list", lambda: require_compiled(inside, sources))
    expect_error("empty list", lambda: require_compiled([], sources), "empty mutant inventory")
    expect_error("outside list", lambda: require_compiled(outside, sources), "gated.rs")
    check("rejected file", lambda: require_uncompiled(outside, sources, "crate/src/gated.rs"))
    expect_error(
        "rejected file was compiled",
        lambda: require_uncompiled(inside, sources, "crate/src/lib.rs"),
        "fall outside",
    )
    expect_error(
        "rejection names two files",
        lambda: require_uncompiled(mixed, sources, "crate/src/gated.rs"),
        "only mutants",
    )
    expect_error("empty rejection", lambda: require_uncompiled([], sources, "crate/src/gated.rs"), "empty")
    expect_error("empty inventory", lambda: parse_inventory("[]", Path("inventory")), "non-empty")
    expect_error("no source in scope", lambda: selected_sources(sources, "missing"), "no compiled source")
    if selected_sources(sources, "crate/src/bin") != ["crate/src/bin/tool.rs"]:
        failures.append("scope kept a source outside the directory")
    if selected_sources(sources, "") != sorted(sources):
        failures.append("an empty scope did not keep every compiled source")
    expect_error(
        "control without its mutant",
        lambda: parse_arguments(["--packages", "p", "--inventory", "i", "--control-file", "f"]),
        "needs its mutant",
    )
    expect_error(
        "mutant without its control file",
        lambda: parse_arguments(["--packages", "p", "--inventory", "i", "--control-mutant", "m"]),
        "needs its file",
    )
    shard = SHARD_WORKFLOW.read_text()
    failures += workflow_failures(NIGHTLY.read_text(), shard)
    # The fault of 2026-11-27: the filter as a word of its own. The check
    # above must refuse the shard workflow written that way.
    joined = '--control-tests="$CONTROL_TESTS"'
    if joined not in shard:
        failures.append(f"the shard workflow does not write {joined}")
    elif not workflow_failures(NIGHTLY.read_text(), shard.replace(joined, '--control-tests "$CONTROL_TESTS"')):
        failures.append("a control filter passed as its own word was accepted")
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("mutants_scope: selftest passed")
    return 0


def build_parser():
    """The command line of every mode."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", required=True)
    parser.add_argument("--features", default="")
    parser.add_argument("--scope", default="")
    parser.add_argument("--inventory", type=Path, required=True)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--verify", type=Path)
    mode.add_argument("--reject-file")
    mode.add_argument("--control-file")
    parser.add_argument("--reject-features", default="")
    parser.add_argument("--control-mutant", default="")
    parser.add_argument("--control-tests", default="")
    return parser


def parse_arguments(argv):
    """The arguments of one run. A control names both its file and its mutant."""
    args = build_parser().parse_args(argv)
    if args.control_file and not args.control_mutant:
        raise ValueError("a control file needs its mutant description")
    if args.control_mutant and not args.control_file:
        raise ValueError("a control mutant needs its file")
    return args


def main(argv):
    args = parse_arguments(argv)
    packages = args.packages.split()
    if args.verify:
        require_compiled(json.loads(args.verify.read_text()), load_inventory(args.inventory))
        return
    if args.reject_file:
        require_uncompiled(
            list_mutants(packages, args.reject_features, args.reject_file, ""),
            load_inventory(args.inventory),
            args.reject_file,
        )
        print(f"Rejected mutants of {args.reject_file}")
        return
    if args.control_file:
        catch_control(
            packages,
            args.features,
            args.control_file,
            args.control_mutant,
            args.control_tests.split(),
            load_inventory(args.inventory),
            args.inventory.parent / "control",
        )
        return
    sources = compiled_sources(packages, args.features)
    args.inventory.write_text(json.dumps(sorted(sources)))
    print("\n".join(selected_sources(sources, args.scope)))


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        sys.exit(selftest())
    try:
        main(sys.argv[1:])
    except (ValueError, subprocess.CalledProcessError, json.JSONDecodeError, OSError) as error:
        print(f"mutants: {error}", file=sys.stderr)
        sys.exit(1)
