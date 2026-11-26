#!/bin/bash
# Stop shekyld@testnet, run the quiet P248 timing suite, always start the node again.
# Uses only the approved sudo -n systemctl/journalctl invocations.
set -u
W=$HOME/shekyl-timing
OUT=$W/results/runE_quiet
mkdir -p "$OUT" "$W/logs"
exec > >(tee -a "$OUT/quiet_run.log") 2>&1

STOP=(sudo -n /usr/bin/systemctl stop shekyld@testnet)
START=(sudo -n /usr/bin/systemctl start shekyld@testnet)
ACTIVE=(sudo -n /usr/bin/systemctl is-active shekyld@testnet)
STATUS=(sudo -n /usr/bin/systemctl status shekyld@testnet)
JOURNAL=(sudo -n /usr/bin/journalctl -u shekyld@testnet)

note_procs() {
  local tag=$1
  {
    echo "== procs $tag $(date -Is) =="
    ps -eo pid,user,ppid,pcpu,pmem,stat,comm,args | awk 'NR==1 || /shekyld|shekyl-wallet|guard_sample|capture_a12| tor$|\/tor / && !/awk/'
    echo "listeners:"
    ss -lnt | awk 'NR==1 || /:12030|:12021|:29701/'
    echo "is-active: $("${ACTIVE[@]}" 2>&1 || true)"
  } | tee -a "$OUT/procs_$tag.txt"
}

height_from_journal() {
  # Startup and sync lines only. Cut long lines.
  "${JOURNAL[@]}" --since "$1" --no-pager -o short-iso 2>&1 \
    | grep -iE "height|chain sync|the chain holds|store.s next height|Tor is up|Tor is carrying|Main PID|Started Shekyl|Stopped Shekyl" \
    | cut -c1-260
}

restart_node() {
  echo "==== START shekyld@testnet $(date -Is) ===="
  "${START[@]}"; echo "start rc=$?"
  local i
  for i in $(seq 1 30); do
    st=$("${ACTIVE[@]}" 2>&1 || true)
    echo "is-active try $i: $st"
    [ "$st" = active ] && break
    sleep 2
  done
  SYSTEMD_PAGER=cat "${STATUS[@]}" 2>&1 | head -20
  note_procs after_start
}

NODE_STOPPED=0
NODE_STARTED=0
on_exit() {
  if [ "$NODE_STOPPED" = 1 ] && [ "$NODE_STARTED" != 1 ]; then
    echo "==== EXIT trap: starting node $(date -Is) ===="
    restart_node
    NODE_STARTED=1
  fi
}
trap on_exit EXIT

echo "==== quiet_run begin $(date -Is) ===="
note_procs before_stop
bash "$W/snapshot.sh" 3 > "$OUT/state_node_up.txt" 2>&1
echo "==== journal snapshot before stop ===="
height_from_journal "2026-11-26 00:00:00" | tail -30 | tee "$OUT/journal_before_stop.txt"

echo "==== STOP shekyld@testnet $(date -Is) ===="
"${STOP[@]}"; echo "stop rc=$?"
NODE_STOPPED=1
for i in $(seq 1 60); do
  st=$("${ACTIVE[@]}" 2>&1 || true)
  echo "is-active after stop try $i: $st"
  if [ "$st" = inactive ] || [ "$st" = failed ]; then
    break
  fi
  sleep 2
done
# Wait until the shekyld binary is gone, in case systemd reports inactive late.
for i in $(seq 1 30); do
  if ! pgrep -x shekyld >/dev/null && ! pgrep -x tor >/dev/null; then
    echo "shekyld and tor processes gone"
    break
  fi
  echo "waiting for shekyld/tor to exit ($i)"
  pgrep -a -x shekyld || true
  pgrep -a -x tor || true
  sleep 2
done
# Do not pgrep-kill anything; just record.
note_procs after_stop
# Confirm tor is gone only as shekyld's child; a leftover tor would be noted.
bash "$W/snapshot.sh" 3 > "$OUT/state_quiet_before.txt" 2>&1

# ---- timing ----
# Dev (patched, STAGE2C_ONLY=p248): four G7 512-repeat runs, one 4096, two 10k-round splits.
# a5a6f60 (unpatched, all five sets): two G7 512-repeat runs if the first suite finished.
set +e
bash "$W/run_suite.sh" runE_quiet_core3_dev 3 "$W/stage2c-dev-patched" \
  "orig:512 orig:512 orig:512 orig:512 orig:4096 split:10000:11 split:10000:12"
DEV_RC=$?
echo "dev suite rc=$DEV_RC"
bash "$W/run_suite.sh" runF_quiet_core3_a5a6f60 3 "$W/stage2c-a5a6f60" \
  "orig:512 orig:512"
A60_RC=$?
echo "a5a6f60 suite rc=$A60_RC"
set -e
bash "$W/snapshot.sh" 3 > "$OUT/state_quiet_after.txt" 2>&1

# ---- always start ----
START_SINCE=$(date -u +"%Y-%m-%d %H:%M:%S")
restart_node
NODE_STARTED=1

echo "==== journal after start ===="
sleep 8
height_from_journal "$START_SINCE" | tee "$OUT/journal_after_start.txt"
echo "==== wait and journal again (height should advance or sync state change) ===="
sleep 30
height_from_journal "$START_SINCE" | tee "$OUT/journal_after_start_plus30s.txt"
note_procs final
echo "==== quiet_run done $(date -Is) dev_rc=$DEV_RC a60_rc=$A60_RC ===="
echo QUIET_DONE
