"""Cross-check of the committed Stage 2c reducer fixtures (prototype vectors/*.txt, read from
the merged tree on origin/dev) and ideal corpora (ideals/*.txt) against the oracle.
Fixture rows: class t u0..u3 v0..v3 expected(12) = reduced u, reduced v, normalized form.
The oracle's own exact reduction must reproduce the fixed point, and for the guarded rows the
oracle's general intersection must equal the lattice of the fixture's (u,v)."""
import sys, json, subprocess
from oracle import *
from gen_corpus import params, SETS
import check

ROOT = sys.argv[1] if len(sys.argv) > 1 else "/workspace/wt-pr47"
BIN = sys.argv[2] if len(sys.argv) > 2 else ROOT + "/target/release/examples/sio_dump"

def val(t):
    return int(t, 16)

res = {}
for name in SETS:
    f, e, p, L, B = params(name)
    lines = open(f"{ROOT}/shekyl-prism/examples/stage2c/vectors/{name}.txt").read().split("\n")
    rows = [l.split() for l in lines if l.strip()]
    stats = {"rows": 0, "reduced_equal": 0, "form_equal": 0, "lattice_ideal": 0, "gram_exact": 0}
    detail = []
    for r in rows:
        cls = r[0]
        t = val(r[1]); u = tuple(val(x) for x in r[2:6]); v = tuple(val(x) for x in r[6:10])
        exp = [val(x) for x in r[10:22]]
        ru, rv, steps, swaps = lagrange(p, u, v)
        stats["rows"] += 1
        eu, ev, ef = tuple(exp[0:4]), tuple(exp[4:8]), tuple(exp[8:12])
        ok_red = (ru, rv) == (eu, ev)
        stats["reduced_equal"] += ok_red
        g = norm_gram(p, t, ru, rv)
        # prototype 'form': find which normalisation matches
        cand = {"A,C,h0,h1/2T": g}
        stats["gram_exact"] += g is not None
        form_eq = g is not None and g[0] % 2 == 0 and g[1] % 2 == 0 and (g[0] // 2, g[1] // 2, g[2], g[3]) == ef  # prototype form = (A'/2, C'/2, h0', h1')
        stats["form_equal"] += form_eq
        S = gaussian_span(u, v)
        ideal_ok = len(S) == 4 and is_left_O0_ideal(p, S) and lat_index(S) == 4 * t * t
        stats["lattice_ideal"] += ideal_ok
        detail.append({"class": cls, "steps_to_fixed_point": steps, "swaps": swaps, "reduced_equal": ok_red,
                       "form_equal": form_eq, "ideal_ok": ideal_ok})
    # ideal corpus through the prototype harness and the oracle checker
    ideals = [l for l in open(f"{ROOT}/shekyl-prism/examples/stage2c/ideals/{name}.txt").read().split("\n") if l.strip()]
    tmp = f"/tmp/ideals_{name}.tsv"
    with open(tmp, "w") as fh:
        for i, l in enumerate(ideals):
            fh.write(f"committed-ideal-{i}\t{name} {l}\n")
    rep, mism = check.check_set(name, BIN, tmp, quiet=True)
    stats["ideal_corpus_rows"] = len(rep); stats["ideal_corpus_agree"] = sum(1 for x in rep if x["ok"])
    res[name] = {"stats": stats, "detail": detail}
    print(name, stats, flush=True)
json.dump(res, open("results/fixtures_check.json", "w"), indent=1)
