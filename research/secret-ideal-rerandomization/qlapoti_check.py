# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Test whether per-signature re-randomization of I_sec -> J = I_sec * (-conj(beta))/N
(which yields the equivalent signing ideal L' = J * K_J = L * (-conj(beta))/N)
makes quat_qlapoti's prepared small ideal S and its work distribution
independent of the key / of S.

Reuses the faithful C-reference model from research/fixed-schedule-norm-equation
(fne_ref.py) on the maximal order O0 across P248..P664.
"""
import csv
import time
from multiprocessing import Pool
from pathlib import Path

import numpy as np
from scipy import stats

from fne_ref import (
    SET_MAP, SETS, prime_of, sec_degree, tape, random_prime_ideal,
    prime_norm_equivalent, intersect_coprime, prepare_small, qlapoti,
    Ctr, work_units, lll_cols, right_mul, nrd_num, gmpy2,
)


def keygen(name, kid, p, SEC):
    rng = tape(b'keygen', name, kid)
    I0 = random_prime_ideal(SEC, p, rng)
    Is, _ = prime_norm_equivalent(I0, p, rng)
    red_Is = lll_cols(Is, p)
    return Is, red_Is


def challenge(name, e, kid, mid):
    rng = tape(b'chal', name, kid, mid)
    while True:
        s = rng.getrandbits(e - 4) | 1
        q = (1 << (2 * e - 5)) - s * s
        if gmpy2.is_prime(q):
            return q, rng


def draw_beta(Is, red_Is, p, q, rng, box=256):
    d2 = Is.den * Is.den * Is.norm
    while True:
        x = [rng.randint(-box, box) for _ in range(4)]
        if not any(x):
            continue
        beta = tuple(sum(x[k] * red_Is[k][t] for k in range(4)) for t in range(4))
        n_new = nrd_num(beta, p) // d2
        if n_new > 1 and (n_new & 1) and (n_new % q != 0):
            return beta, n_new


def push_L(L, Is, beta, n_new, p):
    """L' = L * (-conj(beta)) / N(I_sec) = J * K_J, of norm n_new * q."""
    el = (-beta[0], beta[1], beta[2], beta[3])
    q = L.norm // Is.norm
    return right_mul(L, el, Is.den * Is.norm, p, n_new * q)


def run_q(S, p, e, rng):
    c = Ctr()
    qlapoti(S, p, e - 2, rng, c)
    w = work_units(c.asdict(), int(S.norm).bit_length() + e)
    return w, c.passes


def welch_t(a, b):
    a, b = np.asarray(a, float), np.asarray(b, float)
    return float(stats.ttest_ind(a, b, equal_var=False).statistic)


def same_ideal(S1, S2):
    return int(S1.norm == S2.norm and S1.den == S2.den and sorted(S1.cols) == sorted(S2.cols))


def eval_set(args):
    name, n_tr = args
    _, e, f = SET_MAP[name]
    p = prime_of(e, f)
    SEC = sec_degree(name)
    t0 = time.time()

    # 1. Fixed key (varying messages) vs Random keys: baseline vs re-randomized
    Is_fx, red_fx = keygen(name, 10 ** 6, p, SEC)
    fx_base, fx_rr = [], []
    same_n_fk = same_S_fk = 0
    for m in range(n_tr):
        q, crng = challenge(name, e, 10 ** 6, m)
        Iq = random_prime_ideal(q, p, crng)
        L = intersect_coprime(Is_fx, Iq)
        Sb = prepare_small(L, p)
        beta, n_new = draw_beta(Is_fx, red_fx, p, q, tape(b'rr-fk', name, m))
        Lr = push_L(L, Is_fx, beta, n_new, p)
        Sr = prepare_small(Lr, p)
        same_n_fk += int(Sb.norm == Sr.norm)
        same_S_fk += same_ideal(Sb, Sr)
        wb, _ = run_q(Sb, p, e, tape(b'q-fk-b', name, m))
        wr, _ = run_q(Sr, p, e, tape(b'q-fk-r', name, m))
        fx_base.append(wb)
        fx_rr.append(wr)

    rd_base, rd_rr = [], []
    rd_v2, rd_rr_v2 = [], []
    for k in range(n_tr):
        kid = 2 * 10 ** 6 + k
        Is, red = keygen(name, kid, p, SEC)
        q, crng = challenge(name, e, kid, 0)
        Iq = random_prime_ideal(q, p, crng)
        L = intersect_coprime(Is, Iq)
        Sb = prepare_small(L, p)
        beta, n_new = draw_beta(Is, red, p, q, tape(b'rr-rd', name, k))
        Lr = push_L(L, Is, beta, n_new, p)
        Sr = prepare_small(Lr, p)
        wb, _ = run_q(Sb, p, e, tape(b'q-rd-b', name, k))
        wr, _ = run_q(Sr, p, e, tape(b'q-rd-r', name, k))
        rd_base.append(wb)
        rd_rr.append(wr)
        rd_v2.append(int(gmpy2.bit_scan1(Sb.norm)))
        rd_rr_v2.append(int(gmpy2.bit_scan1(Sr.norm)))

    # 2. Fixed (key, challenge) small-ideal test (fixS), picking 2 normal + 2 with v2(n)>0
    fixS_base_ts, fixS_rr_ts = [], []
    same_n_fs = same_S_fs = n_fs_total = 0
    picked = []
    for cls in range(40):
        kid = 3 * 10 ** 6 + cls
        Is_s, red_s = keygen(name, kid, p, SEC)
        q_s, crng_s = challenge(name, e, kid, 0)
        L_s = intersect_coprime(Is_s, random_prime_ideal(q_s, p, crng_s))
        S_s = prepare_small(L_s, p)
        if len(picked) < 2 or (gmpy2.bit_scan1(S_s.norm) > 0 and len(picked) < 4):
            picked.append((cls, Is_s, red_s, q_s, L_s, S_s))
        if len(picked) == 4:
            break
    for cls, Is_s, red_s, q_s, L_s, S_s in picked:
        wb_list, wr_list = [], []
        for m in range(n_tr):
            beta, n_new = draw_beta(Is_s, red_s, p, q_s, tape(b'rr-fs', name, cls, m))
            Lr = push_L(L_s, Is_s, beta, n_new, p)
            Sr = prepare_small(Lr, p)
            same_n_fs += int(Sr.norm == S_s.norm)
            same_S_fs += same_ideal(Sr, S_s)
            n_fs_total += 1
            wb, _ = run_q(S_s, p, e, tape(b'q-fs-b', name, cls, m))
            wr, _ = run_q(Sr, p, e, tape(b'q-fs-r', name, cls, m))
            wb_list.append(wb)
            wr_list.append(wr)
        fixS_base_ts.append(abs(welch_t(wb_list, rd_base)))
        fixS_rr_ts.append(abs(welch_t(wr_list, rd_rr)))

    rho_v2_base = stats.spearmanr(rd_v2, rd_base).correlation
    rho_v2_rr = stats.spearmanr(rd_rr_v2, rd_rr).correlation
    out = dict(
        set=name, n_tr=n_tr, sec=round(time.time() - t0, 1),
        t_fixkey_base=round(welch_t(fx_base, rd_base), 2),
        t_fixkey_rr=round(welch_t(fx_rr, rd_rr), 2),
        fixS_max_abs_t_base=round(max(fixS_base_ts), 2),
        fixS_max_abs_t_rr=round(max(fixS_rr_ts), 2),
        same_n_rate=round(same_n_fk / n_tr, 4),
        same_basis_rate=round(same_S_fk / n_tr, 4),
        fixS_same_n_rate=round(same_n_fs / n_fs_total, 4),
        fixS_same_basis_rate=round(same_S_fs / n_fs_total, 4),
        rho_v2_base=round(float(rho_v2_base), 3),
        rho_v2_rr=round(float(rho_v2_rr), 3),
    )
    print(out, flush=True)
    return out


def main():
    with Pool(2) as pool:
        rows = pool.map(eval_set, [(s[0], 300) for s in SETS])
    out = Path('results/qlapoti_rerand_table.csv')
    with out.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f'wrote {out}')


if __name__ == '__main__':
    main()
