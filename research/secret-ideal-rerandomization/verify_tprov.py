# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Verify Z[i]-Lagrange fixed schedule at the certified T_prov tour counts
(from research/lagrange-tour-bound) on both the baseline product I ∩ I_q
and the re-randomized product J * K_J, plus keygen reduction."""
import csv
from pathlib import Path

from hdl import (SETS, prime_of, shake, stream_job_data, select_xy,
                 ideal_from_classes, gram_norm, apply)
from fsr import zi_gauss_fixed, lll_exact, ZI_STEP_COST
from run_fsr import msg_of, secret_class_var, n_bound
from rr import (keygen_reduce, sample_any, rerandomized_product,
                is_standard, map_back, canon)

T_PROV = {'p248': 301, 'p324': 387, 'p376': 446, 'p500': 587, 'p664': 773}
SET_MAP = {n: (n, e, f) for n, e, f in SETS}


def main(n_keys=50, n_msgs=2):
    rows = []
    for set_name, e, f in SETS:
        p = prime_of(e, f)
        tp = T_PROV[set_name]
        traces = set()
        ok_base = ok_odd_norm = ok_odd_gauss = 0
        same_on = same_og = 0
        mul_ex_sum = div_ex_sum = 0
        n_total = n_keys * n_msgs
        for kid in range(n_keys):
            sk = shake(b'sk-rr-pair', set_name.encode(), kid.to_bytes(4, 'little'), n=32)
            N, xN, yN, nb = secret_class_var(sk, p)
            kr = keygen_reduce(N, xN, yN, p)
            for mid in range(n_msgs):
                msg = msg_of(set_name, kid, mid)
                data = stream_job_data(sk, msg, set_name, e, f)
                q = data['q']
                xq, yq, _ = select_xy(q, p, data['seed'])
                Bp, T = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
                _, d, _, c_ex = lll_exact(gram_norm(Bp, p))
                mul_ex_sum += c_ex.mul
                div_ex_sum += c_ex.div
                ub, _, nub, _, cb, _ = zi_gauss_fixed(T, Bp[2], p, tp)
                traces.add(cb.trace.hexdigest()[:16])
                ok_base += int(nub <= d[1])

                seed = shake(b'rr-seed', sk, msg, data['salt'])
                # odd_norm
                ron = sample_any(kr, N, p, seed, 'odd', 'norm', 1 << 8, n_bound(p))
                Bon = rerandomized_product(Bp, ron['beta'], N, p)
                uon, _, nuon, _, con, _ = zi_gauss_fixed(ron['Np'] * q, Bon[2], p, tp)
                traces.add(con.trace.hexdigest()[:16])
                ok_odd_norm += int(nuon * N <= d[1] * ron['Np'])
                same_on += int(canon(map_back(uon, ron['beta'], ron['Np'], p)) == canon(ub))

                # odd_gauss
                rog = sample_any(kr, N, p, seed, 'odd', 'gauss', 1 << 8, n_bound(p))
                Bog = rerandomized_product(Bp, rog['beta'], N, p)
                uog, _, nuog, _, cog, _ = zi_gauss_fixed(rog['Np'] * q, Bog[2], p, tp)
                traces.add(cog.trace.hexdigest()[:16])
                ok_odd_gauss += int(nuog * N <= d[1] * rog['Np'])
                same_og += int(canon(map_back(uog, rog['beta'], rog['Np'], p)) == canon(ub))

        ex_md = (mul_ex_sum + div_ex_sum) / n_total
        zi_mul = 10 + ZI_STEP_COST['mul'] * tp
        zi_div = ZI_STEP_COST['div'] * tp
        zi_sel = 10 + ZI_STEP_COST['sel'] * tp
        zi_md = zi_mul + zi_div
        rows.append(dict(
            set=set_name, T_prov=tp, n=n_total,
            zi_traces=len(traces),
            zi_mul=zi_mul, zi_div=zi_div, zi_sel=zi_sel, zi_mul_div=zi_md,
            ex_mul_div_mean=round(ex_md, 1),
            ratio_vs_ex_md=round(zi_md / ex_md, 3),
            base_short_rate=round(ok_base / n_total, 4),
            odd_norm_short_rate=round(ok_odd_norm / n_total, 4),
            odd_gauss_short_rate=round(ok_odd_gauss / n_total, 4),
            odd_norm_same_rate=round(same_on / n_total, 4),
            odd_gauss_same_rate=round(same_og / n_total, 4),
        ))
    out = Path('results/tprov_table.csv')
    with out.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f'wrote {out}')


if __name__ == '__main__':
    main()
