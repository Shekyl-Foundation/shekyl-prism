# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Salt-reshaped 4-D lattices for PRISM targets. Research model only.

Step counting in Python; no constant-time or security claim. Nothing here
is a proposal to change the signature format.

The 4-D harness (research/latticeprism-4d, on the sibling experiment
branch) found that LLL on a left ideal of norm q costs about 1.5 steps
per bit of q, across the Shekyl sets P248..P664. This module tests
whether a salt that *reshapes* the grid — rather than only picking which
ideal of that norm to reduce — can cut that size-to-step link.

Salt model (Shekyl PRISM, ePrint 2026/443 §5 with a salt):
  salt = SHAKE-256("salt" || sk || msg)[:SALT_LEN],
  SALT_LEN = width_bits(e)/8 = 32, 48, 48, 64, 96 for P248..P664,
  q = FS(pk, salt, msg). The salt is carried in the signature.
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

# (name, e, f) from prism-field; p = f*2^e - 1.
SETS = [('p248', 248, 5), ('p324', 324, 3), ('p376', 376, 65),
        ('p500', 500, 27), ('p664', 664, 17)]

# prism-field: SALT_LEN = width_bits(e)/8, width_bits = 128*ceil(e/128).
SALT_LEN = {'p248': 32, 'p324': 48, 'p376': 48, 'p500': 64, 'p664': 96}
PK_BYTES = {'p248': 64, 'p324': 82, 'p376': 96, 'p500': 128, 'p664': 168}

# Typical challenge bit length 2e-5, and a global product-norm target
# (secret-norm bits ~ e/2 plus P664's q).
Q_STAR_BITS = 2 * 664 - 5          # 1323
PROD_STAR_BITS = Q_STAR_BITS + 664 // 2 + 8   # ~1663, covers N*q at P664

DOMAIN = b'SaltReshape/v0/'


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


def keypair(sk: bytes, set_name: str) -> bytes:
    return shake(b'pk', sk, set_name.encode(), n=PK_BYTES[set_name])


def salt_of(sk: bytes, msg: bytes, set_name: str) -> bytes:
    return shake(b'salt', sk, msg, n=SALT_LEN[set_name])


def fs_challenge(e: int, *parts: bytes):
    """Odd s of e-4 free bits, q = 2^(2e-5) - s^2, until q is prime."""
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


def det4(B):
    return int(pari.matdet(pari.matrix(
        4, 4, [B[r][c] for r in range(4) for c in range(4)])))


def matmul(A, B):
    n = len(B[0])
    return [[sum(A[r][t] * B[t][c] for t in range(4)) for c in range(n)]
            for r in range(4)]


def apply(H, B):
    return matmul(H, B)


def in_span(v, B):
    """True if v is in the Z-span of the rows of B."""
    try:
        # PARI fills by columns; put each basis row in a column.
        # matsolve wants a t_MAT column, not a t_VEC.
        M = pari.matrix(4, 4, [B[c][r] for r in range(4) for c in range(4)])
        col = pari.matrix(4, 1, [int(x) for x in v])
        sol = pari.matsolve(M, col)
        return all(int(pari.denominator(sol[i, 0])) == 1 for i in range(4))
    except Exception:
        return False


def crt(a, m, b, n):
    return int((a + m * ((b - a) * int(gmpy2.invert(m, n)) % n)) % (m * n))


def crt_many(pairs):
    """pairs: list of (value, modulus), moduli pairwise coprime."""
    x, m = pairs[0]
    for b, n in pairs[1:]:
        x, m = crt(x, m, b, n), m * n
    return x, m


# ------------------------------------------------------------------ LLL
def lll_gram(G0, delta=(99, 100)):
    """Cohen Alg. 2.6.7 (integral LLL on a Gram matrix), delta = 99/100."""
    n = len(G0)
    H = [[int(r == c) for c in range(n)] for r in range(n)]

    def ip(a, b):
        ha, hb = H[a], H[b]
        return sum(ha[r] * G0[r][s] * hb[s]
                   for r in range(n) for s in range(n) if ha[r] and hb[s])

    d = [0] * (n + 1)
    d[0] = 1
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
    # A 4-D integral LLL on a genuine ideal is a few thousand iterations.
    # A non-definite or badly quantized Gram can loop; cap it.
    max_iter = 50_000
    while k < n and tr['iterations'] < max_iter:
        tr['iterations'] += 1
        if k > kmax:
            kmax = k
            compute_row(k)
            if d[k + 1] == 0:
                tr['failed'] = 1
                return H, tr
        redi(k, k - 1)
        tr['lovasz'] += 1
        if dd * d[k + 1] * d[k - 1] < dn * d[k] * d[k] - dd * lam[k][k - 1] ** 2:
            swapi(k, kmax)
            k = max(1, k - 1)
        else:
            for l in range(k - 2, -1, -1):
                redi(k, l)
            k += 1
    if k < n:
        tr['failed'] = 1
    return H, tr


