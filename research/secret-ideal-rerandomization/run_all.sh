#!/bin/sh
# Resumable driver: each chunk is skipped if its CSV exists.
set -e
cd "$(dirname "$0")"
PY=${PY:-/workspace/mamba/envs/sage/bin/python}
$PY run_rr.py --panels samp sampg --procs 2
$PY run_rr.py --panels pair --pair-keys 150 --pair-msgs 6 --procs 2
$PY run_rr.py --panels dudect_fixed dudect_rand --dudect 1000 --procs 2
