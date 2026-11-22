#!/usr/bin/env bash
# Resumable driver: every stage skips outputs that already exist.
set -euo pipefail
cd "$(dirname "$0")"
PY=${PY:-python3}
mkdir -p results
$PY run_fsr.py --sweep-msgs 40                                   >> results/pass1.log 2>&1
$PY run_fsr.py --panels nvar nbound dudect_fixed dudect_rand     >> results/pass1.log 2>&1
$PY run_zi_tail.py                                               >> results/tail.log 2>&1
$PY analyze.py --schedules                                       >> results/analyze.log 2>&1
$PY run_budget.py                                                >> results/pass2.log 2>&1
$PY crosscheck_pari.py                                           >> results/pari_crosscheck.log 2>&1
$PY analyze.py                                                   >> results/analyze.log 2>&1
echo CHAIN_DONE                                                  >> results/analyze.log
