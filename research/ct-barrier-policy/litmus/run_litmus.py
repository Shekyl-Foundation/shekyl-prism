# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Compile the litmus kernels for every candidate x toolchain x target
(opt 3, cgu 1, --emit asm) and count conditional branches per kernel.
The kernels have fixed sizes, so loops unroll and a constant-time kernel
has no conditional branch at all; any branch is listed for review.
Writes ../results/litmus.json and ../results/litmus.md. -j1."""
import glob
import json
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
W = os.environ.get('W', '/workspace/ctb-work')
CANDS = ['asm', 'crate', 'none', 'weak', 'bbmask', 'bbuse', 'noinline', 'selunpred', 'subtle-bb', 'subtle-vol',
         'volatile', 'cmov', 'ctutils']
TARGETS = ['x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu', 'armv7-unknown-linux-gnueabihf',
           'thumbv7em-none-eabihf', 'thumbv6m-none-eabi', 'riscv64gc-unknown-linux-gnu', 'riscv32imc-unknown-none-elf']
TCS = ['1.97.1', '1.85.1']
KERNELS = ['k_dalek_sub', 'k_barrel', 'k_cswap', 'k_cmov_u8', 'k_table', 'k_cond_sub']
# conditional branch mnemonics per family (AT&T x86, LLVM asm for the rest)
BR = {
    'x86': re.compile(r'^j(?!mp)[a-z]+$'),
    'aarch64': re.compile(r'^(b\.[a-z]+|cbz|cbnz|tbz|tbnz)$'),
    'arm': re.compile(r'^(b(eq|ne|cs|hs|cc|lo|mi|pl|vs|vc|hi|ls|ge|lt|gt|le)(\.w|\.n)?|cbz|cbnz)$'),
    'riscv': re.compile(r'^(beq|bne|blt|bge|bltu|bgeu|beqz|bnez|blez|bgez|bltz|bgtz|bgt|ble|bgtu|bleu)$'),
}


def fam(t):
    return 'x86' if t.startswith('x86') else 'aarch64' if t.startswith('aarch64') else 'riscv' if t.startswith('riscv') else 'arm'


def scan(sfile, family):
    out = {}
    cur = None
    for line in open(sfile):
        m = re.match(r'^(k_\w+):', line)
        if m:
            cur = m.group(1)
            out[cur] = dict(br=0, insn=0, calls=0)
            continue
        if cur and (line.startswith('.Lfunc_end') or re.match(r'^\s*\.size\s+' + cur, line)):
            cur = None
            continue
        if cur:
            s = line.strip()
            if not s or s.startswith(('.', '@', '#', '//', '/*')) or s.endswith(':'):
                continue
            op = s.split()[0]
            out[cur]['insn'] += 1
            if BR[family].match(op):
                out[cur]['br'] += 1
            if op in ('call', 'callq', 'bl', 'blx', 'jal') or op.startswith('call'):
                out[cur]['calls'] += 1
    return out


res = {}
dst = os.path.join(HERE, '..', 'results', 'litmus.json')
if os.path.exists(dst):
    res = json.load(open(dst))
for tc in TCS:
    for t in TARGETS:
        td = f'{W}/tgt/litmus-{tc}-{t}'
        for c in CANDS:
            key = f'{tc}|{t}|{c}'
            if key in res:
                continue
            for f in glob.glob(f'{td}/{t}/release/deps/ctb_litmus-*.s'):
                os.remove(f)
            r = subprocess.run(['cargo', f'+{tc}', 'rustc', '-q', '--release', '--lib', '-j1', '--target', t,
                                '--features', f'b-{c}', '--target-dir', td, '--', '--emit', 'asm'],
                               cwd=HERE, capture_output=True, text=True,
                               env=dict(os.environ, PATH=os.path.expanduser('~/.cargo/bin') + ':' + os.environ['PATH']))
            sf = glob.glob(f'{td}/{t}/release/deps/ctb_litmus-*.s')
            if r.returncode != 0 or not sf:
                err = r.stderr.strip().splitlines()
                why = next((l for l in err if 'error' in l), err[-1] if err else '?')
                res[key] = dict(error=why[:200])
            else:
                res[key] = scan(sf[0], fam(t))
            json.dump(res, open(dst, 'w'), indent=1, sort_keys=True)
            print(key, 'ERR' if 'error' in res[key] else
                  ' '.join(f"{k[2:]}={v['br']}" for k, v in sorted(res[key].items())), flush=True)

# Table: per candidate x target, conditional branches in excess of the
# reference build on the same toolchain/target/kernel. Reference: `crate`
# (audited asm) on x86_64/aarch64, `cmov` elsewhere (its mask generation
# is asm on arm/riscv). The kernels' loops do not all unroll (the barrel
# stage loop, the table loop, and most loops on 32-bit targets), so raw
# counts include loop-counter branches; the excess over a known-opaque
# barrier is the signal, and every excess was spot-checked by eye.
def ref_for(t):
    return 'crate' if fam(t) in ('x86', 'aarch64') else 'cmov'


def excess(tc, t, c):
    e = res.get(f'{tc}|{t}|{c}')
    r = res.get(f'{tc}|{t}|{ref_for(t)}')
    if e is None or r is None:
        return None
    if 'error' in e:
        return 'n/b'
    if 'error' in r:
        return '?'
    ex = {k[2:]: e[k]['br'] - r[k]['br'] for k in KERNELS if e[k]['br'] - r[k]['br'] > 0}
    return ex


summary = {}
L = ['| candidate | ' + ' | '.join(TARGETS) + ' |', '|---|' + '---|' * len(TARGETS)]
for c in CANDS:
    row = []
    for t in TARGETS:
        cellv = []
        for tc in TCS:
            ex = excess(tc, t, c)
            summary[f'{tc}|{t}|{c}'] = ex
            if ex is None:
                cellv.append('-')
            elif isinstance(ex, str):
                cellv.append(ex)
            elif not ex:
                cellv.append('0')
            else:
                cellv.append('**+' + str(sum(ex.values())) + '** (' + ','.join(ex) + ')')
        row.append(' / '.join(cellv))
    L.append(f'| `{c}` | ' + ' | '.join(row) + ' |')
json.dump(summary, open(os.path.join(HERE, '..', 'results', 'litmus_excess.json'), 'w'), indent=1, sort_keys=True)
open(os.path.join(HERE, '..', 'results', 'litmus.md'), 'w').write(
    'Conditional branches in the six litmus kernels in excess of the reference build '
    '(`crate` on x86_64/aarch64, `cmov` elsewhere), as 1.97.1 / 1.85.1. 0 = no extra branch; '
    'n/b = does not build (ctutils MSRV 1.87, select_unpredictable needs 1.88, in-crate u64 asm and '
    'prism-ct-barrier have no 32-bit/riscv form). Raw counts are in litmus.json.\n\n' + '\n'.join(L) + '\n')
print('\n'.join(L))
