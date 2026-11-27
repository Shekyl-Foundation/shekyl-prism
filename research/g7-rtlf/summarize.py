#!/usr/bin/env python3
"""Join the manifests with the RTLF batch outputs and write the report tables.

  summarize.py WORKDIR OUTDIR

T threshold: |t| > 4.5 (the dudect / TVLA line). RTLF: the pinned reference
at alpha = 0.09 (its default) and 0.01. Writes OUTDIR/*.json and prints
markdown tables.
"""
import collections, csv, glob, json, math, os, sys

W, O = sys.argv[1], sys.argv[2]
os.makedirs(O, exist_ok=True)
TT = 4.5

def load_tsv(paths):
    r = {}
    for p in paths:
        if not os.path.exists(p):
            continue
        for row in csv.DictReader(open(p), delimiter="\t"):
            r[row["file"]] = row
    return r

# R reference results take priority; the numpy port fills the rest.
R09 = load_tsv([W + "/np_new_a09.tsv", W + "/np_main_a09.tsv", W + "/main/rtlf_a09.tsv"])
R01 = load_tsv([W + "/np_new_a01.tsv", W + "/np_main_a01.tsv"])

def man(name):
    return json.load(open("%s/%s/manifest.json" % (W, name)))

def verdicts(r):
    a = R09.get(r["file"]); b = R01.get(r["file"])
    tp, tw = r.get("t_paired", float("nan")), r.get("t_welch", float("nan"))
    return dict(
        tp=tp, tw=tw,
        T_paired=int(abs(tp) > TT) if tp == tp else None, T_welch=int(abs(tw) > TT),
        rtlf09=int(a["detect"]) if a else None, rtlf09_deciles=a["sig_deciles"] if a else None,
        rtlf01=int(b["detect"]) if b else None)

def rate(rows, key):
    v = [r[key] for r in rows if r.get(key) is not None]
    return "%d/%d" % (sum(v), len(v)) if v else "-"

main = [r for r in man("main") if r["run"][0] not in "BF"] + man("bf")
main = [r for r in main if r["file"] in R09 or True]
for r in main:
    r.update(verdicts(r))
runG = man("runG")
for r in runG:
    r.update(verdicts(r))
xb = man("xbin")
for r in xb:
    r.update(verdicts(r))
pc = man("posctl")
for r in pc:
    r.update(verdicts(r))
rH = man("runH") if os.path.exists(W + "/runH/manifest.json") else []
for r in rH:
    r.update(verdicts(r))

out = []
P = out.append

def fmt(x):
    return "%+.2f" % x if isinstance(x, float) and x == x else "–"

def mark(r):
    s = "RTLF%s" % ("✱" if r["rtlf09"] else "·") if r["rtlf09"] is not None else "RTLF?"
    return s

# 1. G7 contrasts (orig) per run file
P("## G7 contrasts, original harness\n")
P("| run | file | class | fixture | n | paired t | Welch t | RTLF α=.09 | deciles | RTLF α=.01 |")
P("|---|---|---|---|---|---|---|---|---|---|")
for r in main:
    if r["kind"] == "orig" and r["run"] != "D_procs":
        P("| %s | %s | %s | %d | %d | %s | %s | %s | %s | %s |" % (r["run"], r["src"], r["cls"][:6], r["idx"], r["n"], fmt(r["tp"]), fmt(r["tw"]),
          r["rtlf09"], r["rtlf09_deciles"] or "", "" if r["rtlf01"] is None else r["rtlf01"]))

def agg(rows, label):
    return "| %s | %d | %s | %s | %s | %s |" % (label, len(rows), rate(rows, "T_paired"), rate(rows, "T_welch"), rate(rows, "rtlf09"), rate(rows, "rtlf01"))

P("\n## Detection counts (|t| > 4.5; RTLF)\n")
P("| set | contrasts | paired t | Welch t | RTLF α=.09 | RTLF α=.01 |")
P("|---|---|---|---|---|---|")
groups = collections.OrderedDict()
for r in main:
    if r["kind"] == "orig":
        g = "orig real fixtures, " + ("runD 48 procs" if r["run"] == "D_procs" else ("quiet E/F" if r["run"][0] in "EF" else "noisy A/B/C"))
    elif r["kind"] == "aa":
        g = "orig A/A (reference halves), " + ("runD" if r["run"] == "D_procs" else ("quiet" if r["run"][0] in "EF" else "noisy"))
    elif r["kind"] == "split":
        g = "split %s %s, %s" % ("NULL" if r["null"] else "real", r["phase"], "runD" if r["run"] == "D_procs" else ("quiet" if r["run"][0] == "E" else "noisy"))
    groups.setdefault(g, []).append(r)
