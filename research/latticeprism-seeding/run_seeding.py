# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Run the four seed constructions (plus baseline) through H2 selection.

Panels (defaults; scale down with flags):
  stream  N keys x M msgs at each set: seed, q, H2 index, q-ideal LLL,
          determinism recompute, attacker recompute, D0 cost on the same q.
  keys    a subset: I_sec * I_q product lattice (secret-leakage panel).
  reverse map-LLL on a fixed-covolume public lattice * U, for REV only.

Messages and secret keys are drawn from SHAKE with an experiment seed, so
the run is repeatable.
"""
import argparse, csv, os, secrets, sys, time
from multiprocessing import Pool
from pathlib import Path

import gmpy2

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'latticeprism-4d'))
from constructions import (  # noqa: E402
    SETS, prime_of, PK_BYTES, NAMES, construction, keypair, shake,
    h2_select, h2_reduce, attacker_seeds, attacker_q, encode_U,
)
from lp4d import (  # noqa: E402
    unimodular_from_xof, gram_eucl, lll_gram, apply, ideal_basis, gram_norm,
    nrd, matmul, det4, alpha_for,
)
from run_secret import crt  # noqa: E402
from run_4d import t1_target  # noqa: E402


def exp_bytes(tag, *idx, n=32):
    return shake(b'exp/' + tag, *idx, n=n)


def make_sk(i):
    return exp_bytes(b'sk', i.to_bytes(4, 'little'))


def make_msg(i, j):
    return exp_bytes(b'msg', i.to_bytes(4, 'little'), j.to_bytes(4, 'little'))


def y_frac(y, q):
    """y/q as a float. Direct float(y) overflows for p664 (q is 1,323 bits)."""
    shift = max(0, q.bit_length() - 53)
    return float(y >> shift) / float(q >> shift)



def secret_class(sk, p, nb):
    """Deterministic secret ideal class of prime norm ~ 2^{nb}."""
    ctr = 0
    while True:
        raw = shake(b'sec-N', sk, ctr.to_bytes(4, 'little'), n=nb // 8 + 4)
        N = int.from_bytes(raw, 'little') & ((1 << nb) - 1)
        N |= (1 << (nb - 1)) | 1
        ctr += 1
        if gmpy2.is_prime(N):
            break
    ctr = 0
    while True:
        yN = int.from_bytes(shake(b'sec-y', sk, ctr.to_bytes(4, 'little'), n=nb // 8 + 4), 'little') % N
        xN = alpha_for(N, p, yN)
        ctr += 1
        if xN is not None:
            return N, xN, yN


def d0_reduce(T, p):
    """Deterministic smallest-y ideal, for cost comparison."""
    from lp4d import embed_D0
    B, info = embed_D0(T, p)
    H, tr = lll_gram(gram_norm(B, p))
    beta = apply(H, B)[0]
    ok = nrd(beta, p) % T == 0
    return tr['iterations'], info.get('alpha_tries', 0), int(ok)


# REV's pre-challenge timed step reduces the seed-derived unimodular U
# itself (rows of a det-1 lattice, 128-bit entries). The covolume is 1,
# public, and independent of q. A larger public covolume (for example
# det = 2^{2e-5}) is equivalent up to a diagonal scaling and was too
# expensive in this Python LLL for a full panel.
def fixed_public_basis(e):
    return [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]


def rev_map_reduce(U, e):
    B = matmul(U, fixed_public_basis(e))
    H, tr = lll_gram(gram_eucl(B))
    return tr['iterations'], tr['swaps']


def stream_job(args):
    name, e, f, set_name, kid, mid, do_lll, do_rev_map = args
    sk = make_sk(kid)
    pk = keypair(sk, set_name)
    msg = make_msg(kid, mid)
    p = prime_of(e, f)
    rec = construction(name, sk, pk, msg, e, p, set_name)
    q, seed = rec['q'], rec['seed']
    x, y, tries = h2_select(q, p, seed)
    row = dict(
        set=set_name, construction=name, key=kid, msg=mid,
        q_bits=q.bit_length(), q_mod16=q % 16, qtries=rec['qtries'],
        y_over_q=y_frac(y, q), y_mod256=int(y % 256),
        sign=int(x > (q - x) if False else (x != min(x, q - x))),
        h2_tries=tries, q_mode=rec['q_mode'],
    )
    # sign of x: 0 if x was the smaller root
    row['sign'] = int(x != min(x, q - x))
    # determinism: recompute construction + index
    rec2 = construction(name, sk, pk, msg, e, p, set_name)
    x2, y2, _ = h2_select(rec2['q'], p, rec2['seed'])
    row['det_seed'] = int(rec2['seed'] == seed)
    row['det_q'] = int(rec2['q'] == q)
    row['det_index'] = int((x2, y2) == (x, y))
    # attacker recompute of the seed (and therefore of the index)
    b, a = attacker_seeds(name, pk, msg, rec['sig'], e)
    row['pre_before'] = int(b == seed) if b is not None else 0
    row['pre_after'] = int(a == seed) if a is not None else 0
    row['attacker_has_seed_before'] = int(b is not None)
    row['attacker_has_seed_after'] = int(a is not None)
    # verifier recomputes q
    row['verifier_q_ok'] = int(attacker_q(name, pk, msg, rec['sig'], e) == q)
    if do_lll:
        tr = h2_reduce(q, p, seed, xy=(x, y, tries))
        row['sign_iterations'] = tr['iterations']
        row['sign_swaps'] = tr['swaps']
        row['sign_ok'] = tr['ok']
        row['ratio_bits'] = tr['ratio_bits']
        d0i, d0t, d0ok = d0_reduce(q, p)
        row['D0_iterations'] = d0i
        row['D0_tries'] = d0t
        row['D0_ok'] = d0ok
    row['map_iterations'] = row['map_swaps'] = row['det_map'] = ''
    if do_rev_map and name == 'REV':
        mi, ms = rev_map_reduce(rec['U'], e)
        row['map_iterations'] = mi
        row['map_swaps'] = ms
        rec3 = construction(name, sk, pk, msg, e, p, set_name)
        row['det_map'] = int(encode_U(rec3['U']) == encode_U(rec['U']))
    return row


def product_job(args):
    """I_sec * I_q, CRT of the secret class and the H2 class of q."""
    name, e, f, set_name, kid, mid = args
    sk = make_sk(kid)
    pk = keypair(sk, set_name)
    msg = make_msg(kid, mid)
    p = prime_of(e, f)
    rec = construction(name, sk, pk, msg, e, p, set_name)
    q, seed = rec['q'], rec['seed']
    N, xN, yN = secret_class(sk, p, p.bit_length() // 2)
    xq, yq, _ = h2_select(q, p, seed)
    x, y = crt(xN, N, xq, q), crt(yN, N, yq, q)
    T = N * q
    B = ideal_basis(T, p, (x, y, 1, 0))
    H, tr = lll_gram(gram_norm(B, p))
    beta = apply(H, B)[0]
    ok = nrd(beta, p) % T == 0
    return dict(set=set_name, construction=name, key=kid, msg=mid,
                q_bits=q.bit_length(), N_bits=N.bit_length(),
                prod_iterations=tr['iterations'], prod_swaps=tr['swaps'],
                prod_ok=int(ok),
                y_over_q=y_frac(yq, q))


def make_q(e, i):
    """Repeatable challenge-degree prime, independent of any key."""
    import secrets as _s
    # not used; deterministic via shake
    ctr = 0
    while True:
        raw = shake(b'fixed-q', e.to_bytes(4, 'little'), i.to_bytes(4, 'little'), ctr.to_bytes(4, 'little'), n=e // 8 + 16)
        s = ((int.from_bytes(raw, 'little') & ((1 << (e - 4)) - 1)) << 1) | 1
        q = (1 << (2 * e - 5)) - s * s
        ctr += 1
        if gmpy2.is_prime(q):
            return q


def fixedq_job(args):
    """Hold q fixed; vary the key (and its msg 0). Probe target dependence."""
    name, e, f, set_name, qid, kid, q = args
    p = prime_of(e, f)
    sk = make_sk(1000 + kid)  # disjoint from the stream keys
    pk = keypair(sk, set_name)
    msg = make_msg(1000 + kid, qid)
    # Force this q through a shallow wrapper of the seed rule, keeping
    # the construction's published fields for the attacker test.
    rec = construction(name, sk, pk, msg, e, p, set_name)
    # Rebind q to the panel's fixed target, and re-derive the seed as the
    # construction would if it were given this q (see report §method).
    if name == 'B0':
        seed = shake(b'B0', pk, q.to_bytes((q.bit_length() + 7) // 8, 'little'))
    elif name == 'Hpk':
        seed = shake(b'Hpk', pk, msg)
    elif name in ('Hsk', 'REV', 'CMB'):
        seed = shake(b'Hsk', sk, msg, q.to_bytes((q.bit_length() + 7) // 8, 'little'))
    elif name == 'COM':
        seed = rec['seed']  # r = H(sk||msg), independent of q already
    elif name == 'EPH':
        seed = rec['seed']  # H(epk||msg), independent of q already
    else:
        raise ValueError(name)
    x, y, tries = h2_select(q, p, seed)
    tr = h2_reduce(q, p, seed, xy=(x, y, tries))
    d0i, d0t, d0ok = d0_reduce(q, p)
    return dict(set=set_name, construction=name, qid=qid, key=kid,
                q_bits=q.bit_length(), y_over_q=y_frac(y, q),
                y_mod256=int(y % 256), sign=int(x != min(x, q - x)),
                h2_tries=tries, sign_iterations=tr['iterations'],
                sign_ok=tr['ok'], D0_iterations=d0i, D0_ok=d0ok)


def write(path, rows):
    if not rows:
        return
    os.makedirs(os.path.dirname(path) or '.', exist_ok=True)
    with open(path, 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader(); w.writerows(rows)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', nargs='+', default=[s[0] for s in SETS])
    ap.add_argument('--keys', type=int, default=50)
    ap.add_argument('--msgs', type=int, default=200)
    ap.add_argument('--keys-prod', type=int, default=50)
    ap.add_argument('--msgs-prod', type=int, default=200)
    ap.add_argument('--constructions', nargs='+', default=list(NAMES))
    ap.add_argument('--fixed-q', type=int, default=24)
    ap.add_argument('--fixed-draws', type=int, default=150)
    ap.add_argument('--no-lll', action='store_true')
    ap.add_argument('--procs', type=int, default=7)
    ap.add_argument('--out', default='results/raw')
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    with Pool(a.procs) as pool:
        for set_name, e, f in [s for s in SETS if s[0] in a.sets]:
            t0 = time.time()
            nstream = nprod = nfixed = 0
            sp = f'{a.out}/stream_{set_name}.csv'
            if os.path.exists(sp):
                print(f'{set_name}: skip stream (exists)', flush=True)
            else:
                jobs = [(name, e, f, set_name, kid, mid, not a.no_lll, True)
                        for name in a.constructions
                        for kid in range(a.keys)
                        for mid in range(a.msgs)]
                rows = pool.map(stream_job, jobs, chunksize=8)
                write(sp, rows); nstream = len(rows)
            pp = f'{a.out}/prod_{set_name}.csv'
            if os.path.exists(pp):
                print(f'{set_name}: skip prod (exists)', flush=True)
            else:
                pjobs = [(name, e, f, set_name, kid, mid)
                         for name in a.constructions
                         for kid in range(a.keys_prod)
                         for mid in range(a.msgs_prod)]
                prows = pool.map(product_job, pjobs, chunksize=4)
                write(pp, prows); nprod = len(prows)
            fp = f'{a.out}/fixedq_{set_name}.csv'
            if os.path.exists(fp):
                print(f'{set_name}: skip fixedq (exists)', flush=True)
            elif a.fixed_q <= 0 or a.fixed_draws <= 0:
                print(f'{set_name}: skip fixedq (disabled)', flush=True)
            else:
                qs = [make_q(e, i) for i in range(a.fixed_q)]
                fjobs = [(name, e, f, set_name, qid, kid, qs[qid])
                         for name in a.constructions
                         for qid in range(a.fixed_q)
                         for kid in range(a.fixed_draws)]
                frows = pool.map(fixedq_job, fjobs, chunksize=8)
                write(fp, frows); nfixed = len(frows)
            print(f'{set_name} stream={nstream} prod={nprod} fixedq={nfixed} in {time.time()-t0:.0f}s', flush=True)


if __name__ == '__main__':
    main()
