"""Shortest-equivalent-ideal shortcut: formulas, direct (quaternion + HNF) reference, admission predicate.

Conventions as research/signing-intersection-oracle/oracle.py (copied here as oracle.py, unchanged):
  numerator X=(a,b,c,d) = 2x, z1=a+bi, z2=c+di, x = (z1 + z2 j)/2, j z = conj(z) j, j^2=-p.
  Left mult by i is J.  H(w,w') = z1 conj(z1') + p z2 conj(z2')  (Gaussian).  det(U,V) = z1u z2v - z1v z2u.
"""
from oracle import *
from sage.all import ZZ

def gm(x, y):  return (x[0]*y[0]-x[1]*y[1], x[0]*y[1]+x[1]*y[0])
def gsub(x, y): return (x[0]-y[0], x[1]-y[1])
def gadd(x, y): return (x[0]+y[0], x[1]+y[1])
def gconj(x):  return (x[0], -x[1])
def gnorm(x):  return x[0]*x[0]+x[1]*x[1]
UNITS = [(1, 0), (0, 1), (-1, 0), (0, -1)]

def Hform(p, w, w2):
    """Gaussian Hermitian product H(w,w2) = z1 conj(z1') + p z2 conj(z2')."""
    a = gm((w[0], w[1]), gconj((w2[0], w2[1])))
    b = gm((w[2], w[3]), gconj((w2[2], w2[3])))
    return (a[0] + p*b[0], a[1] + p*b[1])

def detUV(u, v):
    return gsub(gm((u[0], u[1]), (v[2], v[3])), gm((v[0], v[1]), (u[2], u[3])))

def conj_num(X):
    return (X[0], -X[1], -X[2], -X[3])

def qconj_mul(p, w, beta):
    """Numerator of w * conj(beta) (exact quaternion product of numerators' elements)."""
    return qmul(p, w, conj_num(beta))

# -------------------------------------------------------------- the shortcut

