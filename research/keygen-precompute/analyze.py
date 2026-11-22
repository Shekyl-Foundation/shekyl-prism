# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Analyze keygen-precompute results and produce CSV tables and plots."""
import glob
import json
from pathlib import Path

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
from scipy import stats

from fsr import per_tour_cost
from hdl import SETS
from kpc import PARAMS, key_sizes_bytes

ROOT = Path(__file__).resolve().parent
RAW = ROOT / 'results' / 'raw'
OUT = ROOT / 'results'
PLOTS = ROOT / 'plots'
OUT.mkdir(parents=True, exist_ok=True)
PLOTS.mkdir(parents=True, exist_ok=True)

SET_NAMES = [s[0] for s in SETS]
ZI_STEP_MUL = 25  # 23 mul + 2 div
ZI_INIT_MUL = 10


def load(pat):
    fs = sorted(glob.glob(str(RAW / pat)))
    return pd.concat([pd.read_csv(f) for f in fs], ignore_index=True) if fs else None


def limb_factor(bits):
    w = max(1, (int(bits) + 63) // 64)
    return w * w


def main():
    # 1. Key sizes table
    ks_rows = []
    for s in SET_NAMES:
        pi = PARAMS[s]
        ks = key_sizes_bytes(pi)
        row = dict(set=s, L=pi['L'], e=pi['e'], B=pi['B'], **ks)
        row['delta_red_nocan_vs_spec'] = ks['exp_red_compact_no_can'] - ks['base_spec_sk']
        row['delta_red_can_vs_spec'] = ks['exp_red_compact_with_can'] - ks['base_spec_sk']
        row['delta_hnf_can_vs_spec'] = ks['exp_hnf_with_can'] - ks['base_spec_sk']
        row['delta_red_compat_vs_cref'] = ks['exp_red_compat'] - ks['cref_sk']
        ks_rows.append(row)
    ks_df = pd.DataFrame(ks_rows)
    ks_df.to_csv(OUT / 'key_sizes_table.csv', index=False)

    # 2. Setup + keygen + intersection costs
    setup = load('setup_*.csv')
    setup_rows = []
    if setup is not None:
        for s in SET_NAMES:
            g = setup[setup.set == s]
            if len(g) == 0:
                continue
            g_dual = g[g.dual_mul >= 0]
            # per-phase mean op counts and limb ops
            row = dict(
                set=s, n=len(g),
                kg_gen_time_ms=round(1000 * g.kg_gen_time.mean(), 3),
                kg_zi_time_ms=round(1000 * g.kg_zi_time.mean(), 3),
                kg_zi_muldiv=int(round((g.kg_zi_mul + g.kg_zi_div).mean())),
                kg_zi_maxbits=int(g.kg_zi_maxbits.max()),
                kg_uv_big_bits_max=int(g.kg_uv_big_bits.max()),
                kg_uv_small_bits_max=int(g.kg_uv_small_bits.max()),
                create_muldiv=int(round((g.create_mul + g.create_div).mean())),
                create_xgcd=round(g.create_xgcd.mean(), 1),
                create_maxbits=int(g.create_maxbits.max()),
                create_limb_ops=int(round(g.create_limb_ops.mean())),
                q3_muldiv=int(round((g.q3_mul + g.q3_div).mean())),
                q3_xgcd=round(g.q3_xgcd.mean(), 1),
                q3_maxbits=int(g.q3_maxbits.max()),
                q3_limb_ops=int(round(g.q3_limb_ops.mean())),
                gbase_mul=int(round(g.gbase_mul.mean())),
                gbase_limb_ops=int(round(g.gbase_limb_ops.mean())),
                dual_muldiv=int(round((g_dual.dual_mul + g_dual.dual_div).mean())) if len(g_dual) else -1,
                dual_maxbits=int(g_dual.dual_maxbits.max()) if len(g_dual) else -1,
                dual_limb_ops=int(round(g_dual.dual_limb_ops.mean())) if len(g_dual) else -1,
                dual_pre_limb_ops=int(round(g_dual.dual_pre_limb_ops.mean())) if len(g_dual) else -1,
                exp_inter_muldiv=int(round((g.exp_inter_mul + g.exp_inter_div).mean())),
                exp_inter_xgcd=round(g.exp_inter_xgcd.mean(), 1),
                exp_inter_maxbits=int(g.exp_inter_maxbits.max()),
                exp_inter_limb_ops=int(round(g.exp_inter_limb_ops.mean())),
                exp_inter_time_ms=round(1000 * g.exp_inter_time.mean(), 3),
                q3_time_ms=round(1000 * g.q3_time.mean(), 3),
            )
            row['kg_zi_limb_ops'] = row['kg_zi_muldiv'] * limb_factor(row['kg_zi_maxbits'])
            setup_rows.append(row)
    setup_df = pd.DataFrame(setup_rows)
    setup_df.to_csv(OUT / 'setup_table.csv', index=False)

    # 3. Reduction table (nvar + nbound)
    nvar = load('nvar_*.csv')
    nbnd = load('nbound_*.csv')
    ud_pt, ud_init = per_tour_cost('updown')
    ud_tour_mul = ud_pt['mul'] + ud_pt['div']
    ud_init_mul = ud_init['mul'] + ud_init['div']

    red_rows = []
    comb_rows = []
    if nvar is not None and nbnd is not None:
        for s in SET_NAMES:
            gv = nvar[nvar.set == s]
            gb = nbnd[nbnd.set == s]
            if len(gv) == 0 or len(gb) == 0:
                continue
            srow = setup_df[setup_df.set == s].iloc[0] if len(setup_df) else None

            # Calibrated T from max observed across nvar + nbound
            Tb_zi = int(max(gv.zb_red.max(), gb.zb_red.max()))
            Te_zi = int(max(gv.ze_red.max(), gb.ze_red.max()))
            Te_p64 = int(gv.ze_p64_red.max())
            Te_p32 = int(gv.ze_p32_red.max())
            Te_p16 = int(max(gv.ze_p16_red.max(), gb.ze_p16_red.max()))

            # 4-D updown calibrated T (rows where u4 >= 0)
            gvu = gv[gv.u4b_b1 >= 0]
            gbu = gb[gb.u4b_b1 >= 0]
            Tb_4d = int(max(gvu.u4b_b1.max(), gbu.u4b_b1.max()))
            Te_4d = int(max(gvu.u4e_b1.max(), gbu.u4e_b1.max()))

            # Exact LLL regression vs N_bits (rows where exb_lovasz >= 0)
            gvl = gv[gv.exb_lovasz >= 0]
            sl_b, _, r_b, p_b, _ = stats.linregress(gvl.N_bits, gvl.exb_lovasz)
            sl_e, _, r_e, p_e, _ = stats.linregress(gvl.N_bits, gvl.exe_lovasz)

            # Z[i] early-exit regression vs N_bits
            sl_zb, _, r_zb, p_zb, _ = stats.linregress(gv.N_bits, gv.zb_red)
            sl_ze, _, r_ze, p_ze, _ = stats.linregress(gv.N_bits, gv.ze_red)

            exb_mul_mean = float((gvl.exb_mul + gvl.exb_div).mean())
            exe_mul_mean = float((gvl.exe_mul + gvl.exe_div).mean())
            zb_fixed_mul = ZI_INIT_MUL + Tb_zi * ZI_STEP_MUL
            ze_fixed_mul = ZI_INIT_MUL + Te_zi * ZI_STEP_MUL
            u4b_fixed_mul = ud_init_mul + Tb_4d * ud_tour_mul
            u4e_fixed_mul = ud_init_mul + Te_4d * ud_tour_mul

            zb_mb = int(max(gv.zb_maxbits.max(), gb.zb_maxbits.max()))
            ze_mb = int(max(gv.ze_maxbits.max(), gb.ze_maxbits.max()))
            exb_mb = int(gvl.exb_maxbits.max())
            exe_mb = int(gvl.exe_maxbits.max())

            red_rows.append(dict(
                set=s, n_zi=len(gv) + len(gb), n_lll=len(gvl),
                exb_lovasz_mean=round(gvl.exb_lovasz.mean(), 1),
                exb_mul_mean=round(exb_mul_mean, 1),
                exb_slope=round(sl_b, 3), exb_r=round(r_b, 3), exb_p=float(p_b),
                exe_lovasz_mean=round(gvl.exe_lovasz.mean(), 1),
                exe_mul_mean=round(exe_mul_mean, 1),
                exe_slope=round(sl_e, 3), exe_r=round(r_e, 3), exe_p=float(p_e),
                zb_slope=round(sl_zb, 3), zb_r=round(r_zb, 3),
                ze_slope=round(sl_ze, 3), ze_r=round(r_ze, 3),
                Tb_zi=Tb_zi, Te_zi=Te_zi,
                Te_p64=Te_p64, Te_p32=Te_p32, Te_p16=Te_p16,
                ze_lam1_rate=round(float(pd.concat([gv.ze_eq_lam1, gb.ze_eq_lam1]).mean()), 6),
                ze_p16_lam1_rate=round(float(pd.concat([gv.ze_p16_eq_lam1, gb.ze_p16_eq_lam1]).mean()), 6),
                zb_fixed_mul=zb_fixed_mul, ze_fixed_mul=ze_fixed_mul,
                zi_step_saving_pct=round(100 * (1 - Te_zi / Tb_zi), 2),
                Tb_4d=Tb_4d, Te_4d=Te_4d,
                u4b_fixed_mul=u4b_fixed_mul, u4e_fixed_mul=u4e_fixed_mul,
                u4_step_saving_pct=round(100 * (1 - Te_4d / Tb_4d), 2),
                zb_maxbits=zb_mb, ze_maxbits=ze_mb, exb_maxbits=exb_mb, exe_maxbits=exe_mb,
                zb_time_ms=round(1000 * gv.zb_time.mean(), 3),
                ze_time_ms=round(1000 * gv.ze_time.mean(), 3),
                exb_time_ms=round(1000 * gvl.exb_time.mean(), 3),
                exe_time_ms=round(1000 * gvl.exe_time.mean(), 3),
            ))

            # Combined Setup + Reduction (op counts & 64-bit limb ops)
            if srow is not None:
                # 1. BASE_SPEC (no create, i.e., session) + Q3 HNF + Z[i]
                base_sess_mul = int(srow['q3_muldiv'] + zb_fixed_mul)
                base_sess_limb = int(srow['q3_limb_ops'] + zb_fixed_mul * limb_factor(zb_mb))
                # 2. BASE_SPEC (disk load: create + Q3 HNF + Z[i])
                base_disk_mul = int(srow['create_muldiv'] + base_sess_mul)
                base_disk_limb = int(srow['create_limb_ops'] + base_sess_limb)
                # 3. BASE_CREF (create + 3-dual inter + Exact LLL)
                cref_disk_mul = int(srow['create_muldiv'] + srow['dual_muldiv'] + srow['gbase_mul'] + exb_mul_mean)
                cref_disk_limb = int(srow['create_limb_ops'] + srow['dual_limb_ops'] + srow['gbase_limb_ops'] + exb_mul_mean * limb_factor(exb_mb))
                # 4. EXP_HNF (skip create, keep Q3 HNF + Z[i]) -> same as base_sess
                exp_hnf_mul = base_sess_mul
                exp_hnf_limb = base_sess_limb
                # 5. EXP_RED_O0 (phi_q/modinv/lift + Z[i])
                exp_red_mul = int(srow['exp_inter_muldiv'] + ze_fixed_mul)
                exp_red_limb = int(srow['exp_inter_limb_ops'] + ze_fixed_mul * limb_factor(ze_mb))

                comb_rows.append(dict(
                    set=s,
                    base_disk_mul=base_disk_mul, base_sess_mul=base_sess_mul,
                    cref_disk_mul=cref_disk_mul, exp_red_mul=exp_red_mul,
                    save_mul_vs_base_sess_pct=round(100 * (1 - exp_red_mul / base_sess_mul), 2),
                    save_mul_vs_base_disk_pct=round(100 * (1 - exp_red_mul / base_disk_mul), 2),
                    save_mul_vs_cref_disk_pct=round(100 * (1 - exp_red_mul / cref_disk_mul), 2),
                    base_disk_limb=base_disk_limb, base_sess_limb=base_sess_limb,
                    cref_disk_limb=cref_disk_limb, exp_red_limb=exp_red_limb,
                    save_limb_vs_base_sess_pct=round(100 * (1 - exp_red_limb / base_sess_limb), 2),
                    save_limb_vs_base_disk_pct=round(100 * (1 - exp_red_limb / base_disk_limb), 2),
                    save_limb_vs_cref_disk_pct=round(100 * (1 - exp_red_limb / cref_disk_limb), 2),
                    kg_zi_limb=int(srow['kg_zi_limb_ops']),
                    # break-even signatures = kg_zi_limb / (base_sess_limb - exp_red_limb)
                    breakeven_sigs_sess=round(srow['kg_zi_limb_ops'] / max(1, base_sess_limb - exp_red_limb), 2),
                    breakeven_sigs_disk=round(srow['kg_zi_limb_ops'] / max(1, base_disk_limb - exp_red_limb), 2),
                ))

    red_df = pd.DataFrame(red_rows)
    red_df.to_csv(OUT / 'reduction_table.csv', index=False)
    comb_df = pd.DataFrame(comb_rows)
    comb_df.to_csv(OUT / 'combined_savings_table.csv', index=False)

    # 4. Dudect table
    dud_rows = []
    dfix = load('dudect_fixed_*.csv')
    drnd = load('dudect_rand_*.csv')
    if dfix is not None and drnd is not None:
        for s in SET_NAMES:
            gf = dfix[dfix.set == s]
            gr = drnd[drnd.set == s]
            if len(gf) == 0 or len(gr) == 0:
                continue
            gfl = gf[gf.exb_lov >= 0]
            grl = gr[gr.exb_lov >= 0]
            t_exb, p_exb = stats.ttest_ind(gfl.exb_lov, grl.exb_lov, equal_var=False)
            t_exe, p_exe = stats.ttest_ind(gfl.exe_lov, grl.exe_lov, equal_var=False)
            t_zb_e, p_zb_e = stats.ttest_ind(gf.zb_early, gr.zb_early, equal_var=False)
            t_ze_e, p_ze_e = stats.ttest_ind(gf.ze_early, gr.ze_early, equal_var=False)
            n_uniq_traces = pd.concat([gf.ze_trace, gr.ze_trace]).nunique()
            dud_rows.append(dict(
                set=s, n_zi=len(gf), n_lll=len(gfl),
                fix_Nbits=int(gf.N_bits.iloc[0]),
                exb_fix_mean=round(gfl.exb_lov.mean(), 1), exb_rnd_mean=round(grl.exb_lov.mean(), 1),
                exb_t=round(float(t_exb), 2), exb_p=float(p_exb),
                exe_fix_mean=round(gfl.exe_lov.mean(), 1), exe_rnd_mean=round(grl.exe_lov.mean(), 1),
                exe_t=round(float(t_exe), 2), exe_p=float(p_exe),
                zb_early_fix=round(gf.zb_early.mean(), 2), zb_early_rnd=round(gr.zb_early.mean(), 2),
                zb_early_t=round(float(t_zb_e), 2), zb_early_p=float(p_zb_e),
                ze_early_fix=round(gf.ze_early.mean(), 2), ze_early_rnd=round(gr.ze_early.mean(), 2),
                ze_early_t=round(float(t_ze_e), 2), ze_early_p=float(p_ze_e),
                ze_fixed_traces=int(n_uniq_traces), ze_fixed_t=0.0,
            ))
    dud_df = pd.DataFrame(dud_rows)
    dud_df.to_csv(OUT / 'dudect_table.csv', index=False)

    # 5. Plots
    # Plot 1: Key sizes
    fig, ax = plt.subplots(figsize=(8.5, 4.5))
    x = np.arange(len(SET_NAMES))
    w = 0.19
    ax.bar(x - 1.5*w, ks_df.base_spec_sk, w, label='Baseline (spec compact)')
    ax.bar(x - 0.5*w, ks_df.exp_red_compact_no_can, w, label='EXP_RED (no canonical basis)')
    ax.bar(x + 0.5*w, ks_df.exp_hnf_with_can, w, label='EXP_HNF (+ canonical basis)')
    ax.bar(x + 1.5*w, ks_df.exp_red_compact_with_can, w, label='EXP_RED (+ canonical basis)')
    ax.set_xticks(x)
    ax.set_xticklabels([s.upper() for s in SET_NAMES])
    ax.set_ylabel('Secret key size (bytes)')
    ax.set_title('Secret-key size across parameter sets')
    ax.legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(PLOTS / 'key_sizes.png', dpi=120)
    plt.close(fig)

    # Plot 2: Combined 64-bit limb-ops comparison
    if len(comb_df):
        fig, ax = plt.subplots(figsize=(9, 4.5))
        x = np.arange(len(comb_df))
        w = 0.22
        ax.bar(x - w, comb_df.base_disk_limb / 1e6, w, label='BASE disk (create + Q3 HNF + Z[i])')
        ax.bar(x, comb_df.base_sess_limb / 1e6, w, label='BASE session / EXP_HNF (Q3 HNF + Z[i])')
        ax.bar(x + w, comb_df.exp_red_limb / 1e6, w, label='EXP_RED_O0 (phi_q inter + Z[i])')
        ax.set_xticks(x)
        ax.set_xticklabels([s.upper() for s in comb_df.set])
        ax.set_ylabel('64-bit limb ops (millions)')
        ax.set_title('Per-signature setup + reduction limb-op cost')
        ax.legend(fontsize=8)
        fig.tight_layout()
        fig.savefig(PLOTS / 'setup_and_reduce_limbs.png', dpi=120)
        plt.close(fig)

    # Plot 3: Steps vs bits(N) for BASE vs EXP_RED_O0 (P248 & P500)
    if nvar is not None:
        fig, axs = plt.subplots(1, 2, figsize=(11, 4.2))
        for ax, s in zip(axs, ('p248', 'p500')):
            gv = nvar[(nvar.set == s) & (nvar.exb_lovasz >= 0)]
            if len(gv) == 0:
                continue
            g_b = gv.groupby('N_bits')['exb_lovasz'].mean()
            g_e = gv.groupby('N_bits')['exe_lovasz'].mean()
            ax.plot(g_b.index, g_b.values, 'o-', ms=3, label='Exact LLL (BASE HNF)')
            ax.plot(g_e.index, g_e.values, 's-', ms=3, label='Exact LLL (EXP_RED)')
            ax.set_xlabel('bits(N)')
            ax.set_ylabel('Mean exact LLL steps')
            ax.set_title(f'{s.upper()}: Exact LLL steps vs bits(N)')
            ax.legend(fontsize=8)
        fig.tight_layout()
        fig.savefig(PLOTS / 'steps_vs_Nbits.png', dpi=120)
        plt.close(fig)

        # Plot 4: Z[i] CDF at bits(N) = B (nbound)
        fig, axs = plt.subplots(1, 2, figsize=(11, 4.2))
        for ax, s in zip(axs, ('p248', 'p500')):
            gb = nbnd[nbnd.set == s]
            if len(gb) == 0:
                continue
            for col, lbl in [('zb_red', 'Z[i] BASE (HNF)'), ('ze_red', 'Z[i] EXP_RED')]:
                vals = np.sort(gb[col].to_numpy())
                y = np.arange(1, len(vals) + 1) / len(vals)
                ax.plot(vals, y, label=f"{lbl} (max={vals[-1]})")
            ax.set_xlabel('Z[i]-Lagrange steps to reduced fixed point')
            ax.set_ylabel('CDF (bits(N) = B)')
            ax.set_title(f'{s.upper()}: Z[i] worst-case step distribution')
            ax.legend(fontsize=8)
        fig.tight_layout()
        fig.savefig(PLOTS / 'zi_cdf_nbound.png', dpi=120)
        plt.close(fig)

    summary = dict(
        key_sizes=ks_rows,
        setup=setup_rows,
        reduction=red_rows,
        combined=comb_rows,
        dudect=dud_rows,
    )
    (OUT / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print("=== KEY SIZES ===")
    print(ks_df.to_string(index=False))
    print("\n=== SETUP ===")
    print(setup_df.to_string(index=False))
    print("\n=== REDUCTION ===")
    print(red_df.to_string(index=False))
    print("\n=== COMBINED SAVINGS ===")
    print(comb_df.to_string(index=False))
    print("\n=== DUDECT ===")
    print(dud_df.to_string(index=False))


if __name__ == '__main__':
    main()
