from sq import *
import random

def check_basis(p, N, u, v, tag="", orient=True, full_direct=True):
    """All checks for one Z[i]-basis (u,v) of a left O0 ideal of norm N.  Returns a record (dict of counters)
    and a list of failure strings.  Failures are *unexpected* (a counterexample or a bug)."""
    rec = dict(bases=1, orient=0, integral=0, unit=0, lattice_eq=0, ideal_out=0, hnf_shape_eq=0, gram_inv=0,
               even_out=0, A_even=0, C_even=0, admit_in=0, admit_out=0, out_even_only_fail=0, out_other_fail=0,
               guard_transfer=0, red_in=0, red_out=0, red_out_tie_only=0, second_col=0)
    fails = []
    ri = admit_report(p, N, u, v)
    assert ri["ideal"], (tag, "input is not an ideal basis", ri)
    rec["admit_in"] += ri["admissible"]
    A0 = N4(p, u); C0 = N4(p, v)
    rec["A_even"] += (A0 % (2*N) == 0 and (A0 // (2*N)) % 2 == 0)
    rec["C_even"] += (C0 % (2*N) == 0 and (C0 // (2*N)) % 2 == 0)
    rec["red_in"] += reduced_flag(p, u, v)
    okj, whyj = jidentities(p, N, u, v)
    rec["j_identities"] = rec.get("j_identities", 0) + okj
    if not okj: fails.append(f"{tag}: j-identities {whyj}")
    cases = [(eps, u, v, "u") for eps in (UNITS if orient else UNITS[:1])]
    cases.append(((1, 0), v, u, "v"))                    # second column as beta, roles swapped
    for eps, a, b, which in cases:
        s = shortcut(p, N, a, b, eps)
        if "fail" in s:
            fails.append(f"{tag} {which} eps={eps}: {s['fail']}")
            continue
        rec["orient"] += 1
        rec["integral"] += 1; rec["unit"] += 1
        beta = tuple(x * 0 for x in a)
        # beta = eps * a (left multiplication by the Gaussian unit eps on the numerator)
        beta = a
        for _ in range({(1,0):0,(0,1):1,(-1,0):2,(0,-1):3}[eps]):
            beta = J(beta)
        Np = s["Np"]
        Ld = direct(p, N, a, b, beta)
        if Ld is None:
            fails.append(f"{tag} {which} eps={eps}: direct product not divisible by N"); continue
        # the shortcut Z[i]-basis lattice
        Ls = gaussian_lattice([s["Up"], s["Vp"]])
        ok_eq = (Ld == Ls)
        rec["lattice_eq"] += ok_eq
        if not ok_eq: fails.append(f"{tag} {which} eps={eps}: lattice mismatch")
        ideal_ok = (len(Ld) == 4 and is_left_O0_ideal(p, list(Ld)) and lat_index(Ld) == 4 * Np * Np
                    and ideal_norm(Ld) == Np)
        rec["ideal_out"] += ideal_ok
        if not ideal_ok: fails.append(f"{tag} {which} eps={eps}: output not an ideal of norm N'")
        # canonical generic HNF columns equal the direct lattice
        cols = canonical_cols(Np, s["root"])
        rows = [tuple(c) for c in cols]
        ok_h = (hnf_rows(rows) == Ld) and is_canonical_hnf_shape(cols)
        rec["hnf_shape_eq"] += ok_h
        if not ok_h: fails.append(f"{tag} {which} eps={eps}: canonical generic HNF != direct lattice")
        # Gram invariance: (A', C', root) = (A', C', conj(e eps) g conj(eps)); C'' = (x^2+y^2+p)/(2N')
        x, y = s["root"]
        num = x*x + y*y + p
        Cp = N4(p, b) // (2 * N) if which == "u" else N4(p, b) // (2*N)
        ok_g = (num % (2 * Np) == 0 and num // (2 * Np) == Cp and gnorm(s["g"]) == s["A"] * Cp - p)
        rec["gram_inv"] += ok_g
        if not ok_g: fails.append(f"{tag} {which} eps={eps}: Gram not invariant")
        if Np % 2 == 0: rec["even_out"] += 1
        # output admission
        Vpp = (x, y, 1, 0); Upp = (2 * Np, 0, 0, 0)
        ro = admit_report(p, Np, Upp, Vpp)
        rec["admit_out"] += ro["admissible"]
        if not ro["ideal"]: fails.append(f"{tag} {which} eps={eps}: normalised output basis not an ideal basis")
        if which == "u" and eps == (1, 0):
            if ri["guard_c2_le_p_cap2"] == ro["guard_c2_le_p_cap2"] and ri["c_ge_a"] == ro["c_ge_a"]:
                rec["guard_transfer"] += 1
            else:
                fails.append(f"{tag}: guard did not transfer")
            if not ro["admissible"]:
                bad = [k for k, vv in ro.items() if vv is False and k not in ("admissible",)]
                if bad == ["N_odd"] or (set(bad) <= {"N_odd"}):
                    rec["out_even_only_fail"] += 1
                else:
                    rec["out_other_fail"] += 1
                    if ri["admissible"]:
                        fails.append(f"{tag}: output admission failures {bad}")
            # reducedness of (Upp',Vpp) in the normalised-column basis with root = conj(e) g
            ur, vr = (2 * Np, 0, 0, 0), (x, y, 1, 0)
            rec["red_out"] += reduced_flag(p, ur, vr)
            if reduced_flag(p, u, v) and not reduced_flag(p, ur, vr):
                # only a half tie on the box boundary may break reducedness
                A = N4(p, ur); h0, h1 = D(p, vr, ur), D(p, vr, J(ur))
                tie = (2 * h0 == A or 2 * h1 == A)    # +A/2 rounds up under floor(h/A+1/2)
                rec["red_out_tie_only"] += tie
                if not tie: fails.append(f"{tag}: reduced input gives non-reduced output without a tie")
    return rec, fails

def add(a, b):
    for k, v in b.items():
        a[k] = a.get(k, 0) + v
    return a

def unimodular(rng, p, u, v, kmax=3, bits=0):
    """Random Z[i]-unimodular change of basis: (u,v) -> (a u + b v, c u + d v), ad - bc a unit."""
    def rnd(k):
        return (rng.randint(-k, k), rng.randint(-k, k))
    for _ in range(100):
        # product of elementary matrices
        M = [[(1, 0), (0, 0)], [(0, 0), (1, 0)]]
        for _ in range(rng.randint(1, 5)):
            t = rnd(kmax if not bits else (1 << rng.randint(1, bits)))
            if rng.random() < .5:
                M = [[gadd(M[0][0], gm(t, M[1][0])), gadd(M[0][1], gm(t, M[1][1]))], M[1]]
            else:
                M = [M[0], [gadd(M[1][0], gm(t, M[0][0])), gadd(M[1][1], gm(t, M[0][1]))]]
            if rng.random() < .3: M = [M[1], M[0]]
            if rng.random() < .3:
                eps = UNITS[rng.randint(0, 3)]
                M = [[gm(eps, M[0][0]), gm(eps, M[0][1])], M[1]]
        def comb(a, b):
            out = [0, 0, 0, 0]
            for coef, w in ((a, u), (b, v)):
                w1 = tuple(coef[0] * x for x in w)
                jw = J(w); w2 = tuple(coef[1] * x for x in jw)
                out = [o + x + y for o, x, y in zip(out, w1, w2)]
            return tuple(out)
        return comb(*M[0]), comb(*M[1])
