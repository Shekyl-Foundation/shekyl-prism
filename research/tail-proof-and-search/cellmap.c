/* Copyright (c) 2025-2026, The Shekyl Foundation.  All rights reserved.  BSD-3-Clause
 *
 * cellmap: tail certificate by adaptive cell mapping (SHE-24 follow-up).
 * Written from the recurrence only; shares no code with lagrange-tour-bound.
 *
 * State X = (lam, eps) after a swap; eps is the residual of the next step,
 * taken in the CLOSED square [-1/2,1/2]^2 (superset of the contract's
 * half-open square), lam in [2^-LMIN, 1].  Step: r = lam + |eps|^2; it swaps
 * iff r < 1; then lam' = lam/r^2, q = conj(eps)/r, mu = round(q), eps' = q-mu.
 *
 * G_k^f = set of states from which FEWER than k further swaps occur, where
 * f=1 records that the swap producing X had r >= 1/2 (Lemma C applies).
 * G_k^f is built as an adaptive kd-tree whose leaves are marked GOOD (proved
 * inside G_k^f) or BAD (not proved).  A cell is GOOD if
 *   r >= 1 on the cell, or
 *   k >= 2 and lam >= 1/2 (Endgame), or
 *   k >= 2, f = 1 and r > 1/3 (Lemma C), or
 *   k >= 2 and for every quotient mu with |mu|^2 >= 2 (Lemma 1) the interval
 *   image of the swapping part of the cell meets no BAD leaf of G_{k-1}^{f'}
 *   (f' = 1 only if r >= 1/2 on the whole cell).
 * Every interval operation is rounded outward by one ulp; all cell
 * boundaries are dyadic (lam cells are linear subdivisions of the octaves
 * [2^-j-1, 2^-j]), so no transcendental functions are evaluated.
 *
 * usage: cellmap LMIN KMAX D2,D3,...,DKMAX [report_lo report_hi]
 *   (Dk = maximum kd-tree depth for G_k)
 */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

typedef struct { double lo, hi; } iv;
static double dn(double x) { return nextafter(x, -INFINITY); }
static double up(double x) { return nextafter(x, INFINITY); }
static iv add(iv a, iv b) { iv r = { dn(a.lo + b.lo), up(a.hi + b.hi) }; return r; }
static iv sq(iv a) {
    iv r;
    if (a.lo >= 0) { r.lo = dn(a.lo * a.lo); r.hi = up(a.hi * a.hi); }
    else if (a.hi <= 0) { r.lo = dn(a.hi * a.hi); r.hi = up(a.lo * a.lo); }
    else { r.lo = 0; r.hi = up(fmax(a.lo * a.lo, a.hi * a.hi)); }
    if (r.lo < 0) r.lo = 0;
    return r;
}
static iv mul(iv a, iv b) {
    double p[4] = { a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi };
    double lo = p[0], hi = p[0];
    for (int i = 1; i < 4; i++) { if (p[i] < lo) lo = p[i]; if (p[i] > hi) hi = p[i]; }
    iv r = { dn(lo), up(hi) }; return r;
}
static iv divpos(iv a, iv b) {
    double p[4] = { a.lo / b.lo, a.lo / b.hi, a.hi / b.lo, a.hi / b.hi };
    double lo = p[0], hi = p[0];
    for (int i = 1; i < 4; i++) { if (p[i] < lo) lo = p[i]; if (p[i] > hi) hi = p[i]; }
    iv r = { dn(lo), up(hi) }; return r;
}

/* kd-tree node: box [b0,b1] x [b2,b3] x [b4,b5] in (lam, ex, ey) */
typedef struct { double b[6]; int32_t c0, c1; int8_t status; } node; /* status 1 GOOD, 0 BAD, -1 internal */
typedef struct { node *n; int64_t len, cap; } tree;
static int64_t newnode(tree *t, const double *b) {
    if (t->len == t->cap) { t->cap = t->cap ? 2 * t->cap : 1 << 20; t->n = realloc(t->n, t->cap * sizeof(node)); if (!t->n) { fprintf(stderr, "oom\n"); exit(2); } }
    node *x = &t->n[t->len]; memcpy(x->b, b, sizeof x->b); x->c0 = x->c1 = -1; x->status = -1;
    return t->len++;
}