# ------------------------------------------------------------------ ideals
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


def next_prime_from_salt(start, salt, tag, max_hops=10_000):
    """Prime at or above `start`, nudged by a salt-derived even offset."""
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
    """Left ideal of norm prod(m) whose (x, y) class is the CRT of classes.

    classes: list of (x, y, modulus) with pairwise-coprime moduli.
    """
    x, T = crt_many([(c[0], c[2]) for c in classes])
    y, T2 = crt_many([(c[1], c[2]) for c in classes])
    assert T == T2
    return ideal_basis(T, p, (x, y, 1, 0)), T


def reduce_ideal(B, p, q, extra_divisors=()):
    G = gram_norm(B, p)
    H, tr = lll_gram(G)
    beta = apply(H, B)[0]
    nb = nrd(beta, p)
    ok_q = int(nb % q == 0 and nb > 0)
    extras = [int(nb % d == 0) for d in extra_divisors if d and d > 1]
    in_l = int(in_span(beta, B))
    return dict(iterations=tr['iterations'], swaps=tr['swaps'],
                sizered=tr['sizered'], lovasz=tr['lovasz'],
                ok_q=ok_q, ok_in=in_l,
                ok_extra=int(all(extras) if extras else 1),
                ratio_bits=(nb // q).bit_length() if ok_q else -1,
                det_bits=abs(det4(B)).bit_length(),
                covol_bits=(abs(det4(B)) * p).bit_length(),
                nrd_bits=nb.bit_length())


# ------------------------------------------------------------------ constructions
STREAM_NAMES = ('H2', 'D0', 'SCALE', 'COF', 'SUPER', 'Q256')
PROD_NAMES = ('H2PROD', 'PCOF', 'COMP')


def _lambda_for_scale(B, p, target_covol_bits):
    """Integer scale so covolume(lambda B) is near 2^{target_covol_bits}.

    covolume in the norm metric is |det B| * p (since sqrt(det W) = p).
    Scaling B by lambda multiplies that by lambda^4.
    """
    vol = abs(det4(B)) * p
    if vol.bit_length() >= target_covol_bits:
        return 1
    # lambda^4 * vol ≈ 2^{target}
    # lambda ≈ 2^{(target - bits(vol))/4}
    shift = target_covol_bits - vol.bit_length()
    # 2^{shift/4}
    exp = shift // 4
    if exp <= 0:
        return 1
    return 1 << exp


def _scale_basis(B, lam):
    return [[lam * c for c in row] for row in B]


def _gram_pd(G):
    """Sylvester test: all leading principal minors strictly positive."""
    try:
        for k in range(1, 5):
            d = int(pari.matdet(pari.matrix(
                k, k, [G[r][c] for r in range(k) for c in range(k)])))
            if d <= 0:
                return False
        return True
    except Exception:
        return False


def _quantize_gram(G, prec_bits):
    """Positive-definite integer Gram with about prec_bits in the max entry."""
    mx = max(abs(G[r][c]) for r in range(4) for c in range(4))
    if mx == 0:
        return [row[:] for row in G], 1
    # scale so max entry ~ 2^{prec_bits}
    # G' = round(G * 2^{prec} / mx)
    num, den = 1 << prec_bits, mx
    Gp = [[int(Fraction(G[r][c] * num, den)) for c in range(4)] for r in range(4)]
    # Keep diagonal at least 1 and symmetric.
    for i in range(4):
        if Gp[i][i] <= 0:
            Gp[i][i] = 1
        for j in range(i):
            Gp[i][j] = Gp[j][i] = (Gp[i][j] + Gp[j][i]) // 2
    return Gp, num


def _col_hnf(B):
    """Column-style HNF: 4 integer columns that generate the same Z-module."""
    M = pari.matrix(4, 4, [B[c][r] for r in range(4) for c in range(4)])
    H = pari.mathnf(M)
    return [[int(H[r, c]) for r in range(4)] for c in range(4)]


def _superlattice(B, target_det):
    """A superlattice of span(B) with determinant near |target_det|.

    Shrink HNF column diagonals by factors of the wanted index. The result
    is a coarser Z-module in the same Q-span; short vectors need not lie
    in the original ideal.
    """
    cols = _col_hnf(B)

    def det_of(cs):
        return abs(int(pari.matdet(pari.matrix(
            4, 4, [cs[c][r] for r in range(4) for c in range(4)])))) or 1

    def content(col):
        g = 0
        for x in col:
            g = int(gmpy2.gcd(g, x)) if g else abs(int(x))
        return g or 1

    d = det_of(cols)
    td = max(int(abs(target_det)), 1)
    used = 1
    changed = True
    while d > td and changed:
        changed = False
        for i in sorted(range(4), key=lambda j: -content(cols[j])):
            c = content(cols[i])
            if c <= 1:
                continue
            want = max(d // td, 1)
            g = int(gmpy2.gcd(c, want))
            if g == 1:
                # Typical for I_q: content is q, want is near a power of two.
                # Jump by the whole content if that still leaves a lattice.
                g = c if d // c >= 1 else 1
            if g > 1:
                cols[i] = [x // g for x in cols[i]]
                used *= g
                d = det_of(cols)
                changed = True
                if d <= td:
                    break
    return [c[:] for c in cols], used


def stream_job_data(sk, msg, set_name, e, f):
    p = prime_of(e, f)
    pk = keypair(sk, set_name)
    salt = salt_of(sk, msg, set_name)
    s, q, qtries = fs_challenge(e, pk, salt, msg)
    seed = shake(b'h2-seed', salt, q.to_bytes((q.bit_length() + 7) // 8, 'little'))
    return dict(p=p, pk=pk, salt=salt, s=s, q=q, qtries=qtries, seed=seed)


def run_stream_construction(name, data):
    """Reduce an I_q-side lattice under one reshape. data from stream_job_data."""
    p, q, salt, seed = data['p'], data['q'], data['salt'], data['seed']
    extra = dict(r_bits=0, scale=1, prec=0, index=1,
                 pre_after=1,  # salt and q are in the signature / public
                 format_change=0)
    if name == 'H2':
        x, y, tries = select_xy(q, p, seed)
        B = ideal_basis(q, p, (x, y, 1, 0))
        out = reduce_ideal(B, p, q)
        out['tries'] = tries
    elif name == 'D0':
        x, y, tries = select_xy_d0(q, p)
        B = ideal_basis(q, p, (x, y, 1, 0))
        out = reduce_ideal(B, p, q)
        out['tries'] = tries
    elif name == 'SCALE':
        x, y, tries = select_xy(q, p, seed)
        B = ideal_basis(q, p, (x, y, 1, 0))
        # Global target: covolume bits of a typical P664 I_q, ~ 2*1323 + 664.
        target = 2 * Q_STAR_BITS + 664
        lam = _lambda_for_scale(B, p, target)
        Bs = _scale_basis(B, lam)
        out = reduce_ideal(Bs, p, q)
        # nrd(lambda v) % q == 0 iff nrd(v) % q == 0 when gcd(lambda, q) = 1,
        # which it is (lambda is a power of two, q is an odd prime).
        extra['scale'] = int(lam).bit_length()  # bits of the integer scale, not the integer
        out['tries'] = tries
    elif name == 'COF':
        xq, yq, tries = select_xy(q, p, seed)
        # r * q ≈ 2^{Q_STAR_BITS}
        r0 = max(1, (1 << Q_STAR_BITS) // q)
        r, hops = next_prime_from_salt(r0, salt, b'cof-r')
        if r == 1:
            B = ideal_basis(q, p, (xq, yq, 1, 0))
            T = q
            extra_div = ()
        else:
            xr, yr, tr2 = select_xy(r, p, shake(b'cof-xy', salt), tag=b'Hr')
            B, T = ideal_from_classes(p, [(xq, yq, q), (xr, yr, r)])
            extra_div = (T, r)
            tries += tr2
        out = reduce_ideal(B, p, q, extra_div)
        extra['r_bits'] = 1 if r == 1 else r.bit_length()
        extra['format_change'] = 1  # internal degree becomes r*q unless unused
        out['tries'] = tries + hops
    elif name == 'SUPER':
        x, y, tries = select_xy(q, p, seed)
        B = ideal_basis(q, p, (x, y, 1, 0))
        # The only constant-determinant superlattice of I_q inside O = Z^4
        # is (essentially) O itself: HNF diagonals are (q, q, 1, 1), so the
        # only exact coarsenings have index q or q^2. We take det 1.
        target_det = 1
        Bs, used = _superlattice(B, target_det)
        H, tr = lll_gram(gram_norm(Bs, p))
        beta = apply(H, Bs)[0]
        nb = nrd(beta, p)
        out = dict(iterations=tr['iterations'], swaps=tr['swaps'],
                   sizered=tr['sizered'], lovasz=tr['lovasz'],
                   ok_q=int(nb % q == 0 and nb > 0),
                   ok_in=int(in_span(beta, B)),
                   ok_extra=1,
                   ratio_bits=(nb // q).bit_length() if nb % q == 0 and nb > 0 else -1,
                   det_bits=abs(det4(Bs)).bit_length(),
                   covol_bits=(abs(det4(Bs)) * p).bit_length(),
                   nrd_bits=nb.bit_length())
        extra['index'] = int(used).bit_length()
        extra['format_change'] = 1
        out['tries'] = tries
    elif name == 'Q256':
        x, y, tries = select_xy(q, p, seed)
        B = ideal_basis(q, p, (x, y, 1, 0))
        G = gram_norm(B, p)
        Gp, _ = _quantize_gram(G, 256)
        if not _gram_pd(Gp):
            # 256 bits does not cover the dynamic range of G (~ covolume
            # bits). Record a failed reduction rather than loop.
            out = dict(iterations=0, swaps=0, sizered=0, lovasz=0,
                       ok_q=0, ok_in=0, ok_extra=0, ratio_bits=-1,
                       det_bits=abs(det4(B)).bit_length(),
                       covol_bits=(abs(det4(B)) * p).bit_length(),
                       nrd_bits=-1, tries=tries)
            extra['prec'] = 256
            extra['format_change'] = 1
            out.update(extra)
            return out
        H, tr = lll_gram(Gp)
        beta = apply(H, B)[0]
        nb = nrd(beta, p)
        failed = int(tr.get('failed', 0))
        ok_q = int(not failed and nb % q == 0 and nb > 0)
        out = dict(iterations=tr['iterations'], swaps=tr['swaps'],
                   sizered=tr['sizered'], lovasz=tr['lovasz'],
                   ok_q=ok_q, ok_in=int(not failed and in_span(beta, B)),
                   ok_extra=int(not failed),
                   ratio_bits=(nb // q).bit_length() if ok_q else -1,
                   det_bits=abs(det4(B)).bit_length(),
                   covol_bits=(abs(det4(B)) * p).bit_length(),
                   nrd_bits=nb.bit_length() if nb else -1, tries=tries)
        extra['prec'] = 256
        extra['format_change'] = 1
    else:
        raise ValueError(name)
    out.update(extra)
    return out


def run_prod_construction(name, sk, data):
    """I_sec * I_q, optionally reshaped by a cofactor ideal."""
    p, q, salt, seed = data['p'], data['q'], data['salt'], data['seed']
    N, xN, yN = secret_class(sk, p, p.bit_length() // 2)
    xq, yq, tries = select_xy(q, p, seed)
    extra = dict(r_bits=0, scale=1, prec=0, index=1, pre_after=0,
                 format_change=0, N_bits=N.bit_length())
    if name == 'H2PROD':
        B, T = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
        out = reduce_ideal(B, p, q, (T, N))
        extra['pre_after'] = 0  # I_sec is secret
        out['tries'] = tries
    elif name == 'PCOF':
        # Cofactor from the published salt only: cannot cancel N.
        r0 = max(1, (1 << Q_STAR_BITS) // q)
        r, hops = next_prime_from_salt(r0, salt, b'pcof-r')
        classes = [(xN, yN, N), (xq, yq, q)]
        if r > 1:
            xr, yr, tr2 = select_xy(r, p, shake(b'pcof-xy', salt), tag=b'Hr')
            classes.append((xr, yr, r))
            tries += tr2
        B, T = ideal_from_classes(p, classes)
        out = reduce_ideal(B, p, q, (T, N))
        extra['r_bits'] = 1 if r == 1 else r.bit_length()
        extra['pre_after'] = 0
        extra['format_change'] = 0  # internal only; signature unchanged
        out['tries'] = tries + hops
    elif name == 'COMP':
        # Secret-compensating cofactor: N * q * r ≈ 2^{PROD_STAR_BITS}.
        # r is not a function of the published salt alone.
        r0 = max(1, (1 << PROD_STAR_BITS) // (N * q))
        r, hops = next_prime_from_salt(r0, shake(b'comp-r', sk, salt,
                                                N.to_bytes((N.bit_length() + 7) // 8, 'little')),
                                       b'comp-r')
        classes = [(xN, yN, N), (xq, yq, q)]
        if r > 1:
            xr, yr, tr2 = select_xy(r, p, shake(b'comp-xy', sk, salt), tag=b'Hr')
            classes.append((xr, yr, r))
            tries += tr2
        B, T = ideal_from_classes(p, classes)
        out = reduce_ideal(B, p, q, (T, N))
        extra['r_bits'] = 1 if r == 1 else r.bit_length()
        extra['pre_after'] = 0
        extra['format_change'] = 0
        out['tries'] = tries + hops
    else:
        raise ValueError(name)
    out.update(extra)
    return out
