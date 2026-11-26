"""Write committed-ready fixtures: fixtures/<set>.tsv from the structured corpus, with expected values
that the Rust prototype (merged origin/dev code, extraction harness) and the oracle both produce, and
fixtures/toy_p3.txt for the p = 3, T = 5 case and a small hand-checkable intersection.

fixtures/<set>.tsv columns:
  case | verdict | input | U0 U1 U2 U3 V0 V1 V2 V3 T | normalized Gram A' C' h0' h1' | reduced A' C'
verdict is one of: reject-admission, challenge-refused, marked, ok.
input = `n xn yn u0..u3 v0..v3 s xq yq` (hex, '-' sign), as the prototype's ideals/<set>.txt plus s xq yq.
"""
import os, hashlib, sys
from oracle import *
from gen_corpus import SETS, params
import check

BIN = "/workspace/wt-pr47/target/release/examples/sio_dump"
os.makedirs("fixtures", exist_ok=True)
manifest = []
for name in SETS:
    f, e, p, L, B = params(name)
    items = [l.rstrip("\n").split("\t") for l in open(f"corpus/{name}.tsv") if l.strip()]
    outs = check.run_rust(BIN, [r for _, r in items])
    lines = []
    for (case, rowtxt), outl in zip(items, outs):
        _, N, rs, u, v, s, rq = check.parse_row(rowtxt)
        rust = check.parse_out(outl)
        inp = rowtxt.split(" ", 1)[1]
        if not rust["admit"]:
            lines.append("\t".join([case, "reject-admission", inp, "-", "-", "-"]))
        elif rust["inter"] == "none":
            lines.append("\t".join([case, "challenge-refused", inp, "-", "-", "-"]))
        elif rust["inter"] == "marked":
            lines.append("\t".join([case, "marked", inp, "-", "-", "-"]))
        else:
            vals = [int(x, 16) for x in rust["vals"]]
            U, V, T = tuple(vals[:4]), tuple(vals[4:8]), vals[8]
            g = norm_gram(p, T, U, V)
            ru, rv, _, _ = lagrange(p, U, V)
            gr = norm_gram(p, T, ru, rv)
            hx = lambda x: ("-" if x < 0 else "") + format(abs(x), "x")
            lines.append("\t".join([case, "ok", inp, " ".join(hx(x) for x in (*U, *V, T)),
                                    " ".join(hx(x) for x in g), f"{hx(gr[0])} {hx(gr[1])}"]))
    path = f"fixtures/{name}.tsv"
    open(path, "w").write("\n".join(lines) + "\n")
    manifest.append(f"{hashlib.sha256(open(path,'rb').read()).hexdigest()}  {path}  ({len(lines)} rows)")

# ---- toy p = 3 ----
p = 3
tl = []
T = 5; root = (4, 9)
u, v = (10, 0, 0, 0), (4, 9, 1, 0)
tl.append(f"p=3 T=5 generic: U={u} V={v}")
tl.append(f"  normalized Gram (2T,(x^2+y^2+p)/2T,x,y) = {generic_gram(p, T, root)}  det = {gram_det(p, generic_gram(p, T, root))}")
ru, rv, steps, swaps = lagrange(p, u, v)
tl.append(f"  reduced: u={ru} v={rv} swaps={swaps} active-steps-incl-final={steps}  normalized Gram = {norm_gram(p, T, ru, rv)}")
# hand-checkable intersection: N = 5 (root (4,9)), q = 7 with a root of x^2+y^2+3 = 0 mod 7
N, q = 5, 7
rq = None
for x in range(0, 14, 2):
    for y in range(1, 14, 2):
        if (x * x + y * y + p) % q == 0:
            rq = (x, y); break
    if rq: break
Hs = ideal_root(p, N, root)
u0, v0 = gaussian_basis_of(Hs)
ur, vr, _, _ = lagrange(p, u0, v0)
tl.append(f"p=3 N=5 root=(4,9) Gaussian basis u={ur} v={vr}; q=7 root={rq}")
out = spec_intersection(p, N, ur, vr, q, rq)
G = general_constructions(p, N, root, q, rq, Hs=Hs)
S = gaussian_span(out[0], out[1])
tl.append(f"  specialised: U={out[0]} V={out[1]} T={out[2]}")
tl.append(f"  equals sage={S==G['sage']} pari_dual={S==G['pari_dual']} kernel={S==G['kernel']} q3={S==G['q3']}")
tl.append(f"  canonical row-HNF of the intersection: {G['sage']}")
tl.append(f"  CRT root of norm 35: {crt_root(N, root, q, rq)}  GenericHnf(35) lattice equal: {hnf_rows(generic_hnf_columns(35, crt_root(N, root, q, rq))) == S}")
open("fixtures/toy_p3.txt", "w").write("\n".join(tl) + "\n")
manifest.append("toy_p3.txt written")
open("fixtures/MANIFEST.txt", "w").write("\n".join(manifest) + "\n")
print("\n".join(manifest)); print("\n".join(tl))
