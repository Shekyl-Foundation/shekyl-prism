# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Phase B: for every built binary in $W/bins, the static scan
(asm_scan2.py) and the control-flow trace check under qemu
(trace_check.py), written to results/matrix/<id>.json. Resumable: an
existing result is kept. Two binaries at a time (shared box).
Sets: all five for 1.97.1 config c1; P248, P376, P500 for 1.85.1 c1;
P248 and P500 for the other configs.
Inputs: the dudect fixed row, two random pool rows, and three
adversarial rows (a Rolletschek chain, zero_xy, the ping-pong input);
at P664 only fixed, p1, a0 (10M traced blocks per run)."""
import glob
import json
import os
import re
import subprocess
import sys
from multiprocessing import Pool

HERE = os.path.dirname(os.path.abspath(__file__))
W = os.environ.get('W', '/workspace/ctb-work')
OUT = os.path.join(HERE, 'results', 'matrix')
INPUTS = ['fixed', 'p1', 'p2', 'a0', 'a2', 'a6']


def job(binpath):
    bid = os.path.basename(binpath)
    dst = os.path.join(OUT, bid + '.json')
    if os.path.exists(dst):
        return bid, 'kept'
    tc, arch, cfg, cand = re.fullmatch(r'(.+?)_(x86_64|aarch64)_(c\d)_(.+)', bid).groups()
    scan = json.loads(subprocess.run([sys.executable, os.path.join(HERE, 'harness', 'asm_scan2.py'), binpath, arch],
                                     capture_output=True, text=True, check=True).stdout)
    if cfg == 'c1' and tc == '1.97.1':
        sets = ['p248', 'p324', 'p376', 'p500', 'p664']
    elif cfg == 'c1':
        sets = ['p248', 'p376', 'p500']  # 1.85.1: comparison toolchain, cheaper
    else:
        sets = ['p248', 'p500']
    traces = {}
    for s in sets:
        inputs = ['fixed', 'p1', 'a0'] if s == 'p664' else INPUTS  # P664 traces are ~10M blocks each
        r = subprocess.run([sys.executable, os.path.join(HERE, 'harness', 'trace_check.py'), binpath, arch, s] + inputs,
                           capture_output=True, text=True)
        try:
            traces[s] = json.loads(r.stdout)
        except Exception:
            traces[s] = dict(error=r.stderr[-2000:])
    rec = dict(id=bid, toolchain=tc, arch=arch, config=cfg, candidate=cand, scan=scan['funcs'], trace=traces)
    with open(dst + '.tmp', 'w') as f:
        json.dump(rec, f)
    os.replace(dst + '.tmp', dst)
    return bid, 'done ' + ' '.join(f"{s}:{'EQ' if t.get('cf_equal') else 'DIFF'}{'' if t.get('all_ok') else '!ok'}" for s, t in traces.items())


if __name__ == '__main__':
    os.makedirs(OUT, exist_ok=True)
    # 1.85.1 is the comparison toolchain: configs c1 and c3 only (c2/c4 are built but not traced)
    bins = sorted(b for b in glob.glob(os.path.join(W, 'bins', '*')) if not b.endswith('.fail')
                  and not re.search(r'1\.85\.1_\w+?_c[24]_', os.path.basename(b)))
    with Pool(int(os.environ.get('JOBS', '2'))) as p:
        for bid, st in p.imap_unordered(job, bins):
            print(bid, st, flush=True)
    print('CHECKS DONE', flush=True)
