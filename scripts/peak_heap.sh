#!/bin/sh
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

# Peak heap and stack of one verification, per set, under Valgrind's Massif.
#
#   sh scripts/peak_heap.sh > peak-heap.txt
#
# Two kinds of verification are measured, each as a pair of runs of the
# `verify_once` example whose difference is what the verification holds:
#
#   accept   for each of the six primary vectors: `load` (read and decode
#            the vector, no verification) and `verify` (the same, then one
#            verification that accepts).
#   refuse   for the same six vectors and for each set that has no
#            vectors: `build` (build the response the count records call
#            identity-response-q-difference, no verification) and `refuse`
#            (the same, then one verification, refused as Codomain after
#            both chains). A set without vectors has no accepted
#            verification to measure, so this is its only figure.
#
# Each pair is run with the heap alone, measured to the byte, and its
# second run once more with the stack included. A `PEAK` line gives the
# three; `SITE` lines give what held the heap at its peak, by the function
# that asked for it. Needs valgrind and python3.
set -eu
cd "$(dirname "$0")/.."
command -v valgrind >/dev/null || { echo 'valgrind is not installed' >&2; exit 2; }
cargo build --release --locked -p prism-testkit --example verify_once >&2
bin=target/release/examples/verify_once
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
echo "# Written by scripts/peak_heap.sh."
echo "# $(valgrind --version); $(rustc --version); $(uname -srm)"
echo "# source_commit=$(git rev-parse HEAD) source_dirty=$([ -n "$(git status --porcelain)" ] && echo true || echo false)"
# kind, input, the run that does not verify, the run that does.
cases=""
for n in 1 1b 3 3b 5 5b; do
    vector=prism-vectors/data/verify/s5_lvl$n.txt
    test -f "$vector" || { echo "missing $vector" >&2; exit 1; }
    cases="$cases accept:$vector:load:verify"
done
for n in 1 1b 3 3b 5 5b; do
    cases="$cases refuse:prism-vectors/data/verify/s5_lvl$n.txt:build:refuse"
done
for name in p324 p664; do
    cases="$cases refuse:$name:build:refuse"
done
for case in $cases; do
    kind=${case%%:*}; rest=${case#*:}
    input=${rest%%:*}; rest=${rest#*:}
    before=${rest%%:*}; after=${rest#*:}
    valgrind -q --tool=massif --peak-inaccuracy=0.0 --massif-out-file="$work/load" "$bin" "$input" "$before"
    valgrind -q --tool=massif --peak-inaccuracy=0.0 --massif-out-file="$work/verify" "$bin" "$input" "$after"
    valgrind -q --tool=massif --peak-inaccuracy=0.0 --stacks=yes --massif-out-file="$work/stack" "$bin" "$input" "$after"
    python3 - "$kind" "$(basename "$input")" "$work/load" "$work/verify" "$work/stack" <<'PY'
import re
import sys

kind, name, load, verify, stack = sys.argv[1:]


def snapshots(path):
    found = open(path).read().split("#-----------\nsnapshot=")[1:]
    if not found:
        raise SystemExit(f"{path}: no snapshots")
    return found


def field(snapshot, key):
    return int(re.search(rf"^{key}=(\d+)$", snapshot, re.M).group(1))


def peak(path):
    """The peak snapshot; with inaccuracy 0 it is the true heap maximum."""
    marked = [s for s in snapshots(path) if "heap_tree=peak" in s]
    if len(marked) != 1:
        raise SystemExit(f"{path}: {len(marked)} peak snapshots")
    top = max(field(s, "mem_heap_B") for s in snapshots(path))
    if field(marked[0], "mem_heap_B") != top:
        raise SystemExit(f"{path}: the peak snapshot is not the maximum")
    return marked[0]


before, after = peak(load), peak(verify)
# The stack is sampled, not tracked to its peak: the largest sample seen.
deepest = max(field(s, "mem_stacks_B") for s in snapshots(stack))
# The accepted rows keep the field names their earlier records have.
first, second = ("load", "verify") if kind == "accept" else ("build", "refuse")
tag = f"vector={name}" if kind == "accept" else f"refusal={name}"
print(f"PEAK {tag} {first}_heap_bytes={field(before, 'mem_heap_B')} "
      f"{second}_heap_bytes={field(after, 'mem_heap_B')} "
      f"allocator_overhead_bytes={field(after, 'mem_heap_extra_B')} "
      f"largest_stack_sample_bytes={deepest}")
# Each depth-1 node is an allocation site; name it by the first frame of
# this repository at or below it.
sites = {}
size, owner = 0, None
for line in after.splitlines():
    node = re.match(r"( *)n\d+: (\d+) (.*)", line)
    if not node:
        continue
    depth, amount, frame = len(node.group(1)), int(node.group(2)), node.group(3)
    if depth == 1:
        if size:
            sites[owner or "other"] = sites.get(owner or "other", 0) + size
        size, owner = amount, None
    if depth >= 1 and owner is None:
        named = re.search(r"(prism_\w+(?:::\w+|<[^>]*>)*)", frame)
        if named:
            owner = re.sub(r"<[^>]*>", "", named.group(1))
if size:
    sites[owner or "other"] = sites.get(owner or "other", 0) + size
for owner, amount in sorted(sites.items(), key=lambda item: -item[1]):
    print(f"SITE {tag} bytes={amount} holder={owner}")
PY
done
