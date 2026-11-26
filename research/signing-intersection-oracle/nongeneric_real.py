"""Non-generic secret ideals at the five real sets.

A left O0-ideal whose column-HNF is not (2N,0,0,0),(0,2N,0,0),(x,y,1,0),(2N-y,x,0,1) cannot be
written as GenericHnf(N, root); the prototype's admission has no conversion for it. Here:
  * O0*pi and O0*conj(pi) for a prime N = pi conj(pi) = 1 (mod 4)       (norm N, non-cyclic root structure)
  * O0*l for a prime l, norm l^2                                         (the 'scalar' ideal)
For each: (a) the oracle's Z[i]-basis from Gaussian elimination, (b) the specialised formula against
the four general routes at the real sizes (the mathematics does not need the generic shape),
(c) the prototype's admission verdict on that basis with a stand-in root: must be None.
"""
import json, random, subprocess, sys
from sage.all import two_squares
from oracle import *
from gen_corpus import *
import check

BIN = "/workspace/wt-pr47/target/release/examples/sio_dump"
res = {}
for name in SETS:
    f, e, p, L, B = params(name)
    rng = random.Random("ng-" + name)
    rows = []; meta = []
    while True:
        N = rand_prime_bits(rng, B - 1)
        if N % 4 == 1:
            break
    a, b = two_squares(N)
    cases = []
    for tag, pi in (("O0*pi", (a, b)), ("O0*conj(pi)", (a, -b))):
        H = ideal_from_gens(p, [(2 * pi[0], 2 * pi[1], 0, 0)])
        cases.append((tag, N, H))
    l = rand_prime_bits(rng, B // 2 - 1)
    cases.append(("O0*l", l * l, ideal_from_gens(p, [(2 * l, 0, 0, 0)])))
    s, q, rq = challenge(rng, p, e)
    for tag, Nn, H in cases:
        assert is_left_O0_ideal(p, H) and ideal_norm(H) == Nn
        cols_generic = len(H) == 4 and hnf_rows(generic_hnf_columns(Nn, (0, 1))) != H
        # is it generic at all?  is there (x,y,1,0) in H?  third HNF pivot (row-HNF layout) tells us
        # Reduced Gaussian basis for the real-size formula test
        u0, v0 = gaussian_basis_of(H)
        u, v, _, _ = lagrange(p, u0, v0)
        assert gaussian_span(u, v) == H
        # specialised formula at real size vs general routes (root of I_q from the corpus generator)
        out = spec_intersection(p, Nn, u, v, q, rq, fermat=True)
        entry = {"ideal": tag, "N": Nn, "guard_ok": guard_ok(p, Nn, v) and not N4(p, v) < N4(p, u)}
        if out is None:
            entry["spec"] = "marked"
        else:
            U, V, T = out
            G = general_constructions(p, Nn, None, q, rq, Hs=H)
            S = gaussian_span(U, V)
            entry["spec_equals_routes"] = {k: S == G[k] for k in ("sage", "pari_dual", "kernel", "q3")}
            entry["is_left_ideal"] = is_left_O0_ideal(p, S)
            entry["index_ok"] = lat_index(S) == 4 * (Nn * q) ** 2
        # prototype admission with a stand-in root
        row_ = row(name, Nn, (0, 1), u, v, s, rq)
        outl = check.run_rust(BIN, [row_])[0]
        entry["rust"] = outl[:60]
        res.setdefault(name, []).append(entry)
    print(name, json.dumps(res[name]), flush=True)
json.dump(res, open("results/nongeneric_real.json", "w"), indent=1)
