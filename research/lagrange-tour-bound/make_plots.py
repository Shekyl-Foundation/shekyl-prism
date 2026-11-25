#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

import csv
import json
import os
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HERE, "results")
PLOTS = os.path.join(HERE, "plots")

with open(os.path.join(RES, "cert_summary.json")) as f:
    cert = json.load(f)

sets_info = cert["sets"]
names = [s["set"].upper() for s in sets_info]
T_emp = [s["T_emp"] for s in sets_info]
T_prov = [s["T_prov"] for s in sets_info]
rol_asymp = [s["rolletschek_asymp"] for s in sets_info]
lll_avg = [s["lll_avg_steps"] for s in sets_info]

obs_by_set = {s["set"]: [] for s in sets_info}
with open(os.path.join(RES, "observed_dist.csv")) as f:
    for r in csv.DictReader(f):
        obs_by_set[r["set"]].append(int(r["steps"]))

adv_rol = {}
with open(os.path.join(RES, "adversarial.csv")) as f:
    for r in csv.DictReader(f):
        if r["family"] == "rolletschek_conj_alt":
            adv_rol[r["set"]] = int(r["steps"])
rol_obs = [adv_rol[s["set"]] for s in sets_info]

lehmer_128_24 = {}
with open(os.path.join(RES, "lehmer_study.csv")) as f:
    for r in csv.DictReader(f):
        if int(r["W"]) == 128 and int(r["m_inner"]) == 24:
            lehmer_128_24[r["set"]] = r

# Plot 1: Bounds vs Adversarial vs Observed vs Exact LLL
fig, ax = plt.subplots(figsize=(9.5, 5.4), dpi=150)
x = np.arange(len(names))
ax.plot(x, lll_avg, "o--", color="#6c757d", lw=1.8, label="Exact LLL baseline (avg steps)")
ax.plot(x, T_prov, "s-", color="#d62728", lw=2.4, ms=7, label=r"Provable $Z[i]$-Lagrange bound $T_{\mathrm{prov}}$ (GMP-certified)")
ax.plot(x, rol_obs, "^-", color="#ff7f0e", lw=2.0, ms=7, label=r"Explicit Rolletschek adversary ($\log_{2+\sqrt{3}}$ chain)")
ax.plot(x, T_emp, "D-", color="#1f77b4", lw=2.0, ms=6, label=r"Empirical max schedule $T_{\mathrm{emp}}$ (baseline)")

bp = ax.boxplot([obs_by_set[s["set"]] for s in sets_info], positions=x, widths=0.28,
                patch_artist=True, manage_ticks=False,
                boxprops=dict(facecolor="#a6cee3", color="#1f77b4", alpha=0.8),
                medianprops=dict(color="#08306b", lw=1.5),
                whiskerprops=dict(color="#1f77b4"),
                capprops=dict(color="#1f77b4"),
                flierprops=dict(marker=".", markersize=4, color="#1f77b4", alpha=0.5))

for i in range(len(names)):
    ax.annotate(f"{T_prov[i]}", (x[i], T_prov[i]), textcoords="offset points", xytext=(0, 7),
                ha="center", fontsize=8.5, fontweight="bold", color="#d62728")
    ax.annotate(f"{rol_obs[i]}", (x[i], rol_obs[i]), textcoords="offset points", xytext=(14, -4),
                ha="left", fontsize=8, color="#cc5500")
    ax.annotate(f"{T_emp[i]}", (x[i], T_emp[i]), textcoords="offset points", xytext=(-14, 4),
                ha="right", fontsize=8, color="#1f77b4")

ax.set_xticks(x)
ax.set_xticklabels([f"{n}\n({s['T_max_bits']}b)" for n, s in zip(names, sets_info)])
ax.set_ylabel("Steps / Tours")
ax.set_title(r"$\mathbb{Z}[i]$-Lagrange Tour Counts: Provable Bound vs. Rolletschek Adversary vs. Observed Ideals")
ax.grid(axis="y", alpha=0.3)
ax.legend(loc="upper left", fontsize=8.5, frameon=True)
fig.tight_layout()
fig.savefig(os.path.join(PLOTS, "bounds_vs_observed.png"))
plt.close(fig)

# Plot 2: Scaling law (steps per excess bit above sqrt(p)) & P248/P664 distributions
fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(11, 4.4), dpi=150)

