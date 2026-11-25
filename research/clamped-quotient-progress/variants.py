# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Quotient rules for the fixed-schedule Z[i]-Lagrange reduction.

Every rule maps (h0, h1, A = N4(u)) to (m0, m1, flag). The schedule is the
same for all of them: initial conditional swap, `steps` fixed steps
(herm, quotient, v <- v - m0 u - m1 iu, N4(v), conditional swap), and an
optional exact cleanup step.

Rules:
  exact     m_j = floor((2 h_j + A) / (2 A)), full width (the T_prov rule).
  clamp64   research/rust-lagrange-ct zi.rs quotient(), ported exactly
            (instrumented.quotient_rust): top 64 bits of A, h saturated
            at +-2^124, quotient clamped at 2^62. flag = saturated.
  clamp128  the same design one word wider: top 128 bits of A, h
            saturated at +-2^252, quotient clamped at 2^126 (the
            "wider quotient" fallback).
  corr64    clamp64 plus an exact +-1 correction of the rounded quotient
            (X = 2h + A - 2mA; m -= [X < 0]; m += [X >= 2A]); flag when
            any |q_j| may exceed the window (bl(h_j) - bl(A) > T0). Used
            as "clamp is a sticky overflow": unflagged runs are exact.
  exp64     the proposed rule. Same 64-bit divisor (top 64 bits of A),
            same 128-by-64 division. e_j = bl(|h_j|) - bl(A).
            * window (max e_j <= T0): approximate rounding from the top
              64 bits, then the exact +-1 correction. Result = exact.
            * big (max e_j > T0): common exponent s = max e_j - T0 >= 1;
              per component qtop_j = floor((|h_j| >> (kA + s)) / ns);
              m_j = sign(h_j) (qtop_j - 1) 2^s if qtop_j >= 2, else 0.
              An undershoot: it never swaps and never grows N4(v), and
              it cuts max e_j by at least T0 - 2 bits.
            flag = big sub-step.
