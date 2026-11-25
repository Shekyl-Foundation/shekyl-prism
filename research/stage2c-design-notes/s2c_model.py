# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Shared integer model for the Stage 2c design notes (SHE-25).

Plain Python integers, stdlib only. Not the production reducer: this is
the exact contract of SIGNER_SPEC 2.3 (PR #45) on public toy fixtures, the
normalized-Gram constructor of research/normalized-gram, and the
conversion from the carried Gram to the consumer's lower-triangular form.

Numerator coordinates of O0 throughout: x = (x0, x1, x2, x3) stands for
(x0 + x1 i + x2 j + x3 k)/2 written on the order's numerator lattice
(SIGNER_SPEC 2.1.2 / Q1), N4(x) = 4 nrd(x/2), J = left multiplication by i.
"""


def J(x):
    a, b, c, d = x
    return (-b, a, -d, c)


def D(p, x, y):
    return x[0]*y[0] + x[1]*y[1] + p*(x[2]*y[2] + x[3]*y[3])


def N4(p, x):
    return D(p, x, x)


def comb(cols, coeffs):
    return tuple(sum(c*col[r] for c, col in zip(coeffs, cols)) for r in range(4))


def half_up(h, a):
    """floor(h/a + 1/2) for a > 0: ties toward +infinity (+1/2 -> 1, -1/2 -> 0)."""
    return (2*h + a) // (2*a)


# ---------------------------------------------------------------- reducer

def exact_step(p, u, v):
    """One logical exact step of SIGNER_SPEC 2.3. Returns (u, v, record)."""
    a = N4(p, u)
    ju = J(u)
    h0, h1 = D(p, v, u), D(p, v, ju)
    m0, m1 = half_up(h0, a), half_up(h1, a)
    v = comb((v, u, ju), (1, -m0, -m1))
    swap = N4(p, v) < a
    if swap:
        u, v = v, u
    return u, v, dict(A=a, h0=h0, h1=h1, m0=m0, m1=m1, swap=swap)


def reduce_fixed(p, u, v, steps):
    """Initial strict ordering, then exactly `steps` exact steps (fixed schedule)."""
    if N4(p, v) < N4(p, u):
        u, v = v, u
    trace = []
    for _ in range(steps):
        u, v, rec = exact_step(p, u, v)
        trace.append(rec)
    return u, v, trace


def is_reduced(p, u, v):
    """Terminal state of SIGNER_SPEC 2.3: N4(U) <= N4(V), -A <= 2 h_j < A."""
    a = N4(p, u)
    h0, h1 = D(p, v, u), D(p, v, J(u))
    return a <= N4(p, v) and all(-a <= 2*h < a for h in (h0, h1))


# ---------------------------------------------------------- normalized Gram

def hensel_inverse(t, bits):
    """t^-1 mod 2^bits for odd t, by the research constructor's Newton lift.

    Fixed work, no check: for an even t the result is meaningless and the
    caller's product-back comparison (and its T-odd bit) refuses it.
    """
    mod = 1 << bits
    x = (3*t) ^ 2           # 5 correct bits
    prec = 5
    while prec < bits:
        x = (x * (2 - t*x)) % mod
        prec *= 2
    return x


def hensel_div(n, t, tinv, bits):
    """Exact n / t on magnitudes: Q = |n| t^-1 mod 2^bits, ok iff Q t == |n|.

    The comparison of the full product is the integrality check: if t does
    not divide n, Q t differs from |n| (Q < 2^bits, t < 2^bits, so the
    product is formed without loss at 2 * bits). Sign is reattached, as a
    sign-magnitude SecretInt would do it.
    """
    mag = abs(n)
    q = (mag * tinv) % (1 << bits)
    ok = (q * t == mag)
    return (-q if n < 0 else q), ok


def gram_hnf(p, t, x, y, bits):
    """Generic HNF U=(2T,0,0,0), V=(x,y,1,0) -> (A', C', h0', h1'), ok.

    Checks (all folded into one bit): T odd, x even, y odd, 0 <= x,y < 2T,
    4 | x^2+y^2+p, (x^2+y^2+p)/4 divisible by T (Hensel + product back).
    det = p then holds by algebra.
    """
    ok = (t & 1 == 1) and (x & 1 == 0) and (y & 1 == 1) and 0 <= x < 2*t and 0 <= y < 2*t
    n = x*x + y*y + p
    ok = ok and (n & 3 == 0)
    tinv = hensel_inverse(t, bits)
    q, okd = hensel_div(n >> 2, t, tinv, bits)
    ok = ok and okd
    return (2*t, 2*q, x, y), ok


def gram_general(p, t, u, v, bits):
    """Any O0 pair: diagonal / 4T, projections / 2T, then det == p."""
    tinv = hensel_inverse(t, bits)
    ok = t & 1 == 1
    out = []
    for val, sh in ((N4(p, u), 2), (N4(p, v), 2), (D(p, v, u), 1), (D(p, v, J(u)), 1)):
        ok = ok and (abs(val) & ((1 << sh) - 1)) == 0
        q, okd = hensel_div(val >> sh if val >= 0 else -((-val) >> sh), t, tinv, bits)
        ok = ok and okd
        out.append(q)
    a, c, h0, h1 = out
    a, c = 2*a, 2*c
    ok = ok and (a*c - h0*h0 - h1*h1 == p)
    return (a, c, h0, h1), ok


def carried_exact(p, gram, steps):
    """The exact rule on the normalized Gram alone (what the reducer carries).

    Returns the trace of decisions and the final Gram. Rounding is of h/A,
    swaps compare C' with A: both scale-invariant, so the decisions equal
    those of the coordinate run.
    """
    a, c, h0, h1 = gram
    if c < a:                       # initial strict ordering
        a, c, h1 = c, a, -h1        # swapping conjugates h (h1 changes sign)
    trace = []
    for _ in range(steps):
        m0, m1 = half_up(h0, a), half_up(h1, a)
        r0, r1 = h0 - m0*a, h1 - m1*a
        c2 = c - m0*(h0 + r0) - m1*(h1 + r1)
        swap = c2 < a
        trace.append(dict(m0=m0, m1=m1, swap=swap))
        if swap:
            a, c, h0, h1 = c2, a, r0, -r1
        else:
            c, h0, h1 = c2, r0, r1
    return (a, c, h0, h1), trace


# ---------------------------------------------------------- consumer form

def columns(u, v):
    return (u, J(u), v, J(v))


def form_from_basis(p, u, v, t):
    """SIGNER_SPEC 2.3: F_ii = W_ii/(4T), F_ij = 2 W_ij/(4T) for i > j, 0 above."""
    b = columns(u, v)
    f = [[0]*4 for _ in range(4)]
    for i in range(4):
        for j in range(i + 1):
            num = D(p, b[i], b[j]) * (1 if i == j else 2)
            q, r = divmod(num, 4*t)
            if r:
                raise ValueError("class norm form not integral")
            f[i][j] = q
    return f


def form_from_gram(gram):
    """The exact conversion from the carried normalized Gram (A', C', h0', h1').

    A' = N4(U)/(2T), C' = N4(V)/(2T), h0' = D(V,U)/(2T), h1' = D(V,JU)/(2T).
    Column order (U, JU, V, JV). W(U,JU) = W(V,JV) = 0, W(JU,JU) = W(U,U),
    and the off-diagonal block has rows (h0, -h1), (h1, h0) for (U, JU) x (V, JV):
        F00 = F11 = A'/2, F22 = F33 = C'/2  (A', C' are even: nrd/T is an integer)
        F20 = h0', F21 = h1', F30 = -h1', F31 = h0', F10 = F32 = 0.
    """
    a, c, h0, h1 = gram
    assert a % 2 == 0 and c % 2 == 0
    return [[a//2, 0, 0, 0],
            [0, a//2, 0, 0],
            [h0, h1, c//2, 0],
            [-h1, h0, 0, c//2]]


def eval_form(f, c):
    return sum(c[i]*f[i][j]*c[j] for i in range(4) for j in range(4))


def identity_holds(p, u, v, t, f, coeffs):
    """c^T F c == nrd(Bc/2)/T, i.e. 4T c^T F c == N4(Bc), for every c given."""
    b = columns(u, v)
    return all(4*t*eval_form(f, c) == N4(p, comb(b, c)) for c in coeffs)


def form_is_reduced(f):
    """The terminal state, read off F: F00 <= F22 and -F00 <= F2j < F00 (j = 0, 1).

    Only row 2 carries information: row 3 is (-h1', h0') and is implied.
    In normalized units A' = 2 F00, and -A' <= 2 h' < A' is -F00 <= h' < F00.
    """
    a = f[0][0]
    return a <= f[2][2] and all(-a <= f[2][j] < a for j in (0, 1))


def probe_vectors():
    units = [(1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1)]
    pairs = []
    for i in range(4):
        for j in range(i + 1, 4):
            pairs.append(tuple(units[i][k] + units[j][k] for k in range(4)))
    return units + pairs + [(-3, 1, 4, -2), (2, -5, 1, 1), (7, 0, -3, 2)]
