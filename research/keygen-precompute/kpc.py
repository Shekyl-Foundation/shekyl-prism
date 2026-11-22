# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Keygen precomputation model for Shekyl PRISM (research harness).

Measures how much secret-only work in signing moves to key generation when
stored in an expanded secret key, across the five Shekyl parameter sets
(P248, P324, P376, P500, P664).

PRISM is the scheme of ePrint 2026/443 (Basso, Borin, Castryck,
Corte-Real Santos, Invernizzi, Leroux, Maino, Vercauteren, Wesolowski);
shekyl-prism implements it. Research model only; step/multiply counts in
Python are not CPU cycles.
"""
import hashlib
import math

import gmpy2

from hdl import (
    SETS, SALT_LEN, PK_BYTES, prime_of, shake, qmul, nrd, ideal_basis,
    gram_norm, pari,
)
from fsr import Ctr, _round_q, herm


def set_params():
    """Exact bit and byte parameters for all five Shekyl sets."""
    out = {}
    for name, e, f in SETS:
        p = prime_of(e, f)
        L = p.bit_length()
        W = 128 * ((e + 127) // 128)
        B = (L + 1) // 2 + 32
        fp_len = (L + 7) // 8
        pk_len = 2 * fp_len
        out[name] = dict(
            name=name, e=e, f=f, p=p, L=L, W=W, B=B,
            fp_len=fp_len, pk_len=pk_len,
            salt_len=SALT_LEN[name],
            mat_cref=4 * ((e + 8) // 8),   # PRISM_v2 TORSION_2POWER_BYTES = ceil((e+1)/8)
            mat_spec=4 * ((e + 7) // 8),   # SIGNER_SPEC §1.3 item 29 = ceil(e/8)
        )
    return out


PARAMS = set_params()


def key_sizes_bytes(pinfo):
    """Secret-key serialized sizes (bytes) for baseline vs expanded formats."""
    fp = pinfo['fp_len']
    pk = pinfo['pk_len']
    B = pinfo['B']
    L = pinfo['L']
    mat_c = pinfo['mat_cref']
    mat_s = pinfo['mat_spec']

    n_bytes_spec = (B + 7) // 8
    # I_sec HNF: 4 nontrivial entries in [0, 2N), each <= B+1 bits
    hnf_bytes = 4 * ((B + 1 + 7) // 8)
    # dual(I_sec): 16 entries of <= B+1 bits (only needed by PRISM_v2 dual-inter)
    dual_bytes = 16 * ((B + 1 + 7) // 8)
    # canonical_basis of E_vk[2^e]: (x(P), x(Q), x(P-Q)), 3 * F_{p^2}
    can_basis_bytes = 3 * 2 * fp
    # pre-reduced Z[i]-basis {u_sec, v_sec} of I_sec:
    # proven bound: nrd(v_sec) <= 2*N*p => |v0|,|v1| <= sqrt(2Np) <= (B+L)/2+1 bits,
    # |v2|,|v3| <= sqrt(2N) <= B/2+1 bits (for O0 with /2 denom, +1 bit)
    uv_big_bytes = ((((B + L + 1) // 2) + 2) + 7) // 8
    uv_small_bytes = ((((B + 1) // 2) + 2) + 7) // 8
    uv_compact_bytes = 4 * uv_big_bytes + 4 * uv_small_bytes
    uv_uniform_bytes = 8 * uv_big_bytes
    # Z[i] Gram constants (nu, nv, h0, h1) or 4x4 Gram: <= B + L + 2 bits
    gram_bytes = 4 * ((B + L + 2 + 7) // 8)

    # 1) PRISM_v2 SECRETKEY_BYTES = pk (2*fp + 1 hint) + N (fp) + gen (4*fp) + mat (mat_c)
    cref_sk = (pk + 1) + fp + 4 * fp + mat_c
    # 2) Compact shekyl-prism baseline (no hint byte, N in ceil(B/8), mat_s)
    base_spec_sk = pk + n_bytes_spec + 4 * fp + mat_s

    # 3) EXP_HNF (compact: replaces gen with HNF, adds canonical_basis)
    exp_hnf_no_can = pk + n_bytes_spec + hnf_bytes + mat_s
    exp_hnf_with_can = exp_hnf_no_can + can_basis_bytes
    # 4) EXP_HNF_COMPAT (keeps PRISM_v2 layout + adds HNF + canonical_basis)
    exp_hnf_compat = cref_sk + hnf_bytes + can_basis_bytes
    # 5) EXP_RED_O0 (compact: replaces gen with {u_sec, v_sec})
    exp_red_compact_no_can = pk + n_bytes_spec + uv_compact_bytes + mat_s
    exp_red_compact_with_can = exp_red_compact_no_can + can_basis_bytes
    exp_red_uniform_with_can = pk + n_bytes_spec + uv_uniform_bytes + mat_s + can_basis_bytes
    # 6) EXP_RED_COMPAT (keeps PRISM_v2 layout + adds {u_sec, v_sec} + canonical_basis)
    exp_red_compat = cref_sk + uv_compact_bytes + can_basis_bytes
    # 7) EXP_RED_GRAM (compact + Z[i] Gram constants for 4-D LLL cache)
    exp_red_gram = exp_red_compact_with_can + gram_bytes

    return {
        'cref_sk': cref_sk,
        'base_spec_sk': base_spec_sk,
        'exp_hnf_no_can': exp_hnf_no_can,
        'exp_hnf_with_can': exp_hnf_with_can,
        'exp_hnf_compat': exp_hnf_compat,
        'exp_red_compact_no_can': exp_red_compact_no_can,
        'exp_red_compact_with_can': exp_red_compact_with_can,
        'exp_red_uniform_with_can': exp_red_uniform_with_can,
        'exp_red_compat': exp_red_compat,
        'exp_red_gram': exp_red_gram,
        'can_basis_bytes': can_basis_bytes,
        'hnf_bytes': hnf_bytes,
        'dual_bytes': dual_bytes,
        'uv_compact_bytes': uv_compact_bytes,
        'uv_uniform_bytes': uv_uniform_bytes,
        'gram_bytes': gram_bytes,
    }


# ------------------------------------------------ Instrumented arithmetic
class OpCtr:
    """Counts integer multiplies, divides, xgcd steps, and max bit widths."""
    __slots__ = ('mul', 'div', 'xgcd_steps', 'sel', 'max_bits', 'limb_ops')

    def __init__(self):
        self.mul = 0
        self.div = 0
        self.xgcd_steps = 0
        self.sel = 0
        self.max_bits = 0
        self.limb_ops = 0

    def touch(self, *vals):
        for v in vals:
            b = abs(int(v)).bit_length()
            if b > self.max_bits:
                self.max_bits = b

    def do_mul(self, a, b):
        self.mul += 1
        ba = max(1, (abs(int(a)).bit_length() + 63) // 64)
        bb = max(1, (abs(int(b)).bit_length() + 63) // 64)
        self.limb_ops += ba * bb
        self.touch(a * b)
        return a * b

    def do_div(self, a, b):
        self.div += 1
        ba = max(1, (abs(int(a)).bit_length() + 63) // 64)
        bb = max(1, (abs(int(b)).bit_length() + 63) // 64)
        self.limb_ops += ba * bb
        self.touch(a, b)
        return a // b

    def do_mod(self, a, m):
        self.div += 1
        ba = max(1, (abs(int(a)).bit_length() + 63) // 64)
        bm = max(1, (abs(int(m)).bit_length() + 63) // 64)
        self.limb_ops += ba * bm
        self.touch(a, m)
        return a % m

    def do_xgcd(self, a, b):
        """Instrumented extended GCD: returns (g, u, v) with u*a + v*b = g >= 0."""
        old_r, r = int(a), int(b)
        old_s, s = 1, 0
        old_t, t = 0, 1
        while r != 0:
            self.xgcd_steps += 1
            q = old_r // r
            self.div += 1
            ba = max(1, (abs(old_r).bit_length() + 63) // 64)
            bb = max(1, (abs(r).bit_length() + 63) // 64)
            self.limb_ops += ba * bb
            old_r, r = r, old_r - q * r
            old_s, s = s, old_s - q * s
            old_t, t = t, old_t - q * t
            self.mul += 3
            bq = max(1, (abs(q).bit_length() + 63) // 64)
            self.limb_ops += bq * (bb + max(1, (abs(s).bit_length() + 63) // 64)
                                   + max(1, (abs(t).bit_length() + 63) // 64))
            self.touch(old_r, r, old_s, s, old_t, t)
        if old_r < 0:
            old_r, old_s, old_t = -old_r, -old_s, -old_t
        return old_r, old_s, old_t

    def as_dict(self, prefix):
        return {
            f'{prefix}_mul': self.mul,
            f'{prefix}_div': self.div,
            f'{prefix}_xgcd': self.xgcd_steps,
            f'{prefix}_maxbits': self.max_bits,
            f'{prefix}_limb_ops': self.limb_ops,
        }


def hnf_mod_M(vectors, M, c):
    """4xK lower-triangular / row-HNF modulo fixed M (SIGNER_SPEC §2.1 Q1/Q3).

    Takes row vectors (each length 4) plus implicit M*I_4, and produces the
    4x4 row-HNF basis (lower-triangular in reverse or upper-triangular matching
    hdl.hnf_rows). Here we match hdl.hnf_rows: columns of pari.mathnf are
    upper-triangular; hnf_rows returns its columns as rows, so B is lower-
    triangular: B[r][c] == 0 for c > r, with positive diagonal pivots.
    """
    # Work on rows modulo M: reduce pivot columns 3, 2, 1, 0 in order so
    # row r has last nonzero entry at column r (lower-triangular).
    rows = [[v[j] % M for j in range(4)] for v in vectors]
    rows += [[M if j == r else 0 for j in range(4)] for r in range(4)]
    c.touch(M)
    pivots = [None] * 4
    for col in (3, 2, 1, 0):
        # Accumulate gcd across rows having nonzero entry at col
        cur = None
        for r in rows:
            val = r[col] % M
            if val == 0:
                continue
            if cur is None:
                cur = list(r)
                if cur[col] < 0:
                    cur = [-x for x in cur]
            else:
                g, u, v = c.do_xgcd(cur[col], val)
                a = c.do_div(val, g)
                b = c.do_div(cur[col], g)
                # new_cur = u*cur + v*r (entry at col becomes g)
                # new_r   = -a*cur + b*r (entry at col becomes 0)
                new_cur = []
                new_r = []
                for j in range(4):
                    nc = c.do_mul(u, cur[j]) + c.do_mul(v, r[j])
                    nr = c.do_mul(b, r[j]) - c.do_mul(a, cur[j])
                    new_cur.append(c.do_mod(nc, M))
                    new_r.append(c.do_mod(nr, M))
                cur = new_cur
                for j in range(4):
                    r[j] = new_r[j]
        # Reduce cur against M*e_col if needed
        if cur is None or cur[col] == 0:
            cur = [M if j == col else 0 for j in range(4)]
        else:
            g, u, _ = c.do_xgcd(cur[col], M)
            if g != cur[col]:
                cur = [c.do_mod(c.do_mul(u, x), M) for x in cur]
            if cur[col] == 0:
                cur[col] = M
        pivots[col] = cur
        # Zero out col in all remaining rows (for columns < col)
        pcol = cur[col]
        for r in rows:
            if r[col] != 0:
                q = c.do_div(r[col], pcol)
                for j in range(4):
                    r[j] = c.do_mod(r[j] - c.do_mul(q, cur[j]), M)
    # Reduce off-diagonal entries of upper rows modulo lower-row pivots (standard HNF)
    for col in (2, 1, 0):
        for r_idx in range(col + 1, 4):
            piv = pivots[col][col]
            if pivots[r_idx][col] != 0:
                q = c.do_div(pivots[r_idx][col], piv)
                for j in range(col + 1):
                    pivots[r_idx][j] -= c.do_mul(q, pivots[col][j])
                # keep in [0, M) except pivot check
                if pivots[r_idx][col] < 0:
                    for j in range(col + 1):
                        pivots[r_idx][j] += pivots[col][j]
    return pivots


def mat4_adjugate_det(B, c):
    """Exact 4x4 integer adjugate and determinant (modelling PRISM_v2's
    ibz_mat_4x4_inv_with_det_as_denom), instrumented."""
    # Compute 2x2 minors of rows (0,1) and (2,3)
    def mul(a, b):
        return c.do_mul(a, b)

    s0 = mul(B[0][0], B[1][1]) - mul(B[1][0], B[0][1])
    s1 = mul(B[0][0], B[1][2]) - mul(B[1][0], B[0][2])
    s2 = mul(B[0][0], B[1][3]) - mul(B[1][0], B[0][3])
    s3 = mul(B[0][1], B[1][2]) - mul(B[1][1], B[0][2])
    s4 = mul(B[0][1], B[1][3]) - mul(B[1][1], B[0][3])
    s5 = mul(B[0][2], B[1][3]) - mul(B[1][2], B[0][3])

    c5 = mul(B[2][2], B[3][3]) - mul(B[3][2], B[2][3])
    c4 = mul(B[2][1], B[3][3]) - mul(B[3][1], B[2][3])
    c3 = mul(B[2][1], B[3][2]) - mul(B[3][1], B[2][2])
    c2 = mul(B[2][0], B[3][3]) - mul(B[3][0], B[2][3])
    c1 = mul(B[2][0], B[3][2]) - mul(B[3][0], B[2][2])
    c0 = mul(B[2][0], B[3][1]) - mul(B[3][0], B[2][1])

    det = (mul(s0, c5) - mul(s1, c4) + mul(s2, c3)
           + mul(s3, c2) - mul(s4, c1) + mul(s5, c0))
    adj = [
        [mul(B[1][1], c5) - mul(B[1][2], c4) + mul(B[1][3], c3),
         -mul(B[0][1], c5) + mul(B[0][2], c4) - mul(B[0][3], c3),
         mul(B[3][1], s5) - mul(B[3][2], s4) + mul(B[3][3], s3),
         -mul(B[2][1], s5) + mul(B[2][2], s4) - mul(B[2][3], s3)],
        [-mul(B[1][0], c5) + mul(B[1][2], c2) - mul(B[1][3], c1),
         mul(B[0][0], c5) - mul(B[0][2], c2) + mul(B[0][3], c1),
         -mul(B[3][0], s5) + mul(B[3][2], s2) - mul(B[3][3], s1),
         mul(B[2][0], s5) - mul(B[2][2], s2) + mul(B[2][3], s1)],
        [mul(B[1][0], c4) - mul(B[1][1], c2) + mul(B[1][3], c0),
         -mul(B[0][0], c4) + mul(B[0][1], c2) - mul(B[0][3], c0),
         mul(B[3][0], s4) - mul(B[3][1], s2) + mul(B[3][3], s0),
         -mul(B[2][0], s4) + mul(B[2][1], s2) - mul(B[2][3], s0)],
        [-mul(B[1][0], c3) + mul(B[1][1], c1) - mul(B[1][2], c0),
         mul(B[0][0], c3) - mul(B[0][1], c1) + mul(B[0][2], c0),
         -mul(B[3][0], s3) + mul(B[3][1], s1) - mul(B[3][2], s0),
         mul(B[2][0], s3) - mul(B[2][1], s1) + mul(B[2][2], s0)],
    ]
    c.touch(det)
    for row in adj:
        c.touch(*row)
    return adj, det


def find_ideal_generator(B_sec, N, p):
    """Find a generator of I_sec (as in PRISM_v2 quat_lideal_generator)."""
    int_norm = 0
    while True:
        int_norm += 1
        for a in range(-int_norm, int_norm + 1):
            ra = int_norm - abs(a)
            for b in range(-ra, ra + 1):
                rb = ra - abs(b)
                for c_ in range(-rb, rb + 1):
                    d = rb - abs(c_)
                    if math.gcd(math.gcd(abs(a), abs(b)), math.gcd(abs(c_), abs(d))) != 1:
                        continue
                    v = [a * B_sec[0][k] + b * B_sec[1][k] + c_ * B_sec[2][k] + d * B_sec[3][k]
                         for k in range(4)]
                    nv = nrd(v, p)
                    q, r = divmod(nv, N)
                    assert r == 0
                    if math.gcd(N, q) == 1:
                        return tuple(v)


def sim_quat_lideal_create(gen, N, p):
    """Simulate secret_key_from_bytes -> quat_lideal_create (BASE key-load cost)."""
    c = OpCtr()
    basis_e = ((1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1))
    gens = []
    # quat_lattice_alg_elem_mul: 4 * qmul (8 muls + 4 muls by p each = 12 muls)
    for b in basis_e:
        v = qmul(b, gen, p)
        c.mul += 12
        # touch inputs/outputs
        c.touch(*gen, p, *v)
        c.limb_ops += 8 * max(1, (max(abs(x) for x in gen).bit_length() + 63) // 64)
        c.limb_ops += 4 * max(1, (p.bit_length() + 63) // 64) * max(1, (max(abs(x) for x in gen).bit_length() + 63) // 64)
        gens.append(v)
    B = hnf_mod_M(gens, N, c)
    # quat_lideal_norm: 4x4 det + isqrt
    _, det = mat4_adjugate_det(B, c)
    _ = gmpy2.isqrt(det)
    c.div += 1
    return B, c


def sim_inter_hnf_q3(B_sec, N, B_q, q):
    """SIGNER_SPEC §2.1 Q3 intersection: q*I_sec + N*I_q mod (N*q)."""
    c = OpCtr()
    gens = []
    for row in B_sec:
        gens.append([c.do_mul(q, x) for x in row])
    for row in B_q:
        gens.append([c.do_mul(N, x) for x in row])
    M = c.do_mul(N, q)
    B_prod = hnf_mod_M(gens, M, c)
    return B_prod, c


def sim_inter_dual_cref(B_sec, N, B_q, q, have_dual_sec=False):
    """PRISM_v2 quat_lideal_inter: 3 duals (2 if dual_sec precomputed) + HNF + norm."""
    c = OpCtr()
    # Dual of I_sec: adj(B_sec^T) / det(B_sec) -> denom det_sec = N^2
    if not have_dual_sec:
        adj_sec, det_sec = mat4_adjugate_det(B_sec, c)
    else:
        # still need values for scale
        c_tmp = OpCtr()
        adj_sec, det_sec = mat4_adjugate_det(B_sec, c_tmp)
    # Dual of I_q: adj(B_q^T) / det_q -> det_q = q^2
    adj_q, det_q = mat4_adjugate_det(B_q, c)
    # Scale to common denominator det_sec * det_q, take HNF
    M = c.do_mul(det_sec, det_q)
    gens = ([[c.do_mul(det_q, x) for x in row] for row in adj_sec] +
            [[c.do_mul(det_sec, x) for x in row] for row in adj_q])
    # HNF of dual sum (bounded by M = (N*q)^2)
    B_dual = hnf_mod_M(gens, M, c)
    # Dual back
    _, _ = mat4_adjugate_det(B_dual, c)
    # quat_lideal_norm: 4x4 det + isqrt
    B_prod_expected = hnf_mod_M(
        [[c.do_mul(q, x) for x in row] for row in B_sec] +
        [[c.do_mul(N, x) for x in row] for row in B_q],
        c.do_mul(N, q), OpCtr()
    )
    _, det_p = mat4_adjugate_det(B_prod_expected, c)
    _ = gmpy2.isqrt(det_p)
    c.div += 1
    return B_prod_expected, c


def _cmod(x, q):
    x %= q
    return x - q if x > q // 2 else x


def zi_mul_quat(c0, c1, u, c=None):
    if c is None:
        return [c0 * u[0] - c1 * u[1],
                c0 * u[1] + c1 * u[0],
                c0 * u[2] - c1 * u[3],
                c0 * u[3] + c1 * u[2]]
    return [c.do_mul(c0, u[0]) - c.do_mul(c1, u[1]),
            c.do_mul(c0, u[1]) + c.do_mul(c1, u[0]),
            c.do_mul(c0, u[2]) - c.do_mul(c1, u[3]),
            c.do_mul(c0, u[3]) + c.do_mul(c1, u[2])]


def inter_prereduced_zi(u_sec, v_sec, xq, yq, q):
    """Intersect I_sec (given by Z[i]-basis {u_sec, v_sec}) with I_q (xq, yq, q).

    Returns (w1, w2, OpCtr) where {w1, w2} is a Z[i]-basis of I_sec ∩ I_q.
    """
    c = OpCtr()
    # phi_q(u) = (u0 + u1*i) - (u2 + u3*i)*(xq + yq*i) mod q
    ru0 = c.do_mod(u_sec[0] - (c.do_mul(u_sec[2], xq) - c.do_mul(u_sec[3], yq)), q)
    ru1 = c.do_mod(u_sec[1] - (c.do_mul(u_sec[2], yq) + c.do_mul(u_sec[3], xq)), q)
    rv0 = c.do_mod(v_sec[0] - (c.do_mul(v_sec[2], xq) - c.do_mul(v_sec[3], yq)), q)
    rv1 = c.do_mod(v_sec[1] - (c.do_mul(v_sec[2], yq) + c.do_mul(v_sec[3], xq)), q)

    nrm_rv = c.do_mod(c.do_mul(rv0, rv0) + c.do_mul(rv1, rv1), q)
    if nrm_rv != 0:
        g, inv_nrm, _ = c.do_xgcd(nrm_rv, q)
        assert g == 1
        inv_nrm %= q
        # gamma = -r_u * conj(r_v) * inv_nrm mod q
        t0 = c.do_mod(-(c.do_mul(ru0, rv0) + c.do_mul(ru1, rv1)), q)
        t1 = c.do_mod(-(c.do_mul(ru1, rv0) - c.do_mul(ru0, rv1)), q)
        g0 = _cmod(c.do_mul(t0, inv_nrm), q)
        g1 = _cmod(c.do_mul(t1, inv_nrm), q)
        w1 = [c.do_mul(q, x) for x in v_sec]
        gv = zi_mul_quat(g0, g1, v_sec, c)
        w2 = [u_sec[r] + gv[r] for r in range(4)]
        c.touch(*w1, *w2)
        return w1, w2, c

    # Rare fallback (probability <= 2/q): try r_u invertible
    nrm_ru = c.do_mod(c.do_mul(ru0, ru0) + c.do_mul(ru1, ru1), q)
    if nrm_ru != 0:
        g, inv_nrm, _ = c.do_xgcd(nrm_ru, q)
        assert g == 1
        inv_nrm %= q
        t0 = c.do_mod(-(c.do_mul(rv0, ru0) + c.do_mul(rv1, ru1)), q)
        t1 = c.do_mod(-(c.do_mul(rv1, ru0) - c.do_mul(rv0, ru1)), q)
        g0 = _cmod(c.do_mul(t0, inv_nrm), q)
        g1 = _cmod(c.do_mul(t1, inv_nrm), q)
        w1 = [c.do_mul(q, x) for x in u_sec]
        gu = zi_mul_quat(g0, g1, u_sec, c)
        w2 = [v_sec[r] + gu[r] for r in range(4)]
        c.touch(*w1, *w2)
        return w1, w2, c

    # Zero-divisor fallback (both r_u, r_v null divisors mod q)
    raise RuntimeError("both r_u and r_v are zero-divisors mod q")


def basis_from_zi(u, v):
    """4-D Z-basis [u, i*u, v, i*v] from Z[i]-basis {u, v}."""
    iu = [-u[1], u[0], -u[3], u[2]]
    iv = [-v[1], v[0], -v[3], v[2]]
    return [list(u), iu, list(v), iv]


def gram_from_zi(u, v, p, c):
    """4x4 reduced-norm Gram matrix of [u, i*u, v, i*v] using i-isometry (20 mults)."""
    nu = nrd(u, p)
    nv = nrd(v, p)
    c.mul += 10
    h0, h1 = herm(v, u, p, c)
    return [
        [nu, 0, h0, h1],
        [0, nu, -h1, h0],
        [h0, -h1, nv, 0],
        [h1, h0, 0, nv],
    ]


def gram_cost_base(B, p):
    """Cost of computing gram_norm(B, p) directly for a general 4x4 basis."""
    c = OpCtr()
    # 10 symmetric entries, each 4 mults + 2 mults by p = 6 mults => 60 mults
    G = gram_norm(B, p)
    for r in range(4):
        for s in range(r, 4):
            for t in range(4):
                c.do_mul(B[r][t], B[s][t])
            c.do_mul(p, B[r][2] * B[s][2])
            c.do_mul(p, B[r][3] * B[s][3])
            c.touch(G[r][s])
    return G, c


def zi_gauss_uv(u0, v0, p, steps, on_step=None, keep_trace=True, prec=None):
    """Fixed number of Z[i]-Lagrange steps starting from arbitrary {u0, v0}."""
    c = Ctr(keep_trace=keep_trace)
    u = list(u0)
    v = list(v0)
    nu = nrd(u, p)
    nv = nrd(v, p)
    c.mul += 10
    bit = int(nv < nu)
    nb = 1 - bit
    u, v = ([bit * a + nb * b for a, b in zip(v, u)],
            [bit * a + nb * b for a, b in zip(u, v)])
    nu, nv = bit * nv + nb * nu, bit * nu + nb * nv
    c.sel += 10
    c.op('init')
    eff = 0
    for t in range(1, steps + 1):
        h0, h1 = herm(v, u, p, c)
        m0 = _round_q(h0, nu, prec)
        m1 = _round_q(h1, nu, prec)
        c.div += 2
        iu = (-u[1], u[0], -u[3], u[2])
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        c.mul += 8
        nv = nrd(v, p)
        c.mul += 5
        c.op('red', t)
        bit = int(nv < nu)
        nb = 1 - bit
        eff += bit
        u, v = ([bit * a + nb * b for a, b in zip(v, u)],
                [bit * a + nb * b for a, b in zip(u, v)])
        nu, nv = bit * nv + nb * nu, bit * nu + nb * nv
        c.sel += 10
        c.lovasz += 1
        c.swap += 1
        c.op('cswap', t)
        if on_step is not None:
            on_step(t, u, v, nu, nv)
    return u, v, nu, nv, c, eff
