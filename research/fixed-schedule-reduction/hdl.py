# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Higher-dimension and one-characteristic-at-a-time lattice ablation.

Research model only. Step counting in Python; no constant-time or
security claim. Nothing here is a proposal to change the signature
format.

The baseline is the 4-D H2 ideal of the PRISM challenge, reduced with
Cohen's integral LLL (delta = 99/100) under the reduced-norm form.
Each other arm changes one lattice characteristic and holds the rest
fixed, so a single table can say which characteristic buys
target-independence and which buys a low-CV trace.

Salt model (Shekyl PRISM, ePrint 2026/443 §5 with a salt):
  salt = SHAKE-256("salt" || sk || msg)[:SALT_LEN],
  SALT_LEN = 32, 48, 48, 64, 96 for P248..P664,
  q = FS(pk, salt, msg).
"""
import hashlib
from fractions import Fraction

import gmpy2
import cypari2

pari = cypari2.Pari()
try:
    pari.allocatemem(512 * 1024 * 1024, silent=True)
except Exception:
    pass

SETS = [('p248', 248, 5), ('p324', 324, 3), ('p376', 376, 65),
        ('p500', 500, 27), ('p664', 664, 17)]
SALT_LEN = {'p248': 32, 'p324': 48, 'p376': 48, 'p500': 64, 'p664': 96}
PK_BYTES = {'p248': 64, 'p324': 82, 'p376': 96, 'p500': 128, 'p664': 168}
Q_STAR_BITS = 2 * 664 - 5  # 1323
DOMAIN = b'HigherDim/v0/'

# Construction metadata: (name, characteristic, dim, solves_expected)
# "solves_expected" is what we *aim* to solve, not a measured rate.
STREAM_ARMS = [
    # baseline
    ('H2_4', 'baseline', 4, True),
    # (7) which ideal is selected — not a property of a fixed lattice,
    # but the 2-D→4-D claim needs this control (H1 kept 21–26% target η²).
    ('D0_4', 'selection', 4, True),
    ('H1_4', 'selection', 4, True),
    # (1) dimension only: same I_q, same form, extra unit-scale axes
    ('EMB6', 'dimension', 6, True),
    ('EMB8', 'dimension', 8, True),
    ('EMB12', 'dimension', 12, True),
    ('EMB16', 'dimension', 16, True),
    ('DS8', 'dimension', 8, True),
    # (2) covolume, dim stays 4
    ('SIZE_COF', 'covolume', 4, True),
    # (3) starting-basis skew, same H2 lattice
    ('SKEW_U', 'skew', 4, True),
    # (4) quadratic form
    ('FORM_EUCL', 'form', 4, True),
    # (5) arithmetic precision
    ('PREC_256', 'precision', 4, True),
    # (6) reduction parameter
    ('DELTA_75', 'delta', 4, True),
    # non-solving controls (flatness-by-ignoring / volume still tracks q)
    ('QARY8', 'ignore', 8, False),
    ('FIX8', 'ignore', 8, False),
]

PROD_ARMS = [
    ('H2_4', 'baseline', 4, True),
    ('H1_4', 'selection', 4, True),
    ('EMB8', 'dimension', 8, True),
    ('EMB12', 'dimension', 12, True),
    ('DS8', 'dimension', 8, True),
    ('SIZE_COF', 'covolume', 4, True),
    ('SKEW_U', 'skew', 4, True),
    ('FORM_EUCL', 'form', 4, True),
    ('PREC_256', 'precision', 4, True),
    ('DELTA_75', 'delta', 4, True),
]

ARM_META = {n: (ch, d, sol) for n, ch, d, sol in STREAM_ARMS}


def prime_of(e, f):
    p = f * (1 << e) - 1
    assert gmpy2.is_prime(p) and p % 4 == 3
    return p


def shake(tag: bytes, *parts: bytes, n=32) -> bytes:
    h = hashlib.shake_256(DOMAIN + tag)
    for p in parts:
        h.update(len(p).to_bytes(4, 'little'))
        h.update(p)
    return h.digest(n)


def keypair(sk, set_name):
    return shake(b'pk', sk, set_name.encode(), n=PK_BYTES[set_name])


def salt_of(sk, msg, set_name):
    return shake(b'salt', sk, msg, n=SALT_LEN[set_name])


def fs_challenge(e, *parts):
    ctr = 0
    while True:
        raw = shake(b'FS', *parts, ctr.to_bytes(4, 'little'), n=(e // 8) + 16)
        s = ((int.from_bytes(raw, 'little') & ((1 << (e - 4)) - 1)) << 1) | 1
        q = (1 << (2 * e - 5)) - s * s
        ctr += 1
        if gmpy2.is_prime(q):
            return s, q, ctr


def qmul(u, v, p):
    a1, b1, c1, d1 = u
    a2, b2, c2, d2 = v
    return (a1 * a2 - b1 * b2 - p * c1 * c2 - p * d1 * d2,
            a1 * b2 + b1 * a2 + p * c1 * d2 - p * d1 * c2,
            a1 * c2 - b1 * d2 + c1 * a2 + d1 * b2,
            a1 * d2 + b1 * c2 - c1 * b2 + d1 * a2)


def nrd(v, p):
    return v[0] ** 2 + v[1] ** 2 + p * (v[2] ** 2 + v[3] ** 2)


def hnf_rows(gens):
    M = pari.matrix(4, len(gens), [g[r] for r in range(4) for g in gens])
    H = pari.mathnf(M)
    return [[int(H[r, c]) for r in range(4)] for c in range(4)]


def sqrt_mod(a, T):
    return int(pari.lift(pari.Mod(a, T).sqrt()))


def alpha_for(T, p, y):
    a = (-p - y * y) % T
    if a == 0 or gmpy2.legendre(a, T) != 1:
        return None
    return sqrt_mod(a, T)


def ideal_basis(T, p, alpha):
    gens = [qmul(b, alpha, p) for b in
            ((1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1))]
    gens += [(T, 0, 0, 0), (0, T, 0, 0), (0, 0, T, 0), (0, 0, 0, T)]
    return hnf_rows(gens)


def gram_norm(B, p):
    w = (1, 1, p, p)
    return [[sum(B[r][t] * B[s][t] * w[t] for t in range(4))
             for s in range(4)] for r in range(4)]


def gram_eucl(B):
    k = len(B[0])
    return [[sum(B[r][t] * B[s][t] for t in range(k))
             for s in range(len(B))] for r in range(len(B))]


def det_int(B):
    n = len(B)
    return int(pari.matdet(pari.matrix(
        n, n, [B[r][c] for r in range(n) for c in range(n)])))


def matmul(A, B):
    m, k, n = len(A), len(B), len(B[0])
    return [[sum(A[r][t] * B[t][c] for t in range(k)) for c in range(n)]
            for r in range(m)]


def apply(H, B):
    return matmul(H, B)


def in_span(v, B):
    n = len(B)
    if len(v) != n:
        return False
    try:
        M = pari.matrix(n, n, [B[c][r] for r in range(n) for c in range(n)])
        col = pari.matrix(n, 1, [int(x) for x in v])
        sol = pari.matsolve(M, col)
        return all(int(pari.denominator(sol[i, 0])) == 1 for i in range(n))
    except Exception:
        return False


def crt(a, m, b, n):
    return int((a + m * ((b - a) * int(gmpy2.invert(m, n)) % n)) % (m * n))


def crt_many(pairs):
    x, m = pairs[0]
    for b, n in pairs[1:]:
        x, m = crt(x, m, b, n), m * n
    return x, m


def xof_ints(seed, target, tag, count, bits):
    nb = (bits + 7) // 8
    raw = shake(tag, seed, target.to_bytes((target.bit_length() + 7) // 8 or 1, 'little'),
                n=nb * count)
    return [int.from_bytes(raw[i * nb:(i + 1) * nb], 'little') for i in range(count)]


def select_xy(T, p, seed, tag=b'H2'):
    ctr = 0
    bits = T.bit_length() + 64
    while True:
        y, sgn = xof_ints(seed, T, tag + ctr.to_bytes(4, 'little'), 2, bits)
        y %= T
        x = alpha_for(T, p, y)
        ctr += 1
        if x is not None:
            if sgn & 1:
                x = T - x
            return x, y, ctr


def select_xy_d0(T, p):
    y = 0
    while True:
        x = alpha_for(T, p, y)
        if x is not None:
            return min(x, T - x), y, y + 1
        y += 1


def secret_class(sk, p, nb):
    ctr = 0
    while True:
        raw = shake(b'sec-N', sk, ctr.to_bytes(4, 'little'), n=nb // 8 + 4)
        N = int.from_bytes(raw, 'little') & ((1 << nb) - 1)
        N |= (1 << (nb - 1)) | 1
        ctr += 1
        if gmpy2.is_prime(N):
            break
    ctr = 0
    while True:
        yN = int.from_bytes(shake(b'sec-y', sk, ctr.to_bytes(4, 'little'),
                                  n=nb // 8 + 4), 'little') % N
        xN = alpha_for(N, p, yN)
        ctr += 1
        if xN is not None:
            return N, xN, yN


def ideal_from_classes(p, classes):
    x, T = crt_many([(c[0], c[2]) for c in classes])
    y, T2 = crt_many([(c[1], c[2]) for c in classes])
    assert T == T2
    return ideal_basis(T, p, (x, y, 1, 0)), T


def next_prime_from_salt(start, salt, tag, max_hops=10_000):
    if start < 3:
        return 1, 0
    off = int.from_bytes(shake(tag, salt, n=16), 'little') % 4096
    n = start + off
    if n % 2 == 0:
        n += 1
    hops = 0
    while hops < max_hops:
        if gmpy2.is_prime(n):
            return int(n), hops
        n += 2
        hops += 1
    return int(gmpy2.next_prime(n)), hops


def unimodular_from_xof(seed, T, bits=128, dim=4):
    """Unipotent L*R times a salt permutation, det ±1, for any dim >= 2."""
    n = dim
    n_s = n * (n - 1)  # strictly lower + strictly upper
    v = xof_ints(seed, T, b'uni' + bytes([n]), n_s + 1, bits + 1)
    s = [(x >> 1) * (1 if x & 1 else -1) for x in v[:n_s]]
    L = [[int(r == c) for c in range(n)] for r in range(n)]
    R = [[int(r == c) for c in range(n)] for r in range(n)]
    k = 0
    for r in range(n):
        for c in range(r):
            L[r][c] = s[k]
            k += 1
    for r in range(n):
        for c in range(r + 1, n):
            R[r][c] = s[k]
            k += 1
    U = matmul(L, R)
    perm = list(range(n))
    z = v[n_s]
    for i in range(n - 1, 0, -1):
        j = z % (i + 1)
        z //= (i + 1)
        perm[i], perm[j] = perm[j], perm[i]
    return [U[i] for i in perm]


def orthogonality_defect(G):
    """log2(prod ||b_i|| / vol), vol = sqrt(det G). None if det <= 0."""
    n = len(G)
    try:
        d = det_int(G)
        if d <= 0:
            return None
        # prod of Euclidean lengths from the Gram diagonal
        # use bit-length proxy via logs: sum 0.5 log2(G_ii) - 0.5 log2(det)
        s = 0.0
        for i in range(n):
            if G[i][i] <= 0:
                return None
            s += (G[i][i].bit_length() - 1)
        # s ≈ sum log2(||b_i||^2) = 2 sum log2(||b_i||)
        return round(0.5 * s - 0.5 * (d.bit_length() - 1), 2)
    except Exception:
        return None


# ------------------------------------------------------------------ LLL
def lll_gram(G0, delta=(99, 100), max_iter=None):
    """Cohen Alg. 2.6.7 on a Gram matrix of any dimension."""
    n = len(G0)
    H = [[int(r == c) for c in range(n)] for r in range(n)]

    def ip(a, b):
        ha, hb = H[a], H[b]
        return sum(ha[r] * G0[r][s] * hb[s]
                   for r in range(n) for s in range(n) if ha[r] and hb[s])

    d = [0] * (n + 1)
    d[0] = 1
    lam = [[0] * n for _ in range(n)]
    tr = dict(iterations=0, lovasz=0, swaps=0, sizered=0, redi=0, failed=0)
    dn, dd = delta
    if max_iter is None:
        max_iter = 20_000 * n

    def compute_row(k):
        for j in range(k + 1):
            u = ip(k, j)
            for i in range(j):
                if d[i] == 0:
                    tr['failed'] = 1
                    return
                u = (d[i + 1] * u - lam[k][i] * lam[j][i]) // d[i]
            if j < k:
                lam[k][j] = u
            else:
                d[k + 1] = u

    def redi(k, l):
        tr['redi'] += 1
        if d[l + 1] == 0:
            tr['failed'] = 1
            return
        if 2 * abs(lam[k][l]) > d[l + 1]:
            q = (2 * lam[k][l] + d[l + 1]) // (2 * d[l + 1])
            H[k] = [a - q * b for a, b in zip(H[k], H[l])]
            lam[k][l] -= q * d[l + 1]
            for i in range(l):
                lam[k][i] -= q * lam[l][i]
            tr['sizered'] += 1

    def swapi(k, kmax):
        tr['swaps'] += 1
        H[k], H[k - 1] = H[k - 1], H[k]
        for j in range(k - 1):
            lam[k][j], lam[k - 1][j] = lam[k - 1][j], lam[k][j]
        lm = lam[k][k - 1]
        if d[k] == 0:
            tr['failed'] = 1
            return
        Bv = (d[k - 1] * d[k + 1] + lm * lm) // d[k]
        for i in range(k + 1, kmax + 1):
            t = lam[i][k]
            lam[i][k] = (d[k + 1] * lam[i][k - 1] - lm * t) // d[k]
            if d[k + 1] == 0:
                tr['failed'] = 1
                return
            lam[i][k - 1] = (Bv * t + lm * lam[i][k]) // d[k + 1]
        d[k] = Bv

    compute_row(0)
    k, kmax = 1, 0
    while k < n and tr['iterations'] < max_iter and not tr['failed']:
        tr['iterations'] += 1
        if k > kmax:
            kmax = k
            compute_row(k)
            if tr['failed'] or d[k + 1] == 0:
                tr['failed'] = 1
                break
        redi(k, k - 1)
        tr['lovasz'] += 1
        if dd * d[k + 1] * d[k - 1] < dn * d[k] * d[k] - dd * lam[k][k - 1] ** 2:
            swapi(k, kmax)
            if tr['failed']:
                break
            k = max(1, k - 1)
        else:
            for l in range(k - 2, -1, -1):
                redi(k, l)
            k += 1
    if k < n:
        tr['failed'] = 1
    return H, tr


def _gram_pd(G):
    try:
        n = len(G)
        for k in range(1, n + 1):
            d = int(pari.matdet(pari.matrix(
                k, k, [G[r][c] for r in range(k) for c in range(k)])))
            if d <= 0:
                return False
        return True
    except Exception:
        return False


def _quantize_gram(G, prec_bits):
    n = len(G)
    mx = max(abs(G[r][c]) for r in range(n) for c in range(n))
    if mx == 0:
        return [row[:] for row in G]
    num = 1 << prec_bits
    Gp = [[int(Fraction(G[r][c] * num, mx)) for c in range(n)] for r in range(n)]
    for i in range(n):
        if Gp[i][i] <= 0:
            Gp[i][i] = 1
        for j in range(i):
            Gp[i][j] = Gp[j][i] = (Gp[i][j] + Gp[j][i]) // 2
    return Gp


def qary_basis(n, mod, seed, tag=b'qary'):
    """Rank-n q-ary lattice {v : v_0 + a·v' ≡ 0 mod mod}, covolume = mod."""
    ctr = 0
    while True:
        a = [x % mod for x in xof_ints(seed, mod, tag + ctr.to_bytes(4, 'little'),
                                       n, mod.bit_length() + 64)]
        ctr += 1
        if gmpy2.gcd(a[0], mod) == 1:
            inv = int(gmpy2.invert(a[0], mod))
            B = [[0] * n for _ in range(n)]
            B[0][0] = mod
            for t in range(1, n):
                B[t][0] = ((-a[t] * inv) % mod)
                B[t][t] = 1
            return B


def embed_extend(B4, n, salt, T, m=1 << 16):
    """Rank-n extension of a 4-D basis: (b_i, 0) plus (alpha_j, m e_j),
    alpha_j a salt integer combination of the b_i, hence in the same
    Z-module. Extra axes have fixed scale m, independent of q.
    Returns n basis rows in Z^n, and the 4-D projection of those rows.
    """
    extra = n - 4
    assert extra >= 1
    coeffs = xof_ints(salt, T, b'emb-a' + bytes([n]), extra * 4, 16)
    rows = []
    for i in range(4):
        rows.append(list(B4[i]) + [0] * extra)
    for j in range(extra):
        alpha = [0, 0, 0, 0]
        for t in range(4):
            c = (coeffs[j * 4 + t] % 5) - 2  # small mix in {-2..2}
            for k in range(4):
                alpha[k] += c * B4[t][k]
        tail = [0] * extra
        tail[j] = m
        rows.append(list(alpha) + tail)
    return rows


def block_sum_basis(B1, B2):
    """8-D block-diagonal basis of L1 ⊕ L2."""
    rows = []
    for i in range(4):
        rows.append(list(B1[i]) + [0, 0, 0, 0])
    for i in range(4):
        rows.append([0, 0, 0, 0] + list(B2[i]))
    return rows


def gram_block_norm(B8, p):
    """Norm form nrd on each 4-block, no cross terms."""
    def nrd_dot(u, v):
        w = (1, 1, p, p)
        return sum(u[t] * v[t] * w[t] for t in range(4))
    G = [[0] * 8 for _ in range(8)]
    for r in range(8):
        for s in range(8):
            G[r][s] = (nrd_dot(B8[r][:4], B8[s][:4])
                       + nrd_dot(B8[r][4:], B8[s][4:]))
    return G


def gram_embed(Bn, p, m, extra):
    """nrd on the first 4 coordinates, m^2 Euclidean on the extra axes."""
    n = len(Bn)
    w4 = (1, 1, p, p)
    G = [[0] * n for _ in range(n)]
    for r in range(n):
        for s in range(n):
            g = sum(Bn[r][t] * Bn[s][t] * w4[t] for t in range(4))
            g += sum(Bn[r][4 + t] * Bn[s][4 + t] * (m * m) for t in range(extra))
            G[r][s] = g
    return G


def _result(tr, ok_q, ok_in, defect, dim, extra=None):
    d = dict(iterations=tr['iterations'], swaps=tr['swaps'],
             sizered=tr['sizered'], lovasz=tr['lovasz'],
             failed=int(tr.get('failed', 0)),
             ok_q=int(ok_q), ok_in=int(ok_in),
             defect=defect, dim=dim)
    if extra:
        d.update(extra)
    return d


def _fail_result(dim, extra=None):
    return _result(dict(iterations=0, swaps=0, sizered=0, lovasz=0, failed=1),
                   0, 0, None, dim, extra)


def stream_job_data(sk, msg, set_name, e, f):
    p = prime_of(e, f)
    pk = keypair(sk, set_name)
    salt = salt_of(sk, msg, set_name)
    s, q, qtries = fs_challenge(e, pk, salt, msg)
    seed = shake(b'h2-seed', salt, q.to_bytes((q.bit_length() + 7) // 8, 'little'))
    return dict(p=p, pk=pk, salt=salt, s=s, q=q, qtries=qtries, seed=seed)


def _h2_basis(data):
    p, q, seed = data['p'], data['q'], data['seed']
    x, y, tries = select_xy(q, p, seed)
    B = ideal_basis(q, p, (x, y, 1, 0))
    return B, tries


def _eval_4d(B, p, q, G, tr_H=None, delta=(99, 100)):
    if tr_H is None:
        H, tr = lll_gram(G, delta=delta)
    else:
        H, tr = tr_H
    if tr.get('failed'):
        return _fail_result(4, dict(defect=orthogonality_defect(G)))
    beta = apply(H, B)[0]
    ok_q = nrd(beta, p) % q == 0 and nrd(beta, p) > 0
    ok_in = in_span(beta, B)
    return _result(tr, ok_q, ok_in, orthogonality_defect(G), 4)


def run_stream_arm(name, data):
    p, q, salt, seed = data['p'], data['q'], data['salt'], data['seed']
    ch, dim, _ = ARM_META[name]

    if name == 'H2_4':
        B, tries = _h2_basis(data)
        return _eval_4d(B, p, q, gram_norm(B, p)), tries

    if name == 'D0_4':
        x, y, tries = select_xy_d0(q, p)
        B = ideal_basis(q, p, (x, y, 1, 0))
        return _eval_4d(B, p, q, gram_norm(B, p)), tries

    if name == 'H1_4':
        # Historical H1: mask of the *D0* ideal (same lattice as D0, not H2).
        x, y, tries = select_xy_d0(q, p)
        B0 = ideal_basis(q, p, (x, y, 1, 0))
        U = unimodular_from_xof(seed, q, bits=128, dim=4)
        B = matmul(U, B0)
        return _eval_4d(B, p, q, gram_norm(B, p)), tries

    if name == 'SKEW_U':
        # Same H2 lattice, only the starting basis changes.
        B0, tries = _h2_basis(data)
        U = unimodular_from_xof(salt, q, bits=128, dim=4)
        B = matmul(U, B0)
        return _eval_4d(B, p, q, gram_norm(B, p)), tries

    if name == 'FORM_EUCL':
        B, tries = _h2_basis(data)
        G = gram_eucl(B)
        H, tr = lll_gram(G)
        if tr.get('failed'):
            return _fail_result(4), tries
        beta = apply(H, B)[0]
        ok_q = nrd(beta, p) % q == 0 and nrd(beta, p) > 0
        return _result(tr, ok_q, in_span(beta, B), orthogonality_defect(G), 4), tries

    if name == 'PREC_256':
        B, tries = _h2_basis(data)
        G = gram_norm(B, p)
        Gp = _quantize_gram(G, 256)
        if not _gram_pd(Gp):
            return _fail_result(4, dict(defect=orthogonality_defect(G))), tries
        H, tr = lll_gram(Gp)
        if tr.get('failed'):
            return _fail_result(4), tries
        beta = apply(H, B)[0]
        ok_q = nrd(beta, p) % q == 0 and nrd(beta, p) > 0
        return _result(tr, ok_q, in_span(beta, B), orthogonality_defect(G), 4), tries

    if name == 'DELTA_75':
        B, tries = _h2_basis(data)
        return _eval_4d(B, p, q, gram_norm(B, p), delta=(3, 4)), tries

    if name == 'SIZE_COF':
        xq, yq, tries = select_xy(q, p, seed)
        r0 = max(1, (1 << Q_STAR_BITS) // q)
        r, hops = next_prime_from_salt(r0, salt, b'cof-r')
        if r == 1:
            B = ideal_basis(q, p, (xq, yq, 1, 0))
        else:
            xr, yr, tr2 = select_xy(r, p, shake(b'cof-xy', salt), tag=b'Hr')
            B, _T = ideal_from_classes(p, [(xq, yq, q), (xr, yr, r)])
            tries += tr2
        out = _eval_4d(B, p, q, gram_norm(B, p))
        out['r_bits'] = 1 if r == 1 else r.bit_length()
        return out, tries + hops

    if name in ('EMB6', 'EMB8', 'EMB12', 'EMB16'):
        n = int(name[3:])
        B4, tries = _h2_basis(data)
        m = 1 << 16
        Bn = embed_extend(B4, n, salt, q, m=m)
        G = gram_embed(Bn, p, m, n - 4)
        H, tr = lll_gram(G)
        if tr.get('failed'):
            return _fail_result(n, dict(defect=orthogonality_defect(G))), tries
        # Extra axes are short (fixed m), so the first reduced vectors may
        # have a zero projection. Search the whole reduced basis for a
        # nonzero projection that lies in I_q and satisfies q | nrd.
        ok_q = ok_in = False
        for vec in apply(H, Bn):
            proj = vec[:4]
            if all(x == 0 for x in proj):
                continue
            if in_span(proj, B4):
                ok_in = True
                if nrd(proj, p) % q == 0 and nrd(proj, p) > 0:
                    ok_q = True
                    break
        return _result(tr, ok_q, ok_in, orthogonality_defect(G), n), tries

    if name == 'DS8':
        Bq, tries = _h2_basis(data)
        # Second block: a fresh H2-style ideal of the *same* q, other class,
        # so volume matches the baseline; only dimension (and a second copy)
        # changes. A salt-shifted seed makes a different (x, y).
        seed2 = shake(b'ds8-seed', salt, seed)
        x2, y2, t2 = select_xy(q, p, seed2, tag=b'H2b')
        Br = ideal_basis(q, p, (x2, y2, 1, 0))
        B8 = block_sum_basis(Bq, Br)
        G = gram_block_norm(B8, p)
        H, tr = lll_gram(G)
        if tr.get('failed'):
            return _fail_result(8, dict(defect=orthogonality_defect(G))), tries + t2
        red = apply(H, B8)
        # A reduced vector solves the q-problem if either 4-block is in I_q
        # or I_r (both have norm q).
        ok_q = ok_in = False
        for vec in red:
            p1, p2 = vec[:4], vec[4:]
            if nrd(p1, p) % q == 0 and nrd(p1, p) > 0 and in_span(p1, Bq):
                ok_q = ok_in = True
                break
            if nrd(p2, p) % q == 0 and nrd(p2, p) > 0 and in_span(p2, Br):
                ok_q = ok_in = True
                break
        return _result(tr, ok_q, ok_in, orthogonality_defect(G), 8), tries + t2

    if name == 'QARY8':
        B = qary_basis(8, q, seed, tag=b'qary8')
        G = gram_eucl(B)
        H, tr = lll_gram(G)
        if tr.get('failed'):
            return _fail_result(8), 0
        # Not an ideal lattice: ok_q is whether a projected 4-tuple
        # accidentally satisfies the norm equation — expected rare.
        beta = apply(H, B)[0]
        proj = beta[:4]
        ok_q = nrd(proj, p) % q == 0 and nrd(proj, p) > 0
        return _result(tr, ok_q, False, orthogonality_defect(G), 8), 0

    if name == 'FIX8':
        # Constant-volume 8-D q-ary lattice, modulus independent of q.
        B = qary_basis(8, 3329, seed, tag=b'fix8')
        G = gram_eucl(B)
        H, tr = lll_gram(G)
        if tr.get('failed'):
            return _fail_result(8), 0
        beta = apply(H, B)[0]
        proj = beta[:4]
        ok_q = nrd(proj, p) % q == 0 and nrd(proj, p) > 0
        return _result(tr, ok_q, False, orthogonality_defect(G), 8), 0

    raise ValueError(name)


def run_prod_arm(name, sk, data):
    """Same arms, on the secret product I_sec · I_q (or I_sec ⊕ I_q for DS8)."""
    p, q, salt, seed = data['p'], data['q'], data['salt'], data['seed']
    N, xN, yN = secret_class(sk, p, p.bit_length() // 2)
    xq, yq, tries = select_xy(q, p, seed)
    Bprod, T = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
    pdata = dict(data)
    # Reuse stream machinery where the arm only needs a 4-D basis in place
    # of I_q. We pass a shim by running specialized cases.

    def eval_on(B, **kw):
        G = kw.get('G', gram_norm(B, p))
        delta = kw.get('delta', (99, 100))
        H, tr = lll_gram(kw['Gp'] if 'Gp' in kw else G, delta=delta)
        if tr.get('failed'):
            return _fail_result(kw.get('dim', 4), dict(N_bits=N.bit_length()))
        if 'Bn' in kw:
            beta = apply(H, kw['Bn'])[0]
            proj = beta[:4]
            ok_q = nrd(proj, p) % q == 0 and nrd(proj, p) > 0
            ok_in = in_span(proj, B)
            dim = kw['dim']
            defect = orthogonality_defect(kw.get('G8', G))
        else:
            beta = apply(H, B)[0]
            ok_q = nrd(beta, p) % q == 0 and nrd(beta, p) > 0
            ok_in = in_span(beta, B)
            dim = 4
            defect = orthogonality_defect(G)
        out = _result(tr, ok_q, ok_in, defect, dim, dict(N_bits=N.bit_length()))
        return out

    if name == 'H2_4':
        return eval_on(Bprod), tries
    if name == 'H1_4':
        # Mask of the *product* lattice (skew/selection analogue).
        U = unimodular_from_xof(seed, T, bits=128, dim=4)
        return eval_on(matmul(U, Bprod)), tries
    if name == 'SKEW_U':
        U = unimodular_from_xof(salt, T, bits=128, dim=4)
        return eval_on(matmul(U, Bprod)), tries
    if name == 'FORM_EUCL':
        return eval_on(Bprod, G=gram_eucl(Bprod)), tries
    if name == 'PREC_256':
        G = gram_norm(Bprod, p)
        Gp = _quantize_gram(G, 256)
        if not _gram_pd(Gp):
            return _fail_result(4, dict(N_bits=N.bit_length())), tries
        return eval_on(Bprod, Gp=Gp, G=G), tries
    if name == 'DELTA_75':
        return eval_on(Bprod, delta=(3, 4)), tries
    if name == 'SIZE_COF':
        r0 = max(1, (1 << Q_STAR_BITS) // q)
        r, hops = next_prime_from_salt(r0, salt, b'pcof-r')
        classes = [(xN, yN, N), (xq, yq, q)]
        if r > 1:
            xr, yr, t2 = select_xy(r, p, shake(b'pcof-xy', salt), tag=b'Hr')
            classes.append((xr, yr, r))
            tries += t2
        B, _ = ideal_from_classes(p, classes)
        out = eval_on(B)
        out['r_bits'] = 1 if r == 1 else r.bit_length()
        return out, tries + hops
    if name in ('EMB8', 'EMB12'):
        n = int(name[3:])
        m = 1 << 16
        Bn = embed_extend(Bprod, n, salt, T, m=m)
        G = gram_embed(Bn, p, m, n - 4)
        H, tr = lll_gram(G)
        if tr.get('failed'):
            return _fail_result(n, dict(N_bits=N.bit_length())), tries
        ok_q = ok_in = False
        for vec in apply(H, Bn):
            proj = vec[:4]
            if all(x == 0 for x in proj):
                continue
            if in_span(proj, Bprod):
                ok_in = True
                if nrd(proj, p) % q == 0 and nrd(proj, p) > 0:
                    ok_q = True
                    break
        return _result(tr, ok_q, ok_in, orthogonality_defect(G), n,
                      dict(N_bits=N.bit_length())), tries
    if name == 'DS8':
        # I_sec ⊕ I_q: secret in one block, challenge in the other.
        Bsec = ideal_basis(N, p, (xN, yN, 1, 0))
        Bq = ideal_basis(q, p, (xq, yq, 1, 0))
        B8 = block_sum_basis(Bsec, Bq)
        G = gram_block_norm(B8, p)
        H, tr = lll_gram(G)
        if tr.get('failed'):
            return _fail_result(8, dict(N_bits=N.bit_length())), tries
        red = apply(H, B8)
        ok_q = ok_in = False
        for vec in red:
            p2 = vec[4:]
            if nrd(p2, p) % q == 0 and nrd(p2, p) > 0 and in_span(p2, Bq):
                ok_q = ok_in = True
                break
        out = _result(tr, ok_q, ok_in, orthogonality_defect(G), 8,
                      dict(N_bits=N.bit_length()))
        return out, tries
    raise ValueError(name)
