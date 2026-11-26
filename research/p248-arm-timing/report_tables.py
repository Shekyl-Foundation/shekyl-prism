#!/usr/bin/env python3
"""Markdown tables for REPORT.md from results/summary/*.json."""
import json, glob, os
S = os.path.join(os.path.dirname(__file__), "results", "summary")
def load(n): return json.load(open(os.path.join(S, n)))
print("### G7-mode contrasts (guarded class, other - reference, paired t, n = repeats)\n")
print("| Run | Build | Core | Repeats | fixture 1 | fixture 2 | fixture 3 | fixture 4 | fixture 5 | max abs t |")
print("|---|---|---|---:|---:|---:|---:|---:|---:|---:|")
runs = [("A1","runA_core3_dev_01_orig_512","dev 470d537+patch",3,512),("A2","runA_core3_dev_02_orig_512","dev 470d537+patch",3,512),
        ("A3","runA_core3_dev_03_orig_512","dev 470d537+patch",3,512),("A4","runA_core3_dev_04_orig_4096","dev 470d537+patch",3,4096),
        ("B1","runB_core3_a5a6f60_01_orig_512","a5a6f60 unmodified",3,512),("B2","runB_core3_a5a6f60_02_orig_512","a5a6f60 unmodified",3,512),
        ("C1","runC_core2_dev_01_orig_512","dev 470d537+patch",2,512)]
for name, f, build, core, rep in runs:
    d = load(f"orig_{f}.json")
    g = [d[f"p248 prered-guard256 reviewed other={i}"] for i in range(1, 6)]
    print(f"| {name} | {build} | {core} | {rep} | " + " | ".join(f"{x['t']:+.2f} ({x['mean_ns']:+.0f} ns)" for x in g) + f" | {max(abs(x['t']) for x in g):.2f} |")
print("\n### Phase split, randomized rounds (guarded class). Paired difference other - reference, per round\n")
print("Columns: winsorized-5% mean diff (ns) / blocked t (20 blocks) / median diff (ns); the last column is the ANOVA p across the six real fixtures (rounds as blocks).\n")
for run, n in [("A1+A2 (core 3, 20,000 rounds)", "split_A12.json"), ("C3 (core 2, 5,000 rounds)", "split_C3.json")]:
    d = load(n)
    print(f"**{run}**\n")
    print("| Phase | fixture 1 | fixture 2 | fixture 3 | fixture 4 | fixture 5 | NULL (reference copy) | ANOVA p |")
    print("|---|---|---|---|---|---|---|---:|")
    for ph in ["adm", "red", "full", "inv", "herm"]:
        cells = []
        for lab in ["fixture 1","fixture 2","fixture 3","fixture 4","fixture 5","NULL(ref copy)"]:
            v = d[f"p248 prered-guard256 {ph} {lab}"]
            cells.append(f"{v['mean_winsor5_ns']:+.0f} / {v['t_blocked20']:+.2f} / {v['median_diff_ns']:+.0f}")
        a = d[f"p248 prered-guard256 {ph} ANOVA"]
        print(f"| {ph} | " + " | ".join(cells) + f" | {a['p_real']:.2f} |")
    print()
