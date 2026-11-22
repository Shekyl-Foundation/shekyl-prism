# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Per-signature re-randomization of the secret ideal.

Research model only (index-4 order Z<1,i,j,k>, simulated keys, operation
counts in Python, not cycles). PRISM is the scheme of ePrint 2026/443
(Basso et al.); shekyl-prism implements it.

Key generation (once):  I = O*(xN + yN i + j) + O*N, reduced with the
  Z[i]-Lagrange fixed schedule sized from the bound B, kept as
  {u, i u, v, i v}.
Signing (per signature): beta = c0 u + c1 i u + c2 v + c3 i v with small
  coefficients from an XOF, N' = nrd(beta)/N, J = I * conj(beta) / N.
  The signing product I ∩ I_q = I*K_I is replaced by
      J*K_J = (I ∩ I_q) * conj(beta) / N        (K_J = g^-1 K_I g, g = conj(beta)/N)
  which has norm N' q and is similar to I ∩ I_q (right multiplication by
  g scales nrd by N'/N). A short element d' of J*K_J maps back to
  d = d' * beta / N' in I ∩ I_q, and (J K_J) conj(d') / (N' q) equals
  (I K_I) conj(d) / (N q) exactly, so the signer's output is unchanged.
"""
import gmpy2

from hdl import shake, qmul, nrd, ideal_basis, hnf_rows
from fsr import zi_gauss_fixed
from run_fsr import tmax, n_bound

ACCEPT = ('prime', 'odd')
SHAPES = ('plain', 'norm')


def conj(b):
    return (b[0], -b[1], -b[2], -b[3])


def ileft(u):
    """i * u (left multiplication by i)."""
    return (-u[1], u[0], -u[3], u[2])


def keygen_reduce(N, xN, yN, p):
    """Reduced Z[i]-basis of the secret ideal; schedule sized from B."""
    B = ideal_basis(N, p, (xN, yN, 1, 0))
    alpha = B[2]
    steps = tmax(n_bound(p), 'zi')
    u, v, nu, nv, c, eff = zi_gauss_fixed(N, alpha, p, steps, keep_trace=False)
    return dict(B=B, u=tuple(u), v=tuple(v), nu=nu, nv=nv, steps=steps)


def is_standard(Bh, T):
    """HNF of a cyclic ideal of norm T in the {T, T i, alpha, i alpha} shape."""
    return (Bh[0] == [T, 0, 0, 0] and Bh[1] == [0, T, 0, 0]
            and Bh[2][2:] == [1, 0] and Bh[3][2:] == [0, 1])


def coeff_ranges(kr, C, shape):
    if shape == 'plain':
        return C, C
    # shape-normalized: C_u^2 nrd(u) = C_v^2 nrd(v) = C^2 sqrt(nrd(u) nrd(v))
    nu, nv = kr['nu'], kr['nv']
    r = gmpy2.mpfr(nv) / gmpy2.mpfr(nu)
    cu = int(C * gmpy2.root(r, 4))
    cv = int(C / gmpy2.root(r, 4))
    return max(cu, 1), max(cv, 1)


def draw_beta(kr, seed, ctr, cu, cv):
    raw = shake(b'rr-beta', seed, ctr.to_bytes(4, 'little'), n=4 * 8)
    w = [int.from_bytes(raw[8 * t:8 * t + 8], 'little') for t in range(4)]
    c = [w[0] % (2 * cu + 1) - cu, w[1] % (2 * cu + 1) - cu,
         w[2] % (2 * cv + 1) - cv, w[3] % (2 * cv + 1) - cv]
    u, v = kr['u'], kr['v']
    iu, iv = ileft(u), ileft(v)
    beta = tuple(c[0] * u[r] + c[1] * iu[r] + c[2] * v[r] + c[3] * iv[r]
                 for r in range(4))
    return beta, c


def J_basis(IB, beta, N, p):
    gb = conj(beta)
    rows = []
    for b in IB:
        t = qmul(tuple(b), gb, p)
        assert all(x % N == 0 for x in t)
        rows.append(tuple(x // N for x in t))
    return hnf_rows(rows)


def sample(kr, N, p, seed, accept, shape, C, nb_max, kmax=100_000):
    """Returns dict(beta, Np, tries, mr, hnf, Jstd) for the first accepted
    candidate. Counts: tries (candidates drawn), mr (primality tests),
    hnf (shape checks)."""
    cu, cv = coeff_ranges(kr, C, shape)
    mr = hn = 0
    for ctr in range(kmax):
        beta, c = draw_beta(kr, seed, ctr, cu, cv)
        n = nrd(beta, p)
        if n == 0:
            continue
        assert n % N == 0
        Np = n // N
        if Np % 2 == 0 or Np.bit_length() > nb_max or Np == 1:
            continue
        if accept == 'prime':
            mr += 1
            if not gmpy2.is_prime(Np):
                continue
        hn += 1
        JB = J_basis(kr['B'], beta, N, p)
        if not is_standard(JB, Np):
            continue
        return dict(beta=beta, Np=Np, tries=ctr + 1, mr=mr, hnf=hn,
                    cu=cu, cv=cv, JB=JB)
    raise RuntimeError('sampler exhausted')


def rerandomized_product(Bprod, beta, N, p):
    """HNF of (I ∩ I_q) * conj(beta) / N."""
    gb = conj(beta)
    rows = []
    for b in Bprod:
        t = qmul(tuple(b), gb, p)
        assert all(x % N == 0 for x in t)
        rows.append(tuple(x // N for x in t))
    return hnf_rows(rows)


def map_back(d, beta, Np, p):
    t = qmul(tuple(d), tuple(beta), p)
    assert all(x % Np == 0 for x in t), 'map-back not integral'
    return tuple(x // Np for x in t)


def canon(u):
    """Representative of u up to left units {±1, ±i} (Z[i]-module)."""
    iu = ileft(u)
    cands = [tuple(u), tuple(-x for x in u), tuple(iu), tuple(-x for x in iu)]
    return max(cands)


# --------------------------------------------- Gaussian-then-round sampler
# The box sampler's coefficient box is shaped by the key's reduced basis,
# so the law of nrd(beta) depends (slightly) on that shape. Here x is a
# continuous Gaussian that is isotropic for the reduced norm (independent
# of any basis), with width far above the basis lengths, and beta is x
# rounded to the lattice (Babai rounding on the stored reduced basis).
import math as _math
from gmpy2 import mpfr as _mpfr

_GPREC = 512


def _inv4(M):
    """Inverse of a 4x4 matrix (rows), mpfr Gauss-Jordan."""
    with gmpy2.local_context(gmpy2.get_context(), precision=_GPREC):
        n = 4
        A = [[_mpfr(M[r][c]) for c in range(n)] + [_mpfr(int(r == c)) for c in range(n)]
             for r in range(n)]
        for c in range(n):
            piv = max(range(c, n), key=lambda r: abs(A[r][c]))
            A[c], A[piv] = A[piv], A[c]
            pv = A[c][c]
            A[c] = [x / pv for x in A[c]]
            for r in range(n):
                if r != c:
                    f = A[r][c]
                    A[r] = [a - f * b for a, b in zip(A[r], A[c])]
        return [row[n:] for row in A]


def gauss_setup(kr, p, extra_bits=16):
    """Precomputable at key generation: basis rows, inverse, width."""
    u, v = kr['u'], kr['v']
    rows = [tuple(u), ileft(u), tuple(v), ileft(v)]
    L = p.bit_length()
    kr['g_rows'] = rows
    kr['g_inv'] = _inv4(rows)
    kr['g_tbits'] = (L + 1) // 2 + extra_bits   # target E[bits(N')] scale
    return kr


def draw_beta_gauss(kr, N, p, seed, ctr):
    with gmpy2.local_context(gmpy2.get_context(), precision=_GPREC):
        raw = shake(b'rr-gauss', seed, ctr.to_bytes(4, 'little'), n=4 * 32)
        us = [(_mpfr(int.from_bytes(raw[32 * t:32 * t + 32], 'little')) + 1) / _mpfr(2) ** 256
              for t in range(4)]
        g = []
        for a, b in ((us[0], us[1]), (us[2], us[3])):
            rad = gmpy2.sqrt(-2 * gmpy2.log(a))
            g += [rad * gmpy2.cos(2 * gmpy2.const_pi() * b),
                  rad * gmpy2.sin(2 * gmpy2.const_pi() * b)]
        # nrd(x) = sigma^2 * sum g_t^2, E = 4 sigma^2; target nrd/N ~ 2^tbits
        sigma = gmpy2.sqrt(_mpfr(N) * _mpfr(2) ** kr['g_tbits'] / 4)
        w = (1, 1, p, p)
        x = [sigma * g[t] / gmpy2.sqrt(_mpfr(w[t])) for t in range(4)]
        # coefficients c = x * B^-1 (row-vector convention), rounded
        inv = kr['g_inv']
        c = [int(gmpy2.rint(sum(x[r] * inv[r][s] for r in range(4)))) for s in range(4)]
    rows = kr['g_rows']
    beta = tuple(sum(c[s] * rows[s][t] for s in range(4)) for t in range(4))
    return beta, c


def sample_gauss(kr, N, p, seed, accept, nb_max, kmax=100_000):
    if 'g_inv' not in kr:
        gauss_setup(kr, p)
    mr = hn = 0
    for ctr in range(kmax):
        beta, c = draw_beta_gauss(kr, N, p, seed, ctr)
        n = nrd(beta, p)
        if n == 0:
            continue
        assert n % N == 0
        Np = n // N
        if Np % 2 == 0 or Np.bit_length() > nb_max or Np == 1:
            continue
        if accept == 'prime':
            mr += 1
            if not gmpy2.is_prime(Np):
                continue
        hn += 1
        JB = J_basis(kr['B'], beta, N, p)
        if not is_standard(JB, Np):
            continue
        return dict(beta=beta, Np=Np, tries=ctr + 1, mr=mr, hnf=hn, JB=JB)
    raise RuntimeError('sampler exhausted')


def sample_any(kr, N, p, seed, accept, shape, C, nb_max):
    if shape == 'gauss':
        return sample_gauss(kr, N, p, seed, accept, nb_max)
    return sample(kr, N, p, seed, accept, shape, C, nb_max)
