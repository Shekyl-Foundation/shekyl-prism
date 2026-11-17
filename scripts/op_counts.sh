#!/bin/sh
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# The operation-count records of the verifier.
#
#   scripts/op_counts.sh           rewrite the three records
#   scripts/op_counts.sh --check   fail unless a fresh run equals them
#
# docs/benchmarks/op-counts.txt holds the field operations of one
# verification per stage, for every honest vector and for the first
# negative of each refusal. docs/benchmarks/negative-verdicts.tsv holds the
# verdict and total cost of every negative. docs/benchmarks/op-counts-typed.txt
# holds every honest vector again through the entry split by object: the
# key's checks, the signature's checks and the verification, their sum, the
# byte-level total, and the one basis search between them. The reporter
# fails unless sum plus search is the byte-level total on every vector.
# All three are deterministic: CI runs
# --check on x86_64 and aarch64, so a change in either file is a change in
# the code and is committed with it.
#
# A record cannot name the commit that contains it. Each carries a digest
# of the sources it was counted from instead, and `git log -1 -- <record>`
# names the commit.
set -eu
case "${1:-}" in
    '') check=0 ;;
    --check) check=1 ;;
    *) echo 'usage: op_counts.sh [--check]' >&2; exit 2 ;;
esac
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$root"
counts=docs/benchmarks/op-counts.txt
verdicts=docs/benchmarks/negative-verdicts.tsv
typed=docs/benchmarks/op-counts-typed.txt
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# The counted sources: the three crates of the verifier, what pins their
# dependencies, and the vectors crate with its data: every input a count
# is taken on, and the curve and message the rows without a corpus name.
# The digest is over each file's bytes and its path, so a file that moves
# changes it. Tracked files and new files not
# yet added, never ignored ones, so the digest is the same before and
# after `git add`.
git ls-files -co --exclude-standard -- \
    Cargo.toml Cargo.lock rust-toolchain.toml prism-field prism-isogeny prism-verify \
    prism-vectors \
    | LC_ALL=C sort > "$work/files"
test -s "$work/files"
digest=$(xargs sha256sum < "$work/files" | sha256sum | cut -d' ' -f1)
# The build goes to target/ whatever CARGO_TARGET_DIR says, as the run below
# names that path.
cargo build --release --locked --target-dir target -p prism-verify \
    --example op_counts --features prism-verify/op-count >&2
target/release/examples/op_counts > "$work/raw"
{
    echo '# Field operations of one verification, by stage.'
    echo '# Written by scripts/op_counts.sh. Do not edit.'
    echo '#'
    echo '# ext_* is a call through the Field trait in the extension field, base_* a'
    echo '# call made directly in the base field. Additions and comparisons are not'
    echo '# counted. Hashing and Baillie-PSW do not use the field, so the challenge'
    echo '# stage reads zero and this metric does not see their cost.'
    echo '# Profile: release. The counts do not depend on the machine.'
    echo "# Sources: sha256 $digest"
    grep '^COUNT ' "$work/raw" | sed 's/^COUNT //'
} > "$work/counts"
{
    echo "# Sources: sha256 $digest"
    printf 'set\tvector\tcase\tverdict\text_mul\text_square\text_inv\text_sqrt\text_is_square\n'
    grep '^VERDICT' "$work/raw" | cut -f2-
} > "$work/verdicts"
{
    echo '# Field operations of one verification through the entry split by object'
    echo '# (prism_verify::prepared, which the typed interface of shekyl-prism calls).'
    echo '# Written by scripts/op_counts.sh. Do not edit.'
    echo '#'
    echo '# part=key: PreparedKey::prepare. part=signature: checked_response.'
    echo '# part=verify: verify_prepared. part=sum: those three. part=byte-level: the'
    echo '# total of prism_verify::verify on the same vector, as in op-counts.txt.'
    echo '# part=one-basis-search: basis_hint on the key, counted alone. The reporter'
    echo '# fails unless sum plus one-basis-search equals byte-level on every vector.'
    echo '# The fields are those of op-counts.txt.'
    echo "# Sources: sha256 $digest"
    grep '^TYPED ' "$work/raw" | sed 's/^TYPED //'
} > "$work/typed"
# A run that printed no line of a kind is a broken reporter, not an empty
# record.
test "$(grep -c 'stage=total' "$work/counts")" -ge 113
test "$(wc -l < "$work/verdicts")" -gt 1000
test "$(grep -c 'part=sum' "$work/typed")" -ge 113
if [ "$check" -eq 1 ]; then
    status=0
    cmp "$work/counts" "$counts" || status=1
    cmp "$work/verdicts" "$verdicts" || status=1
    cmp "$work/typed" "$typed" || status=1
    if [ "$status" -ne 0 ]; then
        echo 'the operation counts or verdicts differ from the committed records;' >&2
        echo 'run scripts/op_counts.sh and commit the result with the change' >&2
        diff "$counts" "$work/counts" | head -40 >&2 || true
        diff "$verdicts" "$work/verdicts" | head -20 >&2 || true
        diff "$typed" "$work/typed" | head -20 >&2 || true
    fi
    exit "$status"
fi
cp "$work/counts" "$counts"
cp "$work/verdicts" "$verdicts"
cp "$work/typed" "$typed"
