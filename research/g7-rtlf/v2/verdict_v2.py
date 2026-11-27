#!/usr/bin/env python3
"""Apply RULE-v2 to extracted v2 contrasts and RTLF results.

  verdict_v2.py WORKDIR OUTDIR RUN [RERUN]

WORKDIR/RUN/manifest.json (extract_v2.py) and WORKDIR/RUN_a01.tsv, RUN_a09.tsv
(rtlf_batch.R). RERUN is an optional second run label (the 4x re-run).
Writes OUTDIR/verdict_<RUN>.md and .json.
"""
import collections, csv, json, os, sys
W, O, RUN = sys.argv[1], sys.argv[2], sys.argv[3]
RERUN = sys.argv[4] if len(sys.argv) > 4 else None
TICK = 18.5
TT = 4.5
os.makedirs(O, exist_ok=True)

def tsv(p):
    return {r["file"]: r for r in csv.DictReader(open(p), delimiter="\t")} if os.path.exists(p) else {}

def load(run):
    man = json.load(open("%s/%s/manifest.json" % (W, run)))
    a01 = tsv("%s/%s_a01.tsv" % (W, run)); a09 = tsv("%s/%s_a09.tsv" % (W, run))
    for r in man:
        a, b = a01.get(r["file"]), a09.get(r["file"])
        r["R01"] = int(a["detect"]) if a else None
        r["R09"] = int(b["detect"]) if b else None
        r["dec01"] = a["sig_deciles"] if a else ""
        r["maxdiff"] = max(float(x) for x in a["diff"].split(";")) if a else None
        r["T"] = int(abs(r["t_welch"]) > TT)
        r["sign"] = 1 if r["median_diff_ns"] > 0 else -1
        r["flag"] = int(bool(r["R01"]) and (r["maxdiff"] or 0) >= 3 * TICK) or r["T"]
        r["flagR"] = int(bool(r["R01"]) and (r["maxdiff"] or 0) >= 3 * TICK)
    return man

def block(man, label):
    L = []
    procs = sorted({r["src"] for r in man})
    # validity: a process is void if any null arm (whole run) flags by either test
    void = {}
    for p in procs:
        nulls = [r for r in man if r["src"] == p and r["arm"] == "null" and r["span"] == "all"]
        void[p] = [r["phase"] for r in nulls if r["flag"]]
    L.append("### %s: process validity\n" % label)
    L.append("| process | seed | reference | perm | null arm flagged in | valid |")
    L.append("|---|---|---|---|---|---|")
    for p in procs:
        m = next(r for r in man if r["src"] == p)
        L.append("| %s | %s | %s | %s | %s | %s |" % (p, m["seed"], m["ref"], m["perm"], ",".join(void[p]) or "none", "no" if void[p] else "yes"))
    valid = [p for p in procs if not void[p]]
    # resolution from the same-session control
    L.append("\n### %s: same-session injected-delay control (delay nominal, whole-run RTLF α=.01 / Welch t)\n" % label)
    L.append("| process | phase | arm | n | mean shift ns | median shift ns | Welch t | RTLF .01 | flagged |")
    L.append("|---|---|---|---|---|---|---|---|---|")
    res = {}
    for ph in ("adm", "herm", "full"):
        rr = []
        for arm in ("injL", "injM", "injH"):
            ok = []
            for p in valid:
                x = next(r for r in man if r["src"] == p and r["phase"] == ph and r["arm"] == arm and r["span"] == "all")
                ok.append(bool(x["R01"]))
                L.append("| %s | %s | %s | %d | %+.0f | %+.0f | %+.2f | %s | %s |" % (p, ph, arm, x["n"], x["mean_diff_ns"], x["median_diff_ns"], x["t_welch"], x["R01"], x["flagR"]))
            rr.append((arm, all(ok) and bool(ok)))
        res[ph] = next((a for a, d in rr if d), None)
    L.append("\nSmallest control level detected by RTLF in every valid process: %s\n" % ", ".join("%s: %s" % (k, v or "none") for k, v in res.items()))
    # chunk detection rates for the floor
    L.append("### %s: control detection per chunk (n = 4096 rounds per chunk)\n" % label)
    L.append("| phase | arm | chunks | RTLF .01 | RTLF .09 | Welch t > 4.5 |")
    L.append("|---|---|---|---|---|---|")
    for ph in ("adm", "herm", "full"):
        for arm in ("null", "injL", "injM", "injH"):
            ch = [r for r in man if r["phase"] == ph and r["arm"] == arm and r["span"] != "all" and r["src"] in valid]
            if ch:
                L.append("| %s | %s | %d | %d | %d | %d |" % (ph, arm, len(ch), sum(1 for r in ch if r["R01"]), sum(1 for r in ch if r["R09"]), sum(r["T"] for r in ch)))
    # contrasts
    L.append("\n### %s: fixture and key contrasts per process\n" % label)
    L.append("| phase | contrast | " + " | ".join("%s" % p.split("_")[-1] for p in valid) + " | processes flagged (same sign) | verdict |")
    L.append("|---|---|" + "---|" * len(valid) + "---|---|")
    verdicts = {}
    for ph in ("adm", "herm", "full"):
        keys = sorted({(r["arm"], r["versus"]) for r in man if r["phase"] == ph and r["kind"] in ("fixture", "keys")})
        for arm, vs in keys:
            cells = []; flags = []
            for p in valid:
                rr = [r for r in man if r["src"] == p and r["phase"] == ph and r["arm"] == arm and r["versus"] == vs and r["span"] == "all"]
                if not rr:
                    cells.append("–"); continue
                x = rr[0]
                cells.append("%s%+.0f ns t%+.1f%s" % ("**" if x["flag"] else "", x["median_diff_ns"], x["t_welch"], "**" if x["flag"] else ""))
                if x["flag"]:
                    flags.append((p, x["sign"], x["flagR"], x["T"]))
            nf = len(flags); same = nf >= 2 and len({s for _, s, _, _ in flags}) == 1
            both = any(fr and t for _, _, fr, t in flags)
            if nf == 0: v = "no flag"
            elif same and (both or nf >= 2): v = "CANDIDATE"
            else: v = "NOISE"
            verdicts[(ph, arm, vs)] = (nf, same, both, v)
            L.append("| %s | %s vs %s | %s | %d (%s) | %s |" % (ph, arm, vs, " | ".join(cells), nf, "same sign" if same else "mixed/none", v))
    return L, valid, res, verdicts

man = load(RUN)
L, valid, res, v1 = block(man, RUN)
out = ["# RULE-v2 application: %s\n" % RUN] + L
if RERUN:
    man2 = load(RERUN)
    L2, valid2, res2, v2 = block(man2, RERUN)
    out += ["\n# Re-run: %s\n" % RERUN] + L2
    out.append("\n### persistence (first run candidate, re-run same sign, null clean)\n")
    out.append("| phase | contrast | first run | re-run | persistent |")
    out.append("|---|---|---|---|---|")
    for k, a in sorted(v1.items()):
        b = v2.get(k)
        if a[3] == "CANDIDATE":
            pers = b is not None and b[3] == "CANDIDATE"
            out.append("| %s | %s vs %s | %s | %s | %s |" % (k[0], k[1], k[2], a[3], b[3] if b else "-", "**LEAK (until explained)**" if pers else "no"))
open("%s/verdict_%s.md" % (O, RUN), "w").write("\n".join(out) + "\n")
print("\n".join(out))