excess_bits = [s["T_max_bits"] - 0.5 * s["L_bits_p"] for s in sets_info]
obs_mean = [np.mean(obs_by_set[s["set"]]) for s in sets_info]
ax1.plot(excess_bits, T_prov, "s-", color="#d62728", lw=2.0, label=r"$T_{\mathrm{prov}}$ (slope $\approx 0.576$ steps/bit)")
ax1.plot(excess_bits, rol_obs, "^-", color="#ff7f0e", lw=2.0, label=r"Rolletschek chain ($1/\log_2(2+\sqrt{3}) \approx 0.526$)")
ax1.plot(excess_bits, T_emp, "D--", color="#1f77b4", lw=1.6, label=r"$T_{\mathrm{emp}}$ (observed max)")
ax1.plot(excess_bits, obs_mean, "o:", color="#2ca02c", lw=1.8, label=r"Random PRISM ideals mean ($\approx 0.320$)")
ax1.set_xlabel(r"Effective Reduction Span $\log_2(T_{\max}/\sqrt{p})$ (bits)")
ax1.set_ylabel("Reduction Steps")
ax1.set_title("Asymptotic Step-per-Bit Scaling")
ax1.grid(alpha=0.3)
ax1.legend(fontsize=8)

# Histogram for P664
p664_vals = obs_by_set["p664"]
ax2.hist(p664_vals, bins=18, color="#4c78a8", alpha=0.85, edgecolor="white", label="300 PRISM ideals (P664)")
ax2.axvline(465, color="#1f77b4", ls="--", lw=2, label=r"$T_{\mathrm{emp}} = 465$")
ax2.axvline(714, color="#ff7f0e", ls="-.", lw=2, label=r"Rolletschek = 714")
ax2.axvline(773, color="#d62728", ls="-", lw=2.2, label=r"$T_{\mathrm{prov}} = 773$")
ax2.set_xlabel("Exact Fixed-Point Steps (P664)")
ax2.set_ylabel("Count")
ax2.set_title("P664 Observed Distribution vs. Worst-Case Bounds")
ax2.grid(axis="y", alpha=0.3)
ax2.legend(fontsize=8)
fig.tight_layout()
fig.savefig(os.path.join(PLOTS, "scaling_and_p664_hist.png"))
plt.close(fig)

# Plot 3: Relative Arithmetic Cost vs Exact LLL (normalized)
fig, ax = plt.subplots(figsize=(9.0, 4.6), dpi=150)
# Baseline fsr.py reported zi_gauss_fixed(T_emp) cost vs Exact LLL = 0.29x..0.28x (mean 0.29x).
# Scale T_prov cost = 0.29 * (T_prov / T_emp), and Lehmer (W=128,m=24) cost = 0.29 * work_ratio_vs_exact_emp.
emp_lll_ratio = [0.30, 0.29, 0.30, 0.29, 0.28]
prov_lll_ratio = [round(emp_lll_ratio[i] * T_prov[i] / T_emp[i], 3) for i in range(5)]
leh_lll_ratio = [round(emp_lll_ratio[i] * float(lehmer_128_24[s["set"]]["work_ratio_vs_exact_emp"]), 4) for i, s in enumerate(sets_info)]

w = 0.24
ax.axhline(1.0, color="#6c757d", ls="--", lw=1.4, label="Exact LLL baseline (1.00x)")
ax.bar(x - w, prov_lll_ratio, width=w, color="#d62728", label=r"Unbatched $T_{\mathrm{prov}}$ (0.45x-0.48x LLL)")
ax.bar(x, emp_lll_ratio, width=w, color="#1f77b4", label=r"Unbatched $T_{\mathrm{emp}}$ (0.28x-0.30x LLL)")
ax.bar(x + w, leh_lll_ratio, width=w, color="#2ca02c", label=r"Lehmer $W=128, m=24$ (0.012x-0.015x LLL)")

for i in range(5):
    ax.text(x[i] - w, prov_lll_ratio[i] + 0.015, f"{prov_lll_ratio[i]:.2f}x", ha="center", fontsize=8)
    ax.text(x[i], emp_lll_ratio[i] + 0.015, f"{emp_lll_ratio[i]:.2f}x", ha="center", fontsize=8)
    ax.text(x[i] + w, leh_lll_ratio[i] + 0.015, f"{leh_lll_ratio[i]:.3f}x", ha="center", fontsize=8)

ax.set_xticks(x)
ax.set_xticklabels(names)
ax.set_ylim(0, 1.15)
ax.set_ylabel("Arithmetic Cost Relative to Exact LLL")
ax.set_title("Constant-Time Schedule Cost vs. Exact LLL (Unbatched Proven, Empirical, and Lehmer Batching)")
ax.grid(axis="y", alpha=0.3)
ax.legend(loc="upper right", fontsize=8.5)
fig.tight_layout()
fig.savefig(os.path.join(PLOTS, "cost_comparison.png"))
plt.close(fig)
print("Saved 3 plots to", PLOTS)
