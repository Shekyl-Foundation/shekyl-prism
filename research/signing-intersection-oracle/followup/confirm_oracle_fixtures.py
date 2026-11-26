"""Run the prototype (origin/dev 470d537, local harness) on the oracle-only fixtures and compare.
For `ok` rows: Rust's carried norm equals the fixture T, and the canonical column HNF (PARI mathnf) of the lattice
spanned by Rust's U, JU, V, JV equals the fixture matrix entry for entry.  Verdicts must match as well:
reject-admission <-> admit=0, challenge-refused <-> inter=none, marked <-> all nine outputs marked."""
import sys, json
from common import *
import check

BIN = sys.argv[1] if len(sys.argv) > 1 else "/workspace/wt-dev2/target/release/examples/sio_dump"
summary = {}
allok = True
for name in SETS:
    items = [l.rstrip("\n").split("\t") for l in open(f"fixtures_oracle/{name}.tsv") if l.strip()]
    rows = [name + " " + it[2] for it in items]
    outs = check.run_rust(BIN, rows)
    res = []
    for it, outl in zip(items, outs):
        case, verdict = it[0], it[1]
        rust = check.parse_out(outl)
        if not rust["admit"]:
            got = "reject-admission"
        elif rust["inter"] == "none":
            got = "challenge-refused"
        elif rust["inter"] == "marked":
            got = "marked" if all(x == "MARKED" for x in rust["vals"]) else "partial-mark"
        else:
            got = "ok"
        rec = {"case": case, "expected": verdict, "rust": got}
        good = got == verdict
        if verdict == "ok" and got == "ok":
            vals = [int(x, 16) for x in rust["vals"]]
            U, V, T = tuple(vals[0:4]), tuple(vals[4:8]), vals[8]
            exp_T = int(it[3], 16)
            exp_M = [[int(x, 16) for x in it[4].split()[4 * i:4 * i + 4]] for i in range(4)]
            M = col_hnf([U, J(U), V, J(V)])
            rec["T_equal"] = (T == exp_T)
            rec["hnf_equal"] = (M == exp_M)
            good = good and rec["T_equal"] and rec["hnf_equal"]
        rec["agree"] = good
        res.append(rec)
        allok &= good
    n = sum(r["agree"] for r in res)
    summary[name] = {"rows": len(res), "agree": n, "ok_rows": sum(1 for r in res if r["expected"] == "ok")}
    print(name, summary[name], [r["case"] for r in res if not r["agree"]], flush=True)
json.dump({"summary": summary, "rows": None}, open("results_confirm.json", "w"), indent=1)
print("ALL AGREE" if allok else "MISMATCH")
