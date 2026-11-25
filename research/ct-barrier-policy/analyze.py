# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Tables for REPORT.md: results/matrix/*.json (asm scan + qemu trace),
results/ctgrind/*.txt (memcheck taint, x86-64), harness/results/raw/
dudect_*.csv (x86-64). Writes results/tables.md and results/summary.json.

Verdicts:
  trace  EQ   = identical guest-PC sequence inside zi::reduce and
                ct::exact_round on every input of every checked set, and
                every output correct; DIFF = at least one set differs.
  asm-clean   = trace EQ and no integer division instruction in those
                functions (x86 div/idiv, aarch64 udiv/sdiv).
  taint clean = memcheck reports 0 error contexts in the constant-time
                code on all five sets.
  dudect pass = max |t| < 4.5 (raw, cropped, 2nd order) at P248 and P500.
"""
import glob
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, 'harness'))
from analyze_dudect_orig import dudect  # noqa: E402

CANDS = ['asm', 'crate', 'none', 'weak', 'bbmask', 'bbuse', 'noinline', 'selunpred', 'subtle-bb', 'subtle-vol',
         'volatile', 'cmov', 'ctutils']
UNSAFE = {
    'asm': 'unsafe asm in the signer crate',
    'crate': 'unsafe asm, isolated in prism-ct-barrier',
    'none': 'safe (negative control)',
    'weak': 'safe (negative control)',
    'bbmask': 'safe',
    'bbuse': 'safe',
    'noinline': 'safe',
    'selunpred': 'safe (Rust >= 1.88)',
    'subtle-bb': 'safe (subtle, core_hint_black_box)',
    'subtle-vol': 'unsafe read_volatile inside subtle',
    'volatile': 'unsafe read_volatile in the signer crate',
    'cmov': 'unsafe asm inside cmov',
    'ctutils': 'unsafe asm inside cmov (via ctutils)',
}
CFGS = ['c1', 'c2', 'c3', 'c4']
CFG_DESC = {'c1': 'opt 3, fat LTO, cgu 1', 'c2': 'opt 2, fat LTO, cgu 1', 'c3': 'opt 3, no LTO, cgu 16',
            'c4': 'opt 3, fat LTO, cgu 1, target-cpu (x86: native = this Xeon, AVX-512; aarch64: neoverse-v1)'}
TCS = ['1.97.1', '1.85.1']
ARCHS = ['x86_64', 'aarch64']


def load_matrix():
    m = {}
    for f in glob.glob(os.path.join(HERE, 'results', 'matrix', '*.json')):
        d = json.load(open(f))
        tr = d['trace']
        eq = all(t.get('cf_equal') and t.get('all_ok') for t in tr.values())
        diff_sets = [s for s, t in tr.items() if not t.get('cf_equal')]
        bad_out = [s for s, t in tr.items() if not t.get('all_ok')]
        div = sum(v.get('div', 0) for v in d['scan'].values())
        brreg = sum(v.get('condbr_reg', 0) for k, v in d['scan'].items())
        br = sum(v.get('condbr', 0) for k, v in d['scan'].items())
        cm = sum(v.get('cmov_set', 0) for k, v in d['scan'].items())
        has_mask_fn = any(k.endswith(':mask') for k in d['scan'])
        m[(d['toolchain'], d['arch'], d['config'], d['candidate'])] = dict(
            eq=eq, diff_sets=diff_sets, bad_out=bad_out, div=div, condbr=br, condbr_reg=brreg, csel=cm,
            clean=eq and div == 0, sets=list(tr), mask_fn=has_mask_fn)
    fails = set()
    for f in glob.glob('/workspace/ctb-work/bins/*.fail'):
        b = os.path.basename(f)[:-5]
        mm = re.fullmatch(r'(.+?)_(x86_64|aarch64)_(c\d)_(.+)', b)
        fails.add(mm.groups())
    return m, fails


def load_ctgrind():
    r = {}
    for f in glob.glob(os.path.join(HERE, 'results', 'ctgrind', '*.txt')):
        b = os.path.basename(f)[:-4]
        txt = open(f).read()
        if 'build failed' in txt:
            r[b] = dict(status='build failed')
            continue
        sets = re.findall(r'^(p\d+): runs=(\S+) .* outside_glibc=(\d+) in_ct_code=(\d+)', txt, re.M)
        n_in = sum(int(s[3]) for s in sets)
        n_out = sum(int(s[2]) for s in sets)
        fin = all(s[1] != 'NOT FINISHED' for s in sets)
        r[b] = dict(status='ok' if fin else 'not finished', sets=len(sets), in_ct=n_in, outside_glibc=n_out,
                    clean=fin and n_in == 0 and len(sets) == 5)
    return r


def load_dudect():
    r = {}
    for f in glob.glob(os.path.join(HERE, 'harness', 'results', 'raw', 'dudect_*_ct_*.csv')):
        b = os.path.basename(f)[len('dudect_'):-4]
        s, rest = b.split('_ct_', 1)
        r[(s, rest)] = dudect(f)
    return r


def cell(m, fails, tc, arch, cand):
    vals = []
    for cfg in CFGS:
        k = (tc, arch, cfg, cand)
        if k in fails:
            vals.append('build')
        elif k in m:
            vals.append('ok' if m[k]['clean'] else 'BR')
        else:
            vals.append('-')
    if all(v == 'build' for v in vals):
        return 'n/a (does not build)'
    if all(v == '-' for v in vals):
        return 'pending'
    n_ok = vals.count('ok')
    n = sum(v in ('ok', 'BR') for v in vals)
    bad = [CFGS[i] for i, v in enumerate(vals) if v == 'BR']
    return f"{n_ok}/{n} clean" + (f" (branch: {','.join(bad)})" if bad else '')


def main():
    m, fails = load_matrix()
    cg = load_ctgrind()
    du = load_dudect()
    L = []
    L.append('## Recommendation table\n')
    L.append('asm-clean: control-flow trace identical across fixed, random and adversarial inputs under qemu '
             '(and correct outputs), no integer division; counted over the 4 build configs. '
             'Taint: memcheck errors in the constant-time code, 5 sets, x86-64, config c1. '
             'dudect: max |t| at P248 / P500, x86-64, 1.97.1, c1 (threshold 4.5).\n')
    L.append('| candidate | unsafe where | toolchain | x86-64 asm-clean | aarch64 asm-clean | x86-64 taint | x86-64 dudect max\\|t\\| (P248 / P500) |')
    L.append('|---|---|---|---|---|---|---|')
    summ = {}
    for cand in CANDS:
        for tc in TCS:
            x = cell(m, fails, tc, 'x86_64', cand)
            a = cell(m, fails, tc, 'aarch64', cand)
            g = cg.get(f'{tc}_c1_{cand}')
            gt = 'pending' if g is None else (g['status'] if g['status'] != 'ok' else
                                               ('0 (clean)' if g['clean'] else f"{g['in_ct']} errors"))
            ds = []
            for s in ('p248', 'p500'):
                d = du.get((s, f'{cand}_{tc}_c1'))
                ds.append('-' if d is None else f"{d['t_max_abs']:.2f}")
            dt = ' / '.join(ds) if any(v != '-' for v in ds) else ('not run' if tc == '1.85.1' else '-')
            L.append(f'| `{cand}` | {UNSAFE[cand]} | {tc} | {x} | {a} | {gt} | {dt} |')
            summ[f'{cand}|{tc}'] = dict(x86=x, aarch64=a, taint=gt, dudect=dt)
    L.append('\n## Per configuration (trace verdict: ok / BR = secret-dependent control flow / build = does not compile)\n')
    for c, dsc in CFG_DESC.items():
        L.append(f'- `{c}`: {dsc}')
    L.append('')
    L.append('| candidate | ' + ' | '.join(f'{tc} {a[:3]} {c}' for tc in TCS for a in ARCHS for c in CFGS) + ' |')
    L.append('|---|' + '---|' * (len(TCS) * len(ARCHS) * len(CFGS)))
    for cand in CANDS:
        row = []
        for tc in TCS:
            for a in ARCHS:
                for c in CFGS:
                    k = (tc, a, c, cand)
                    if k in fails:
                        row.append('build')
                    elif k in m:
                        e = m[k]
                        row.append('ok' if e['clean'] else ('BR' + ('' if not e['diff_sets'] else '')))
                    else:
                        row.append('-')
        L.append(f'| `{cand}` | ' + ' | '.join(row) + ' |')
    L.append('\n## Static shape (config c1, sum over the five sets of zi::reduce + ct::exact_round)\n')
    L.append('| candidate | ' + ' | '.join(f'{tc} {a[:3]}' for tc in TCS for a in ARCHS) + ' |')
    L.append('|---|' + '---|' * 4)
    for cand in CANDS:
        row = []
        for tc in TCS:
            for a in ARCHS:
                e = m.get((tc, a, 'c1', cand))
                if (tc, a, 'c1', cand) in fails:
                    row.append('build')
                elif e is None:
                    row.append('-')
                else:
                    row.append(f"br {e['condbr']} (reg {e['condbr_reg']}), sel {e['csel']}, div {e['div']}" +
                               (' [mask not inlined]' if e['mask_fn'] else '') +
                               ('' if e['eq'] else f" TRACE DIFF {','.join(e['diff_sets'])}"))
        L.append(f'| `{cand}` | ' + ' | '.join(row) + ' |')
    L.append('\n## dudect detail (x86-64, pinned core)\n')
    L.append('| set | candidate / toolchain / config | n | t raw | max t cropped | t 2nd order | max abs t | r(cycles, bits N) | median cycles fixed / random |')
    L.append('|---|---|---|---|---|---|---|---|---|')
    for (s, tag), d in sorted(du.items()):
        L.append(f"| {s} | {tag} | {d['n']} | {d['t_raw']:.2f} | {d['t_crop_max']:.2f} | {d['t_2nd']:.2f} | "
                 f"{d['t_max_abs']:.2f} | {d['r_bitsN']:.3f} | {d['med_fixed']:.0f} / {d['med_rand']:.0f} |")
    L.append('\n## Memcheck detail (x86-64)\n')
    L.append('| run | sets | errors in CT code | errors outside glibc allocator |')
    L.append('|---|---|---|---|')
    for b, g in sorted(cg.items()):
        if g['status'] == 'build failed':
            L.append(f'| {b} | - | build failed | - |')
        else:
            L.append(f"| {b} | {g['sets']} | {g['in_ct']} | {g['outside_glibc']} |")
    out = '\n'.join(L) + '\n'
    open(os.path.join(HERE, 'results', 'tables.md'), 'w').write(out)
    json.dump(dict(table=summ, matrix={'|'.join(k): v for k, v in m.items()},
                   ctgrind=cg, dudect={f'{s}|{t}': d for (s, t), d in du.items()},
                   build_failures=sorted('|'.join(f) for f in fails)),
              open(os.path.join(HERE, 'results', 'summary.json'), 'w'), indent=1, sort_keys=True)
    print(out)


if __name__ == '__main__':
    main()
