# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Pass 1: exact LLL baseline vs fixed-schedule reductions, per sample.

Panels (per set):
  stream  the H2 ideal of norm q (q = FS(pk, salt, msg)), keys x msgs;
  prod    the secret product I_sec * I_q for the same (sk, msg);
  target  a fixed q per "target" with several H2 ideal seeds, for the
          fixed-size target-dependence test.

Every row records, for each reduction, the first tour/step after which
the output meets the success criterion (b1 no longer than exact LLL's b1,
i.e. nrd(b1) <= nrd(b1_exact)); -1 if never within the run. Because a
fixed schedule of T tours is a prefix of a longer run, "success at T" is
"first <= T". Output is chunked by key block and skipped if present.
"""
import argparse
import csv
import time
from multiprocessing import Pool
from pathlib import Path

from hdl import (SETS, SALT_LEN, shake, stream_job_data, select_xy,
                 ideal_basis, gram_norm, secret_class, ideal_from_classes, nrd)
from fsr import (lll_exact, reduced_gram, enum_budget, fixed_tours,
                 is_lll_reduced, zi_gauss_fixed, herm, Ctr)

SET_MAP = {n: (n, e, f) for n, e, f in SETS}


def sk_of(set_name, kid):
    return shake(b'sk', set_name.encode(), kid.to_bytes(4, 'little'), n=32)


def msg_of(set_name, kid, mid):
    return shake(b'msg', set_name.encode(), kid.to_bytes(4, 'little'),
                 mid.to_bytes(4, 'little'), n=32)


def tmax(bits, kind):
    return {'sweep': int(0.45 * bits) + 40, 'updown': int(0.25 * bits) + 30,
            'zi': int(0.32 * bits) + 30}[kind]


def _bits(xs):
    return max(abs(x).bit_length() for x in xs)


ZI_PRECS = {'zi': None, 'zi_p64': 64, 'zi_p32': 32, 'zi_p16': 16, 'zi_p8': 8}


def _measure_zi(zv, T, alpha, p, bits, b1, lam1):
    """Z[i]-Lagrange fixed steps; prefix zv ('zi' exact quotient, 'zi_pP'
    bounded-precision quotient)."""
    out = {}
    prec = ZI_PRECS[zv]
    st = dict(b1=-1, lam1=-1, red=-1, mb=0)

    def zhook(t, u, v, nu, nv):
        if st['b1'] < 0 and nu <= b1:
            st['b1'] = t
        if st['lam1'] < 0 and nu == lam1:
            st['lam1'] = t
        if st['red'] < 0:
            h0, h1 = herm(v, u, p, Ctr())
            m0 = (2 * h0 + nu) // (2 * nu)
            m1 = (2 * h1 + nu) // (2 * nu)
            if m0 == 0 and m1 == 0 and nu <= nv:
                st['red'] = t
        m = max(_bits(u), _bits(v), nu.bit_length(), nv.bit_length())
        if m > st['mb']:
            st['mb'] = m

    Sm = tmax(bits, 'zi')
    t0 = time.time()
    u, v, nu, nv, cz, eff = zi_gauss_fixed(T, alpha, p, Sm, on_step=zhook, prec=prec)
    out[f'{zv}_time'] = round(time.time() - t0, 4)
    out[f'{zv}_tmax'] = Sm
    out[f'{zv}_b1'] = st['b1']
    out[f'{zv}_lam1'] = st['lam1']
    out[f'{zv}_red'] = st['red']
    out[f'{zv}_maxbits'] = st['mb']
    out[f'{zv}_eff'] = eff
    out[f'{zv}_trace'] = cz.trace.hexdigest()[:16]
    # the Z[i]-reduced u must be an element of the ideal with q | nrd
    out[f'{zv}_ok_q'] = int(nu % T == 0 and nu > 0)
    return out

def measure_lattice(B, T, p, bits_nom, do_sweep=True, do_updown=True,
                    zi_variants=('zi',)):
    assert B[0] == [T, 0, 0, 0] and B[1] == [0, T, 0, 0]
    assert B[2][2:] == [1, 0] and B[3][2:] == [0, 1]
    alpha = B[2]
    # row 3 is i*alpha reduced mod T
    assert (B[3][0] + alpha[1]) % T == 0 and (B[3][1] - alpha[0]) % T == 0
    G = gram_norm(B, p)
    # schedule lengths depend on the parameter set and panel only
    bits = bits_nom
    out = dict(T_bits=T.bit_length(), bits_nom=bits_nom)

    t0 = time.time()
    H, d, lam, c = lll_exact(G)
    out.update(c.as_dict('ex'))
    out['ex_time'] = round(time.time() - t0, 4)
    b1 = d[1]
    Gr = reduced_gram(G, H)
    r = enum_budget(Gr, d, lam, 0, 10 ** 7)
    lam1 = r['best']
    out['ex_is_lam1'] = int(b1 == lam1)
    out['ex_maxbits'] = max(_bits(d), _bits([x for row in lam for x in row]))

    for order in (['sweep'] if do_sweep else []) + (['updown'] if do_updown else []):
        st = dict(b1=-1, lam1=-1, red=-1, mb=0)

        def hook(t, H_, d_, l_):
            if st['b1'] < 0 and d_[1] <= b1:
                st['b1'] = t
            if st['lam1'] < 0 and d_[1] == lam1:
                st['lam1'] = t
            if st['red'] < 0 and is_lll_reduced(d_, l_):
                st['red'] = t
            m = max(_bits(d_), _bits([x for row in l_ for x in row]),
                    _bits([x for row in H_ for x in row]))
            if m > st['mb']:
                st['mb'] = m

        Tm = tmax(bits, order)
        t0 = time.time()
        _, _, _, cf, eff = fixed_tours(G, Tm, order, on_tour=hook)
        out[f'{order}_time'] = round(time.time() - t0, 4)
        out[f'{order}_tmax'] = Tm
        out[f'{order}_b1'] = st['b1']
        out[f'{order}_lam1'] = st['lam1']
        out[f'{order}_red'] = st['red']
        out[f'{order}_maxbits'] = st['mb']
        out[f'{order}_eff'] = eff
        out[f'{order}_trace'] = cf.trace.hexdigest()[:16]

    for zv in zi_variants:
        out.update(_measure_zi(zv, T, alpha, p, bits, b1, lam1))
    return out


def n_bound(p):
    """Signer-spec bound on bits(N) of the secret ideal: B = ceil(L/2)+32,
    L = bits(p); typical length L/2 + 16."""
    L = p.bit_length()
    return (L + 1) // 2 + 32


def secret_class_var(sk, p, force_nb=None):
    """Secret ideal class with a per-key norm length: bits(N) =
    ceil(L/2) + u, u uniform in 0..32 from H(sk) (mean ceil(L/2)+16, max
    B). Simulated keys: N prime of that exact length, then H2-style (x, y)."""
    L = p.bit_length()
    nb = (L + 1) // 2 + shake(b'sec-nbits', sk, n=1)[0] % 33
    if force_nb is not None:
        nb = force_nb
    N, xN, yN = secret_class(sk + b'var', p, nb)
    return N, xN, yN, nb


VAR_PANELS = ('nvar', 'nbound', 'dudect_fixed', 'dudect_rand')
ZI_ALL = ('zi', 'zi_p64', 'zi_p32', 'zi_p16', 'zi_p8')


def job(args):
    set_name, kid, mid, panel, do_sweep = args
    if panel in VAR_PANELS:
        return job_var(args)
    _, e, f = SET_MAP[set_name]
    if panel == 'target':
        # target = (key kid, message 0); mid indexes the ideal seed
        sk = sk_of(set_name, kid)
        msg = msg_of(set_name, kid, 0)
    else:
        sk = sk_of(set_name, kid)
        msg = msg_of(set_name, kid, mid)
    data = stream_job_data(sk, msg, set_name, e, f)
    p, q = data['p'], data['q']
    seed = data['seed']
    if panel == 'target':
        seed = shake(b'target-seed', seed, mid.to_bytes(4, 'little'))
    xq, yq, tries = select_xy(q, p, seed)
    row = dict(set=set_name, panel=panel, key=kid, msg=mid,
               q_bits=q.bit_length(), p_bits=p.bit_length(),
               s_top=data['s'] >> (data['s'].bit_length() - 8),
               salt_len=SALT_LEN[set_name], qtries=data['qtries'],
               xytries=tries)
    if panel in ('stream', 'target'):
        B = ideal_basis(q, p, (xq, yq, 1, 0))
        T = q
        bits_nom = 2 * e - 5
    else:
        N, xN, yN = secret_class(sk, p, p.bit_length() // 2)
        B, T = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
        row['N_bits'] = N.bit_length()
        bits_nom = 2 * e - 5 + p.bit_length() // 2
    row.update(measure_lattice(B, T, p, bits_nom, do_sweep=do_sweep))
    return row


def job_var(args):
    """Product I_sec * I_q with a per-key secret norm length; every
    schedule is sized from the bound B, never from the actual N.
      nvar          key kid (few hundred keys), message mid;
      dudect_fixed  one fixed key (id 0 of its own namespace), message mid;
      dudect_rand   a fresh random key per trace (key kid), message 0."""
    set_name, kid, mid, panel, _ = args
    _, e, f = SET_MAP[set_name]
    if panel == 'dudect_fixed':
        sk = shake(b'sk-dudect-fixed', set_name.encode(), n=32)
        msg = msg_of(set_name, 10 ** 6, mid)
    elif panel == 'dudect_rand':
        sk = shake(b'sk-dudect-rand', set_name.encode(),
                   kid.to_bytes(4, 'little'), n=32)
        msg = msg_of(set_name, 10 ** 6, kid)
    elif panel == 'nbound':
        sk = shake(b'sk-nbound', set_name.encode(), kid.to_bytes(4, 'little'), n=32)
        msg = msg_of(set_name, kid, mid)
    else:
        sk = shake(b'sk-nvar', set_name.encode(), kid.to_bytes(4, 'little'), n=32)
        msg = msg_of(set_name, kid, mid)
    data = stream_job_data(sk, msg, set_name, e, f)
    p, q = data['p'], data['q']
    xq, yq, tries = select_xy(q, p, data['seed'])
    N, xN, yN, nb = secret_class_var(
        sk, p, force_nb=n_bound(p) if panel == 'nbound' else None)
    B, T = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
    bits_nom = 2 * e - 5 + n_bound(p)
    row = dict(set=set_name, panel=panel, key=kid, msg=mid,
               q_bits=q.bit_length(), p_bits=p.bit_length(), N_bits=nb,
               N_bound=n_bound(p), salt_len=SALT_LEN[set_name],
               qtries=data['qtries'], xytries=tries)
    row.update(measure_lattice(B, T, p, bits_nom, do_sweep=False,
                               do_updown=panel in ('nvar', 'nbound'),
                               zi_variants=ZI_ALL if panel in ('nvar', 'nbound') else ('zi',)))
    return row


def write_csv(path, rows):
    path.parent.mkdir(parents=True, exist_ok=True)
    keys = []
    for r in rows:
        for k in r:
            if k not in keys:
                keys.append(k)
    tmp = path.with_suffix('.tmp')
    with tmp.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=keys)
        w.writeheader()
        w.writerows(rows)
    tmp.rename(path)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sets', nargs='+', default=[s[0] for s in SETS])
    ap.add_argument('--panels', nargs='+', default=['stream', 'prod', 'target'])
    ap.add_argument('--keys', type=int, default=100)
    ap.add_argument('--msgs', type=int, default=40)
    ap.add_argument('--sweep-msgs', type=int, default=10)
    ap.add_argument('--targets', type=int, default=25)
    ap.add_argument('--seeds', type=int, default=16)
    ap.add_argument('--nvar-keys', type=int, default=400)
    ap.add_argument('--nvar-msgs', type=int, default=10)
    ap.add_argument('--dudect', type=int, default=3000)
    ap.add_argument('--block', type=int, default=10)
    ap.add_argument('--procs', type=int, default=7)
    ap.add_argument('--out', default='results/raw')
    a = ap.parse_args()
    out = Path(a.out)
    with Pool(a.procs) as pool:
        for set_name in a.sets:
            for panel in a.panels:
                nk = a.targets if panel == 'target' else a.keys
                nm = a.seeds if panel == 'target' else a.msgs
                blk = a.block
                if panel == 'nvar':
                    nk, nm, blk = a.nvar_keys, a.nvar_msgs, 50
                elif panel == 'nbound':
                    nk, nm, blk = 200, 10, 50
                elif panel == 'dudect_fixed':
                    nk, nm, blk = 1, a.dudect, 1
                elif panel == 'dudect_rand':
                    nk, nm, blk = a.dudect, 1, 500
                for k0 in range(0, nk, blk):
                    path = out / f'{panel}_{set_name}_k{k0:03d}.csv'
                    if path.exists():
                        print(f'skip {path}', flush=True)
                        continue
                    t0 = time.time()
                    jobs = [(set_name, k, m, panel,
                             panel == 'target' or m < a.sweep_msgs)
                            for k in range(k0, min(nk, k0 + blk))
                            for m in range(nm)]
                    rows = pool.map(job, jobs, chunksize=1)
                    write_csv(path, rows)
                    print(f'wrote {path} ({len(rows)}) {time.time() - t0:.0f}s',
                          flush=True)
    print('ALL_DONE', flush=True)


if __name__ == '__main__':
    main()
