# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""O29 / postcondition check against the pinned C reference (PRISM_v2 @
87fad7d + QLSTAT patch). Variants (see build.sh): e2 (pin, used power
e-2), e1, e0 (used power e), e2np / e0np (postcondition disabled: every
theta from the norm equation accepted). Per variant, level and seed:
keygen, sign, open; records success, theta-chain warnings printed during
keygen/sign, QLSTAT lines and pk/sm hex. Resumable (jsonl per variant and
level)."""
import json
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

C = Path('/workspace/qlr-c')
OUT = Path(__file__).resolve().parent / 'runs'


def one(var, lvl, seed):
    exe = C / f'bld-{var}' / 'test' / f'prism_test_scheme_lvl{lvl}'
    try:
        r = subprocess.run([str(exe), f'--seed={seed}'], capture_output=True, text=True, timeout=600)
        txt = r.stdout + r.stderr
        rc = r.returncode
    except subprocess.TimeoutExpired:
        return dict(var=var, lvl=lvl, seed=seed, rc='timeout')
    pre = txt.split('THIS SHOULD FAIL')[0]
    d = dict(var=var, lvl=lvl, seed=seed, rc=rc,
             keygen_ok='Keygen is working!' in txt,
             sign_ok='Signing is working!' in txt,
             verify_ok='Verification is working!' in txt,
             warn_pre=len(re.findall(r'^warning', pre, re.M)),
             qlstat=re.findall(r'QLSTAT ([\d ]+(?:NOPOST \d+)?)', txt))
    for k in ('pk', 'sm'):
        m = re.search(rf'^{k}: ([0-9a-f]+)', txt, re.M)
        d[k] = m.group(1) if m else None
    return d


def main():
    variants = sys.argv[1].split(',')
    lvls = [int(x) for x in sys.argv[2].split(',')]
    seeds = range(1, int(sys.argv[3]) + 1)
    OUT.mkdir(exist_ok=True)
    with ThreadPoolExecutor(3) as ex:
        for var in variants:
            for lvl in lvls:
                fn = OUT / f'{var}_lvl{lvl}.jsonl'
                done = set()
                if fn.exists():
                    done = {json.loads(l)['seed'] for l in open(fn)}
                todo = [s for s in seeds if s not in done]
                with open(fn, 'a') as fh:
                    for d in ex.map(lambda s: one(var, lvl, s), todo):
                        fh.write(json.dumps(d) + '\n')
                        fh.flush()
                print(var, lvl, 'done', flush=True)


if __name__ == '__main__':
    main()
