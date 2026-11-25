/* Copyright (c) 2025-2026, The Shekyl Foundation
 * All rights reserved.  BSD-3-Clause
 *
 * Independent tail certifier (SHE-24 review of SHE-23).
 * Written from the recurrence only; shares no code with lagrange-tour-bound.
 *
 * State after a swap t:  X_t = (lam_t, eps_t), lam_t = Delta/A_t^2, eps_t the
 * residual of step t+1 (closed square [-1/2,1/2]^2, a superset of the
 * contract's half-open square).  Step t+1 swaps iff r = lam + |eps|^2 < 1; then
 *   lam_{t+1} = lam / r^2,  q = conj(eps)/r,  mu = floor(q + 1/2),
 *   eps_{t+1} = q - mu,
 * and by Lemma 1 the step after can only swap if |mu|^2 >= 2.
 *
 * Claim proved: from every X with lam in [LMIN, 1], fewer than K further
 * swaps occur.  Rigorous interval arithmetic in IEEE double, every result
 * widened outward by one ulp (nextafter), so all enclosures are sound.
 *
 * usage: tailcert LMIN_EXP K [maxdepth-per-level] [use_endgame=1] [use_lemma_c=1]
 */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct { double lo, hi; } iv;

static double dn(double x) { return nextafter(x, -INFINITY); }
static double up(double x) { return nextafter(x, INFINITY); }

static iv add(iv a, iv b) { iv r = { dn(a.lo + b.lo), up(a.hi + b.hi) }; return r; }
static iv sq(iv a) {
    iv r;
    if (a.lo >= 0) { r.lo = dn(a.lo * a.lo); r.hi = up(a.hi * a.hi); }
    else if (a.hi <= 0) { r.lo = dn(a.hi * a.hi); r.hi = up(a.lo * a.lo); }
    else { double m = fmax(a.lo * a.lo, a.hi * a.hi); r.lo = 0; r.hi = up(m); }
    if (r.lo < 0) r.lo = 0;
    return r;
}
static iv mul(iv a, iv b) {
    double p[4] = { a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi };
    double lo = p[0], hi = p[0];
    for (int i = 1; i < 4; i++) { if (p[i] < lo) lo = p[i]; if (p[i] > hi) hi = p[i]; }
    iv r = { dn(lo), up(hi) }; return r;
}
static iv divpos(iv a, iv b) { /* b.lo > 0 */
    double p[4] = { a.lo / b.lo, a.lo / b.hi, a.hi / b.lo, a.hi / b.hi };
    double lo = p[0], hi = p[0];
    for (int i = 1; i < 4; i++) { if (p[i] < lo) lo = p[i]; if (p[i] > hi) hi = p[i]; }
    iv r = { dn(lo), up(hi) }; return r;
}

static long long nboxes = 0;
static int maxdepth_seen = 0;
static int MAXDEPTH = 60;
static double witness[6];
static int TOPK = 0, witk = 0;
static int USE_ENDGAME = 1;

