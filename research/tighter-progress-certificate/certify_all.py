# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""The whole certificate, end to end:
  1. cellcert check (Rust, integer) of the cell potential at every band;
  2. check_potential.py (Python, exact rationals, independent code) of the
     same (skip with --fast);
  3. certify_tail_exact (lagrange-tour-bound/certify.py, unchanged): from
     lam >= 2^-11, 5 swaps are impossible, so tau = 4;
  4. counts.py: lemmas and the counts per class and set.
Writes results/step_counts_new.csv and results/certify_all.txt."""
import csv, os, subprocess, sys, time
from fractions import Fraction as F
from gmpy2 import mpq
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, 'vendor'))
import certify
import counts as CN

CERT = os.path.join(HERE, 'results', 'cert_2685.txt')
RHO1 = '537/2000'
BANDS = [(20, RHO1), (16, '271313/1000000'), (14, '300770/1000000'), (13, '300838/1000000'),
         (12, '300973/1000000'), (11, '304449/1000000')]
TAU_LAM, TAU = 11, 4

# PR #45 (abed5c1) native table: O0 exact (research tables, O0 rows) and exp64 any-basis
OLD_EXACT = {'sign-generic': (301, 387, 447, 587, 774), 'prered-guard256': (287, 373, 432, 573, 759),
             'keygen-D': (223, 347, 331, 442, 686), 'keygen-Isec': (22, 22, 23, 22, 23),
             'challenge': (212, 276, 319, 425, 565), 'prered-guard2': (283, 369, 429, 569, 755),
             'prered-unguarded': (354, 462, 537, 713, 945)}
OLD_EXP64 = {'sign-generic': (312, 401, 463, 609, 803), 'prered-guard256': (297, 387, 448, 594, 787),
             'keygen-D': (231, 360, 344, 459, 712), 'keygen-Isec': (24, 25, 26, 26, 29),
             'challenge': (220, 287, 331, 442, 587), 'prered-guard2': (293, 383, 445, 590, 783),
             'prered-unguarded': (365, 477, 555, 736, 976)}
# normalized-gram REPORT section 4 (normalized E), signing etc.
NORM_E = {'sign-generic': (306, 394, 455, 598, 788), 'prered-guard256': (292, 379, 440, 583, 773),
          'keygen-D': (227, 353, 337, 450, 699), 'keygen-Isec': (24, 24, 26, 26, 28),
          'challenge': (216, 281, 325, 433, 576)}
SETN = ['p248', 'p324', 'p376', 'p500', 'p664']
OUT = []


def say(s):
    print(s, flush=True); OUT.append(s)


def main():
    fast = '--fast' in sys.argv
    exe = os.path.join(HERE, 'cellcert', 'target', 'release', 'cellcert')
    if not os.path.exists(exe):
        subprocess.run(['cargo', 'build', '--release'], cwd=os.path.join(HERE, 'cellcert'), check=True)
    say('== 1. cellcert check (integer arithmetic)')
    for l, r in BANDS:
        o = subprocess.run([exe, 'check', r, str(l), CERT], capture_output=True, text=True, check=True).stdout.strip()
        say('  ' + o)
    if not fast:
        say('== 2. check_potential.py (exact rationals, independent)')
        o = subprocess.run([sys.executable, os.path.join(HERE, 'check_potential.py'), CERT, BANDS[0][1], str(BANDS[0][0])]
                           + [f'{r}:{l}' for l, r in BANDS[1:]], capture_output=True, text=True, check=True).stdout
        for line in o.strip().splitlines():
            say('  ' + line)
    say('== 3. tail (certify_tail_exact, unchanged)')
    t = time.time()
    ok, n, _ = certify.certify_tail_exact(mpq(1, 1 << TAU_LAM), TAU + 1)
    say(f'  from lam >= 2^-{TAU_LAM}: {TAU + 1} swaps impossible: {ok} ({n} boxes, {time.time()-t:.0f}s); tau = {TAU}')
    assert ok
    say('== 4. lemmas and counts')
    rho1, lam1, G = CN.read_cert(CERT)
    assert rho1 == F(537, 2000) and lam1 == F(1, 1 << 20)
    bands = [(F(1, 1 << l), F(*map(int, r.split('/')))) for l, r in BANDS]
    say(f'  rho_1 {rho1} = {float(rho1)}, G = {float(G):.6f}, top band 2^-{BANDS[-1][0]}, band caps {CN.band_caps(bands)}')
    rows = CN.counts(bands, G, TAU)
    for r in rows:
        k = SETN.index(r['set'])
        r['old_T_exact'] = OLD_EXACT[r['cls']][k]
        r['old_T_exp64'] = OLD_EXP64[r['cls']][k]
        r['normE_T_exp64'] = NORM_E[r['cls']][k] if r['cls'] in NORM_E else ''
        r['save_exact'] = r['old_T_exact'] - r['T_exact']
        r['save_exp64'] = r['old_T_exp64'] - r['T_exp64']
        say(f"  {r['set']} {r['cls']:17s} exact {r['T_exact']:4d} (was {r['old_T_exact']:4d})  "
            f"exp64 {r['T_exp64']:4d} (was {r['old_T_exp64']:4d}, normalized-E {r['normE_T_exp64']})  n1_max {r['n1_max']}")
    with open(os.path.join(HERE, 'results', 'step_counts_new.csv'), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)
    open(os.path.join(HERE, 'results', 'certify_all.txt'), 'w').write('\n'.join(OUT) + '\n')


if __name__ == '__main__':
    main()