static double LMINV; static int MAXDEPTH; static int DEPTHK[16];
static tree T[8][2];          /* T[k][f] */
static int64_t nquery = 0;
static int USE_L1 = 1, USE_LC = 1, USE_EG = 1;   /* ablation switches (env NOL1, NOLC, NOEG) */

/* does box meet a BAD leaf of tree t (closed intersection)? */
static int meets_bad(tree *t, int64_t i, iv lam, iv ex, iv ey) {
    node *x = &t->n[i];
    if (lam.hi < x->b[0] || lam.lo > x->b[1] || ex.hi < x->b[2] || ex.lo > x->b[3] || ey.hi < x->b[4] || ey.lo > x->b[5]) return 0;
    if (x->status == 1) return 0;
    if (x->status == 0) return 1;
    return meets_bad(t, x->c0, lam, ex, ey) || meets_bad(t, x->c1, lam, ex, ey);
}
static int query_bad(int k, int f, iv lam, iv ex, iv ey) {
    nquery++;
    if (lam.lo >= 1.0) return 0;                       /* lam >= 1: no swap ever */
    if (lam.hi > 1.0) lam.hi = 1.0;
    if (lam.lo < LMINV) return 1;                      /* outside the tabulated domain */
    if (k == 1) { iv r = add(lam, add(sq(ex), sq(ey))); return r.lo < 1.0; } /* G_1 = {r >= 1}, analytic */
    /* root node 0 of T[k][f] covers [LMINV,1] x [-1/2,1/2]^2 */
    return meets_bad(&T[k][f], 0, lam, ex, ey);
}

static int test_cell(int k, int f, const double *b) {
    iv lam = { b[0], b[1] }, ex = { b[2], b[3] }, ey = { b[4], b[5] };
    iv r = add(lam, add(sq(ex), sq(ey)));
    if (r.lo >= 1.0) return 1;
    if (k <= 1) return 0;
    if (USE_EG && lam.lo >= 0.5) return 1;            /* Endgame */
    if (USE_LC && f && r.lo > 1.0 / 3.0) return 1;    /* Lemma C */
    iv rr = { r.lo, fmin(r.hi, 1.0) };
    if (rr.lo <= 0) return 0;
    iv lam2 = divpos(lam, mul(rr, rr));
    if (lam2.lo >= 1.0) return 1;
    iv qx = divpos(ex, rr);
    iv nyy = { -ey.hi, -ey.lo };
    iv qy = divpos(nyy, rr);
    long m0a = (long)floor(qx.lo), m0b = (long)floor(qx.hi) + 1;
    long m1a = (long)floor(qy.lo), m1b = (long)floor(qy.hi) + 1;
    if ((m0b - m0a + 1) * (m1b - m1a + 1) > 64) return 0;
    int f2 = rr.lo >= 0.5;
    for (long m0 = m0a; m0 <= m0b; m0++) {
        iv e2x = { fmax(dn(qx.lo - m0), -0.5), fmin(up(qx.hi - m0), 0.5) };
        if (e2x.lo > e2x.hi) continue;
        for (long m1 = m1a; m1 <= m1b; m1++) {
            if (USE_L1 && m0 * m0 + m1 * m1 < 2) continue; /* Lemma 1 */
            iv e2y = { fmax(dn(qy.lo - m1), -0.5), fmin(up(qy.hi - m1), 0.5) };
            if (e2y.lo > e2y.hi) continue;
            if (query_bad(k - 1, f2, lam2, e2x, e2y)) return 0;
        }
    }
    return 1;
}


