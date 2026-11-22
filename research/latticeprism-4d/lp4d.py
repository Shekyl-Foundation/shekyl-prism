# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""4-D hash-expanded lattice embeddings of PRISM targets, with instrumented LLL.

Research model only: step counting in Python, no constant-time claim.

Quaternion algebra B = (-1, -p): i^2 = -1, j^2 = -p, k = ij; order
O = Z<1, i, j, k> (index 4 in the maximal order O0 for p = 3 mod 4; the
index does not matter for step counting). Reduced norm
nrd(a + bi + cj + dk) = a^2 + b^2 + p(c^2 + d^2).

Embeddings of a target T (a prime, public in PRISM's case):
  D0   left ideal I = O*alpha + O*T, alpha = x + y*i + j with
       x^2 + y^2 = -p (mod T), y the smallest that works: a pure function
       of T. HNF basis, norm-form Gram. Reduced first vector beta has
       T | nrd(beta), nrd(beta)/T ~ sqrt(p): the SQIsign/PRISM
       "equivalent ideal of small norm" step. This is the real 4-D problem.
  H1   same ideal as D0, basis multiplied by a unimodular U derived from
       XOF(seed || T || "H1"): the 4-D analogue of 2023/807's mask, but public.
  H2   ideal chosen by XOF(seed || T || "H2"): y = XOF mod T (retry with a
       counter), root sign from XOF. Different, equally valid ideal of norm T.
  H3   q-ary lattice {v in Z^4 : <a, v> = 0 mod T}, a = XOF(seed||T||"H3"),
       Euclidean norm. Covolume T. Encodes nothing about the norm equation.
  H3s  same with a small fixed modulus Q = 3329: covolume constant.
       Encodes nothing about T beyond the hash.
"""
import hashlib, secrets
from fractions import Fraction
import gmpy2
import cypari2

pari = cypari2.Pari()
pari.allocatemem(512 * 1024 * 1024, silent=True) if hasattr(pari, 'allocatemem') else None

# ------------------------------------------------------------------ parameters
# (name, e, f) from prism-field/src/lib.rs parameter_set! invocations and
# docs/PARAMETER_SETS.md; p = f*2^e - 1.  Challenge degree q = 2^(2e-5) - s^2,
# s odd with e-4 free bits (docs/VERIFIER_SPEC.md).
SETS = [('p248', 248, 5), ('p324', 324, 3), ('p376', 376, 65),
        ('p500', 500, 27), ('p664', 664, 17)]


def prime_of(e, f):
    p = f * (1 << e) - 1
    assert gmpy2.is_prime(p) and p % 4 == 3
    return p


# ------------------------------------------------------------------ XOF
def xof(seed: bytes, target: int, tag: bytes, n: int, kind='shake') -> bytes:
    tb = target.to_bytes((target.bit_length() + 7) // 8 + 1, 'little')
    msg = b'LatticePrism/v0/' + tag + b'/' + len(seed).to_bytes(2, 'little') + seed + tb
    if kind == 'blake3':
        import blake3
        return blake3.blake3(msg).digest(length=n)
    return hashlib.shake_256(msg).digest(n)


def xof_ints(seed, target, tag, count, bits, kind='shake'):
    nb = (bits + 7) // 8
    raw = xof(seed, target, tag, nb * count, kind)
    return [int.from_bytes(raw[i * nb:(i + 1) * nb], 'little') for i in range(count)]


def seed_from_pk(pk: bytes) -> bytes:
    return hashlib.shake_256(b'LatticePrism/v0/seed' + pk).digest(32)


FIXED_SEED = hashlib.shake_256(b'LatticePrism/v0/fixed-seed').digest(32)

# ------------------------------------------------------------------ quaternions
def qmul(u, v, p):
    a1, b1, c1, d1 = u; a2, b2, c2, d2 = v
    return (a1*a2 - b1*b2 - p*c1*c2 - p*d1*d2,
            a1*b2 + b1*a2 + p*c1*d2 - p*d1*c2,
            a1*c2 - b1*d2 + c1*a2 + d1*b2,
            a1*d2 + b1*c2 - c1*b2 + d1*a2)


def nrd(v, p):
    return v[0]**2 + v[1]**2 + p * (v[2]**2 + v[3]**2)


def hnf_rows(gens):
    """Z-basis (rows) of the lattice spanned by integer rows `gens` (PARI mathnf)."""
    M = pari.matrix(4, len(gens), [g[r] for r in range(4) for g in gens])
    H = pari.mathnf(M)
    cols = [[int(H[r, c]) for r in range(4)] for c in range(4)]
    return cols


def sqrt_mod(a, T):
    r = pari.Mod(a, T).sqrt()
    return int(pari.lift(r))


def alpha_for(T, p, y):
    """alpha = x + y i + j with nrd(alpha) = 0 mod T, or None if -p-y^2 is a non-residue."""
    a = (-p - y * y) % T
    if a == 0 or gmpy2.legendre(a, T) != 1:
        return None
    x = sqrt_mod(a, T)
    return x


def ideal_basis(T, p, alpha):
    gens = [qmul(b, alpha, p) for b in [(1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1)]]
    gens += [(T, 0, 0, 0), (0, T, 0, 0), (0, 0, T, 0), (0, 0, 0, T)]
    return hnf_rows(gens)


def gram_norm(B, p):
    w = (1, 1, p, p)
    return [[sum(B[r][t] * B[s][t] * w[t] for t in range(4)) for s in range(4)] for r in range(4)]


def gram_eucl(B):
    return [[sum(B[r][t] * B[s][t] for t in range(4)) for s in range(4)] for r in range(4)]


def det4(B):
    return int(pari.matdet(pari.matrix(4, 4, [B[r][c] for r in range(4) for c in range(4)])))


# ------------------------------------------------------------------ embeddings
def embed_D0(T, p):
    y = 0
    while True:
        x = alpha_for(T, p, y)
        if x is not None:
            x = min(x, T - x)
            return ideal_basis(T, p, (x, y, 1, 0)), dict(alpha_tries=y + 1)
        y += 1


def unimodular_from_xof(seed, T, bits=128, kind='shake'):
    v = xof_ints(seed, T, b'H1', 13, bits + 1, kind)
    s = [(x >> 1) * (1 if x & 1 else -1) for x in v[:12]]
    L = [[1, 0, 0, 0], [s[0], 1, 0, 0], [s[1], s[2], 1, 0], [s[3], s[4], s[5], 1]]
    R = [[1, s[6], s[7], s[8]], [0, 1, s[9], s[10]], [0, 0, 1, s[11]], [0, 0, 0, 1]]
    U = [[sum(L[r][t] * R[t][c] for t in range(4)) for c in range(4)] for r in range(4)]
    perm = list(range(4)); z = v[12]
    for i in range(3, 0, -1):            # Fisher-Yates from the XOF
        j = z % (i + 1); z //= (i + 1); perm[i], perm[j] = perm[j], perm[i]
    return [U[i] for i in perm]


def matmul(A, B):
    return [[sum(A[r][t] * B[t][c] for t in range(4)) for c in range(4)] for r in range(4)]


def embed_H1(T, p, seed, kind='shake', B0=None):
    B = B0 if B0 is not None else embed_D0(T, p)[0]
    return matmul(unimodular_from_xof(seed, T, kind=kind), B), {}


def embed_H2(T, p, seed, kind='shake'):
    ctr = 0
    while True:
        y, sgn = xof_ints(seed, T + 0, b'H2' + ctr.to_bytes(4, 'little'), 2, T.bit_length() + 64, kind)
        y %= T
        x = alpha_for(T, p, y)
        ctr += 1
        if x is not None:
            if sgn & 1:
                x = T - x
            return ideal_basis(T, p, (x, y, 1, 0)), dict(alpha_tries=ctr)


def qary_basis(a, mod):
    inv = int(gmpy2.invert(a[0], mod))
    c = [(-a[t] * inv) % mod for t in (1, 2, 3)]
    return [[mod, 0, 0, 0], [c[0], 1, 0, 0], [c[1], 0, 1, 0], [c[2], 0, 0, 1]]


def embed_H3(T, seed, kind='shake', mod=None):
    m = T if mod is None else mod
    ctr = 0
    while True:
        a = [x % m for x in xof_ints(seed, T, b'H3' + ctr.to_bytes(4, 'little'), 4, m.bit_length() + 64, kind)]
        ctr += 1
        if gmpy2.gcd(a[0], m) == 1:
            return qary_basis(a, m), dict(alpha_tries=ctr)


# ------------------------------------------------------------------ integral LLL
def lll_gram(G0, delta=(99, 100)):
    """Cohen Alg. 2.6.7 (integral LLL on a Gram matrix), delta = 99/100.
    Returns (H, trace): H unimodular with reduced basis = H * B.
    Trace counts: iterations (main-loop passes), lovasz (Lovasz checks),
    swaps, sizered (size-reductions with q != 0), redi (size-reduction calls)."""
    n = len(G0)
    H = [[int(r == c) for c in range(n)] for r in range(n)]

    def ip(a, b):
        ha, hb = H[a], H[b]
        return sum(ha[r] * G0[r][s] * hb[s] for r in range(n) for s in range(n) if ha[r] and hb[s])

    d = [0] * (n + 1); d[0] = 1
    lam = [[0] * n for _ in range(n)]
    tr = dict(iterations=0, lovasz=0, swaps=0, sizered=0, redi=0)
    dn, dd = delta

    def compute_row(k):
        for j in range(k + 1):
            u = ip(k, j)
            for i in range(j):
                u = (d[i + 1] * u - lam[k][i] * lam[j][i]) // d[i]
            if j < k:
                lam[k][j] = u
            else:
                d[k + 1] = u

    def redi(k, l):
        tr['redi'] += 1
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
        Bv = (d[k - 1] * d[k + 1] + lm * lm) // d[k]
        for i in range(k + 1, kmax + 1):
            t = lam[i][k]
            lam[i][k] = (d[k + 1] * lam[i][k - 1] - lm * t) // d[k]
            lam[i][k - 1] = (Bv * t + lm * lam[i][k]) // d[k + 1]
        d[k] = Bv

    compute_row(0)
    k, kmax = 1, 0
    while k < n:
        tr['iterations'] += 1
        if k > kmax:
            kmax = k
            compute_row(k)
        redi(k, k - 1)
        tr['lovasz'] += 1
        # d_{k+1} d_{k-1} < (delta d_k^2 - lam^2)  in Cohen's integral form
        if dd * d[k + 1] * d[k - 1] < dn * d[k] * d[k] - dd * lam[k][k - 1] ** 2:
            swapi(k, kmax)
            k = max(1, k - 1)
        else:
            for l in range(k - 2, -1, -1):
                redi(k, l)
            k += 1
    return H, tr


def apply(H, B):
    return [[sum(H[r][t] * B[t][c] for t in range(4)) for c in range(4)] for r in range(4)]


# ------------------------------------------------------------------ fixed-schedule control
def _gso(G):
    n = len(G); mu = [[Fraction(0)] * n for _ in range(n)]; Bn = [Fraction(0)] * n
    for i in range(n):
        for j in range(i):
            mu[i][j] = (Fraction(G[i][j]) - sum(mu[j][t] * mu[i][t] * Bn[t] for t in range(j))) / Bn[j]
        Bn[i] = Fraction(G[i][i]) - sum(mu[i][t] ** 2 * Bn[t] for t in range(i))
    return mu, Bn


def fixed_schedule(G0, max_tours=10_000, delta=Fraction(99, 100)):
    """Sweep LLL run as fixed tours. Each tour: for k = 1..3, size-reduce b_k
    against b_{k-1}, ..., b_0 (6 reductions per tour, executed even when the
    rounded coefficient is 0) and one Lovasz check with a conditional swap
    (3 per tour). A constant-time version runs a FIXED number of tours T, so
    its operation count is 12*T for every input. Here we run until a tour
    changes nothing and return that tour count: the T a fixed schedule
    must cover. Returns (G_reduced, tours_until_stable)."""
    G = [row[:] for row in G0]; n = 4
    for t in range(max_tours):
        changed = False
        for k in range(1, n):
            for l in range(k - 1, -1, -1):
                mu, Bn = _gso(G)
                q = round(mu[k][l])
                if q:
                    changed = True
                    gkk = G[k][k] - 2 * q * G[k][l] + q * q * G[l][l]
                    for c in range(n):
                        if c != k:
                            G[k][c] -= q * G[l][c]
                            G[c][k] = G[k][c]
                    G[k][k] = gkk
            mu, Bn = _gso(G)
            if Bn[k] < (delta - mu[k][k - 1] ** 2) * Bn[k - 1]:
                changed = True
                G[k], G[k - 1] = G[k - 1], G[k]
                for row in G:
                    row[k], row[k - 1] = row[k - 1], row[k]
        if not changed:
            return G, t + 1
    return G, None
