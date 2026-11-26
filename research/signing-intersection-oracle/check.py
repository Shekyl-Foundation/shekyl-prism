"""Compare the prototype's admission + signing intersection with the oracle.

usage: python check.py SET [RUST_DUMP_BINARY] [corpus-file]
The Rust binary reads rows on stdin and prints one verdict line per row (a local,
uncommitted extraction harness around hazmat::ideal::{GenericHnf, GuardedIdeal}).
"""
import subprocess, sys, json
from oracle import *
from gen_corpus import SETS, params, is_prime

LIMBS = {"p248": 11, "p324": 14, "p376": 16, "p500": 21, "p664": 27}

def unhex(t):
    return int(t, 16)

def parse_row(line):
    f = line.split()
    name = f[0]
    n, xn, yn = (unhex(t) for t in f[1:4])
    u = tuple(unhex(t) for t in f[4:8])
    v = tuple(unhex(t) for t in f[8:12])
    s = unhex(f[12])
    xq, yq = unhex(f[13]), unhex(f[14])
    return name, n, (xn, yn), u, v, s, (xq, yq)

def oracle_admit(p, B, N, root, u, v):
    """Mathematical admission of the stored pre-reduced basis, independent of the prototype."""
    if not (0 < N < (1 << B) and N % 2 == 1):
        return False, "norm range/parity"
    cols = generic_hnf_columns(N, root)
    if not is_canonical_hnf_shape(cols):
        return False, "root not canonical (parity/range)"
    S = gaussian_span(u, v)
    if len(S) != 4:
        return False, "rank"
    if lat_index(S) != 4 * N * N:
        return False, "index"
    if hnf_rows(cols) != S:
        return False, "lattice != ideal(norm, root)"
    if not is_left_O0_ideal(p, S):
        return False, "not a left O0 ideal"
    A, C = N4(p, u), N4(p, v)
    if not (0 < A and not C < A):
        return False, "order"
    if C * C > p * (1024 * N) ** 2:
        return False, "guard"
    return True, ""

def oracle_challenge(p, e, s, rq):
    """Public checks on the challenge scalar and degree: returns (q or None, reason)."""
    if s <= 0 or s % 2 == 0:
        return None, "scalar not positive odd"
    q = (1 << (2 * e - 5)) - s * s
    if q <= 0:
        return None, "s^2 too large"
    if not is_prime(q):
        return None, "degree not prime"
    if q.bit_length() != 2 * e - 5:
        return None, "degree bit length"
    return q, ""

def run_rust(binary, rows):
    inp = "\n".join(rows) + "\n"
    out = subprocess.run([binary], input=inp, capture_output=True, text=True, check=True).stdout
    return out.strip().splitlines()

def parse_out(line):
    t = line.split()
    d = {"admit": t[1] == "admit=1"}
    if not d["admit"]:
        return d
    d["inter"] = t[2].split("=")[1]
    if d["inter"] in ("ok", "marked"):
        vals = t[3:]
        assert len(vals) == 9
        d["vals"] = vals
    return d