/* returns 1 if refuted: fewer than k swaps from every state in the box */
static int USE_LEMMAC = 1;
static int refute(iv lam, iv ex, iv ey, int k, int depth, int prev_half) {
    nboxes++;
    if (k > TOPK) TOPK = k;
    if (depth > maxdepth_seen) maxdepth_seen = depth;
    iv r = add(lam, add(sq(ex), sq(ey)));
    if (r.lo >= 1.0) return 1;                     /* no swap at all */
    /* Endgame theorem (re-derived in lemmas.sage): from a post-swap state
       with lam >= 1/2 at most one further swap occurs. */
    if (k >= 2 && lam.lo >= 0.5 && USE_ENDGAME) return 1;
    /* Lemma C (re-derived in lemmas.sage): if the swap that produced this
       state had r >= 1/2 and the next step has r > 1/3, the next two steps
       cannot both swap. */
    if (k >= 2 && prev_half && r.lo > 1.0 / 3.0 && USE_LEMMAC) return 1;
    if (k <= 1) return 0;   /* r is enclosed exactly (independent terms): a swap is possible */
    {
        iv rr = { r.lo, fmin(r.hi, 1.0) };          /* only swapping states */
        if (rr.lo <= 0) goto split;
        iv lam2 = divpos(lam, mul(rr, rr));
        if (lam2.lo >= 1.0) return 1;               /* next state cannot swap */
        if (lam2.hi > 1.0) lam2.hi = 1.0;           /* states with lam>=1 never swap */
        iv qx = divpos(ex, rr);
        iv nyy = { -ey.hi, -ey.lo };
        iv qy = divpos(nyy, rr);
        long m0a = (long)floor(qx.lo), m0b = (long)floor(qx.hi) + 1;
        long m1a = (long)floor(qy.lo), m1b = (long)floor(qy.hi) + 1;
        if ((m0b - m0a + 1) * (m1b - m1a + 1) > 400) goto split;
        int ok = 1;
        for (long m0 = m0a; m0 <= m0b && ok; m0++) {
            iv e2x = { fmax(dn(qx.lo - m0), -0.5), fmin(up(qx.hi - m0), 0.5) };
            if (e2x.lo > e2x.hi) continue;
            for (long m1 = m1a; m1 <= m1b && ok; m1++) {
                if (m0 * m0 + m1 * m1 < 2) continue;    /* Lemma 1 */
                iv e2y = { fmax(dn(qy.lo - m1), -0.5), fmin(up(qy.hi - m1), 0.5) };
                if (e2y.lo > e2y.hi) continue;
                if (!refute(lam2, e2x, e2y, k - 1, 0, rr.lo >= 0.5)) ok = 0;
            }
        }
        if (ok) return 1;
    }
split:
    if (depth >= MAXDEPTH) {
        witk = k;
        witness[0] = lam.lo; witness[1] = lam.hi; witness[2] = ex.lo;
        witness[3] = ex.hi; witness[4] = ey.lo; witness[5] = ey.hi;
        return 0;
    }
    {
        double wl = (lam.hi - lam.lo) / fmax(lam.lo, 1e-300) * 0.25; /* relative */
        double wx = ex.hi - ex.lo, wy = ey.hi - ey.lo;
        if (wl >= wx && wl >= wy) {
            double m = 0.5 * (lam.lo + lam.hi);
            iv a = { lam.lo, m }, b = { m, lam.hi };
            return refute(a, ex, ey, k, depth + 1, prev_half) && refute(b, ex, ey, k, depth + 1, prev_half);
        } else if (wx >= wy) {
            double m = 0.5 * (ex.lo + ex.hi);
            iv a = { ex.lo, m }, b = { m, ex.hi };
            return refute(lam, a, ey, k, depth + 1, prev_half) && refute(lam, b, ey, k, depth + 1, prev_half);
        } else {
            double m = 0.5 * (ey.lo + ey.hi);
            iv a = { ey.lo, m }, b = { m, ey.hi };
            return refute(lam, ex, a, k, depth + 1, prev_half) && refute(lam, ex, b, k, depth + 1, prev_half);
        }
    }
}

int main(int argc, char **argv) {
    /* LMIN_EXP may be "a" (lam in [2^-a, 1]) or "a:b" (lam in [2^-a, 2^-b]),
       a and b decimal (2^-a evaluated with outward rounding). */
    double a = 0, b = 0; int k = atoi(argv[2]);
    if (sscanf(argv[1], "%lf:%lf", &a, &b) < 2) b = 0;
    if (argc > 3) MAXDEPTH = atoi(argv[3]);
    if (argc > 4) USE_ENDGAME = atoi(argv[4]);
    if (argc > 5) USE_LEMMAC = atoi(argv[5]);
    iv lam = { dn(pow(2.0, -a)), b == 0 ? 1.0 : up(pow(2.0, -b)) }, ex = { -0.5, 0.5 }, ey = { -0.5, 0.5 };
    int ok = refute(lam, ex, ey, k, 0, 0);
    printf("lam in [2^-%g, 2^-%g]: fewer than %d further swaps: %s  (boxes %lld, max depth %d)\n",
           a, b, k, ok ? "PROVED" : "NOT PROVED", nboxes, maxdepth_seen);
    if (!ok)
        printf("  unresolved box (remaining k=%d) lam [%.17g, %.17g] ex [%.17g, %.17g] ey [%.17g, %.17g]\n",
               witk, witness[0], witness[1], witness[2], witness[3], witness[4], witness[5]);
    return ok ? 0 : 1;
}
