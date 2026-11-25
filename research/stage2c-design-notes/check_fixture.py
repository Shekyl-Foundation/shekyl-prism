# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Consumer acceptance test, first case: the p=3, T=5 fixture (SHE-25).

The fixture is PR #45's `test_final_size_reduction_after_last_swap`
(scripts/stage2c_oracle.py at 84a798a): U=(10,0,0,0), V=(4,9,1,0).
This script checks, by assertion, every number in NOTES.md section 3:

  1. the input is an admitted generic HNF and the normalized constructor
     accepts it, with det = p;
  2. the exact trace (two active steps, one swap, a +1/2 tie in step 2,
     a -1/2 tie at the fixed point);
  3. the carried-Gram run makes the same decisions as the coordinate run;
  4. the consumer form built from the returned basis equals the form
     converted from the carried Gram, and satisfies c^T F c = nrd(Bc/2)/T;
  5. the early-stop control: after one step the first vector is already
     shortest (a shortest-vector-only test passes) but the form test fails;
  6. sweep: every generic HNF at small p and odd T, at every schedule
     length up to the fixed point, satisfies (4) and (5)'s logic.
  7. constructor negative controls.

Run: python3 -B check_fixture.py   (stdlib only)
"""
import itertools
import sys

from s2c_model import (D, J, N4, carried_exact, columns, comb, eval_form, exact_step,
                       form_from_basis, form_from_gram, form_is_reduced, gram_general,
                       gram_hnf, identity_holds, is_reduced, probe_vectors, reduce_fixed)

BITS = 64          # one limb is plenty for the toy; the arithmetic is identical
P, T = 3, 5
U0, V0 = (10, 0, 0, 0), (4, 9, 1, 0)


def say(*a):
    print(*a)


def main():
    # ---- 1. admission
    assert U0 == (2*T, 0, 0, 0) and V0[2:] == (1, 0)
    x, y = V0[0], V0[1]
    gram0, ok = gram_hnf(P, T, x, y, BITS)
    assert ok, "fixture must be admitted"
    assert gram0 == (10, 10, 4, 9), gram0
    a, c, h0, h1 = gram0
    assert a*c - h0*h0 - h1*h1 == P
    assert (N4(P, U0), N4(P, V0), D(P, V0, U0), D(P, V0, J(U0))) == (100, 100, 40, 90)
    g2, ok2 = gram_general(P, T, U0, V0, BITS)
    assert ok2 and g2 == gram0
    say("1. admitted: numerator (A,C,h0,h1) = (100,100,40,90); normalized (10,10,4,9); det = 3 = p")

    # ---- 2. exact trace
    assert not N4(P, V0) < N4(P, U0)                 # equal norms: no initial swap
    u, v = U0, V0
    u, v, r1 = exact_step(P, u, v)
    assert (r1['A'], r1['h0'], r1['h1'], r1['m0'], r1['m1'], r1['swap']) == (100, 40, 90, 0, 1, True)
    assert (u, v) == ((4, -1, 1, 0), (10, 0, 0, 0)) and N4(P, u) == 20
    u2, v2, r2 = exact_step(P, u, v)
    assert (r2['A'], r2['h0'], r2['h1'], r2['m0'], r2['m1'], r2['swap']) == (20, 40, 10, 2, 1, False)
    assert 2*r2['h1'] == r2['A']                      # exact +1/2 tie, rounded up to 1
    assert (u2, v2) == ((4, -1, 1, 0), (1, -2, -2, -1)) and N4(P, v2) == 20
    u3, v3, r3 = exact_step(P, u2, v2)
    assert (r3['h0'], r3['h1'], r3['m0'], r3['m1'], r3['swap']) == (0, -10, 0, 0, False)
    assert 2*r3['h1'] == -r3['A']                     # exact -1/2 tie, rounded to 0
    assert (u3, v3) == (u2, v2) and is_reduced(P, u2, v2)
    say("2. step 1: m=(0,1), swap -> U=(4,-1,1,0) N4=20; step 2: m=(2,1) [+1/2 tie] no swap"
        " -> V=(1,-2,-2,-1) N4=20; step 3: m=(0,0) [-1/2 tie], fixed point")

    # ---- 3. carried Gram makes the same decisions
    gfin, gtrace = carried_exact(P, gram0, 3)
    assert [(t['m0'], t['m1'], t['swap']) for t in gtrace] == [(0, 1, True), (2, 1, False), (0, 0, False)]
    assert gfin == (2, 2, 0, -1), gfin
    assert gfin == tuple(z // (2*T) for z in (N4(P, u2), N4(P, v2), D(P, v2, u2), D(P, v2, J(u2))))
    say("3. carried normalized Gram after the schedule: (A',C',h0',h1') = (2,2,0,-1); decisions match")

    # ---- 4. consumer form
    f_basis = form_from_basis(P, u2, v2, T)
    f_gram = form_from_gram(gfin)
    assert f_basis == f_gram == [[1, 0, 0, 0], [0, 1, 0, 0], [0, -1, 1, 0], [1, 0, 0, 1]], f_basis
    probes = probe_vectors()
    assert identity_holds(P, u2, v2, T, f_gram, probes)
    assert form_is_reduced(f_gram)
    # The C reference's returned triangular form is 4F on the same basis (SIGNER_SPEC 2.3).
    c_ref = [[4*e for e in row] for row in f_gram]
    assert all(eval_form(c_ref, cc) == N4(P, comb(columns(u2, v2), cc)) // T for cc in probes)
    say("4. F (rows, U JU V JV) = [[1,0,0,0],[0,1,0,0],[0,-1,1,0],[1,0,0,1]];"
        f" identity holds on {len(probes)} probes; reduced")

    # ---- 5. early-stop control
    ue, ve, _ = reduce_fixed(P, U0, V0, 1)
    assert N4(P, ue) == 20                            # already the minimum: see exhaustive check below
    f_early = form_from_basis(P, ue, ve, T)
    assert f_early == [[1, 0, 0, 0], [0, 1, 0, 0], [4, 1, 5, 0], [-1, 4, 0, 5]], f_early
    assert identity_holds(P, ue, ve, T, f_early, probes)   # the identity holds for ANY basis
    assert not form_is_reduced(f_early)                    # ... the terminal test is what fails
    # the shortest-vector check alone would have passed after one step:
    lam = min(N4(P, comb(columns(U0, V0), cc)) for cc in itertools.product(range(-4, 5), repeat=4) if any(cc))
    assert lam == 20 == N4(P, ue)
    # and with the +1/2 tie rounded toward zero instead, the run stops one step short:
    m1_wrong = (2*10) // (2*20)                       # trunc(h/A) with h = A/2 gives 0, not 1
    assert m1_wrong == 0
    say("5. after 1 step: N4(U)=20=lambda1 (shortest-vector test passes) but F20=4, F21=1=F00"
        " -> terminal test fails; identity alone cannot see it")

    # ---- 5b. mutants the fixture catches (exact output compared with the oracle's pair)
    def mutant(steps, strict0=True, strict=True, tie_up=True):
        u, v = U0, V0
        if (N4(P, v) < N4(P, u)) if strict0 else (N4(P, v) <= N4(P, u)):
            u, v = v, u
        for _ in range(steps):
            a = N4(P, u)
            ju = J(u)
            rnd = (lambda h: (2*h + a) // (2*a)) if tie_up else (lambda h: -((-2*h + a) // (2*a)))
            v = comb((v, u, ju), (1, -rnd(D(P, v, u)), -rnd(D(P, v, ju))))
            if (N4(P, v) < a) if strict else (N4(P, v) <= a):
                u, v = v, u
        return u, v
    good = (u2, v2)
    assert mutant(5) == good
    assert mutant(5, strict0=False) != good          # non-strict initial ordering
    assert mutant(5, strict=False) != good           # non-strict swap at step 2's equal norms
    ut, vt = mutant(5, tie_up=False)                 # +1/2 rounded toward zero
    assert (ut, vt) != good and not form_is_reduced(form_from_basis(P, ut, vt, T))
    say("5b. mutants caught: non-strict initial swap and non-strict step swap (output differs),"
        " +1/2 toward zero (output differs and is not reduced)")

    # ---- 6. sweep
    runs = 0
    for p in (3, 7, 11, 19, 23, 31, 43, 47, 59, 67, 71, 79, 83):
        for t in range(1, 60, 2):
            for x in range(0, 2*t, 2):
                for y in range(1, 2*t, 2):
                    if (x*x + y*y + p) % (4*t):
                        continue
                    g, ok = gram_hnf(p, t, x, y, BITS)
                    assert ok
                    u, v = (2*t, 0, 0, 0), (x, y, 1, 0)
                    for steps in range(0, 12):
                        uu, vv, _ = reduce_fixed(p, u, v, steps)
                        gg, _ = carried_exact(p, g, steps)
                        fb = form_from_basis(p, uu, vv, t)
                        assert fb == form_from_gram(gg), (p, t, x, y, steps)
                        assert identity_holds(p, uu, vv, t, fb, probes)
                        assert form_is_reduced(fb) == is_reduced(p, uu, vv)
                        runs += 1
                    assert is_reduced(p, uu, vv)
    say(f"6. sweep: {runs} (ideal, schedule length) runs: carried-Gram conversion == fresh form,"
        " identity holds, form test == coordinate terminal test")

    # ---- 7. constructor negative controls
    assert not gram_hnf(P, T, 5, 9, BITS)[1]          # x odd
    assert not gram_hnf(P, T, 4, 8, BITS)[1]          # y even
    assert not gram_hnf(P, T, 2, 9, BITS)[1]          # 4T does not divide x^2+y^2+p (88)
    assert not gram_hnf(P, 4, 4, 9, BITS)[1]          # T even (Hensel needs odd T)
    assert not gram_general(P, T, U0, (4, 9, 3, 0), BITS)[1]       # not in the ideal
    assert not gram_general(P, T, (20, 0, 0, 0), V0, BITS)[1]      # proper sublattice: det = 4p
    say("7. constructor refuses: odd x, even y, 4T not dividing, even T, non-member, proper sublattice")
    return 0


if __name__ == "__main__":
    sys.exit(main())
