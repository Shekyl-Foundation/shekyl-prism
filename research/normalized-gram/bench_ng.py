# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Run `ng bench` in several processes per set, rotating the set order, and
summarize the median of the process medians (TSC ticks).
Usage: python3 bench_ng.py [--procs 5] [--reps 201] [--cpu 6]"""
import argparse
import os
import platform
import statistics
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
RUST = os.path.join(HERE, 'rust')
RES = os.path.join(HERE, 'results')
SETS = ['p248', 'p324', 'p376', 'p500', 'p664']


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--procs', type=int, default=5)
    ap.add_argument('--reps', type=int, default=201)
    ap.add_argument('--cpu', type=int, default=6)
    a = ap.parse_args()
    env = dict(os.environ, RLC_CPU=str(a.cpu))
    header, rows = None, []
    for r in range(a.procs):
        order = SETS[r % len(SETS):] + SETS[:r % len(SETS)]
        for s in order:
            out = subprocess.run(['./target/release/ng', 'bench', s, str(a.reps), 'header'], cwd=RUST, env=env,
                                 capture_output=True, text=True, check=True).stdout.splitlines()
            header = out[0].split(',')
            rows.append([r] + out[1].split(','))
            print(r, out[1], flush=True)
    with open(os.path.join(RES, 'bench_raw.csv'), 'w') as f:
        f.write('proc,' + ','.join(header) + '\n')
        for row in rows:
            f.write(','.join(str(x) for x in row) + '\n')
    cols = header[3:]
    summ = {}
    for s in SETS:
        rs = [row for row in rows if row[1] == s]
        summ[s] = {c: statistics.median(int(row[4 + i]) for row in rs) for i, c in enumerate(cols)}
        summ[s]['steps'] = int(rs[0][2])
    cpu = [l.split(':', 1)[1].strip() for l in open('/proc/cpuinfo') if l.startswith('model name')][0]
    rustc = subprocess.run(['rustc', '+1.97.1', '--version'], capture_output=True, text=True).stdout.strip()
    with open(os.path.join(RES, 'bench.csv'), 'w') as f:
        f.write('set,steps,' + ','.join(cols) + '\n')
        for s in SETS:
            f.write(f"{s},{summ[s]['steps']}," + ','.join(str(int(summ[s][c])) for c in cols) + '\n')
    with open(os.path.join(RES, 'bench_env.txt'), 'w') as f:
        f.write(f'cpu: {cpu}\nkernel: {platform.release()}\n{rustc}\nprocesses per set: {a.procs}, reps per process: {a.reps} '
                f'(+10 warm-up), pinned to logical CPU {a.cpu}\nmedian of process medians, TSC ticks\n')
    print(open(os.path.join(RES, 'bench.csv')).read())


if __name__ == '__main__':
    main()
