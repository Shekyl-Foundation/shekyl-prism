#!/bin/bash
# Follow-up to campaign.sh: repeat the long runs (matched 4096 showed a
# generic-class shift in runG 06). Same binaries, core 3, no sudo, node untouched.
set -u
W=$HOME/shekyl-timing; TAG=${1:-runH_matched4096_core3}; OUT=$W/results/$TAG; mkdir -p "$OUT"
log() { echo "$(date -Is) $*" >> "$OUT/progress.log"; }
sha256sum "$W/stage2c-matched" "$W/stage2c-dev-patched" > "$OUT/provenance.txt"
bash "$W/snapshot.sh" 3 > "$OUT/state_before.txt" 2>&1
taskset -c 0 python3 "$W/monitor.py" 3 "$OUT/monitor.csv" &
MON=$!
run() { local name=$1; shift; log "start $name"; env STAGE2C_ONLY=p248 "$@" > "$OUT/$name.csv" 2> "$OUT/$name.err"; log "done $name rc=$?"; }
run 01_matched_4096 taskset -c 3 "$W/stage2c-matched" matched 4096
run 02_orig_4096    taskset -c 3 "$W/stage2c-dev-patched" timing 4096
run 03_matched_4096 taskset -c 3 "$W/stage2c-matched" matched 4096
kill $MON
bash "$W/snapshot.sh" 3 > "$OUT/state_after.txt" 2>&1
log SUITE_DONE
