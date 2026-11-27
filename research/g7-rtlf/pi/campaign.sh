#!/bin/bash
# exp/g7-rtlf Pi campaign. Runs as skl-agent on skl-pi4-1, under ~/shekyl-timing only.
# No sudo; the node (shekyld@testnet) is left running.
# Usage: campaign.sh BRANCH TAG
set -u
BRANCH=${1:-exp/g7-rtlf}; TAG=${2:-runG_matched_core3}
W=$HOME/shekyl-timing; cd "$W"
export RUSTUP_HOME=$W/rustup CARGO_HOME=$W/cargo PATH=$W/cargo/bin:$PATH
OUT=$W/results/$TAG; mkdir -p "$OUT" "$W/logs"
log() { echo "$(date -Is) $*" >> "$OUT/progress.log"; }
# Build: pristine dev 470d537 (the tree every earlier P248 run used) + harness-matched.patch
# (cumulative: the p248 research harness plus the matched G7 mode).
BASE=470d537
git -C src fetch --quiet origin "$BRANCH"
[ -d src-g7rtlf ] || git -C src worktree add --quiet --detach "$W/src-g7rtlf" "$BASE"
cd src-g7rtlf && git checkout --quiet --force --detach "$BASE" && git clean -fdq
git -C "$W/src" show "origin/$BRANCH:research/g7-rtlf/harness-matched.patch" > "$W/harness-matched.patch"
git apply "$W/harness-matched.patch"
log "build start base=$(git rev-parse HEAD)"
CARGO_TARGET_DIR=$W/src/target cargo build --release --locked -p shekyl-prism --features signer --example stage2c > "$OUT/build.log" 2>&1 || { log BUILD_FAIL; exit 1; }
cp "$W/src/target/release/examples/stage2c" "$W/stage2c-matched"
cd "$W"
{ echo "base=$BASE"; sha256sum harness-matched.patch stage2c-matched stage2c-dev-patched; rustc --version; } > "$OUT/provenance.txt"
log "build done"
bash "$W/snapshot.sh" 3 > "$OUT/state_before.txt" 2>&1
taskset -c 0 python3 "$W/monitor.py" 3 "$OUT/monitor.csv" &
MON=$!
run() {  # name, env..., -- bin args
  local name=$1; shift
  log "start $name"
  env STAGE2C_ONLY=p248 "$@" > "$OUT/$name.csv" 2> "$OUT/$name.err"
  log "done $name rc=$?"
}
O="taskset -c 3 $W/stage2c-dev-patched"; M="taskset -c 3 $W/stage2c-matched"
run 01_orig_512        $O timing 512
run 02_matched_512     $M matched 512
run 03_orig_512        $O timing 512
run 04_matched_512     $M matched 512
# Positive control: a known busy-wait inside the timed interval of the guarded
# null arm (index 6 = second copy of the reference), so the true effect is the delay.
for d in 250 500 1000 2000 4000 8000; do
  run "$(printf '05_inject_%05d_512' $d)" STAGE2C_INJECT_NS=$d STAGE2C_INJECT_INDEX=6 $M matched 512
done
run 06_matched_4096    $M matched 4096
kill $MON
bash "$W/snapshot.sh" 3 > "$OUT/state_after.txt" 2>&1
log SUITE_DONE
