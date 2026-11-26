#!/bin/bash
# Many short processes, alternating ASLR on / off (setarch -R), to separate
# process-level (layout) offsets from within-process noise.
# Usage: run_procs.sh TAG CORE BIN NPAIRS ROUNDS_SPLIT
set -u
W=$HOME/shekyl-timing; TAG=$1; CORE=$2; BIN=$3; N=$4; SR=$5
OUT=$W/results/$TAG; mkdir -p "$OUT"
bash "$W/snapshot.sh" "$CORE" > "$OUT/state_before.txt" 2>&1
taskset -c 0 python3 "$W/monitor.py" "$CORE" "$OUT/monitor.csv" &
MON=$!
for i in $(seq 1 "$N"); do
  for mode in aslr noaslr; do
    pre=""; [ $mode = noaslr ] && pre="setarch $(uname -m) -R"
    echo "$(date -Is) orig $mode $i" >> "$OUT/progress.log"
    STAGE2C_ONLY=p248 taskset -c "$CORE" $pre "$BIN" timing 512 > "$OUT/orig_${mode}_$i.csv" 2>/dev/null
  done
done
for i in $(seq 1 $((N/2))); do
  for mode in aslr noaslr; do
    pre=""; [ $mode = noaslr ] && pre="setarch $(uname -m) -R"
    echo "$(date -Is) split $mode $i" >> "$OUT/progress.log"
    STAGE2C_ONLY=p248 taskset -c "$CORE" $pre "$BIN" split "$SR" $((1000+i)) > "$OUT/split_${mode}_$i.csv" 2>/dev/null
  done
done
kill $MON
bash "$W/snapshot.sh" "$CORE" > "$OUT/state_after.txt" 2>&1
echo SUITE_DONE >> "$OUT/progress.log"
