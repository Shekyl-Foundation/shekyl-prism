#!/bin/bash
# exp/g7-rule-v2 Pi campaign. Runs as skl-agent on skl-pi4-1, under ~/shekyl-timing only.
# No sudo. shekyld is left exactly as found; its state is recorded at start and end.
# Usage: pi_campaign.sh BRANCH TAG ROUNDS SEED...
set -u
BRANCH=$1; TAG=$2; ROUNDS=$3; shift 3; SEEDS="$*"
W=$HOME/shekyl-timing; cd "$W"
export RUSTUP_HOME=$W/rustup CARGO_HOME=$W/cargo PATH=$W/cargo/bin:$PATH
OUT=$W/results/$TAG; mkdir -p "$OUT" "$W/logs"
log() { echo "$(date -Is) $*" >> "$OUT/progress.log"; }
node() { { echo "== $1 $(date -Is)"; systemctl show shekyld@testnet -p ActiveState,SubState,Result,ExecMainStatus,ActiveEnterTimestamp,InactiveEnterTimestamp,NRestarts --no-pager; pgrep -a shekyld | cut -c1-60; ps -eLo psr,pcpu,comm | awk '$3=="shekyld"' | sort -k2 -nr | head -3; } >> "$OUT/node_state.txt" 2>&1; }
node start
if [ ! -x "$W/stage2c-v2" ]; then
  BASE=470d537
  git -C src fetch --quiet origin "$BRANCH"
  [ -d src-g7rtlf ] || git -C src worktree add --quiet --detach "$W/src-g7rtlf" "$BASE"
  ( cd src-g7rtlf && git checkout --quiet --force --detach "$BASE" && git clean -fdq &&
    git -C "$W/src" show "origin/$BRANCH:research/g7-rtlf/v2/harness-v2.patch" > "$W/harness-v2.patch" && git apply "$W/harness-v2.patch" &&
    CARGO_TARGET_DIR=$W/src/target cargo build --release --locked -p shekyl-prism --features signer --example stage2c > "$OUT/build.log" 2>&1 ) || { log BUILD_FAIL; exit 1; }
  cp "$W/src/target/release/examples/stage2c" "$W/stage2c-v2"
fi
git -C src show "origin/$BRANCH:research/g7-rtlf/v2/keypool_p248.txt.xz" | xz -dc > "$W/keypool_p248.txt"
{ sha256sum harness-v2.patch stage2c-v2 keypool_p248.txt; rustc --version; echo "base=470d537"; } > "$OUT/provenance.txt"
bash "$W/snapshot.sh" 3 > "$OUT/state_before.txt" 2>&1
taskset -c 0 python3 "$W/monitor.py" 3 "$OUT/monitor.csv" &
MON=$!
export STAGE2C_ONLY=p248 STAGE2C_KEYPOOL=$W/keypool_p248.txt
for seed in $SEEDS; do
  log "start seed=$seed rounds=$ROUNDS"
  taskset -c 3 "$W/stage2c-v2" v2 "$ROUNDS" "$seed" > "$OUT/v2_${ROUNDS}_s${seed}.csv" 2> "$OUT/v2_${ROUNDS}_s${seed}.err"
  log "done seed=$seed rc=$?"
  node "after seed $seed"
done
kill $MON
bash "$W/snapshot.sh" 3 > "$OUT/state_after.txt" 2>&1
node end
log SUITE_DONE
