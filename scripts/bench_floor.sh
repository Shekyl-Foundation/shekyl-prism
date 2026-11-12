#!/bin/bash
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
set -euo pipefail
cd "$(dirname "$0")/.."
model=$(python3 -c 'from pathlib import Path; print(Path("/proc/device-tree/model").read_bytes().rstrip(b"\0").decode())')
case "$model" in
    'Raspberry Pi 4 Model B'*) ;;
    *) echo "floor benchmark requires Raspberry Pi 4 Model B; found $model" >&2; exit 2 ;;
esac
test "$(uname -m)" = aarch64
printf 'device=%s\n' "$model"
printf 'source_commit=%s\n' "$(git rev-parse HEAD)"
if [ -n "$(git status --porcelain)" ]; then
    echo 'source_dirty=true (preserve the source diff with this measurement)'
else
    echo 'source_dirty=false'
fi
uname -srvmo
rustc --version
for governor in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    if [ -r "$governor" ]; then
        printf '%s=' "$governor"
        cat "$governor"
    fi
done
cargo run --release --locked -p prism-bench
