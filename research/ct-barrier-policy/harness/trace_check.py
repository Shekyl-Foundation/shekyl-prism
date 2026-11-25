# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Control-flow trace equivalence under qemu-user.

For one binary and one set, runs `rlc one <set> <input>` under
`qemu-<arch> -d nochain,exec -dfilter <ranges>`, where the ranges are the
set's instantiations of zi::reduce and ct::exact_round (runtime addresses
from `RLC_ADDR=1`, sizes from nm). Every translated block executed inside
those functions is logged by guest PC. A fixed-schedule constant-time
reduction must give the *same* PC sequence on every input; a
secret-dependent branch gives different sequences. Memory addresses and
instruction latency are not covered (memcheck and dudect cover those on
x86-64). Timing under qemu means nothing; only the trace is used.

Usage: python3 trace_check.py BIN ARCH SET INPUT... -> one JSON line
"""
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile

BIN, ARCH, SET = sys.argv[1:4]
INPUTS = sys.argv[4:]
QEMU = {'x86_64': 'qemu-x86_64', 'aarch64': 'qemu-aarch64'}[ARCH]
NM = {'x86_64': 'nm', 'aarch64': 'aarch64-linux-gnu-nm'}[ARCH]
QARGS = []  # aarch64 binaries are static (see run_builds.sh)
GEN = {'p248': (11, 22, 4), 'p324': (14, 28, 6), 'p376': (16, 32, 6), 'p500': (21, 42, 8), 'p664': (27, 54, 11)}
C, N, PL = GEN[SET]
env = dict(os.environ, RLC_ADDR='1')
hd = os.path.dirname(os.path.abspath(__file__))

out = subprocess.run([QEMU] + QARGS + [BIN, 'one', SET, INPUTS[0]], capture_output=True, text=True, env=env, cwd=hd).stdout
m = re.search(r'addr reduce (0x[0-9a-f]+) exact_round (0x[0-9a-f]+)', out)
rt_red, rt_er = int(m.group(1), 16), int(m.group(2), 16)
syms = {}
for line in subprocess.run([NM, '-S', '-C', BIN], capture_output=True, text=True).stdout.splitlines():
    f = line.split(None, 3)
    if len(f) == 4:
        syms[f[3]] = (int(f[0], 16), int(f[1], 16))
red = syms[f'rust_lagrange_ct::zi::reduce::<{C}, {N}, {PL}>']
er = syms.get(f'rust_lagrange_ct::ct::exact_round::<{N}>')
base = rt_red - red[0]
ranges = [(base + red[0], base + red[0] + red[1])]
if er:
    assert rt_er - er[0] == base
    ranges.append((base + er[0], base + er[0] + er[1]))
else:
    # exact_round inlined (or merged) into reduce: covered by reduce's range
    pass
dfilter = ','.join(f'{a:#x}..{b - 1:#x}' for a, b in ranges)
res = {}
TR = re.compile(r'^Trace \d+: 0x[0-9a-f]+ \[[0-9a-f]+/([0-9a-f]+)/')


def trace_one(inp):
    with tempfile.NamedTemporaryFile(suffix='.log', dir=os.environ.get('TMPDIR', '/tmp')) as tf:
        r = subprocess.run([QEMU] + QARGS + ['-d', 'nochain,exec', '-dfilter', dfilter, '-D', tf.name, BIN, 'one', SET, inp],
                           capture_output=True, text=True, cwd=hd)
        ok = 'ok=true' in r.stdout
        # guest PC is the second '/'-separated field of each "Trace" line
        sha = subprocess.run(f"awk -F/ '/^Trace/{{print $2}}' {tf.name} | sha256sum", shell=True,
                             capture_output=True, text=True).stdout.split()[0]
        n = int(subprocess.run(['grep', '-c', '^Trace', tf.name], capture_output=True, text=True).stdout.strip() or 0)
    return dict(ok=ok, tbs=n, sha=sha[:16], rc=r.returncode)


for inp in INPUTS:
    rec = trace_one(inp)
    if rec['tbs'] == 0:  # an empty log is a harness failure, not a trace: retry once
        rec = trace_one(inp)
        rec['retried'] = True
    res[inp] = rec
shas = {v['sha'] for v in res.values()}
print(json.dumps(dict(bin=BIN, arch=ARCH, set=SET, exact_round_symbol=bool(er), dfilter=dfilter,
                      all_ok=all(v['ok'] for v in res.values()), cf_equal=len(shas) == 1,
                      distinct_traces=len(shas), tbs=sorted({v['tbs'] for v in res.values()}), per_input=res)))