for r in runG + rH:
    if r["run"].startswith("G_inject") or r["run"].startswith("H_inject"):
        continue
    g = "%s %s %s" % (r["run"][0], r["kind"], "NULL arm" if r["null"] else "real fixtures")
    groups.setdefault(g, []).append(r)
for g in sorted(groups):
    P(agg(groups[g], g))

# split per phase per fixture, the 10000-round files
P("\n## Split mode (10000 / 5000 rounds), per phase\n")
P("| run | file | class | phase | fixture | paired t | Welch t | RTLF .09 | deciles |")
P("|---|---|---|---|---|---|---|---|---|")
for r in main:
    if r["kind"] == "split" and r["run"] != "D_procs":
        P("| %s | %s | %s | %s | %s | %s | %s | %s | %s |" % (r["run"], r["src"], r["cls"][:6], r["phase"], "NULL" if r["null"] else r["idx"],
          fmt(r["tp"]), fmt(r["tw"]), r["rtlf09"], r["rtlf09_deciles"] or ""))

# matched runs
P("\n## Matched-preamble runs (runG, runH)\n")
P("| run | file | class | fixture | n | paired t | Welch t | RTLF .09 | deciles | RTLF .01 |")
P("|---|---|---|---|---|---|---|---|---|---|")
for r in runG + rH:
    if r["kind"] == "aa":
        continue
    P("| %s | %s | %s | %s | %d | %s | %s | %s | %s | %s |" % (r["run"], r["src"], r["cls"][:6], "NULL" if r["null"] else r["idx"], r["n"],
      fmt(r["tp"]), fmt(r["tw"]), r["rtlf09"], r["rtlf09_deciles"] or "", "" if r["rtlf01"] is None else r["rtlf01"]))

# cross-binary
P("\n## dev vs a5a6f60 directly (separate processes; dev vs dev is the control)\n")
P("| pair | class | fixture | n | Welch t | mean diff ns | RTLF .09 | deciles |")
P("|---|---|---|---|---|---|---|---|")
for r in xb:
    P("| %s | %s | %s | %d | %s | %+.0f | %s | %s |" % (r["cond"], r["cls"][:6], "ref" if r["idx"] == 0 else r["idx"], r["n"], fmt(r["tw"]), r["mean_diff_ns"], r["rtlf09"], r["rtlf09_deciles"] or ""))

# positive control
P("\n## Positive control (synthetic delay on the split null arm, guarded)\n")
P("| condition | phase | delay ns | chunks | paired t | Welch t | RTLF .09 | RTLF .01 |")
P("|---|---|---|---|---|---|---|---|")
pcg = collections.defaultdict(list)
for r in pc:
    cond = "noisy" if r["cond"].startswith("noisy") else "quiet"
    pcg[(cond, r["phase"], r["chunk"] == "whole", r["delay"])].append(r)
for (cond, ph, whole, d), rows in sorted(pcg.items()):
    if whole:
        continue
    P("| %s | %s | %d | %d | %s | %s | %s | %s |" % (cond, ph, d, len(rows), rate(rows, "T_paired"), rate(rows, "T_welch"), rate(rows, "rtlf09"), rate(rows, "rtlf01")))
P("\n### whole runs (10000 rounds)\n")
P("| file | phase | delay ns | paired t | Welch t | RTLF .09 | RTLF .01 |")
P("|---|---|---|---|---|---|---|")
for r in pc:
    if r["chunk"] == "whole":
        P("| %s | %s | %d | %s | %s | %s | %s |" % (r["cond"], r["phase"], r["delay"], fmt(r["tp"]), fmt(r["tw"]), r["rtlf09"], "" if r["rtlf01"] is None else r["rtlf01"]))

P("\n## On-Pi injected delay (matched harness, guarded null arm, n=512)\n")
P("| delay ns | paired t | Welch t | mean diff ns | RTLF .09 | deciles | RTLF .01 |")
P("|---|---|---|---|---|---|---|")
for r in runG + rH:
    if "inject" in r["run"] and r["null"] and r["cls"].startswith("prered") and r["kind"] == "matched":
        P("| %s | %s | %s | %+.0f | %s | %s | %s |" % (r["run"].split("_")[-1].lstrip("0"), fmt(r["tp"]), fmt(r["tw"]), r["mean_diff_ns"], r["rtlf09"], r["rtlf09_deciles"] or "", "" if r["rtlf01"] is None else r["rtlf01"]))

open(O + "/tables.md", "w").write("\n".join(out) + "\n")
json.dump(dict(main=main, runG=runG, runH=rH, xbin=xb, posctl=pc), open(O + "/verdicts.json", "w"), indent=0, default=str)
print("\n".join(out))
