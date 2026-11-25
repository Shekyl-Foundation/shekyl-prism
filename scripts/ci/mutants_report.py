#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""What a `cargo mutants` run deals, finds, and keeps.

    mutants_report.py plan    [--only JOB] [--shard K]
    mutants_report.py shard   --out mutants.out --job J --shard K --of N --exit CODE
    mutants_report.py verdict --out mutants.out
    mutants_report.py gather  --dir DIR [--write DIR] [--expect JOB ...] [--explained FILE]
    mutants_report.py record  --file FILE

`plan` is the dispatch. `only` empty deals every mutant job and the rest
of the nightly. `only` names one mutant job. `shard` selects one index of
that job, and is refused when `only` is empty. The shard counts live in
`MUTANT_JOBS`; the nightly reads them back and does not write them again.

`shard` records which shard the output directory is and the tool's exit
code (`shard.json`, beside the tool's lists), and prints the four counts
and the whole missed and timeout lists. It does not judge.

`verdict` judges. A missed mutant and a timeout are reported and do not
fail (exit codes 0, 2 and 3 of cargo-mutants 27.1.0: all caught, some
missed, some timed out). Every other code fails: 1 is a usage error, 4 is
a baseline that already fails, 70 an internal error, and a code this
script has not met is not a pass. A directory without the four lists, a
shard with no mutant in it, a shard index outside its dealing, a record
or a `mutants.json` that cannot be read (a file cut short is one), and
lists that do not add up to the mutants the tool set out to test fail as
well.

`gather` reads every shard directory under `--dir`. It uses `verdict`.
A present shard that fails it is named, left out of the figures, and
fails this command; so is a shard record that cannot be read, and the
shards that can be are still summed. A shard that is absent is named and does not, by
itself. With `--write`, the merged missed and timeout lists are written
only when every expected job is present in full and none of its shards
was rejected. The same text is written as `summary.md`, so the artifact
says which shards it holds. `--expect` names those jobs; without it, the
jobs found are the expected ones. It fails when it finds no shard.

`--explained` names the kept record of mutants that stay missed or
timing out with a reason (`docs/lab/MUTANTS.toml`). For each job whose
shards are all here, `gather` then says how many of its missed mutants
and timeouts the record explains, how many it does not, by file, and
which entries of the record match no mutant any more. A mutant is
matched by its file and the tool's own words for it, without the line
and column, and by count, since one function can have the same mutation
twice. Nothing about the record fails the command except a record that
cannot be read: the counts are printed.

`record` reads the kept record and says how many entries it has. It
fails on a record `gather` could not read, so a pull request that breaks
the file is red there and not in the next nightly's summary.

Text goes to standard output, and to the job summary when the runner
names one.
"""

import argparse
import json
import os
import sys
import tempfile
import tomllib
from collections import Counter, defaultdict
from pathlib import Path
from typing import NamedTuple

LISTS = ("caught", "missed", "timeout", "unviable")
# cargo-mutants 27.1.0, `book/src/exit-codes.md`: the three codes of a run
# that tested its mutants.
REPORT_ONLY = {0: "every mutant caught", 2: "mutants missed", 3: "tests timed out"}


class MutantJob(NamedTuple):
    """One mutant job the nightly can run alone or with the others.

    `output` is the prefix of that job's dispatch outputs:
    `<output>_shards`, `<output>_of`, `<output>_run`. `shards` is how
    many ways the job is dealt. Sixteen and two are the measured sizes:
    the slowest shard stays under 60 minutes (`docs/PRISM_CI_DESIGN.md`).
    Four, for the integer crate, is derived from a run on a build VM and
    not yet measured on a runner.
    """

    name: str
    output: str
    shards: int


MUTANT_JOBS = (
    MutantJob("mutants", "mutants", 16),
    MutantJob("mutants-front", "front", 2),
    MutantJob("mutants-int", "int", 4),
)


class Explained(NamedTuple):
    """One entry of the kept record: a mutant that stays, and why.

    `mutation` is the tool's line for the mutant without `line:col: `, so
    an edit above the mutant does not orphan the entry. `count` is how
    many mutants of the file that text names.
    """

    job: str
    file: str
    mutation: str
    outcome: str
    count: int
    reason: str


class RecordError(Exception):
    """The kept record is not one this script can read."""


def load_explained(path: Path) -> list[Explained]:
    """The entries of the kept record. Raises `RecordError` on a record that is not one."""
    try:
        document = tomllib.loads(path.read_text())
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise RecordError(f"{path}: {error}") from error
    entries = document.get("mutant", [])
    if not isinstance(entries, list) or set(document) - {"mutant"}:
        raise RecordError(f"{path}: the record is a list of [[mutant]] tables and nothing else")
    names = [job.name for job in MUTANT_JOBS]
    out = []
    for number, entry in enumerate(entries, start=1):
        where = f"{path}: entry {number}"
        if not isinstance(entry, dict) or set(entry) != set(Explained._fields):
            raise RecordError(f"{where} does not have exactly the fields {', '.join(Explained._fields)}")
        text = {key: entry[key] for key in ("job", "file", "mutation", "outcome", "reason")}
        if not all(isinstance(value, str) and value.strip() for value in text.values()):
            raise RecordError(f"{where} has an empty field, or one that is not text")
        if entry["job"] not in names:
            raise RecordError(f"{where} names the job {entry['job']!r}")
        if entry["outcome"] not in ("missed", "timeout"):
            raise RecordError(f"{where} has the outcome {entry['outcome']!r}: missed or timeout")
        if not isinstance(entry["count"], int) or isinstance(entry["count"], bool) or entry["count"] < 1:
            raise RecordError(f"{where} has a count that is not a positive integer")
        out.append(Explained(**entry))
    keys = Counter((entry.job, entry.file, entry.mutation, entry.outcome) for entry in out)
    twice = [key for key, count in keys.items() if count > 1]
    if twice:
        raise RecordError(f"{path}: an entry is written twice: {twice[0][1]}: {twice[0][2]}")
    return out


def mutant_key(line: str) -> tuple[str, str]:
    """`path:line:col: what` as `(path, what)`. A line of another shape is its own key."""
    parts = line.split(": ", 1)
    if len(parts) != 2:
        return line, ""
    return parts[0].split(":", 1)[0], parts[1]


def explained_lines(job: str, merged: dict[str, list[str]], record: list[Explained]) -> list[str]:
    """How the kept record and one job's whole lists compare."""
    lines = ["### Against the kept record", ""]
    stale: list[str] = []
    for outcome in ("missed", "timeout"):
        found = Counter(mutant_key(line) for line in merged[outcome])
        stated = {
            (entry.file, entry.mutation): entry.count
            for entry in record
            if entry.job == job and entry.outcome == outcome
        }
        explained = sum(min(count, stated.get(key, 0)) for key, count in found.items())
        left = {key: count - stated.get(key, 0) for key, count in found.items() if count > stated.get(key, 0)}
        lines += [f"{outcome.capitalize()}: {explained} explained, {sum(left.values())} not.", ""]
        by_path: dict[str, int] = defaultdict(int)
        for (path, _), count in left.items():
            by_path[path] += count
        if by_path:
            lines += [f"- `{path}`: {count}" for path, count in sorted(by_path.items())] + [""]
        for key, count in sorted(stated.items()):
            if found.get(key, 0) < count:
                stale.append(f"{key[0]}: {key[1]} ({outcome}; {count} stated, {found.get(key, 0)} found)")
    lines += [f"Entries of the record that match no mutant any more: {len(stale)}.", ""]
    if stale:
        lines += ["```"] + stale + ["```", ""]
    return lines


class DispatchError(Exception):
    """The dispatch does not name a run this workflow has."""
OTHER = {
    -1: "the tool did not finish",
    1: "usage error",
    4: "the unmutated baseline fails or hangs",
    5: "diff filter mismatch",
    6: "diff filter invalid",
    70: "internal error",
}


def read_json(path: Path) -> object:
    """The JSON value in `path`, or None when it cannot be read as JSON.

    A file cut short by a cancelled job is the case this is for. `null`
    is no record either, so the two are not told apart.
    """
    try:
        return json.loads(path.read_text())
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None


def read_lists(out: Path) -> dict[str, list[str]] | None:
    """The four lists, or None when one is absent."""
    lists = {}
    for name in LISTS:
        path = out / f"{name}.txt"
        if not path.is_file():
            return None
        lists[name] = [line for line in path.read_text().splitlines() if line.strip()]
    return lists


def code_text(code: int) -> str:
    return REPORT_ONLY.get(code) or OTHER.get(code) or "a code this script does not know"


def by_file(mutants: list[str]) -> dict[str, list[str]]:
    """`path:line:col: what` grouped by path, in the order met."""
    groups: dict[str, list[str]] = defaultdict(list)
    for mutant in mutants:
        groups[mutant.split(":", 1)[0]].append(mutant)
    return dict(sorted(groups.items()))


def listing(title: str, mutants: list[str]) -> list[str]:
    lines = [f"### {title}: {len(mutants)}", ""]
    if not mutants:
        return lines
    for path, group in by_file(mutants).items():
        lines += [f"`{path}`: {len(group)}", "", "```"] + group + ["```", ""]
    return lines


def emit(lines: list[str]) -> None:
    text = "\n".join(lines) + "\n"
    sys.stdout.write(text)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as handle:
            handle.write(text)


def shard(out: Path, job: str, index: int, of: int, code: int) -> int:
    out.mkdir(parents=True, exist_ok=True)
    record = {"job": job, "shard": index, "of": of, "exit": code}
    (out / "shard.json").write_text(json.dumps(record) + "\n")
    lines = [
        f"## {job}, shard {index} of {of}",
        "",
        f"cargo-mutants exit {code}: {code_text(code)}.",
        "",
    ]
    lists = read_lists(out)
    if lists is None:
        lines.append("The tool left no lists: nothing was tested.")
        emit(lines)
        return 0
    counts = ", ".join(f"{len(lists[name])} {name}" for name in LISTS)
    lines += [f"{sum(len(v) for v in lists.values())} mutants: {counts}.", ""]
    lines += listing("Missed", lists["missed"]) + listing("Timeout", lists["timeout"])
    emit(lines)
    return 0


def verdict_of(out: Path) -> list[str]:
    """Why this shard is red; empty when it is not."""
    record_path = out / "shard.json"
    if not record_path.is_file():
        return ["no shard.json: the report step did not run"]
    record = read_json(record_path)
    try:
        code = record["exit"]
        index = record["shard"]
        of = record["of"]
    except (KeyError, TypeError):
        return ["shard.json is not a shard record"]
    errors = []
    if not isinstance(index, int) or not isinstance(of, int) or of < 1 or not 0 <= index < of:
        errors.append(f"shard {index} is not in 0..{of}")
    if code not in REPORT_ONLY:
        errors.append(f"cargo-mutants exit {code}: {code_text(code)}")
    lists = read_lists(out)
    if lists is None:
        return errors + ["the tool left no lists: nothing was tested"]
    total = sum(len(v) for v in lists.values())
    if total == 0:
        errors.append("the shard holds no mutant")
    planned_path = out / "mutants.json"
    if not planned_path.is_file():
        errors.append("no mutants.json: the tool did not say what it set out to test")
    else:
        planned = read_json(planned_path)
        if not isinstance(planned, list):
            errors.append("mutants.json is not a list of mutants: it was cut short, or it is not the tool's")
        elif len(planned) != total:
            errors.append(f"the tool set out to test {len(planned)} mutants and the lists hold {total}")
    return errors


def verdict(out: Path) -> int:
    errors = verdict_of(out)
    for error in errors:
        print(f"mutants: {error}", file=sys.stderr)
    if not errors:
        print("mutants: the shard tested what it set out to test (missed and timeout are report-only)")
    return 1 if errors else 0


def dispatch_plan(only: str, shard: str) -> dict[str, str]:
    """The indexes each mutant job runs, and whether the other nightly jobs run.

    `shard` empty deals every index of the jobs this dispatch runs. A job
    that does not run still gets one index, so a skipped matrix is one
    job and not a fan-out. Raises `DispatchError` when the dispatch does
    not name a run.
    """
    names = [job.name for job in MUTANT_JOBS]
    if only not in ("", *names):
        raise DispatchError(f"only is {only!r}: empty runs every job, or one of {', '.join(names)}")
    if shard != "" and only == "":
        raise DispatchError("shard is set while only is empty: name the job, or leave both empty")
    chosen = None
    if shard != "":
        if not shard.isdecimal():
            raise DispatchError(f"shard is {shard!r}: one index, from zero")
        chosen = int(shard)
    outputs = {}
    for job in MUTANT_JOBS:
        run = only in ("", job.name)
        if run and chosen is not None:
            if chosen >= job.shards:
                raise DispatchError(f"shard {chosen} is not in 0..{job.shards} for {job.name}")
            indexes = [chosen]
        elif run:
            indexes = list(range(job.shards))
        else:
            indexes = [0]
        outputs[f"{job.output}_shards"] = json.dumps(indexes)
        outputs[f"{job.output}_of"] = str(job.shards)
        outputs[f"{job.output}_run"] = "true" if run else "false"
    outputs["other_jobs"] = "true" if only == "" else "false"
    return outputs


def safe_outputs() -> dict[str, str]:
    """Outputs a refused dispatch still leaves, so a matrix can parse."""
    outputs = {}
    for job in MUTANT_JOBS:
        outputs[f"{job.output}_shards"] = "[0]"
        outputs[f"{job.output}_of"] = str(job.shards)
        outputs[f"{job.output}_run"] = "false"
    outputs["other_jobs"] = "false"
    return outputs


def write_github_output(outputs: dict[str, str]) -> None:
    text = "".join(f"{key}={value}\n" for key, value in outputs.items())
    path = os.environ.get("GITHUB_OUTPUT")
    if path:
        with open(path, "a", encoding="utf-8") as handle:
            handle.write(text)
    else:
        sys.stdout.write(text)


def plan_command(only: str, shard: str) -> int:
    try:
        outputs = dispatch_plan(only, shard)
    except DispatchError as error:
        print(f"mutants: {error}", file=sys.stderr)
        write_github_output(safe_outputs())
        return 1
    write_github_output(outputs)
    return 0


def load_jobs(root: Path) -> tuple[dict[str, list[tuple[Path, dict]]], list[Path]]:
    """Shard records under `root`, grouped by job, and the records that cannot be read.

    An unreadable record names no job, so it cannot be counted with one.
    It is returned apart, and the shards that can be read are still summed.
    """
    jobs: dict[str, list[tuple[Path, dict]]] = defaultdict(list)
    unreadable: list[Path] = []
    for record_path in sorted(root.rglob("shard.json")):
        record = read_json(record_path)
        if (
            not isinstance(record, dict)
            or not isinstance(record.get("job"), str)
            or any(key not in record for key in ("shard", "of", "exit"))
        ):
            unreadable.append(record_path)
            continue
        jobs[record["job"]].append((record_path.parent, record))
    return jobs, unreadable


def job_figures(shards: list[tuple[Path, dict]]) -> tuple[list[str], dict[str, list[str]] | None, bool]:
    """Lines for one job, its merged lists when every shard was accepted, and whether one was rejected.

    The merged lists are `None` when a shard is absent or rejected. The
    lines still count the shards that were accepted, and they do not call
    a partial job "all".
    """
    lines: list[str] = []
    dealings = {record["of"] for _, record in shards}
    if len(dealings) != 1:
        listed = ", ".join(str(count) for count in sorted(dealings))
        lines += [
            f"The shards do not agree on how many there are ({listed}), so there are no figures.",
            "",
        ]
        return lines, None, True
    of = dealings.pop()
    largest = max(job.shards for job in MUTANT_JOBS)
    if not isinstance(of, int) or not 1 <= of <= largest:
        lines += [f"The dealing {of} is not one of 1..{largest}, so there are no figures.", ""]
        return lines, None, True
    indexes = [record["shard"] for _, record in shards]
    if len(indexes) != len(set(indexes)):
        lines += ["A shard index was recorded twice, so there are no figures.", ""]
        return lines, None, True
    absent = [index for index in range(of) if index not in indexes]
    merged: dict[str, list[str]] = {name: [] for name in LISTS}
    kept = 0
    rejected = False
    for out, record in sorted(shards, key=lambda item: item[1]["shard"]):
        errors = verdict_of(out)
        if errors:
            rejected = True
            lines += [f"Shard {record['shard']} is not in the figures: {'; '.join(errors)}.", ""]
            continue
        lists = read_lists(out)
        if lists is None:
            rejected = True
            lines += [f"Shard {record['shard']} is not in the figures: no lists.", ""]
            continue
        for name in LISTS:
            merged[name] += lists[name]
        kept += 1
    if absent:
        missing = ", ".join(str(index) for index in absent)
        lines += [f"Shards in the figures: {kept} of {of}. Not here: {missing}.", ""]
    elif rejected:
        lines += [f"Shards in the figures: {kept} of {of}.", ""]
    else:
        lines += [f"All {of} shards.", ""]
    if kept:
        counts = ", ".join(f"{len(merged[name])} {name}" for name in LISTS)
        total = sum(len(merged[name]) for name in LISTS)
        lines += [f"{total} mutants: {counts}.", ""]
        lines += listing("Missed", merged["missed"]) + listing("Timeout", merged["timeout"])
    complete = None if rejected or absent else merged
    return lines, complete, rejected


def gather(
    root: Path,
    write: Path | None,
    expect: list[str] | None = None,
    explained: Path | None = None,
) -> int:
    record = None
    if explained is not None:
        try:
            record = load_explained(explained)
        except RecordError as error:
            print(f"mutants: {error}", file=sys.stderr)
            return 1
    found, unreadable = load_jobs(root)
    if not found and not unreadable:
        print(f"mutants: no shard under {root}", file=sys.stderr)
        return 1
    expected = list(expect) if expect is not None else sorted(found)
    unexpected = sorted(set(found) - set(expected))
    if unexpected:
        print(f"mutants: unexpected job {', '.join(unexpected)}", file=sys.stderr)
        return 1
    lines = ["# cargo mutants, all shards", ""]
    rejected = bool(unreadable)
    for record_path in unreadable:
        lines += [f"`{record_path.relative_to(root)}` is not a shard record, so its shard is in no figure.", ""]
    complete: dict[str, dict[str, list[str]]] = {}
    for job in expected:
        lines += [f"## {job}", ""]
        shards = found.get(job)
        if not shards:
            lines += ["No shard of this job is here.", ""]
            continue
        job_lines, merged, job_rejected = job_figures(shards)
        lines += job_lines
        rejected = rejected or job_rejected
        if merged is not None:
            complete[job] = merged
        if record is not None:
            if merged is not None:
                lines += explained_lines(job, merged, record)
            else:
                lines += ["The kept record is compared with a job only when all its shards are here.", ""]
    if write is not None and not rejected and len(complete) == len(expected):
        write.mkdir(parents=True, exist_ok=True)
        (write / "summary.md").write_text("\n".join(lines) + "\n")
        for job, merged in complete.items():
            for name in ("missed", "timeout"):
                body = "".join(f"{line}\n" for line in sorted(merged[name]))
                (write / f"{job}-{name}.txt").write_text(body)
    emit(lines)
    if rejected:
        print("mutants: a shard did not test what it set out to test", file=sys.stderr)
        return 1
    return 0


def record_command(path: Path) -> int:
    try:
        record = load_explained(path)
    except RecordError as error:
        print(f"mutants: {error}", file=sys.stderr)
        return 1
    by_outcome = Counter(entry.outcome for entry in record for _ in range(entry.count))
    print(f"{path}: {len(record)} entries, {by_outcome['missed']} missed and {by_outcome['timeout']} timeout")
    return 0


def selftest() -> int:
    def make(root: Path, name: str, code: int, lists: dict[str, list[str]] | None, planned=None, of=8, index=None):
        out = root / name
        out.mkdir(parents=True)
        if lists is not None:
            for key in LISTS:
                (out / f"{key}.txt").write_text("".join(f"{line}\n" for line in lists.get(key, [])))
            count = sum(len(v) for v in lists.values()) if planned is None else planned
            (out / "mutants.json").write_text(json.dumps([{}] * count))
        shard(out, "mutants", int(name[-1]) if index is None else index, of, code)
        return out

    def gather_text(
        directory: Path,
        write: Path | None = None,
        expect: list[str] | None = None,
        explained: Path | None = None,
    ):
        captured: list[list[str]] = []

        def grab(lines: list[str]) -> None:
            captured.append(list(lines))

        previous = globals()["emit"]
        globals()["emit"] = grab
        try:
            code = gather(directory, write, expect, explained)
        finally:
            globals()["emit"] = previous
        return code, "\n".join("\n".join(lines) for lines in captured)

    failures = []
    full = dispatch_plan("", "")
    if json.loads(full["mutants_shards"]) != list(range(16)) or full["mutants_of"] != "16":
        failures.append("plan did not deal mutants into sixteen")
    if json.loads(full["front_shards"]) != list(range(2)) or full["front_of"] != "2":
        failures.append("plan did not deal mutants-front into two")
    if json.loads(full["int_shards"]) != list(range(4)) or full["int_of"] != "4":
        failures.append("plan did not deal mutants-int into four")
    if full["other_jobs"] != "true" or full["mutants_run"] != "true" or full["front_run"] != "true":
        failures.append("an empty dispatch does not run the nightly")
    if full["int_run"] != "true":
        failures.append("an empty dispatch does not run mutants-int")
    one = dispatch_plan("mutants", "3")
    if json.loads(one["mutants_shards"]) != [3] or one["mutants_run"] != "true":
        failures.append("a mutants shard dispatch did not run that shard")
    if one["front_run"] != "false" or json.loads(one["front_shards"]) != [0] or one["other_jobs"] != "false":
        failures.append("a mutants shard dispatch ran more than that job")
    if one["int_run"] != "false" or json.loads(one["int_shards"]) != [0]:
        failures.append("a mutants shard dispatch ran mutants-int")
    alone = dispatch_plan("mutants-int", "3")
    if json.loads(alone["int_shards"]) != [3] or alone["int_run"] != "true" or alone["mutants_run"] != "false":
        failures.append("a mutants-int shard dispatch did not run that shard alone")
    if json.loads(dispatch_plan("mutants", "")["mutants_shards"]) != list(range(16)):
        failures.append("mutants without a shard did not run every shard")
    if json.loads(dispatch_plan("mutants-front", "1")["front_shards"]) != [1]:
        failures.append("mutants-front shard 1 was not dealt")
    for only, shard_text, text in (
        ("fuzz", "", "only is"),
        ("", "1", "shard is set"),
        ("mutants", "16", "not in 0.."),
        ("mutants", "nope", "one index"),
        ("mutants-front", "2", "not in 0.."),
        ("mutants-int", "4", "not in 0.."),
    ):
        try:
            dispatch_plan(only, shard_text)
        except DispatchError as error:
            if text not in str(error):
                failures.append(f"plan {only!r} {shard_text!r} said {error}")
        else:
            failures.append(f"plan accepted {only!r} {shard_text!r}")
    safe = safe_outputs()
    if json.loads(safe["mutants_shards"]) != [0] or safe["other_jobs"] != "false":
        failures.append("a refused dispatch has no matrix a workflow can skip")

    saved = os.environ.pop("GITHUB_STEP_SUMMARY", None)
    stdout, stderr = sys.stdout, sys.stderr
    sys.stdout = sys.stderr = open(os.devnull, "w", encoding="utf-8")
    try:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            some = {"caught": ["a.rs:1:1: x"], "missed": ["a.rs:2:1: y", "b.rs:1:1: z"]}
            late = {"caught": ["a.rs:3:1: x"], "timeout": ["b.rs:9:9: w"]}
            kept = {"missed": ["kept.rs:1:1: kept"]}
            dropped = {"missed": ["drop.rs:1:1: drop"]}
            bad = root / "bad"
            cases = [
                ("missed is report-only", make(root / "good", "s0", 2, some), False),
                ("timeout is report-only", make(root / "good", "s1", 3, late), False),
                ("baseline failure passed", make(bad, "s0", 4, None), True),
                ("unknown code passed", make(bad, "s1", 9, some), True),
                ("usage error passed", make(bad, "s2", 1, some), True),
                ("empty shard passed", make(bad, "s3", 0, {}), True),
                ("short lists passed", make(bad, "s4", 2, some, planned=5), True),
                ("shard index past the dealing passed", make(bad, "s5", 0, some, of=2, index=5), True),
            ]
            for name, out, red in cases:
                if bool(verdict_of(out)) != red:
                    failures.append(name)
            if not verdict_of(root / "nowhere"):
                failures.append("a missing directory passed")
            (root / "none").mkdir()
            if gather(root / "none", None) == 0:
                failures.append("gather passed with no shard")
            whole = root / "whole"
            make(whole, "s0", 2, some, of=2)
            make(whole, "s1", 3, late, of=2)
            written = root / "written"
            code, text = gather_text(whole, written)
            if code != 0 or "All 2 shards." not in text:
                failures.append("gather failed a complete dealing")
            missed = (written / "mutants-missed.txt").read_text().splitlines()
            timeout = (written / "mutants-timeout.txt").read_text().splitlines()
            if missed != ["a.rs:2:1: y", "b.rs:1:1: z"] or timeout != ["b.rs:9:9: w"]:
                failures.append("gather wrote the wrong lists")
            if "All 2 shards." not in (written / "summary.md").read_text():
                failures.append("the published summary does not name the dealing")
            partial = root / "partial"
            make(partial, "s0", 2, some, of=2)
            code, text = gather_text(partial, root / "partial-out")
            if code != 0 or "Not here: 1" not in text or "All 2 shards." in text:
                failures.append("an absent shard was called the whole dealing")
            if (root / "partial-out").exists():
                failures.append("an absent shard was published")
            mixed = root / "mixed"
            make(mixed, "s0", 2, kept, of=2)
            make(mixed, "s1", 2, dropped, of=2, planned=4)
            code, text = gather_text(mixed, root / "mixed-out")
            if code == 0 or "drop.rs" in text or "kept.rs" not in text or "All 2 shards." in text:
                failures.append("a rejected shard was merged into the figures")
            if (root / "mixed-out").exists():
                failures.append("a rejected shard was published")
            code, text = gather_text(whole, root / "expect-out", ["mutants", "mutants-front"])
            if code != 0 or "No shard of this job is here." not in text:
                failures.append("a job that was not run was called present")
            if (root / "expect-out").exists():
                failures.append("a missing job was published")
            other = root / "other"
            make(other, "s0", 0, some, of=1)
            shard(other / "s0", "other", 0, 1, 0)
            if gather(other, None, ["mutants"]) == 0:
                failures.append("an unexpected job passed")
            cut = root / "cut"
            make(cut, "s0", 2, kept, of=2)
            short = make(cut, "s1", 2, dropped, of=2)
            (short / "mutants.json").write_text('[{"name": "dr')
            if not any("mutants.json is not a list" in error for error in verdict_of(short)):
                failures.append("a mutants.json cut short passed, or stopped the verdict")
            (short / "mutants.json").write_text('{"mutants": 1}')
            if not any("mutants.json is not a list" in error for error in verdict_of(short)):
                failures.append("a mutants.json that is no list passed")
            code, text = gather_text(cut, root / "cut-out")
            if code == 0 or "kept.rs" not in text or "drop.rs" in text or "Shard 1 is not in the figures" not in text:
                failures.append("a mutants.json cut short stopped the summary of the other shards")
            torn = root / "torn"
            make(torn, "s0", 2, kept, of=2)
            (make(torn, "s1", 2, dropped, of=2) / "shard.json").write_text('{"job": "mut')
            code, text = gather_text(torn, root / "torn-out")
            if code == 0 or "kept.rs" not in text or "is not a shard record" not in text:
                failures.append("a shard record cut short stopped the summary of the other shards")
            if (root / "cut-out").exists() or (root / "torn-out").exists():
                failures.append("a shard that could not be read was published")
            record = root / "record.toml"
            entry = (
                '[[mutant]]\njob = "mutants"\nfile = "{file}"\nmutation = "{mutation}"\n'
                'outcome = "{outcome}"\ncount = {count}\nreason = "It changes nothing."\n'
            )
            twins = root / "twins"
            same = "a.rs:2:1: replace < with <= in f"
            make(twins, "s0", 2, {"missed": [same, "a.rs:9:1: replace < with <= in f"]}, of=2)
            make(twins, "s1", 3, {"missed": ["b.rs:1:1: delete ! in g"], "timeout": ["b.rs:9:9: w"]}, of=2)
            record.write_text(
                entry.format(file="a.rs", mutation="replace < with <= in f", outcome="missed", count=1)
                + entry.format(file="b.rs", mutation="w", outcome="timeout", count=1)
                + entry.format(file="c.rs", mutation="replace gone -> bool with true", outcome="missed", count=1)
            )
            code, text = gather_text(twins, None, None, record)
            if code != 0:
                failures.append("the kept record failed a run it only counts")
            # One of the two same mutations is explained and one is not, the
            # mutant of `b.rs` is in no entry, and the entry of `c.rs` is stale.
            for wanted in (
                "Missed: 1 explained, 2 not.",
                "- `a.rs`: 1",
                "- `b.rs`: 1",
                "Timeout: 1 explained, 0 not.",
                "match no mutant any more: 1.",
                "c.rs: replace gone -> bool with true (missed; 1 stated, 0 found)",
            ):
                if wanted not in text:
                    failures.append(f"the comparison with the kept record lacks {wanted!r}")
            code, text = gather_text(partial, None, None, record)
            if code != 0 or "only when all its shards are here" not in text or "explained," in text:
                failures.append("a partial job was compared with the kept record")
            for bad in (
                "not toml [",
                '[[mutant]]\njob = "mutants"\n',
                entry.format(file="a.rs", mutation="m", outcome="caught", count=1),
                entry.format(file="a.rs", mutation="m", outcome="missed", count=0),
                entry.format(file="a.rs", mutation="m", outcome="missed", count=1).replace("mutants", "other"),
                entry.format(file="a.rs", mutation="m", outcome="missed", count=1) * 2,
                'title = "x"\n',
            ):
                record.write_text(bad)
                if gather(twins, None, None, record) == 0:
                    failures.append(f"a kept record that is not one passed: {bad[:30]!r}")
            if gather(twins, None, None, root / "no-record.toml") == 0:
                failures.append("a missing kept record passed")
            split = root / "split"
            make(split, "s0", 0, some, of=2)
            make(split, "s1", 0, some, of=3)
            code, text = gather_text(split)
            if code == 0 or "do not agree" not in text:
                failures.append("shards that disagree on the dealing were merged")
    finally:
        sys.stdout.close()
        sys.stdout, sys.stderr = stdout, stderr
        if saved is not None:
            os.environ["GITHUB_STEP_SUMMARY"] = saved
    for failure in failures:
        print(f"selftest: {failure}", file=sys.stderr)
    if failures:
        return 1
    print("mutants-report selftest passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--selftest", action="store_true")
    commands = parser.add_subparsers(dest="command")
    deal = commands.add_parser("plan")
    deal.add_argument("--only", default="")
    deal.add_argument("--shard", default="")
    one = commands.add_parser("shard")
    one.add_argument("--out", type=Path, required=True)
    one.add_argument("--job", required=True)
    one.add_argument("--shard", type=int, required=True)
    one.add_argument("--of", type=int, required=True)
    one.add_argument("--exit", type=int, required=True)
    judge = commands.add_parser("verdict")
    judge.add_argument("--out", type=Path, required=True)
    every = commands.add_parser("gather")
    every.add_argument("--dir", type=Path, required=True)
    every.add_argument("--write", type=Path)
    every.add_argument("--expect", action="append")
    every.add_argument("--explained", type=Path)
    kept = commands.add_parser("record")
    kept.add_argument("--file", type=Path, required=True)
    args = parser.parse_args()
    if args.selftest:
        return selftest()
    if args.command == "plan":
        return plan_command(args.only, args.shard)
    if args.command == "shard":
        return shard(args.out, args.job, args.shard, args.of, args.exit)
    if args.command == "verdict":
        return verdict(args.out)
    if args.command == "record":
        return record_command(args.file)
    if args.command == "gather":
        return gather(args.dir, args.write, args.expect, args.explained)
    parser.error("a command, or --selftest")
    return 2


if __name__ == "__main__":
    sys.exit(main())
