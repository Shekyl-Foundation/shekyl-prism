#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""No pull request starts a job on the Foundation's own runners.

This repository is public. A workflow that a pull request can start runs
the code of whoever opened it, and that must not execute on owned
hardware. So:

1. Every `runs-on:` is a label this file knows: a GitHub-hosted one, the
   expression `${{ matrix.runner }}` whose matrix values are GitHub-hosted
   ones, or the owned runners' label exactly, `[self-hosted, shekyl-gate]`.
   A label it does not know is refused: a mistyped one would wait for a
   runner that does not exist, and a new one has to be placed here first.
2. A workflow that has a job on the owned runners starts on `schedule` or
   `workflow_dispatch` and on nothing else. A dispatch needs write access
   to the repository. Any other trigger is refused, not only
   `pull_request`: `pull_request_target`, `workflow_run`, `issue_comment`
   and `push` are each a way in for someone else's code or input.
3. A workflow that is called (`workflow_call`) and has a job on the owned
   runners has no other trigger, and every workflow that calls it is held
   to rule 2 as if the job were its own.
4. The subject is there: at least one job is on the owned runners, and
   the nightly is such a workflow. A check that found none would be
   reading the wrong files.

The files are read as text, as `check_action_pins.py` reads them: the
standard library has no YAML, and these files keep to a plain shape (a
block `on:`, jobs at two spaces, `runs-on:` and `uses:` at four). A file
that does not have that shape is refused.

`--selftest` plants a breach of each rule in copies of the real files.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
OWNED = "[self-hosted, shekyl-gate]"
MATRIX = "${{ matrix.runner }}"
HOSTED = {"ubuntu-latest", "ubuntu-24.04-arm"}
# What may start a workflow that reaches the owned runners.
TRUSTED_TRIGGERS = {"schedule", "workflow_dispatch"}
CALL = "workflow_call"
NIGHTLY = "nightly.yml"
LOCAL_USE = re.compile(r"^    uses:\s*\./\.github/workflows/([A-Za-z0-9_.-]+\.yml)\s*$")


def code_lines(text):
    """The lines that are not comments, with trailing comments kept: a
    `#` inside a value is rare here and a label never has one."""
    return [line for line in text.splitlines() if not line.lstrip().startswith("#")]


def triggers(name, text):
    """The events of the `on:` block."""
    lines = code_lines(text)
    starts = [index for index, line in enumerate(lines) if re.match(r"^on:\s*$", line)]
    if len(starts) != 1:
        raise SystemExit(f"{name}: expected one block `on:` at the left margin")
    found = set()
    for line in lines[starts[0] + 1:]:
        if line.strip() and not line.startswith(" "):
            break
        match = re.match(r"^  ([a-z_]+):", line)
        if match:
            found.add(match.group(1))
    if not found:
        raise SystemExit(f"{name}: the `on:` block names no event")
    return found


def runners(name, text):
    """Every `runs-on:` value, and every value of a matrix `runner:`."""
    labels, matrix = [], []
    for line in code_lines(text):
        match = re.match(r"^\s*runs-on:\s*(.+?)\s*$", line)
        if match:
            labels.append(match.group(1))
        match = re.match(r"^\s*-?\s*runner:\s*(.+?)\s*$", line)
        if match:
            matrix.append(match.group(1))
    return labels, matrix


def calls(text):
    """The workflows of this repository that this one calls."""
    return {
        match.group(1)
        for line in code_lines(text)
        if (match := LOCAL_USE.match(line))
    }


def fault(files):
    """The first breach, or None. `files` maps a file name to its text."""
    owned = set()
    for name, text in sorted(files.items()):
        labels, matrix = runners(name, text)
        if not labels and not calls(text):
            return f"{name}: no `runs-on:` and no called workflow"
        for label in matrix:
            if label not in HOSTED:
                return f"{name}: matrix runner `{label}` is not a GitHub-hosted label this check knows"
        for label in labels:
            if label == OWNED:
                owned.add(name)
            elif label == MATRIX:
                if not matrix:
                    return f"{name}: `{MATRIX}` with no matrix `runner:` values"
            elif label not in HOSTED:
                return f"{name}: `runs-on: {label}` is not a label this check knows"
    if not owned:
        return "no job is on the owned runners: this check has no subject"
    if NIGHTLY not in files:
        return f"{NIGHTLY} is not among the workflows"

    # Reaching the owned runners: a job of its own there, or a call, at
    # any depth, of a workflow that has one.
    reaches = set(owned)
    changed = True
    while changed:
        changed = False
        for name, text in files.items():
            for called in calls(text):
                if called not in files:
                    return f"{name}: calls {called}, which is not among the workflows"
                if called in reaches and name not in reaches:
                    reaches.add(name)
                    changed = True
    if NIGHTLY not in reaches:
        return f"{NIGHTLY} has no job on the owned runners"
    for name in sorted(reaches):
        events = triggers(name, files[name])
        if CALL in events:
            if events != {CALL}:
                other = ", ".join(sorted(events - {CALL}))
                return f"{name}: is called and reaches the owned runners, and also starts on {other}"
            continue
        untrusted = events - TRUSTED_TRIGGERS
        if untrusted:
            return (
                f"{name}: reaches the owned runners and starts on "
                + ", ".join(sorted(untrusted))
            )
    # A called workflow that reaches the owned runners and that nothing
    # calls is harmless; one that something calls was covered above,
    # since its caller is in `reaches`.
    return None


