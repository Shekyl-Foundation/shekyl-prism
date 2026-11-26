#!/bin/bash
# Usage: run_suite.sh TAG CORE BIN "SPEC SPEC ..."   SPEC = orig:REPEATS | split:ROUNDS:SEED
# orig  = the G7 `timing REPEATS` mode (adjacent reference/other pairs, deterministic order)
# split = phase-split randomized rounds (research harness)
set -u
W=$HOME/shekyl-timing; TAG=$1; CORE=$2; BIN=$3; SUITE=$4
OUT=$W/results/$TAG; mkdir -p "$OUT"
bash "$W/snapshot.sh" "$CORE" > "$OUT/state_before.txt" 2>&1
taskset -c 0 python3 "$W/monitor.py" "$CORE" "$OUT/monitor.csv" &
MON=$!
n=0
for spec in $SUITE; do
  n=$((n+1)); IFS=: read kind a b <<< "$spec"
  name=$(printf "%02d_%s_%s%s" $n "$kind" "$a" "${b:+_s$b}")
  case $kind in
    orig)  args="timing $a";;
    split) args="split $a $b";;
  esac
  echo "$(date -Is) start $name" >> "$OUT/progress.log"
  STAGE2C_ONLY=${ONLY:-p248} taskset -c "$CORE" "$BIN" $args > "$OUT/$name.csv" 2> "$OUT/$name.err"
  echo "$(date -Is) done $name rc=$?" >> "$OUT/progress.log"
done
kill $MON
bash "$W/snapshot.sh" "$CORE" > "$OUT/state_after.txt" 2>&1
echo SUITE_DONE >> "$OUT/progress.log"
