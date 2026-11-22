# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Static scan of the compiled constant-time reduction, one instantiation
per set (`zi::reduce::<C, N, PL>`): integer divisions, calls (and the
size argument of each memcpy), conditional branches and their flag-setting
instruction, cmov/setcc, variable shifts. Usage: python3 asm_scan.py BIN"""
import re
import subprocess
import sys
from collections import Counter

BIN = sys.argv[1]
VERBOSE = '-v' in sys.argv
nm = subprocess.run(['nm', '-S', '-C', BIN], capture_output=True, text=True).stdout
funcs = []
for line in nm.splitlines():
    f = line.split(None, 3)
    if len(f) == 4 and f[3] == 'rust_lagrange_ct::zi::reduce':
        funcs.append((int(f[0], 16), int(f[1], 16)))
funcs.sort()
# instantiation order follows the set order in which they were emitted; we
# identify each by the PL-limb constant p it multiplies by: report size only
JCC = re.compile(r'^j(?!mp)[a-z]+$')
for start, size in funcs:
    out = subprocess.run(['objdump', '-d', '--no-show-raw-insn', '-C',
                          f'--start-address={start:#x}', f'--stop-address={start + size:#x}', BIN],
                         capture_output=True, text=True).stdout
    ins = []
    for l in out.splitlines():
        m = re.match(r'^\s+([0-9a-f]+):\s+(\S+)\s*(.*)$', l)
        if m:
            ins.append((int(m.group(1), 16), m.group(2), m.group(3)))
    review = []
    c = Counter()
    flagsrc = Counter()
    calls = Counter()
    for k, (a, op, arg) in enumerate(ins):
        c['insn'] += 1
        if op in ('div', 'idiv', 'divq', 'idivq', 'divl', 'idivl'):
            c['int_div'] += 1
        if op.startswith('cmov'):
            c['cmov'] += 1
        if op.startswith('set'):
            c['setcc'] += 1
        if op in ('mul', 'mulq', 'mulx', 'imul'):
            c['mul'] += 1
        if op in ('shr', 'shl', 'sar', 'shrd', 'shld', 'shrx', 'shlx', 'sarx') and '%cl' in arg:
            c['var_shift_cl'] += 1
        if op in ('shrx', 'shlx', 'sarx'):
            c['var_shift_bmi'] += 1
        if op.startswith('call'):
            tgt = arg.split('<')[-1].rstrip('>') if '<' in arg else arg
            nbytes = None
            for b in range(k - 1, max(0, k - 8), -1):
                if ins[b][1] == 'mov' and ins[b][2].endswith(',%edx') and ins[b][2].startswith('$'):
                    nbytes = ins[b][2].split(',')[0]
                    break
            calls[f'{tgt} n={nbytes}'] += 1
        if JCC.match(op):
            c['jcc'] += 1
            # flag source: nearest preceding flag-setting instruction
            for b in range(k - 1, max(0, k - 6), -1):
                o2, a2 = ins[b][1], ins[b][2]
                if o2 in ('cmp', 'test', 'inc', 'dec', 'add', 'sub', 'and', 'or', 'xor'):
                    kind = 'imm' if a2.startswith('$') else 'reg/mem'
                    flagsrc[f'{o2} {kind}'] += 1
                    if kind != 'imm' and VERBOSE:
                        review.append(ins[max(0, k - 5):k + 1])
                    break
            else:
                flagsrc['other'] += 1
                if VERBOSE:
                    review.append(ins[max(0, k - 5):k + 1])
    print(f'reduce @ {start:#x} size {size} bytes: ' + ', '.join(f'{k}={v}' for k, v in sorted(c.items())))
    print('   jcc flag sources: ' + ', '.join(f'{k}={v}' for k, v in flagsrc.most_common()))
    print('   calls: ' + ', '.join(f'{k} x{v}' for k, v in calls.most_common()))
    for r in review:
        print('   --- non-immediate branch, context:')
        for a, op, arg in r:
            print(f'      {a:x}: {op} {arg}')