/* validation: status of the leaf containing a point (1 GOOD, 0 BAD) */
static int point_status(tree *t, double l, double x, double y) {
    int64_t i = 0;
    for (;;) {
        node *n = &t->n[i];
        if (n->status >= 0) return n->status;
        node *a = &t->n[n->c0];
        if (l >= a->b[0] && l <= a->b[1] && x >= a->b[2] && x <= a->b[3] && y >= a->b[4] && y <= a->b[5]) i = n->c0; else i = n->c1;
    }
}
/* swaps from a state under the contract (half-up, strict), in long double */
static int nswaps(long double lam, long double ex, long double ey, long double *minmargin) {
    int n = 0; *minmargin = 1;
    for (;;) {
        long double r = lam + ex * ex + ey * ey;
        long double mg = fabsl(r - 1); if (mg < *minmargin) *minmargin = mg;
        if (r >= 1 || n > 20) return n;
        n++;
        lam = lam / (r * r); long double qx = ex / r, qy = -ey / r;
        long double mx = floorl(qx + 0.5L), my = floorl(qy + 0.5L);
        long double t1 = fabsl(qx - mx - 0.5L), t2 = fabsl(qy - my - 0.5L), t3 = fabsl(qx - mx + 0.5L), t4 = fabsl(qy - my + 0.5L);
        if (t1 < *minmargin) *minmargin = t1; if (t2 < *minmargin) *minmargin = t2; if (t3 < *minmargin) *minmargin = t3; if (t4 < *minmargin) *minmargin = t4;
        ex = qx - mx; ey = qy - my;
    }
}
static uint64_t rs = 88172645463325252ULL;
static double urand(void) { rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17; return (rs >> 11) * (1.0 / 9007199254740992.0); }
static int64_t nbad, ngood;
static double badvol;
/* build subtree for node i; returns status of node after collapse (1 all good) */
static int build(int k, int f, int64_t i, int depth) {
    tree *t = &T[k][f];
    double b[6]; memcpy(b, t->n[i].b, sizeof b);
    if (test_cell(k, f, b)) { t->n[i].status = 1; ngood++; return 1; }
    if (depth >= MAXDEPTH) { t->n[i].status = 0; nbad++; badvol += (b[1]-b[0])*(b[3]-b[2])*(b[5]-b[4]); return 0; }
    /* split widest dimension; lam width measured relative to lam.lo, scaled */
    double wl = (b[1] - b[0]) / b[0] * 0.25, wx = b[3] - b[2], wy = b[5] - b[4];
    double b0[6], b1[6]; memcpy(b0, b, sizeof b); memcpy(b1, b, sizeof b);
    int d = (wl >= wx && wl >= wy) ? 0 : (wx >= wy ? 2 : 4);
    double m = 0.5 * (b[d] + b[d + 1]);
    if (d == 0) { /* split lam on octave boundaries first */
        double o = exp2(floor(log2(b[1])));
        if (o == b[1]) o = b[1] / 2;
        if (o > b[0] && o < b[1]) m = o;
    }
    b0[d + 1] = m; b1[d] = m;
    int64_t c0 = newnode(t, b0), c1 = newnode(t, b1);
    t->n[i].c0 = (int32_t)c0; t->n[i].c1 = (int32_t)c1;
    int s0 = build(k, f, c0, depth + 1);
    int s1 = build(k, f, c1, depth + 1);
    if (s0 == 1 && s1 == 1 && c1 == t->len - 1 && c0 == t->len - 2) { /* collapse */
        t->len -= 2; t->n[i].c0 = t->n[i].c1 = -1; t->n[i].status = 1; return 1;
    }
    return 0;
}

