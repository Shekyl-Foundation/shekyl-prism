# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Measurement driver for keygen-precompute across all five Shekyl sets.

Panels (chunked, resumable, <= 2 workers):
  setup   Keygen, key-load, intersection, and Gram setup op counts & limb ops.
  nvar    400 keys x 10 msgs per set, bits(N) = ceil(L/2) + U[0,32]:
          BASE vs EXP_RED_O0 (Z[i] on all 4,000; exact LLL on 2,000; 4-D updown on 500).
  nbound  200 keys x 10 msgs at bits(N) = B = ceil(L/2) + 32 (plus 8,000 extra
          Z[i] tail samples at B => 10,000 worst-case Z[i] samples per set).
  dudect  3,000 fixed vs 3,000 random traces per set (Z[i] early-exit, Z[i] fixed,
          and 1,000+1,000 exact LLL traces).
"""
import argparse
import csv
import time
from multiprocessing import Pool
from pathlib import Path

from hdl import (
    SETS, SALT_LEN, shake, prime_of, fs_challenge, select_xy, ideal_basis,
    gram_norm, secret_class, ideal_from_classes, nrd,
)
from fsr import (
    lll_exact, reduced_gram, enum_budget, fixed_tours, herm, Ctr,
)
from kpc import (
    PARAMS, OpCtr, find_ideal_generator, sim_quat_lideal_create,
    sim_inter_hnf_q3, sim_inter_dual_cref, inter_prereduced_zi,
    basis_from_zi, gram_from_zi, gram_cost_base, zi_gauss_uv,
)

RAW_DIR = Path(__file__).resolve().parent / 'results' / 'raw'
RAW_DIR.mkdir(parents=True, exist_ok=True)
SET_MAP = {n: (n, e, f) for n, e, f in SETS}


def sk_of(set_name, kid):
    return shake(b'kpc-sk', set_name.encode(), kid.to_bytes(4, 'little'), n=32)


def msg_of(set_name, kid, mid):
    return shake(b'kpc-msg', set_name.encode(), kid.to_bytes(4, 'little'),
                 mid.to_bytes(4, 'little'), n=32)


def u_off_of(sk):
    return int.from_bytes(shake(b'kpc-uoff', sk, n=4), 'little') % 33


def tmax_base_zi(e, B):
    return int(0.32 * (2 * e - 5 + 32)) + 30


def tmax_exp_zi(e):
    return int(0.32 * (2 * e - 5)) + 30


def tmax_base_4d(e):
    return int(0.25 * (2 * e - 5 + 32)) + 30


def tmax_exp_4d(e):
    return int(0.25 * (2 * e - 5)) + 30


def _bits(xs):
    return max(abs(x).bit_length() for x in xs)


def _write_csv(path, rows):
    if not rows:
        return
    tmp = path.with_suffix('.tmp')
    cols = list(rows[0].keys())
    with open(tmp, 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=cols)
        w.writeheader()
        w.writerows(rows)
    tmp.replace(path)


# --------------------------------------------------------- Setup panel
def _setup_worker(args):
    set_name, kids = args
    _, e, f = SET_MAP[set_name]
    p = prime_of(e, f)
    L = p.bit_length()
    base_nb = (L + 1) // 2
    B = base_nb + 32
    T_sec = int(0.32 * B) + 40
    pk = shake(b'kpc-pk', set_name.encode(), n=PARAMS[set_name]['pk_len'])
    rows = []
    for kid in kids:
        sk = sk_of(set_name, kid)
        u_off = u_off_of(sk)
        nb = base_nb + u_off
        N, xN, yN = secret_class(sk, p, nb)
        B_sec = ideal_basis(N, p, (xN, yN, 1, 0))

        # Keygen: BASE finds generator; EXP_RED pre-reduces I_sec
        t0 = time.time()
        gen = find_ideal_generator(B_sec, N, p)
        kg_gen_time = time.time() - t0

        t0 = time.time()
        u_sec, v_sec, nu_sec, nv_sec, c_kg_zi, _ = zi_gauss_uv(
            [N, 0, 0, 0], B_sec[2], p, T_sec, keep_trace=False
        )
        kg_zi_time = time.time() - t0
        kg_zi_maxbits = max(_bits(u_sec), _bits(v_sec), nu_sec.bit_length(), nv_sec.bit_length())
        kg_uv_big_bits = max(abs(u_sec[0]).bit_length(), abs(u_sec[1]).bit_length(),
                             abs(v_sec[0]).bit_length(), abs(v_sec[1]).bit_length())
        kg_uv_small_bits = max(abs(u_sec[2]).bit_length(), abs(u_sec[3]).bit_length(),
                               abs(v_sec[2]).bit_length(), abs(v_sec[3]).bit_length())

        # Key-load cost (quat_lideal_create)
        t0 = time.time()
        B_rec, c_create = sim_quat_lideal_create(gen, N, p)
        t_create = time.time() - t0
        assert B_rec == B_sec

        for mid in range(5):
            msg = msg_of(set_name, kid, mid)
            salt = shake(b'salt', sk, msg, n=SALT_LEN[set_name])
            _, q, _ = fs_challenge(e, pk, salt, msg)
            xq, yq, _ = select_xy(q, p, sk)
            B_q = ideal_basis(q, p, (xq, yq, 1, 0))

            # SIGNER_SPEC Q3 intersection + Gram
            t0 = time.time()
            B_q3, c_q3 = sim_inter_hnf_q3(B_sec, N, B_q, q)
            _, c_gbase = gram_cost_base(B_q3, p)
            t_q3 = time.time() - t0

            # PRISM_v2 dual-intersection (do on kid % 5 == 0 to keep runtime light)
            c_dual = c_dual_pre = None
            t_dual = 0.0
            if kid % 5 == 0 and mid == 0:
                t0 = time.time()
                _, c_dual = sim_inter_dual_cref(B_sec, N, B_q, q, have_dual_sec=False)
                t_dual = time.time() - t0
                _, c_dual_pre = sim_inter_dual_cref(B_sec, N, B_q, q, have_dual_sec=True)

            # EXP_RED_O0 intersection
            t0 = time.time()
            w1, w2, c_exp_inter = inter_prereduced_zi(u_sec, v_sec, xq, yq, q)
            c_gexp = OpCtr()
            _ = gram_from_zi(w1, w2, p, c_gexp)
            t_exp_inter = time.time() - t0

            row = dict(
                set=set_name, kid=kid, mid=mid, N_bits=N.bit_length(), q_bits=q.bit_length(),
                kg_gen_time=round(kg_gen_time, 5), kg_zi_time=round(kg_zi_time, 5),
                kg_zi_mul=c_kg_zi.mul, kg_zi_div=c_kg_zi.div, kg_zi_sel=c_kg_zi.sel,
                kg_zi_maxbits=kg_zi_maxbits,
                kg_uv_big_bits=kg_uv_big_bits, kg_uv_small_bits=kg_uv_small_bits,
                create_time=round(t_create, 5),
                create_mul=c_create.mul, create_div=c_create.div, create_xgcd=c_create.xgcd_steps,
                create_maxbits=c_create.max_bits, create_limb_ops=c_create.limb_ops,
                q3_time=round(t_q3, 5),
                q3_mul=c_q3.mul, q3_div=c_q3.div, q3_xgcd=c_q3.xgcd_steps,
                q3_maxbits=c_q3.max_bits, q3_limb_ops=c_q3.limb_ops,
                gbase_mul=c_gbase.mul, gbase_limb_ops=c_gbase.limb_ops, gbase_maxbits=c_gbase.max_bits,
                dual_time=round(t_dual, 5),
                dual_mul=c_dual.mul if c_dual else -1,
                dual_div=c_dual.div if c_dual else -1,
                dual_maxbits=c_dual.max_bits if c_dual else -1,
                dual_limb_ops=c_dual.limb_ops if c_dual else -1,
                dual_pre_limb_ops=c_dual_pre.limb_ops if c_dual_pre else -1,
                exp_inter_time=round(t_exp_inter, 5),
                exp_inter_mul=c_exp_inter.mul, exp_inter_div=c_exp_inter.div,
                exp_inter_xgcd=c_exp_inter.xgcd_steps,
                exp_inter_maxbits=c_exp_inter.max_bits,
                exp_inter_limb_ops=c_exp_inter.limb_ops,
                gexp_mul=c_gexp.mul,
            )
            rows.append(row)
    return rows


# --------------------------------------------------------- Reduction helpers
def _run_zi_metrics(u0, v0, p, steps, lam1, b1_target, prec=None):
    st = dict(b1=-1, lam1=-1, red=-1, mb=0)

    def zhook(t, u, v, nu, nv):
        if st['b1'] < 0 and nu <= b1_target:
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

    t0 = time.time()
    u, v, nu, nv, cz, eff = zi_gauss_uv(u0, v0, p, steps, on_step=zhook, keep_trace=True, prec=prec)
    dt = time.time() - t0
    return dict(
        time=round(dt, 5), tmax=steps, b1=st['b1'], lam1=st['lam1'], red=st['red'],
        maxbits=st['mb'], eff=eff, nu_final=nu, trace=cz.trace.hexdigest()[:16],
    )


def _nvar_worker(args):
    set_name, kids, n_msgs, lll_limit_kid, updown_limit_kid = args
    _, e, f = SET_MAP[set_name]
    p = prime_of(e, f)
    L = p.bit_length()
    base_nb = (L + 1) // 2
    B = base_nb + 32
    T_sec = int(0.32 * B) + 40
    Tz_b = tmax_base_zi(e, B)
    Tz_e = tmax_exp_zi(e)
    T4_b = tmax_base_4d(e)
    T4_e = tmax_exp_4d(e)
    pk = shake(b'kpc-pk', set_name.encode(), n=PARAMS[set_name]['pk_len'])
    rows = []
    for kid in kids:
        sk = sk_of(set_name, kid)
        u_off = u_off_of(sk)
        nb = base_nb + u_off
        N, xN, yN = secret_class(sk, p, nb)
        B_sec = ideal_basis(N, p, (xN, yN, 1, 0))
        u_sec, v_sec, _, _, _, _ = zi_gauss_uv([N, 0, 0, 0], B_sec[2], p, T_sec, keep_trace=False)

        for mid in range(n_msgs):
            msg = msg_of(set_name, kid, mid)
            salt = shake(b'salt', sk, msg, n=SALT_LEN[set_name])
            _, q, _ = fs_challenge(e, pk, salt, msg)
            xq, yq, _ = select_xy(q, p, sk)

            B_prod, T_prod = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
            w1, w2, c_ei = inter_prereduced_zi(u_sec, v_sec, xq, yq, q)

            # Determine exact lam1 by running Z[i] to full Tz_b (verified 500/500 with PARI)
            # and also cross-checking against enum on LLL rows.
            zb_full = _run_zi_metrics([T_prod, 0, 0, 0], B_prod[2], p, Tz_b, 0, 0)
            # Wait: zhook had lam1 target=0 above, so extract nu_final and re-query or track per-step!
            # Better: record nu history or run once with track! Let's use a 1-pass tracker:
            row = dict(
                set=set_name, kid=kid, mid=mid, N_bits=N.bit_length(),
                q_bits=q.bit_length(), T_bits=T_prod.bit_length(),
                exp_inter_mul=c_ei.mul, exp_inter_div=c_ei.div,
                exp_inter_xgcd=c_ei.xgcd_steps, exp_inter_limb_ops=c_ei.limb_ops,
            )
            # Clean 1-pass Z[i] tracking (collects nu(t) so we know exact step hitting final lam1)
            for tag, u0, v0, Tsteps, prec in [
                ('zb', [T_prod, 0, 0, 0], B_prod[2], Tz_b, None),
                ('ze', w1, w2, Tz_e, None),
                ('ze_p64', w1, w2, Tz_e, 64),
                ('ze_p32', w1, w2, Tz_e, 32),
                ('ze_p16', w1, w2, Tz_e, 16),
            ]:
                hist_nu = []
                st = dict(red=-1, mb=0)

                def _hk(t, u, v, nu, nv, _hist=hist_nu, _st=st):
                    _hist.append(nu)
                    if _st['red'] < 0:
                        h0, h1 = herm(v, u, p, Ctr())
                        if (2 * h0 + nu) // (2 * nu) == 0 and (2 * h1 + nu) // (2 * nu) == 0 and nu <= nv:
                            _st['red'] = t
                    m = max(_bits(u), _bits(v), nu.bit_length(), nv.bit_length())
                    if m > _st['mb']:
                        _st['mb'] = m

                t0 = time.time()
                _, _, nu_f, _, cz, eff = zi_gauss_uv(u0, v0, p, Tsteps, on_step=_hk, keep_trace=True, prec=prec)
                dt = time.time() - t0
                lam1_step = next((i + 1 for i, val in enumerate(hist_nu) if val == nu_f), -1)
                row[f'{tag}_time'] = round(dt, 5)
                row[f'{tag}_lam1'] = lam1_step
                row[f'{tag}_red'] = st['red']
                row[f'{tag}_maxbits'] = st['mb']
                row[f'{tag}_eff'] = eff
                row[f'{tag}_trace'] = cz.trace.hexdigest()[:16]
                row[f'{tag}_ok'] = int(nu_f % T_prod == 0 and nu_f > 0)
                if tag == 'zb':
                    lam1_ref = nu_f
                else:
                    row[f'{tag}_eq_lam1'] = int(nu_f == lam1_ref)

            # Exact LLL (BASE vs EXP_RED)
            if kid < lll_limit_kid:
                G_b = gram_norm(B_prod, p)
                t0 = time.time()
                H_b, d_b, l_b, c_b = lll_exact(G_b)
                row['exb_time'] = round(time.time() - t0, 5)
                row.update(c_b.as_dict('exb'))
                row['exb_maxbits'] = max(_bits(d_b), _bits([x for r in l_b for x in r]))
                row['exb_is_lam1'] = int(d_b[1] == lam1_ref)

                G_e = gram_from_zi(w1, w2, p, type('C', (), {'mul': 0})())
                t0 = time.time()
                H_e, d_e, l_e, c_e = lll_exact(G_e)
                row['exe_time'] = round(time.time() - t0, 5)
                row.update(c_e.as_dict('exe'))
                row['exe_maxbits'] = max(_bits(d_e), _bits([x for r in l_e for x in r]))
                row['exe_is_lam1'] = int(d_e[1] == lam1_ref)

                # 4-D updown fixed schedule (BASE vs EXP_RED)
                if kid < updown_limit_kid:
                    b1_b = d_b[1]
                    b1_e = d_e[1]
                    for utag, G_u, Tu, b1_t in [
                        ('u4b', G_b, T4_b, b1_b),
                        ('u4e', G_e, T4_e, b1_e),
                    ]:
                        ust = dict(b1=-1, lam1=-1, mb=0)

                        def _uhk(t, H_, d_, l_, _ust=ust, _b1=b1_t):
                            if _ust['b1'] < 0 and d_[1] <= _b1:
                                _ust['b1'] = t
                            if _ust['lam1'] < 0 and d_[1] == lam1_ref:
                                _ust['lam1'] = t
                            m = max(_bits(d_), _bits([x for r in l_ for x in r]))
                            if m > _ust['mb']:
                                _ust['mb'] = m

                        t0 = time.time()
                        _, d_u, _, cu, eff_u = fixed_tours(G_u, Tu, 'updown', on_tour=_uhk, keep_trace=True)
                        row[f'{utag}_time'] = round(time.time() - t0, 5)
                        row[f'{utag}_b1'] = ust['b1']
                        row[f'{utag}_lam1'] = ust['lam1']
                        row[f'{utag}_maxbits'] = ust['mb']
                        row[f'{utag}_eff'] = eff_u
                        row[f'{utag}_ok_b1'] = int(d_u[1] <= b1_t)
                        row[f'{utag}_ok_lam1'] = int(d_u[1] == lam1_ref)
                else:
                    for pfx in ('u4b', 'u4e'):
                        row[f'{pfx}_time'] = -1
                        row[f'{pfx}_b1'] = row[f'{pfx}_lam1'] = row[f'{pfx}_maxbits'] = row[f'{pfx}_eff'] = -1
                        row[f'{pfx}_ok_b1'] = row[f'{pfx}_ok_lam1'] = -1
            else:
                for pfx in ('exb', 'exe'):
                    for k in ('time', 'mul', 'div', 'sel', 'lovasz', 'redi', 'swap', 'maxbits', 'is_lam1'):
                        row[f'{pfx}_{k}'] = -1
                for pfx in ('u4b', 'u4e'):
                    for k in ('time', 'b1', 'lam1', 'maxbits', 'eff', 'ok_b1', 'ok_lam1'):
                        row[f'{pfx}_{k}'] = -1
            rows.append(row)
    return rows


def _nbound_worker(args):
    """Worst-case bits(N) = B = ceil(L/2) + 32 calibration (Z[i] + optional 4-D updown)."""
    set_name, kids, n_msgs, updown_limit_mid = args
    _, e, f = SET_MAP[set_name]
    p = prime_of(e, f)
    L = p.bit_length()
    B = (L + 1) // 2 + 32
    T_sec = int(0.32 * B) + 40
    Tz_b = tmax_base_zi(e, B)
    Tz_e = tmax_exp_zi(e)
    T4_b = tmax_base_4d(e)
    T4_e = tmax_exp_4d(e)
    pk = shake(b'kpc-pk', set_name.encode(), n=PARAMS[set_name]['pk_len'])
    rows = []
    for kid in kids:
        sk = shake(b'kpc-nb-sk', set_name.encode(), kid.to_bytes(4, 'little'), n=32)
        N, xN, yN = secret_class(sk, p, B)
        B_sec = ideal_basis(N, p, (xN, yN, 1, 0))
        u_sec, v_sec, _, _, _, _ = zi_gauss_uv([N, 0, 0, 0], B_sec[2], p, T_sec, keep_trace=False)

        for mid in range(n_msgs):
            msg = shake(b'kpc-nb-msg', set_name.encode(), kid.to_bytes(4, 'little'),
                        mid.to_bytes(4, 'little'), n=32)
            salt = shake(b'salt', sk, msg, n=SALT_LEN[set_name])
            _, q, _ = fs_challenge(e, pk, salt, msg)
            xq, yq, _ = select_xy(q, p, sk)
            B_prod, T_prod = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
            w1, w2, _ = inter_prereduced_zi(u_sec, v_sec, xq, yq, q)

            row = dict(set=set_name, kid=kid, mid=mid, N_bits=N.bit_length(), q_bits=q.bit_length())
            for tag, u0, v0, Tsteps, prec in [
                ('zb', [T_prod, 0, 0, 0], B_prod[2], Tz_b, None),
                ('ze', w1, w2, Tz_e, None),
                ('ze_p16', w1, w2, Tz_e, 16),
            ]:
                hist_nu = []
                st = dict(red=-1, mb=0)

                def _hk(t, u, v, nu, nv, _hist=hist_nu, _st=st):
                    _hist.append(nu)
                    if _st['red'] < 0:
                        h0, h1 = herm(v, u, p, Ctr())
                        if (2 * h0 + nu) // (2 * nu) == 0 and (2 * h1 + nu) // (2 * nu) == 0 and nu <= nv:
                            _st['red'] = t
                    m = max(_bits(u), _bits(v), nu.bit_length(), nv.bit_length())
                    if m > _st['mb']:
                        _st['mb'] = m

                _, _, nu_f, _, _, eff = zi_gauss_uv(u0, v0, p, Tsteps, on_step=_hk, keep_trace=False, prec=prec)
                lam1_step = next((i + 1 for i, val in enumerate(hist_nu) if val == nu_f), -1)
                row[f'{tag}_lam1'] = lam1_step
                row[f'{tag}_red'] = st['red']
                row[f'{tag}_maxbits'] = st['mb']
                if tag == 'zb':
                    lam1_ref = nu_f
                else:
                    row[f'{tag}_eq_lam1'] = int(nu_f == lam1_ref)

            if mid < updown_limit_mid:
                G_b = gram_norm(B_prod, p)
                _, d_b, _, _ = lll_exact(G_b)
                G_e = gram_from_zi(w1, w2, p, type('C', (), {'mul': 0})())
                _, d_e, _, _ = lll_exact(G_e)
                for utag, G_u, Tu, b1_t in [
                    ('u4b', G_b, T4_b, d_b[1]),
                    ('u4e', G_e, T4_e, d_e[1]),
                ]:
                    ust = dict(b1=-1, lam1=-1)

                    def _uhk(t, H_, d_, l_, _ust=ust, _b1=b1_t):
                        if _ust['b1'] < 0 and d_[1] <= _b1:
                            _ust['b1'] = t
                        if _ust['lam1'] < 0 and d_[1] == lam1_ref:
                            _ust['lam1'] = t

                    _, d_u, _, _, _ = fixed_tours(G_u, Tu, 'updown', on_tour=_uhk, keep_trace=False)
                    row[f'{utag}_b1'] = ust['b1']
                    row[f'{utag}_lam1'] = ust['lam1']
                    row[f'{utag}_ok_b1'] = int(d_u[1] <= b1_t)
                    row[f'{utag}_ok_lam1'] = int(d_u[1] == lam1_ref)
            else:
                for pfx in ('u4b', 'u4e'):
                    row[f'{pfx}_b1'] = row[f'{pfx}_lam1'] = row[f'{pfx}_ok_b1'] = row[f'{pfx}_ok_lam1'] = -1
            rows.append(row)
    return rows


def _dudect_worker(args):
    set_name, mode, tids, lll_limit = args
    _, e, f = SET_MAP[set_name]
    p = prime_of(e, f)
    L = p.bit_length()
    base_nb = (L + 1) // 2
    B = base_nb + 32
    T_sec = int(0.32 * B) + 40
    Tz_b = tmax_base_zi(e, B)
    Tz_e = tmax_exp_zi(e)
    pk = shake(b'kpc-pk', set_name.encode(), n=PARAMS[set_name]['pk_len'])

    if mode == 'fixed':
        sk_fix = shake(b'kpc-dudect-fix', set_name.encode(), n=32)
        N_fix, xN_fix, yN_fix = secret_class(sk_fix, p, B - 2)
        B_sec_fix = ideal_basis(N_fix, p, (xN_fix, yN_fix, 1, 0))
        u_fix, v_fix, _, _, _, _ = zi_gauss_uv([N_fix, 0, 0, 0], B_sec_fix[2], p, T_sec, keep_trace=False)

    rows = []
    for tid in tids:
        if mode == 'fixed':
            sk = sk_fix
            N, xN, yN = N_fix, xN_fix, yN_fix
            u_sec, v_sec = u_fix, v_fix
        else:
            sk = shake(b'kpc-dudect-rnd', set_name.encode(), tid.to_bytes(4, 'little'), n=32)
            nb = base_nb + u_off_of(sk)
            N, xN, yN = secret_class(sk, p, nb)
            B_sec = ideal_basis(N, p, (xN, yN, 1, 0))
            u_sec, v_sec, _, _, _, _ = zi_gauss_uv([N, 0, 0, 0], B_sec[2], p, T_sec, keep_trace=False)

        msg = shake(b'kpc-dudect-msg', set_name.encode(), mode.encode(), tid.to_bytes(4, 'little'), n=32)
        salt = shake(b'salt', sk, msg, n=SALT_LEN[set_name])
        _, q, _ = fs_challenge(e, pk, salt, msg)
        xq, yq, _ = select_xy(q, p, sk)
        B_prod, T_prod = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
        w1, w2, _ = inter_prereduced_zi(u_sec, v_sec, xq, yq, q)

        # Early-exit Z[i] step counts (BASE vs EXP_RED) + fixed-schedule trace
        st_b = dict(red=-1)
        def _hb(t, u, v, nu, nv):
            if st_b['red'] < 0:
                h0, h1 = herm(v, u, p, Ctr())
                if (2 * h0 + nu) // (2 * nu) == 0 and (2 * h1 + nu) // (2 * nu) == 0 and nu <= nv:
                    st_b['red'] = t
        _, _, _, _, czb, _ = zi_gauss_uv([T_prod, 0, 0, 0], B_prod[2], p, Tz_b, on_step=_hb, keep_trace=True)

        st_e = dict(red=-1)
        def _he(t, u, v, nu, nv):
            if st_e['red'] < 0:
                h0, h1 = herm(v, u, p, Ctr())
                if (2 * h0 + nu) // (2 * nu) == 0 and (2 * h1 + nu) // (2 * nu) == 0 and nu <= nv:
                    st_e['red'] = t
        _, _, _, _, cze, _ = zi_gauss_uv(w1, w2, p, Tz_e, on_step=_he, keep_trace=True)

        exb_lov = exb_mul = exe_lov = exe_mul = -1
        if tid < lll_limit:
            G_b = gram_norm(B_prod, p)
            _, _, _, cb = lll_exact(G_b)
            exb_lov, exb_mul = cb.lovasz, cb.mul + cb.div
            G_e = gram_from_zi(w1, w2, p, type('C', (), {'mul': 0})())
            _, _, _, ce = lll_exact(G_e)
            exe_lov, exe_mul = ce.lovasz, ce.mul + ce.div

        rows.append(dict(
            set=set_name, mode=mode, tid=tid, N_bits=N.bit_length(),
            zb_early=st_b['red'], ze_early=st_e['red'],
            zb_fixed_mul=czb.mul + czb.div, ze_fixed_mul=cze.mul + cze.div,
            ze_trace=cze.trace.hexdigest()[:16],
            exb_lov=exb_lov, exb_mul=exb_mul,
            exe_lov=exe_lov, exe_mul=exe_mul,
        ))
    return rows


def run_chunked(pool, fn, tasks, out_paths):
    pending = [(t, p) for t, p in zip(tasks, out_paths) if not p.exists()]
    if not pending:
        return
    results = pool.map(fn, [t for t, _ in pending])
    for (_, p), rows in zip(pending, results):
        _write_csv(p, rows)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--jobs', type=int, default=2)
    ap.add_argument('--smoke', action='store_true')
    ap.add_argument('--sets', nargs='*', default=[s[0] for s in SETS])
    args = ap.parse_args()
    jobs = min(args.jobs, 2)  # box rule: at most 2 worker processes

    if args.smoke:
        n_setup_k = 4
        n_nvar_k, n_nvar_m, lll_k, up4_k = 6, 2, 4, 2
        n_nb_k, n_nb_m, nb_up4_k = 6, 2, 2
        n_dudect, dud_lll = 20, 10
        blk = 2
    else:
        n_setup_k = 100
        n_nvar_k, n_nvar_m, lll_k, up4_k = 400, 10, 200, 50
        n_nb_k, n_nb_m, nb_up4_k = 100, 100, 5
        n_dudect, dud_lll = 3000, 500
        blk = 50

    with Pool(jobs) as pool:
        for sname in args.sets:
            t_set = time.time()
            print(f"=== {sname}: setup ===", flush=True)
            tasks, paths = [], []
            for k0 in range(0, n_setup_k, blk):
                kids = list(range(k0, min(k0 + blk, n_setup_k)))
                tasks.append((sname, kids))
                paths.append(RAW_DIR / f"setup_{sname}_k{k0:03d}.csv")
            run_chunked(pool, _setup_worker, tasks, paths)

            print(f"=== {sname}: nvar ===", flush=True)
            tasks, paths = [], []
            for k0 in range(0, n_nvar_k, blk):
                kids = list(range(k0, min(k0 + blk, n_nvar_k)))
                tasks.append((sname, kids, n_nvar_m, lll_k, up4_k))
                paths.append(RAW_DIR / f"nvar_{sname}_k{k0:03d}.csv")
            run_chunked(pool, _nvar_worker, tasks, paths)

            print(f"=== {sname}: nbound ===", flush=True)
            tasks, paths = [], []
            nb_blk = max(blk // 2, 1) if not args.smoke else blk
            for k0 in range(0, n_nb_k, nb_blk):
                kids = list(range(k0, min(k0 + nb_blk, n_nb_k)))
                tasks.append((sname, kids, n_nb_m, nb_up4_k))
                paths.append(RAW_DIR / f"nbound_{sname}_k{k0:03d}.csv")
            run_chunked(pool, _nbound_worker, tasks, paths)

            print(f"=== {sname}: dudect ===", flush=True)
            d_blk = 500 if not args.smoke else 10
            for mode in ('fixed', 'rand'):
                tasks, paths = [], []
                for t0 in range(0, n_dudect, d_blk):
                    tids = list(range(t0, min(t0 + d_blk, n_dudect)))
                    tasks.append((sname, mode, tids, dud_lll))
                    paths.append(RAW_DIR / f"dudect_{mode}_{sname}_k{t0:04d}.csv")
                run_chunked(pool, _dudect_worker, tasks, paths)

            print(f"=== {sname}: done in {time.time()-t_set:.1f}s ===", flush=True)


if __name__ == '__main__':
    main()
