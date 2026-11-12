#!/bin/sh
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Reproduce the full comparison in pinned Sage, keeping evidence even if
# the invoking client disconnects. Build static Rust probes first:
# cargo build --release --locked --target x86_64-unknown-linux-musl \
#   -p prism-verify --bin s5_challenge --bin verify_bytes
set -eu
sage_run=${1:?usage: run_container.sh UNIQUE-RUN-NAME}
case "$sage_run" in *[!a-zA-Z0-9_-]*|'') echo 'invalid run name' >&2; exit 2;; esac
sage_root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
sage_bins=target/x86_64-unknown-linux-musl/release
sage_image=sagemath/sagemath@sha256:4f5589eb6c565949a006f8665de2876b8414410daf5ac554f4434a15d4f3d528
test -x "$sage_root/$sage_bins/s5_challenge"
test -x "$sage_root/$sage_bins/verify_bytes"
mkdir -p "$sage_root/target/sage"
mkdir "$sage_root/target/sage/$sage_run"
(
    cd "$sage_root"
    git rev-parse HEAD
    printf '%s\n' "$sage_image"
    sha256sum scripts/sage/differential.sage scripts/sage/s5_model.sage "$sage_bins/s5_challenge" "$sage_bins/verify_bytes"
) > "$sage_root/target/sage/$sage_run/provenance.txt"
exec docker run --rm --cpus=1 --user "$(id -u):$(id -g)" --entrypoint /bin/sh \
    -e PRISM_ROOT=/workspace -e "PRISM_RUST_BIN_DIR=/workspace/$sage_bins" \
    -e "PRISM_SAGE_RUN=$sage_run" -v "$sage_root:/workspace" -w /workspace "$sage_image" \
    -c 'sage -python scripts/sage/differential.sage > "target/sage/$PRISM_SAGE_RUN/run.log" 2>&1; sage_status=$?; printf "%s\n" "$sage_status" > "target/sage/$PRISM_SAGE_RUN/status"; exit "$sage_status"'
