# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Parse one memcheck log (logic from research/rust-lagrange-ct/ctgrind/
run.sh): count error contexts, drop static-glibc allocator false
positives, and classify those in the constant-time code:

  branch  "Conditional jump or move depends on uninitialised value" (a
          secret-dependent branch or cmov; for cmov memcheck is
          conservative, so every one is a finding to read)
  shift   "Use of uninitialised value" at a SIMD shift whose *count* is
          secret (psrlq/psllq/vpsrlvq...): memcheck requires a defined
          count; these instructions have data-independent latency on
          x86-64, so this is not a timing finding. Checked by
          disassembling from the reported address to the next branch
          (static-pie, loaded by valgrind at 0x108000) for a SIMD
          shift with an xmm count; the size-1 use is the count byte.
  other   any other "Use of uninitialised value"/"Invalid" (e.g. a
          secret-dependent address): a finding.
Usage: parse_ctgrind.py LOG SET [BIN]"""
import re
import subprocess
import sys

log, s = sys.argv[1], sys.argv[2]
BIN = sys.argv[3] if len(sys.argv) > 3 else None
VG_BASE = 0x108000
SIMD_SHIFT = re.compile(r'^v?ps(rl|ll|ra)[wdq]|^v?ps(rl|ll|ra)v[wdq]')
txt = open(log).read()
blocks = re.split(r'\n==\d+== \n', txt)
errs = [b for b in blocks if re.search(r'(Conditional jump|Use of uninitialised|Invalid)', b)]
GLIBC = re.compile(r'at 0x[0-9A-F]+: (free|brk|sbrk|malloc|realloc|calloc|_int_\w+|sysmalloc|__mem\w+|__str\w+|__libc\w+|_dl_\w+)\b')


def top(b):
    m = re.search(r'(?:at|by) 0x[0-9A-F]+: .*', b)
    return m.group(0) if m else '?'


def insn_at(addr):
    """Mnemonic at the reported address, or the SIMD shift with a register
    count found within the next 64 bytes (memcheck attributes the count
    check to the start of the instruction group VEX translates it with)."""
    if not BIN:
        return '?'
    a = addr - VG_BASE
    out = subprocess.run(['objdump', '-d', '--no-show-raw-insn', f'--start-address={a:#x}', f'--stop-address={a + 64:#x}', BIN],
                         capture_output=True, text=True).stdout
    first = '?'
    for l in out.splitlines():
        m = re.match(r'^\s+([0-9a-f]+):\s+(\S+)\s*(\S*)', l)
        if not m:
            continue
        if first == '?':
            first = m.group(2)
        if SIMD_SHIFT.match(m.group(2)) and '%xmm' in m.group(3).split(',')[0]:
            return m.group(2)
        if m.group(2).startswith(('j', 'call', 'ret')):
            break
    return first


real = [b for b in errs if not GLIBC.search(top(b))]
in_ct = [b for b in real if 'zi::reduce' in b or 'ct.rs' in b or 'zi.rs' in b or 'prism_ct_barrier' in b
         or 'subtle' in b or 'cmov' in b or 'ctutils' in b]
kinds = {'branch': 0, 'shift': 0, 'other': 0}
ex = []
for b in in_ct:
    if 'Conditional jump' in b:
        kinds['branch'] += 1
        ex.append(('branch', top(b)))
        continue
    m = re.search(r'at 0x([0-9A-F]+):', b)
    op = insn_at(int(m.group(1), 16)) if m else '?'
    if 'Use of uninitialised value of size 1' in b and SIMD_SHIFT.match(op):
        kinds['shift'] += 1
    else:
        kinds['other'] += 1
        ex.append((f'other[{op}]', top(b)))
done = re.search(r'ctgrind: (\d+) runs done', txt)
print(f"{s}: runs={done.group(1) if done else 'NOT FINISHED'} memcheck_error_contexts={len(errs)} "
      f"static_glibc_allocator_false_positives={len(errs) - len(real)} outside_glibc={len(real)} in_ct_code={len(in_ct)} "
      f"branch={kinds['branch']} simd_shift_count={kinds['shift']} other={kinds['other']}")
for k, t in ex[:3]:
    print(f'  {k} | {t}')