def check_set(name, binary, corpus=None, quiet=False):
    f, e, p, L, B = params(name)
    corpus = corpus or f"corpus/{name}.tsv"
    items = [l.rstrip("\n").split("\t") for l in open(corpus) if l.strip()]
    rows = [r for _, r in items]
    outs = run_rust(binary, rows)
    assert len(outs) == len(rows), (len(outs), len(rows))
    report = []
    mism = []
    for (case, rowtxt), outl in zip(items, outs):
        _, N, rs, u, v, s, rq = parse_row(rowtxt)
        rust = parse_out(outl)
        adm, why = oracle_admit(p, B, N, rs, u, v)
        rec = {"case": case, "oracle_admit": adm, "oracle_admit_reason": why, "rust_admit": rust["admit"]}
        ok = True
        if adm != rust["admit"]:
            ok = False
            rec["problem"] = f"admit mismatch (oracle {adm}: {why})"
        elif adm:
            q, qwhy = oracle_challenge(p, e, s, rq)
            if q is None:
                exp = "none"
            else:
                exp = "ok" if spec_intersection(p, N, u, v, q, rq, fermat=True) is not None else "marked"
            rec["oracle_inter"] = exp
            rec["rust_inter"] = rust["inter"]
            if exp != rust["inter"]:
                ok = False
                rec["problem"] = f"intersection status mismatch (oracle {exp} {qwhy})"
            elif exp == "ok":
                vals = [unhex(x) for x in rust["vals"]]
                U, V, T = tuple(vals[0:4]), tuple(vals[4:8]), vals[8]
                sU, sV, sT = spec_intersection(p, N, u, v, q, rq, fermat=True)
                rec["exact_vectors_equal_spec"] = (U, V, T) == (sU, sV, sT)
                if not rec["exact_vectors_equal_spec"]:
                    ok = False; rec["problem"] = "Rust differs from independent spec re-implementation"
                # general constructions, built from ideals by left closure (no shape assumed)
                G = general_constructions(p, N, rs, q, rq, Hs=hnf_rows(generic_hnf_columns(N, rs)))
                S = gaussian_span(U, V)
                rec["routes"] = {k: (S == G[k]) for k in ("sage", "pari_dual", "kernel", "q3")}
                rec["routes_agree_each_other"] = len({G[k] for k in ("sage", "pari_dual", "kernel", "q3")}) == 1
                # generic-HNF constructor with the CRT root (the 'general coprime ideal' constructor)
                T2 = N * q
                rc = crt_root(N, rs, q, rq)  # reduces both roots mod their norms first
                cols = generic_hnf_columns(T2, rc)
                rec["crt_root_hnf_shape"] = is_canonical_hnf_shape(cols)
                rec["crt_generic_lattice_equal"] = (hnf_rows(cols) == S)
                rec["T_equals_Nq"] = (T == T2)
                rec["index_ok"] = (lat_index(S) == 4 * T2 * T2)
                rec["left_ideal"] = is_left_O0_ideal(p, S)
                g = norm_gram(p, T2, U, V)
                rec["gram_exact"] = g is not None
                rec["gram_det_p"] = g is not None and gram_det(p, g) == p
                gg = generic_gram(p, T2, rc)
                rec["generic_gram_exact"] = gg is not None
                rec["generic_gram_det_p"] = gg is not None and gram_det(p, gg) == p
                if g is not None and gg is not None:
                    ru_, rv_, st1, sw1 = lagrange(p, U, V)
                    r2u, r2v, st2, sw2 = lagrange(p, (2 * T2, 0, 0, 0), (rc[0], rc[1], 1, 0))
                    g1, g2 = norm_gram(p, T2, ru_, rv_), norm_gram(p, T2, r2u, r2v)
                    rec["reduced_A_C_equal"] = (g1[0], g1[1]) == (g2[0], g2[1])
                    rec["reduced_gram_spec"] = list(g1)
                    rec["swaps_spec_vs_generic"] = [sw1, sw2]
                    rec["steps_spec_vs_generic"] = [st1, st2]
                else:
                    rec["reduced_A_C_equal"] = False
                bits = max(abs(x).bit_length() for x in U + V + (T,))
                rec["max_output_bits"] = bits
                rec["width_bits"] = 64 * LIMBS[name]
                needed = ["T_equals_Nq", "index_ok", "left_ideal", "gram_exact", "gram_det_p",
                          "generic_gram_exact", "generic_gram_det_p", "reduced_A_C_equal",
                          "routes_agree_each_other", "crt_generic_lattice_equal", "crt_root_hnf_shape"]
                if not (all(rec["routes"].values()) and all(rec[k] for k in needed)):
                    ok = False; rec["problem"] = "lattice/Gram check failed"
            elif exp == "marked":
                rec["all_marked"] = all(x == "MARKED" for x in rust["vals"])
                if not rec["all_marked"]:
                    ok = False; rec["problem"] = "partial mark"
        rec["ok"] = ok
        report.append(rec)
        if not ok:
            mism.append(rec)
    if not quiet:
        n_ok = sum(1 for r in report if r["ok"])
        print(f"{name}: {n_ok}/{len(report)} rows agree", flush=True)
        for m in mism:
            print("  MISMATCH", json.dumps(m)[:400])
    return report, mism

if __name__ == "__main__":
    name = sys.argv[1]
    binary = sys.argv[2] if len(sys.argv) > 2 else "/workspace/wt-pr47/target/release/examples/sio_dump"
    corpus = sys.argv[3] if len(sys.argv) > 3 else None
    rep, mism = check_set(name, binary, corpus)
    with open(f"results/check_{name}.json", "w") as fh:
        json.dump(rep, fh, indent=1)
