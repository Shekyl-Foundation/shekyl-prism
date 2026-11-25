# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Static scan of the compiled reduction for x86_64 and aarch64, one JSON
line per binary: for each set's zi::reduce and ct::exact_round (where it
is a separate symbol), counts of instructions, conditional branches,
conditional moves/selects, integer divisions, calls and (x86) the flag
source of each conditional branch (immediate compare = loop counter
shape; register compare = needs the dynamic check).

A conditional branch is not by itself a finding: loop counters compile
to branches. The verdict on secret dependence comes from trace_check.py
(control flow) and memcheck (x86); this scan gives the shape and catches
division and variable-latency instructions.

Usage: python3 asm_scan2.py BIN ARCH
"""
import json
import re
import subprocess
import sys
from collections import Counter

BIN, ARCH = sys.argv[1], sys.argv[2]
OBJDUMP = {'x86_64': 'objdump', 'aarch64': 'aarch64-linux-gnu-objdump'}[ARCH]
NM = {'x86_64': 'nm', 'aarch64': 'aarch64-linux-gnu-nm'}[ARCH]
GEN = {(11, 22, 4): 'p248', (14, 28, 6): 'p324', (16, 32, 6): 'p376', (21, 42, 8): 'p500', (27, 54, 11): 'p664'}
NOF = {22: 'p248', 28: 'p324', 32: 'p376', 42: 'p500', 54: 'p664'}
X86_JCC = re.compile(r'^j(?!mp)[a-z]+$')
A64_BC = re.compile(r'^(b\.[a-z]+|cbz|cbnz|tbz|tbnz)$')
X86_CMOV = re.compile(r'^(cmov[a-z]+|set[a-z]+)$')
A64_CSEL = re.compile(r'^(csel|csinc|csinv|csneg|cset|csetm|cinc|cinv|cneg)$')
X86_DIV = {'div', 'idiv', 'divq', 'idivq', 'divl', 'idivl'}
A64_DIV = {'udiv', 'sdiv'}
X86_VARLAT = {'bsr', 'bsf'}  # (lzcnt/tzcnt are fixed latency but were also absent before)

syms = []
for line in subprocess.run([NM, '-S', '-C', BIN], capture_output=True, text=True).stdout.splitlines():
    f = line.split(None, 3)
    if len(f) != 4:
        continue
    if f[3] in ('rust_lagrange_ct::zi::reduce', 'rust_lagrange_ct::ct::exact_round'):
        # legacy mangling (1.85.1): no generics in the name; label by address
        syms.append((f'inst@{f[0][-6:]}', f[3].split('::')[-1], int(f[0], 16), int(f[1], 16)))
        continue
    m = re.fullmatch(r'rust_lagrange_ct::zi::reduce::<(\d+), (\d+), (\d+)>', f[3])
    if m:
        syms.append((GEN[tuple(map(int, m.groups()))], 'reduce', int(f[0], 16), int(f[1], 16)))
    m = re.fullmatch(r'rust_lagrange_ct::ct::exact_round::<(\d+)>', f[3])
    if m:
        syms.append((NOF[int(m.group(1))], 'exact_round', int(f[0], 16), int(f[1], 16)))
    m = re.fullmatch(r'rust_lagrange_ct::ct::mask', f[3])
    if m:
        syms.append(('all', 'mask', int(f[0], 16), int(f[1], 16)))

out = {}
for sset, fn, start, size in sorted(syms):
    dis = subprocess.run([OBJDUMP, '-d', '--no-show-raw-insn', '-C', f'--start-address={start:#x}',
                          f'--stop-address={start + size:#x}', BIN], capture_output=True, text=True).stdout
    ins = []
    for l in dis.splitlines():
        m = re.match(r'^\s+([0-9a-f]+):\s+(\S+)\s*(.*)$', l)
        if m:
            ins.append((m.group(2), m.group(3)))
    c = Counter()
    for k, (op, arg) in enumerate(ins):
        c['insn'] += 1
        if ARCH == 'x86_64':
            if X86_JCC.match(op):
                c['condbr'] += 1
                for b in range(k - 1, max(-1, k - 6), -1):
                    o2, a2 = ins[b]
                    if o2 in ('cmp', 'test', 'inc', 'dec', 'add', 'sub', 'and', 'or', 'xor'):
                        c['condbr_imm' if a2.startswith('$') else 'condbr_reg'] += 1
                        break
                else:
                    c['condbr_other'] += 1
            if X86_CMOV.match(op):
                c['cmov_set'] += 1
            if op in X86_DIV:
                c['div'] += 1
            if op in X86_VARLAT:
                c['bsr_bsf'] += 1
            if op.startswith('call'):
                c['call'] += 1
                if 'panic' in arg:
                    c['call_panic'] += 1
        else:
            if A64_BC.match(op):
                c['condbr'] += 1
                if op in ('cbz', 'cbnz', 'tbz', 'tbnz'):
                    c['condbr_reg'] += 1
                else:
                    # b.cond: flag source = nearest cmp/cmn/tst/subs/adds
                    for b in range(k - 1, max(-1, k - 6), -1):
                        o2, a2 = ins[b]
                        if o2 in ('cmp', 'cmn', 'tst', 'subs', 'adds', 'ands', 'ccmp'):
                            c['condbr_imm' if '#' in a2.split(',')[-1] else 'condbr_reg'] += 1
                            break
                    else:
                        c['condbr_other'] += 1
            if A64_CSEL.match(op):
                c['cmov_set'] += 1
            if op in A64_DIV:
                c['div'] += 1
            if op == 'bl' or op == 'blr':
                c['call'] += 1
                if 'panic' in arg:
                    c['call_panic'] += 1
    out[f'{sset}:{fn}'] = dict(c, size=size)
print(json.dumps(dict(bin=BIN, arch=ARCH, funcs=out)))