T0 = 60.
"""
from q5common import N4, herm
from instrumented import quotient_rust

T0 = 60


def bl(x):
    return abs(x).bit_length()


def exact_round(h, A):
    return (2 * h + A) // (2 * A)


def q_exact(h0, h1, A):
    return exact_round(h0, A), exact_round(h1, A), False


def q_clamp64(h0, h1, A):
    m0, s0 = quotient_rust(h0, A)
    m1, s1 = quotient_rust(h1, A)
    return m0, m1, bool(s0 or s1)


def _clamp_w(h, A, w, hsat, qcap):
    k = max(0, A.bit_length() - w)
    hs = h >> k
    ns = A >> k
    sat = False
    if not (-hsat <= hs < hsat):
        hs = hsat if hs >= 0 else -hsat
        sat = True
    num = 2 * hs + ns
    den = 2 * ns
    a = abs(num)
    if num < 0:
        a += den - 1
    qa = a // den
    if qa >= qcap:
        qa = qcap
        sat = True
    return (-qa if num < 0 else qa), sat


def q_clamp128(h0, h1, A):
    m0, s0 = _clamp_w(h0, A, 128, 1 << 252, 1 << 126)
    m1, s1 = _clamp_w(h1, A, 128, 1 << 252, 1 << 126)
    return m0, m1, bool(s0 or s1)


def _window_round(h, A, ns, kA, w=64):
    """Top-w-bit rounding inside the window, then the exact +-1 fix."""
    hs = h >> kA                       # |hs| < 2^(w + T0 + 2) here
    m = (2 * hs + ns) // (2 * ns)
    X = 2 * h + A - 2 * m * A          # exact: m is right iff 0 <= X < 2A
    lo = X < 0
    hi = X >= 2 * A
    m = m - int(lo) + int(hi)
    return m, lo or hi


def q_corr64(h0, h1, A):
    kA = max(0, A.bit_length() - 64)
    ns = A >> kA
    e = max(bl(h0), bl(h1)) - A.bit_length()
    if e > T0:
        m0, s0 = quotient_rust(h0, A)
        m1, s1 = quotient_rust(h1, A)
        return m0, m1, True
    m0, _ = _window_round(h0, A, ns, kA)
    m1, _ = _window_round(h1, A, ns, kA)
    return m0, m1, False


def q_expw(h0, h1, A, w=64, t0=None, info=None):
    """The exp rule at divisor precision w and window t0 (default w - 4).
    info, if a dict, receives the path taken and the intermediates the
    lemma checks need."""
    if t0 is None:
        t0 = w - 4
    kA = max(0, A.bit_length() - w)
    ns = A >> kA
    e = max(bl(h0), bl(h1)) - A.bit_length()
    if e <= t0:
        m0, c0 = _window_round(h0, A, ns, kA, w)
        m1, c1 = _window_round(h1, A, ns, kA, w)
        if info is not None:
            info.update(path='window', e=e, corrections=int(c0) + int(c1))
        return m0, m1, False
    s = e - t0
    out = []
    for h in (h0, h1):
        a = abs(h)
        num = a >> (kA + s)
        assert num < (1 << (2 * w))   # a 2w-bit numerator (128 bits at w = 64)
        qtop = num // ns
        assert qtop < (1 << w)
        mag = ((qtop - 1) << s) if qtop >= 2 else 0
        out.append(-mag if h < 0 else mag)
    if info is not None:
        info.update(path='big', e=e, s=s)
    return out[0], out[1], True


def q_exp64(h0, h1, A):
    return q_expw(h0, h1, A, 64, T0)


RULES = dict(exact=q_exact, clamp64=q_clamp64, clamp128=q_clamp128,
             corr64=q_corr64, exp64=q_exp64)


def run(u0, v0, p, steps, rule, cleanup=False, trace=False):
    """Fixed schedule. Returns dict(u, v, nu, nv, flags, M0, Mmax, swaps,
    maxbits (largest |intermediate| the model forms, incl. the correction
    numerator), log=[...] if trace)."""
    qf = RULES[rule]
    u = list(u0); v = list(v0)
    nu, nv = N4(u, p), N4(v, p)
    M0 = max(nu, nv); Mmax = M0; big = 0
    if nv < nu:
        u, v, nu, nv = v, u, nv, nu
    flags = 0; swaps = 0; log = []
    for t in range(steps):
        h0, h1 = herm(v, u, p)
        m0, m1, fl = qf(h0, h1, nu)
        flags += fl
        big = max(big, abs(2 * h0) + nu + abs(2 * m0 * nu), abs(2 * h1) + nu + abs(2 * m1 * nu))
        iu = [-u[1], u[0], -u[3], u[2]]
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        nv = N4(v, p)
        Mmax = max(Mmax, nv)
        sw = nv < nu
        if sw:
            u, v, nu, nv = v, u, nv, nu
            swaps += 1
        if trace:
            log.append((t + 1, m0, m1, int(fl), int(sw), nu.bit_length(), nv.bit_length()))
    if cleanup:
        h0, h1 = herm(v, u, p)
        m0, m1 = exact_round(h0, nu), exact_round(h1, nu)
        iu = [-u[1], u[0], -u[3], u[2]]
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        nv = N4(v, p)
        if nv < nu:
            u, v, nu, nv = v, u, nv, nu
    return dict(u=u, v=v, nu=nu, nv=nv, flags=flags, M0=M0, Mmax=Mmax, swaps=swaps,
                big_num=big, log=log)


def is_reduced(r, p):
    """Z[i]-Lagrange reduced: exact quotient of v on u is 0 and N4(u) <= N4(v)."""
    h0, h1 = herm(r['v'], r['u'], p)
    return exact_round(h0, r['nu']) == 0 and exact_round(h1, r['nu']) == 0 and r['nu'] <= r['nv']


def exact_logical(u0, v0, p, max_steps=10 ** 6):
    """Exact early-exit run: list of (A_before, |q|_inf bits e, swapped) per
    logical step, and the steps count including the final m=0 step."""
    u = list(u0); v = list(v0)
    nu, nv = N4(u, p), N4(v, p)
    if nv < nu:
        u, v, nu, nv = v, u, nv, nu
    es = []
    for steps in range(1, max_steps):
        h0, h1 = herm(v, u, p)
        m0, m1 = exact_round(h0, nu), exact_round(h1, nu)
        es.append(max(bl(h0), bl(h1)) - nu.bit_length())
        if m0 == 0 and m1 == 0 and nu <= nv:
            return steps, es, nu
        iu = [-u[1], u[0], -u[3], u[2]]
        v = [v[r] - m0 * u[r] - m1 * iu[r] for r in range(4)]
        nv = N4(v, p)
        if nv < nu:
            u, v, nu, nv = v, u, nv, nu
    raise RuntimeError('no convergence')
