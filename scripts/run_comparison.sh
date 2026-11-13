#!/bin/sh
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Run one comparison driver from a committed tree and keep its raw output
# with the commit, toolchain, backend digests, command and load.
#
#   scripts/run_comparison.sh <record> <driver> [driver arguments...]
#
# <record> names docs/comparison/<record>.txt and its .provenance sibling,
# for example e3/trial-07. <driver> is a prism-compare binary: density,
# index_leak, unsignable or replay. The native backend, where a driver
# needs it, is target/native, built by `scripts/export_s5_vectors.sh
# backend`. Tracked files must match HEAD; an existing record is never
# overwritten. The exit status is the driver's.
set -eu
record=${1:?usage: run_comparison.sh RECORD DRIVER [ARGS...]}
driver=${2:?usage: run_comparison.sh RECORD DRIVER [ARGS...]}
shift 2
case "$record" in *[!a-zA-Z0-9/_-]*|''|/*|*..*) echo 'invalid record name' >&2; exit 2;; esac
case "$driver" in density|index_leak|unsignable|replay) ;; *) echo 'unknown driver' >&2; exit 2;; esac
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$root"
if ! git diff --quiet HEAD --; then
    echo 'tracked files differ from HEAD; commit before recording a run' >&2
    exit 2
fi
out=docs/comparison/$record.txt
note=docs/comparison/$record.provenance
if [ -e "$out" ] || [ -e "$note" ]; then
    echo "$out already recorded" >&2
    exit 2
fi
mkdir -p "$(dirname "$out")"
# The digest and the run below name target/release, so the build is put
# there whatever CARGO_TARGET_DIR or a cargo config says.
cargo build --release --locked --target-dir target -p prism-compare --features experiments --bin "$driver" >&2
{
    printf 'commit %s\n' "$(git rev-parse HEAD)"
    printf 'toolchain %s\n' "$(rustc --version)"
    printf 'machine %s\n' "$(uname -m)"
    printf 'command target/release/%s %s\n' "$driver" "$*"
    sha256sum "target/release/$driver"
    for backend in target/native/prism_signer_lvl1 target/native/prism_signer_lvl3 target/native/prism_signer_lvl5; do
        if [ -x "$backend" ]; then sha256sum "$backend"; fi
    done
    printf 'started %s load %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(cut -d' ' -f1-3 /proc/loadavg)"
} > "$note"
status=0
"target/release/$driver" "$@" > "$out" || status=$?
{
    printf 'finished %s load %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(cut -d' ' -f1-3 /proc/loadavg)"
    printf 'exit %s\n' "$status"
    sha256sum "$out"
} >> "$note"
exit "$status"