def gdiv_exact(z, m):
    """z / m for integer m if both parts divisible, else None."""
    if z[0] % m or z[1] % m:
        return None
    return (z[0] // m, z[1] // m)

def shortcut(p, N, u, v, eps=(1, 0)):
    """I * conj(beta) / N for beta = eps*u (eps a Gaussian unit), from (u, v) only.
    Returns dict or None with the reason.  Uses: A' = N4(u)/2N, g = H(v,u)/2N, e = det(u,v)/2N.
      N'  = A'/2;  Z[i]-basis of the output  U' = (A' conj(eps), 0),   V' = (g conj(eps), e*eps).
    Normalised by the unit conj(e*eps): root r = conj(e)*conj(eps)^2 * g, V'' = (r, 1)."""
    m = 2 * N
    A2 = N4(p, u)
    if A2 % m: return dict(fail="A not divisible by 2N")
    A = A2 // m
    if A % 2: return dict(fail="A' odd")
    g = gdiv_exact(Hform(p, v, u), m)
    if g is None: return dict(fail="H(v,u) not divisible by 2N")
    e = gdiv_exact(detUV(u, v), m)
    if e is None: return dict(fail="det/2N not integral")
    if gnorm(e) != 1: return dict(fail="det/2N not a unit", e=e)
    ce = gconj(eps)
    Up = (A * ce[0], A * ce[1], 0, 0)          # numerators of U' = u conj(beta)/N  (Z[i]-part only)
    Vp = (*gm(g, ce), *gm(e, eps))
    root = gm(gconj(gm(e, eps)), gm(g, ce))   # conj(e eps) * (g conj(eps))
    return dict(Np=A // 2, A=A, g=g, e=e, Up=Up, Vp=Vp, root=root)

def shortcut_second(p, N, u, v):
    """beta = v (second column): swap roles, I*conj(v)/N."""
    return shortcut(p, N, v, u)

def gaussian_lattice(vecs):
    out = []
    for w in vecs:
        out += [w, J(w)]
    return hnf_rows(out)

def direct(p, N, u, v, beta):
    """Reference: Z-lattice of { w * conj(beta) / N : w in {u, Ju, v, Jv} }, canonical row HNF; or None."""
    rows = []
    for w in (u, J(u), v, J(v)):
        X = qconj_mul(p, w, beta)
        if any(x % N for x in X):
            return None
        rows.append(tuple(x // N for x in X))
    return hnf_rows(rows)

def canonical_cols(Np, root):
    """GenericHnf columns for norm Np and Gaussian root reduced into [0, 2Np)."""
    piv = 2 * Np
    x, y = root[0] % piv, root[1] % piv
    return ((piv, 0, 0, 0), (0, piv, 0, 0), (x, y, 1, 0), (piv - y, x, 0, 1))

# --------------------------------------------------- admission predicate (spec, independent re-implementation)

GUARD_BITS = 8

def admit_report(p, N, u, v, guard=True):
    """Returns dict of the individual admission checks of GuardedIdeal::admit on the Z[i]-basis (u,v)
    (HNF-column checks replaced by the lattice-level equalities they certify)."""
    L = p.bit_length(); B = -(-L // 2) + 32
    r = {}
    r["N_odd"] = (N % 2 == 1)
    r["N_pos"] = N > 0
    r["N_lt_2B"] = N < (1 << B)
    det = detUV(u, v)
    r["det_sq_eq_4N2"] = gnorm(det) == 4 * N * N
    r["parity_O0"] = all(in_O0(w) and in_O0(J(w)) for w in (u, v))
    S = gaussian_lattice([u, v])
    r["index_4N2"] = len(S) == 4 and lat_index(S) == 4 * N * N
    r["left_closed"] = len(S) == 4 and all(lat_contains(S, qmul(p, e, w)) for e in O0 for w in (u, v, J(u), J(v)))
    a, c = N4(p, u), N4(p, v)
    r["a_pos"] = a > 0
    r["c_ge_a"] = c >= a
    cap = N * (1 << (2 + GUARD_BITS))
    r["guard_c2_le_p_cap2"] = c * c <= p * cap * cap
    r["ideal"] = r["parity_O0"] and r["left_closed"] and r["index_4N2"]
    r["admissible"] = all(r[k] for k in ("N_odd","N_pos","N_lt_2B","det_sq_eq_4N2","parity_O0","index_4N2","left_closed","a_pos","c_ge_a","guard_c2_le_p_cap2"))
    return r

def reduced_flag(p, u, v):
    """Size-reduced under the contract: quotient floor(h/A+1/2) is 0 for both components, and N4(u) <= N4(v)."""
    A = N4(p, u)
    h0, h1 = D(p, v, u), D(p, v, J(u))
    return half_up_div(h0, A) == 0 and half_up_div(h1, A) == 0 and N4(p, v) >= A

SETS = {"p248": (5, 248), "p324": (3, 324), "p376": (65, 376), "p500": (27, 500), "p664": (17, 664)}

def params(name):
    f, e = SETS[name]
    p = f * (1 << e) - 1
    assert ZZ(p).is_prime()
    L = p.bit_length()
    return f, e, p, L, (L + 1) // 2 + 32

def j_act(p, w):
    """left multiplication by j on numerators: (z1, z2) -> (-p conj z2, conj z1)."""
    return (-p * w[2], p * w[3], w[0], -w[1])

def coords_in_basis(u, v, w):
    """Solve w = a u + b v over Z[i] (a,b Gaussian) or None, via Cramer on (z1,z2)."""
    D_ = detUV(u, v)
    W = ((w[0], w[1]), (w[2], w[3]))
    U_ = ((u[0], u[1]), (u[2], u[3])); V_ = ((v[0], v[1]), (v[2], v[3]))
    # w = a u + b v: z1: a u1 + b v1 = w1, z2: a u2 + b v2 = w2.
    # a = (w1 v2 - v1 w2)/D, b = (u1 w2 - w1 u2)/D
    an = gsub(gm(W[0], V_[1]), gm(V_[0], W[1]))
    bn = gsub(gm(U_[0], W[1]), gm(W[0], U_[1]))
    cd = gconj(D_); n = gnorm(D_)
    a = gm(an, cd); b = gm(bn, cd)
    if any(x % n for x in a + b): return None
    return (a[0]//n, a[1]//n), (b[0]//n, b[1]//n)

def jidentities(p, N, u, v):
    """S from j u = a u + b v, j v = c u + d v;  det S * D = p conj D;  det S = p*unit;  Gram = D Omega S^T."""
    ju, jv = j_act(p, u), j_act(p, v)
    r1 = coords_in_basis(u, v, ju); r2 = coords_in_basis(u, v, jv)
    if r1 is None or r2 is None: return False, "j u / j v not in span"
    (a, b), (c, d) = r1, r2
    Dd = detUV(u, v)
    dS = gsub(gm(a, d), gm(b, c))
    if gm(dS, Dd) != (p * Dd[0], -p * Dd[1]): return False, "det S * D != p conj D"
    if gnorm(dS) != p * p: return False, "|det S| != p"
    # Gram identities: H(u,u) = D b, H(u,v) = D d, H(v,u) = -D a, H(v,v) = -D c
    ok = (Hform(p, u, u) == gm(Dd, b) and Hform(p, u, v) == gm(Dd, d) and
          Hform(p, v, u) == gm((-Dd[0], -Dd[1]), a) and Hform(p, v, v) == gm((-Dd[0], -Dd[1]), c))
    if not ok: return False, "Gram != D Omega S^T"
    e = gdiv_exact(Dd, 2 * N)
    if e is None or gnorm(e) != 1: return False, "D/2N not unit"
    # conj(D) = eps' D with eps' = conj(e)/e = conj(e)^2 = +-1
    return True, ""

