#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""What a `cargo mutants` shard found, kept and summed.

    mutants_report.py shard   --out mutants.out --job J --shard K --of N --exit CODE
    mutants_report.py verdict --out mutants.out
    mutants_report.py gather  --dir DIR [--write DIR]

`shard` records which shard the output directory is and the tool's exit
code (`shard.json`, beside the tool's lists), and prints the four counts
and the whole missed and timeout lists. It does not judge.

`verdict` judges. A missed mutant and a timeout are reported and do not
fail (exit codes 0, 2 and 3 of cargo-mutants 27.1.0: all caught, some
missed, some timed out). Every other code fails: 1 is a usage error, 4 is
a baseline that already fails, 70 an internal error, and a code this
script has not met is not a pass. A directory without the four lists, a
shard with no mutant in it, and lists that do not add up to the mutants
the tool set out to test fail as well: a shard that tested nothing has
not passed.

`gather` reads every shard directory under `--dir`, and prints for each
job the shards it found against the shards there are, the totals, and the
missed and timeout mutants by file. With `--write` it leaves one
`<job>-missed.txt` and one `<job>-timeout.txt` there. It fails when it
finds no shard.

Text goes to standard output, and to the job summary when the runner
names one.
"""

import argparse
import json
import os
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

LISTS = ("caught", "missed", "timeout", "unviable")
# cargo-mutants 27.1.0, `book/src/exit-codes.md`: the three codes of a run
# that tested its mutants.
REPORT_ONLY = {0: "every mutant caught", 2: "mutants missed", 3: "tests timed out"}
OTHER = {
    -1: "the tool did not finish",
    1: "usage error",
    4: "the unmutated baseline fails or hangs",
    5: "diff filter mismatch",
    6: "diff filter invalid",
    70: "internal error",
}


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
    code = json.loads(record_path.read_text())["exit"]
    errors = []
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
        planned = len(json.loads(planned_path.read_text()))
        if planned != total:
            errors.append(f"the tool set out to test {planned} mutants and the lists hold {total}")
    return errors


def verdict(out: Path) -> int:
    errors = verdict_of(out)
    for error in errors:
        print(f"mutants: {error}", file=sys.stderr)
    if not errors:
        print("mutants: the shard tested what it set out to test (missed and timeout are report-only)")
    return 1 if errors else 0


def gather(root: Path, write: Path | None) -> int:
    jobs: dict[str, list[tuple[dict, dict[str, list[str]] | None]]] = defaultdict(list)
    for record_path in sorted(root.rglob("shard.json")):
        record = json.loads(record_path.read_text())
        jobs[record["job"]].append((record, read_lists(record_path.parent)))
    if not jobs:
        print(f"mutants: no shard under {root}", file=sys.stderr)
        return 1
    lines = ["# cargo mutants, all shards", ""]
    for job, shards in sorted(jobs.items()):
        shards.sort(key=lambda pair: pair[0]["shard"])
        of = shards[0][0]["of"]
        found = [record["shard"] for record, _ in shards]
        absent = [k for k in range(of) if k not in found]
        lines += [f"## {job}", ""]
        if absent:
            lines += [
                f"**Shards here: {len(found)} of {of}. The figures below are theirs alone.** "
                f"Not here: {', '.join(map(str, absent))}.",
                "",
            ]
        else:
            lines += [f"All {of} shards.", ""]
        merged: dict[str, list[str]] = {name: [] for name in LISTS}
        for record, lists in shards:
            if record["exit"] not in REPORT_ONLY or lists is None:
                lines += [
                    f"Shard {record['shard']}: exit {record['exit']}, {code_text(record['exit'])}"
                    + ("" if lists is not None else "; no lists")
                    + ".",
                    "",
                ]
            for name in LISTS:
                merged[name] += (lists or {}).get(name, [])
        counts = ", ".join(f"{len(merged[name])} {name}" for name in LISTS)
        lines += [f"{sum(len(v) for v in merged.values())} mutants: {counts}.", ""]
        lines += listing("Missed", merged["missed"]) + listing("Timeout", merged["timeout"])
        if write is not None:
            write.mkdir(parents=True, exist_ok=True)
            for name in ("missed", "timeout"):
                body = "".join(f"{line}\n" for line in sorted(merged[name]))
                (write / f"{job}-{name}.txt").write_text(body)
    emit(lines)
    return 0


def selftest() -> int:
    def make(root: Path, name: str, code: int, lists: dict[str, list[str]] | None, planned=None):
        out = root / name
        out.mkdir(parents=True)
        if lists is not None:
            for key in LISTS:
                (out / f"{key}.txt").write_text("".join(f"{m}\n" for m in lists.get(key, [])))
            count = sum(len(v) for v in lists.values()) if planned is None else planned
            (out / "mutants.json").write_text(json.dumps([{}] * count))
        shard(out, "mutants", int(name[-1]), 3, code)
        return out

    saved = os.environ.pop("GITHUB_STEP_SUMMARY", None)
    stdout, stderr = sys.stdout, sys.stderr
    sys.stdout = sys.stderr = open(os.devnull, "w", encoding="utf-8")
    failures = []
    try:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            some = {"caught": ["a.rs:1:1: x"], "missed": ["a.rs:2:1: y", "b.rs:1:1: z"]}
            late = {"caught": ["a.rs:3:1: x"], "timeout": ["b.rs:9:9: w"]}
            good, bad = root / "good", root / "bad"
            cases = [
                ("missed is report-only", make(good, "s0", 2, some), False),
                ("timeout is report-only", make(good, "s1", 3, late), False),
                ("baseline failure passed", make(bad, "s0", 4, None), True),
                ("unknown code passed", make(bad, "s1", 9, some), True),
                ("usage error passed", make(bad, "s2", 1, some), True),
                ("empty shard passed", make(bad, "s3", 0, {}), True),
                ("short lists passed", make(bad, "s4", 2, some, planned=5), True),
            ]
            for name, out, red in cases:
                if bool(verdict_of(out)) != red:
                    failures.append(name)
            if not verdict_of(root / "nowhere"):
                failures.append("a missing directory passed")
            (root / "none").mkdir()
            if gather(root / "none", None) == 0:
                failures.append("gather passed with no shard")
            written = root / "written"
            if gather(good, written) != 0:
                failures.append("gather failed on two shards")
            missed = (written / "mutants-missed.txt").read_text().splitlines()
            timeout = (written / "mutants-timeout.txt").read_text().splitlines()
            if missed != ["a.rs:2:1: y", "b.rs:1:1: z"] or timeout != ["b.rs:9:9: w"]:
                failures.append("gather wrote the wrong lists")
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
    args = parser.parse_args()
    if args.selftest:
        return selftest()
    if args.command == "shard":
        return shard(args.out, args.job, args.shard, args.of, args.exit)
    if args.command == "verdict":
        return verdict(args.out)
    if args.command == "gather":
        return gather(args.dir, args.write)
    parser.error("a command, or --selftest")
    return 2


if __name__ == "__main__":
    sys.exit(main())