int main(int argc, char **argv) {
    double lmin = atof(argv[1]); int kmax = atoi(argv[2]);
    { char *p = argv[3]; for (int k = 2; k < 16; k++) { DEPTHK[k] = (int)strtol(p, &p, 10); if (*p == ',') p++; else { for (int j = k + 1; j < 16; j++) DEPTHK[j] = DEPTHK[k]; break; } } }
    double rlo = argc > 5 ? atof(argv[4]) : lmin, rhi = argc > 5 ? atof(argv[5]) : 0;
    setvbuf(stdout, NULL, _IOLBF, 0);
    USE_L1 = !getenv("NOL1"); USE_LC = !getenv("NOLC"); USE_EG = !getenv("NOEG");
    printf("cellmap LMIN=2^-%g KMAX=%d depths %s; Lemma 1 %s, Lemma C %s, Endgame %s\n", lmin, kmax, argv[3], USE_L1?"on":"off", USE_LC?"on":"off", USE_EG?"on":"off");
    LMINV = exp2(-lmin);  /* lmin is an integer: exact */
    if (LMINV != ldexp(1.0, -(int)lmin)) { fprintf(stderr, "LMIN must be an integer\n"); return 2; }
    for (int k = 2; k <= kmax; k++) for (int f = 0; f < 2; f++) {
        if (k >= 4) { free(T[k-2][f].n); T[k-2][f].n = NULL; }
        double b[6] = { LMINV, 1.0, -0.5, 0.5, -0.5, 0.5 };
        newnode(&T[k][f], b);
        nbad = ngood = 0; badvol = 0; nquery = 0;
        MAXDEPTH = DEPTHK[k];
        int s = build(k, f, 0, 0);
        printf("G_%d^%d: nodes %lld, good leaves %lld, bad leaves %lld, bad volume %.3g, queries %lld%s\n",
               k, f, (long long)T[k][f].len, (long long)ngood, (long long)nbad, badvol, (long long)nquery, s ? "  ALL GOOD" : "");
        /* lowest lam of a bad leaf */
        double lowbad = 2; for (int64_t i = 0; i < T[k][f].len; i++) if (T[k][f].n[i].status == 0 && T[k][f].n[i].b[0] < lowbad) lowbad = T[k][f].n[i].b[0];
        if (lowbad < 2) printf("   lowest bad lam %.6g = 2^%.4f\n", lowbad, log2(lowbad));
        /* report: is [2^-rlo, 2^-rhi] x square free of BAD in G_k^0 ? */
        iv L = { exp2(-rlo), exp2(-rhi) }, E = { -0.5, 0.5 };
        if (L.lo < LMINV) L.lo = LMINV;
        if (f == 0 && getenv("VALIDATE")) { long nsamp = atol(getenv("VALIDATE")); long viol = 0, inbad = 0, kplus = 0;
            for (long j = 0; j < nsamp; j++) {
                double l = LMINV * exp2(-log2(LMINV) * urand()), x = urand() - 0.5, y = urand() - 0.5;
                if (l > 1) l = 1;
                long double mm; int n = nswaps(l, x, y, &mm);
                int st = point_status(&T[k][0], l, x, y);
                if (n >= k) { kplus++; if (st == 1 && mm > 1e-12L) { viol++; if (viol < 5) printf("   VIOLATION lam %.17g eps (%.17g, %.17g): %d swaps in a GOOD leaf\n", l, x, y, n); } }
                if (st == 0) inbad++;
            }
            printf("   validation: %ld samples, %ld with >= %d swaps, %ld in BAD leaves, violations %ld\n", nsamp, kplus, k, inbad, viol); }
        if (k == kmax && f == 0) { int c = 0; for (int64_t i = 0; i < T[k][f].len && c < 40; i++) if (T[k][f].n[i].status == 0) { node *x = &T[k][f].n[i]; printf("   BAD lam[%.10g,%.10g] ex[%.10g,%.10g] ey[%.10g,%.10g]\n", x->b[0],x->b[1],x->b[2],x->b[3],x->b[4],x->b[5]); c++; } }
        if (f == 0) printf("   region lam in [2^-%g, 2^-%g]: fewer than %d further swaps %s\n", rlo, rhi, k,
               meets_bad(&T[k][0], 0, L, E, E) ? "NOT PROVED" : "PROVED");
    }
    return 0;
}
