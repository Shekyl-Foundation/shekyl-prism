# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Instrumented fixed-schedule Z[i]-Lagrange reduction.

The arithmetic is fsr.py `zi_gauss_fixed` / `zi_gauss_uv` with the
bounded-precision quotient of research/rust-lagrange-ct/src/zi.rs
`quotient` ported exactly (top 64 bits of nrd(u), the h saturation at
+-2^124, the 2^62 quotient clamp), followed by the one exact cleanup step
(`exact_cleanup`) and the conversion of the Z[i]-basis back to a 4-D
basis and its Gram matrix. Every intermediate the Rust code (or a
prism-int port in the same order) materialises is recorded by name: the
largest |value| seen and its bit length.
"""
from q5common import N4, herm, times_i, bits

PREC = 64


class Rec:
    def __init__(self):
        self.mx = {}

    def r(self, name, *vals):
        m = self.mx.get(name, 0)
        for x in vals:
            a = abs(int(x))
            if a > m:
                m = a
        self.mx[name] = m

    def merge(self, other):
        for k, v in other.mx.items():
            if v > self.mx.get(k, 0):
                self.mx[k] = v


def herm_rec(v, u, p, rec, pre):
    # same order as zi.rs herm(): h0 = v0u0 (+ v1u1) (+ p*(v2u2 + v3u3))
    a = v[0] * u[0]; b = v[1] * u[1]; s = a + b
    c = v[2] * u[2]; d = v[3] * u[3]; t = c + d; pt = p * t
    h0 = s + pt
    rec.r(pre + 'herm.prod', a, b)
    rec.r(pre + 'herm.prod_j', c, d)
    rec.r(pre + 'herm.partial', s, pt)
    rec.r(pre + 'herm.partial_j', t)
    a = v[1] * u[0]; b = v[0] * u[1]; s = a - b
    c = v[3] * u[2]; d = v[2] * u[3]; t = c - d; pt = p * t
    h1 = s + pt
    rec.r(pre + 'herm.prod', a, b)
    rec.r(pre + 'herm.prod_j', c, d)
    rec.r(pre + 'herm.partial', s, pt)
    rec.r(pre + 'herm.partial_j', t)
    rec.r(pre + 'herm.h', h0, h1)
    return h0, h1


def nrd_rec(v, p, rec, pre):
    a = v[0] * v[0]; b = v[1] * v[1]; s = a + b
    c = v[2] * v[2]; d = v[3] * v[3]; t = c + d; pt = p * t
    n = s + pt
    rec.r(pre + 'nrd.prod', a, b)
    rec.r(pre + 'nrd.prod_j', c, d)
    rec.r(pre + 'nrd.partial', s, pt)
    rec.r(pre + 'nrd.partial_j', t)
    rec.r(pre + 'nrd.N4', n)
    return n


def quotient_rust(h, nu, rec=None):
    """Exact port of zi.rs quotient(): returns (m, saturated_flag)."""
    bl = nu.bit_length()
    k = max(0, bl - PREC)
    hs = h >> k                 # arithmetic shift = floor
    ns = nu >> k                # < 2^64
    sat = False
    if not (-(1 << 124) <= hs < (1 << 124)):
        hs = (1 << 124) if hs >= 0 else -(1 << 124)
        sat = True
    num = 2 * hs + ns
    den = 2 * ns
    a = abs(num)
    if num < 0:
        a += den - 1
    fits = (a >> 64) < den
    qa = a // den
    if not fits or qa >= (1 << 62):
        qa = 1 << 62
        sat = True
    m = -qa if num < 0 else qa
    if rec is not None:
        rec.r('q64.hs', hs); rec.r('q64.ns', ns); rec.r('q64.num', num)
        rec.r('q64.den', den); rec.r('q64.m', m)
    return m, sat


def exact_round(h, nu):
    return (2 * h + nu) // (2 * nu)


def update_rec(v, u, m0, m1, rec, pre):
    # zi.rs step(): a0 = m0 u0 - m1 u1, a1 = m0 u1 + m1 u0, a2 = m0 u2 - m1 u3,
    # a3 = m0 u3 + m1 u2; v_r -= a_r
    prods = [m0 * u[0], m1 * u[1], m0 * u[1], m1 * u[0],
             m0 * u[2], m1 * u[3], m0 * u[3], m1 * u[2]]
    rec.r(pre + 'upd.mu_u', *prods[0:4])
    rec.r(pre + 'upd.mu_u_j', *prods[4:8])
    a = [prods[0] - prods[1], prods[2] + prods[3], prods[4] - prods[5], prods[6] + prods[7]]
    rec.r(pre + 'upd.a', a[0], a[1])
    rec.r(pre + 'upd.a_j', a[2], a[3])
    nv = [v[r] - a[r] for r in range(4)]
    rec.r(pre + 'upd.v', nv[0], nv[1])
    rec.r(pre + 'upd.v_j', nv[2], nv[3])
    return nv


def reduce_instrumented(u0, v0, p, steps, rec, cleanup=True, prec64=True):
    """Fixed schedule from an arbitrary Z[i]-basis {u0, v0}. Returns
    (u, v, nu, nv, info). info has M0 (max starting N4), Mmax (max N4 of
    the basis over the run, after every update), n_sat (saturated
    quotients), swaps."""
    u = list(u0); v = list(v0)
    rec.r('in.coord', *u, *v)
    nu = nrd_rec(u, p, rec, 'init.')
    nv = nrd_rec(v, p, rec, 'init.')
    M0 = max(nu, nv)
    Mmax = M0
    if nv < nu:
        u, v, nu, nv = v, u, nv, nu
    nsat = 0
    swaps = 0
    for t in range(steps):
        h0, h1 = herm_rec(v, u, p, rec, '')
        if prec64:
            m0, s0 = quotient_rust(h0, nu, rec)
            m1, s1 = quotient_rust(h1, nu, rec)
            nsat += s0 + s1
        else:
            m0, m1 = exact_round(h0, nu), exact_round(h1, nu)
            rec.r('qex.m', m0, m1)
        v = update_rec(v, u, m0, m1, rec, '')
        nv = nrd_rec(v, p, rec, '')
        if nv > Mmax:
            Mmax = nv
        if nv < nu:
            u, v, nu, nv = v, u, nv, nu
            swaps += 1
        rec.r('state.coord', *u, *v)
    if cleanup:
        h0, h1 = herm_rec(v, u, p, rec, 'cl.')
        n0 = 2 * h0 + nu; n1 = 2 * h1 + nu; den = 2 * nu
        rec.r('cl.round_num', n0, n1); rec.r('cl.round_den', den)
        m0, m1 = n0 // den, n1 // den
        rec.r('cl.m', m0, m1)
        v = update_rec(v, u, m0, m1, rec, 'cl.')
        nv = nrd_rec(v, p, rec, 'cl.')
        if nv > Mmax:
            Mmax = nv
        if nv < nu:
            u, v, nu, nv = v, u, nv, nu
            swaps += 1
        rec.r('state.coord', *u, *v)
    # conversion back: 4-D basis [u, iu, v, iv] and its Gram matrix
    iu, iv = times_i(u), times_i(v)
    rec.r('conv.coord', *u, *iu, *v, *iv)
    h0, h1 = herm(v, u, p)
    rec.r('conv.gram', nu, nv, h0, h1)
    rec.r('conv.gram_ref2x', 2 * nu, 2 * nv, 2 * h0, 2 * h1)
    return u, v, nu, nv, dict(M0=M0, Mmax=Mmax, nsat=nsat, swaps=swaps)