def read():
    return {path.name: path.read_text() for path in sorted(WORKFLOWS.glob("*.yml"))}


def selftest(files):
    real = fault(files)
    if real:
        raise SystemExit("selftest: the tree itself fails: " + real)

    def changed(name, old, new, count=1):
        text = files[name]
        if old not in text:
            raise SystemExit(f"selftest: {name} no longer has the text a plant needs: {old!r}")
        return {**files, name: text.replace(old, new, count)}

    ci = "ci.yml"
    shard = "mutants-shard.yml"
    plants = {
        "the nightly starts on a pull request":
            (changed(NIGHTLY, "on:\n  schedule:", "on:\n  pull_request:\n  schedule:"),
             "starts on pull_request"),
        "the nightly starts on a push":
            (changed(NIGHTLY, "on:\n  schedule:", "on:\n  push:\n  schedule:"),
             "starts on push"),
        "the nightly starts on pull_request_target":
            (changed(NIGHTLY, "on:\n  schedule:", "on:\n  pull_request_target:\n  schedule:"),
             "starts on pull_request_target"),
        "a job of the PR workflow is on the owned runners":
            (changed(ci, "runs-on: ubuntu-latest", f"runs-on: {OWNED}"),
             "starts on pull_request"),
        "the PR workflow calls the shard workflow":
            (changed(ci, "\njobs:\n", "\njobs:\n  borrowed:\n    uses: ./.github/workflows/"
                     + shard + "\n"),
             "starts on pull_request"),
        "the shard workflow can also be started by a pull request":
            (changed(shard, "on:\n  workflow_call:", "on:\n  pull_request:\n  workflow_call:"),
             "also starts on pull_request"),
        "a matrix leg is on the owned runners":
            (changed(ci, "runner: ubuntu-latest", "runner: self-hosted"),
             "matrix runner"),
        "a runner label nobody placed":
            (changed(ci, "runs-on: ubuntu-latest", "runs-on: self-hosted"),
             "not a label this check knows"),
        "the owned label, mistyped":
            (changed(shard, f"runs-on: {OWNED}", "runs-on: [self-hosted, shekyl-gates]"),
             "not a label this check knows"),
        "no job is on the owned runners":
            ({name: text.replace(f"runs-on: {OWNED}", "runs-on: ubuntu-latest")
              for name, text in files.items()},
             "no subject"),
        "the nightly is not there":
            ({name: text for name, text in files.items() if name != NIGHTLY},
             f"{NIGHTLY} is not among"),
        "a call of a workflow that does not exist":
            (changed(NIGHTLY, "uses: ./.github/workflows/" + shard,
                     "uses: ./.github/workflows/absent.yml"),
             "not among the workflows"),
    }
    for name, (planted, expected) in plants.items():
        found = fault(planted)
        if found is None:
            raise SystemExit(f"selftest: not refused: {name}")
        if expected not in found:
            raise SystemExit(f"selftest: {name}: refused for another reason: {found}")
    print(f"selftest: {len(plants)} planted breaches refused, each for its own reason")


def main():
    files = read()
    if sys.argv[1:] == ["--selftest"]:
        selftest(files)
        return
    if sys.argv[1:]:
        raise SystemExit("usage: check_runner_boundary.py [--selftest]")
    found = fault(files)
    if found:
        raise SystemExit(found)
    owned = sorted(
        name for name, text in files.items() if OWNED in runners(name, text)[0]
    )
    print(
        f"{len(files)} workflows: jobs on the owned runners in {', '.join(owned)}, "
        "reached only from a schedule or a dispatch; every other runner is GitHub's"
    )


if __name__ == "__main__":
    main()
