# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Independent exact-rational checker of a cell certificate (gmpy2.mpq,
no rounding at all; shares no code with cellcert/src/main.rs).

Checks, for the leaf partition and g values in the certificate file:
  * the leaves tile [-1/2, 1/2]^2 exactly (area and disjointness via the
    quadtree), and g >= 1;
  * for every leaf c, with r <= rhi(c) over c x [0, LAM]:
      rhi(c) * max{g(c') : c' meets an image box of c} <= rho * g(c),
    and for a wild leaf rhi(c) * max g <= rho * g(c).
Usage: check_potential.py CERT RHO LAMEXP [...more RHO:LAMEXP pairs]
"""
import sys, os, time
from multiprocessing import Pool
from gmpy2 import mpq
from cells import Tree, image

CERT = None


def load(path):
    leaves = {}
    for line in open(path):
        if line[0] in '#rl':
            continue
        L, i, j, g = map(int, line.split())
        k = (L, i, j)
        assert k not in leaves
        leaves[k] = mpq(g, 1 << 32)
    return leaves


def make_tree(leaves):
    t = Tree.__new__(Tree)
    t.leaves = set(leaves); t.internal = set()
    for (L, i, j) in leaves:
        for lv in range(L):
            a = (lv, i >> (L - lv), j >> (L - lv))
            assert a not in t.leaves, 'overlapping leaves'
            t.internal.add(a)
    area = sum(mpq(1, 4 ** L) for (L, _, _) in leaves)
    assert area == 1, 'leaves do not tile the square'
    return t


def init(path):
    global CERT
    leaves = load(path)
    CERT = (leaves, make_tree(leaves), max(leaves.values()))


def worst_ratio(args):
    keys, lam = args
    leaves, tree, gmax = CERT
    worst = mpq(0)
    for k in keys:
        rh, rl, outs = image(k, lam)
        if outs is None:
            gs = gmax
        else:
            s = set()
            for _, _, bx in outs:
                tree.query(*bx, s)
            gs = max((leaves[c] for c in s), default=mpq(0))
        worst = max(worst, rh * gs / leaves[k])
    return worst


def main():
    path = sys.argv[1]
    pairs = [(sys.argv[2], sys.argv[3])] + [tuple(a.split(':')) for a in sys.argv[4:]]
    init(path)
    leaves = CERT[0]
    assert min(leaves.values()) >= 1
    keys = sorted(leaves)
    chunks = [keys[a::64] for a in range(64)]
    with Pool(8, initializer=init, initargs=(path,)) as pool:
        for rho_s, lexp in pairs:
            n, d = rho_s.split('/'); rho = mpq(int(n), int(d)); lam = mpq(1, 1 << int(lexp))
            t = time.time()
            w = max(pool.map(worst_ratio, [(c, lam) for c in chunks]))
            ok = w <= rho
            print(f'{os.path.basename(path)}: {len(keys)} leaves tile the square, g >= 1, Gmax {float(CERT[2]):.6f}; '
                  f'LAM 2^-{lexp}: max rhi*g(succ)/g = {float(w):.9f} <= rho {rho_s}: {ok} ({time.time()-t:.0f}s)', flush=True)
            assert ok


if __name__ == '__main__':
    main()
