# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# SHE-24: symbolic re-derivation of every identity the SHE-23 certificate
# uses, from the reducer contract.  Independent of earlier research code.
# Run: sage lemmas.sage
ok = True
def check(name, cond):
    global ok
    cond = bool(cond)
    ok = ok and cond
    print(("OK   " if cond else "FAIL ") + name)

# ---------------------------------------------------------------- units ---
p, T, x, y = var('p T x y')
def D(X, Y): return X[0]*Y[0] + X[1]*Y[1] + p*(X[2]*Y[2] + X[3]*Y[3])
def J(X): return vector(SR, [-X[1], X[0], -X[3], X[2]])
U = vector(SR, [2*T, 0, 0, 0]); V = vector(SR, [x, y, 1, 0])
A = D(U, U); C = D(V, V); h0 = D(V, U); h1 = D(V, J(U))
Delta = (A*C - h0^2 - h1^2).expand()
check("O0 numerators: Delta = 4 p T^2", (Delta - 4*p*T^2).expand() == 0)
check("reduced-norm units (entries/4): Delta = p T^2/4", ((A/4)*(C/4) - (h0/4)^2 - (h1/4)^2 - p*T^2/4).expand() == 0)
Cn = (x^2 + y^2 + p)/(2*T)
check("normalized Gram (2T, (x^2+y^2+p)/2T, x, y): Delta = p", ((2*T)*Cn - x^2 - y^2 - p).simplify_full() == 0)
for (a_, d_, nm) in [(A, Delta, 'O0'), (A/4, p*T^2/4, 'reduced-norm'), (2*T, p, 'normalized')]:
    check("A0^2/Delta = 4T^2/p in %s units (unswapped generic)" % nm, (a_^2/d_ - 4*T^2/p).simplify_full() == 0)
G4 = matrix(SR, 4, 4, lambda i, j: D([U, J(U), V, J(V)][i], [U, J(U), V, J(V)][j]))
check("4x4 real Gram determinant = Delta^2", (G4.det() - Delta^2).expand() == 0)
X = vector(SR, var('a0 a1 a2 a3')); Y = vector(SR, var('b0 b1 b2 b3'))
check("J is D-orthogonal", (D(J(X), J(Y)) - D(X, Y)).expand() == 0)
check("D(X, JX) = 0", D(X, J(X)).expand() == 0)
H = lambda X, Y: (D(X, Y), D(X, J(Y)))
check("H Hermitian: H(Y,X) = conj H(X,Y)",
      (H(Y, X)[0] - H(X, Y)[0]).expand() == 0 and (H(Y, X)[1] + H(X, Y)[1]).expand() == 0)
m0, m1 = var('m0 m1')
W = X - m0*Y - m1*J(Y)
check("H(V - mu U, U) = H(V,U) - mu A (mu = m0 + m1 i acting by J)",
      (H(W, Y)[0] - (H(X, Y)[0] - m0*D(Y, Y))).expand() == 0 and
      (H(W, Y)[1] - (H(X, Y)[1] - m1*D(Y, Y))).expand() == 0)
check("N4(V - mu U) = C - 2(m0 h0 + m1 h1) + |mu|^2 A",
      (D(W, W) - (D(X, X) - 2*(m0*H(X, Y)[0] + m1*H(X, Y)[1]) + (m0^2 + m1^2)*D(Y, Y))).expand() == 0)

# ------------------------------------------------------- step recurrence ---
# State before a step: A, lam = Delta/A^2, residual eps = h/A - mu.
Aa, lam, e0, e1 = var('Aa lam e0 e1')
Dl = lam*Aa^2
hh0 = Aa*(m0 + e0); hh1 = Aa*(m1 + e1)
Cc = (Dl + hh0^2 + hh1^2)/Aa                      # from Delta = AC - |h|^2
Anew = Cc - 2*(m0*hh0 + m1*hh1) + (m0^2 + m1^2)*Aa
check("A' = (lam + |eps|^2) A  (r = lam + |eps|^2)", (Anew - (lam + e0^2 + e1^2)*Aa).simplify_full() == 0)
r = lam + e0^2 + e1^2
# after the swap the pair is (v', u); h_new = conj(h - mu A) = A conj(eps)
q0 = (Aa*e0)/Anew; q1 = (-Aa*e1)/Anew
check("next exact quotient q = conj(eps)/r", (q0 - e0/r).simplify_full() == 0 and (q1 + e1/r).simplify_full() == 0)
check("lam' = lam / r^2", (Dl/Anew^2 - lam/r^2).simplify_full() == 0)
check("|q|^2 <= 1/r  (r - |eps|^2 = lam >= 0)", ((1/r - (e0^2 + e1^2)/r^2)*r^2 - lam).simplify_full() == 0)

# ----------------------------------------------------------------- Lemma 1 --
# Steps t and t+1 both swap.  Let k0,k1 be mu_{t+1}.  A_{t+1} in terms of A_{t-1}.
k0, k1 = var('k0 k1')
At = r*Aa
E0 = e0/r - k0; E1 = -e1/r - k1                # eps_t
At1 = (lam/r^2 + E0^2 + E1^2)*At
ident = Aa - 2*(k0*e0 - k1*e1)*Aa + (k0^2 + k1^2)*At   # A_{t-1} - 2Re(conj(mu)conj(eps))A_{t-1} + |mu|^2 A_t
check("Lemma 1 identity A_{t+1} = A_{t-1}(1 - 2Re(conj mu conj eps)) + |mu|^2 A_t",
      (At1 - ident).simplify_full() == 0)
# |mu|^2 = 1: A_{t+1} - A_t = A_{t-1}(1 - 2 s e_j) with |e_j| <= 1/2  => >= 0 => no strict swap.
for (a_, b_) in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
    d = (ident.subs(k0=a_, k1=b_) - At).simplify_full() / Aa
    check("Lemma 1, mu = %+d%+di: A_{t+1} - A_t = A_{t-1}*(%s), >= 0 on |e_j| <= 1/2" % (a_, b_, d.simplify_full()),
          all(bool(d.subs(e0=s0, e1=s1) >= 0) for s0 in (-1/2, 1/2) for s1 in (-1/2, 1/2)) and d.degree(e0) <= 1 and d.degree(e1) <= 1)
check("Lemma 1, mu = 0: A_{t+1} = A_{t-1}", (ident.subs(k0=0, k1=0) - Aa).simplify_full() == 0)

# ----------------------------------------------- Endgame identity (signs) ---
# If |q_{t+1,j}| < 3/2 and |mu|^2 >= 2 then mu_j = sign(q_j); q = (e0, -e1)/r.
for s0 in (1, -1):
    for s1 in (1, -1):
        sub = {k0: s0, k1: -s1}                     # sign(q0) = sign(e0), sign(q1) = -sign(e1)
        lhs = (ident.subs(sub) - At)/Aa
        rhs = lam - 1 + (1 - s0*e0)^2 + (1 - s1*e1)^2   # |e_j| = s_j e_j
        check("Endgame identity, sign(e) = (%+d,%+d): A_{t+1}-A_t = A_{t-1}(lam-1+(1-|e0|)^2+(1-|e1|)^2)" % (s0, s1),
              (lhs - rhs).simplify_full() == 0)
# Endgame: lam >= 1/2, |e_j| <= 1/2  =>  lam - 1 + (1-|e0|)^2 + (1-|e1|)^2 >= lam - 1/2 >= 0.
# Also the premise |q_j| <= (1/2)/r_{t} < 3/2 holds there because r_t >= sqrt(lam_{t-1}) >= 1/sqrt 2 when
# the following step swaps (lam_t < 1).  Checked: (1/2)*sqrt(2) < 3/2.
check("Endgame premise: (1/2)/(1/sqrt2) < 3/2", bool(sqrt(2)/2 < 3/2))

# ----------------------------------------------------------------- Lemma C --
# Steps t+1, t+2, t+3 all swap, r_{t+1} >= 1/2 (so |q_{t+2,j}| <= 1) and r_{t+2} > 1/3
# (so |q_{t+3,j}| < 3/2).  Lemma 1 forces mu_{t+2}, mu_{t+3} = sign pattern, |m_j| = 1.
# Then |eps_{t+1,j}| = 1 - |e_j|/r and the Endgame identity at t+1 gives
#   A_{t+3} - A_{t+2} = A_{t+1} (lam_{t+1} - 1 + sum (|e_j|/r)^2) = A_{t+1} (1/r - 1) >= 0.
u0, u1 = var('u0 u1')   # u_j = |e_j|
lam1 = lam/(lam + u0^2 + u1^2)^2
rr = lam + u0^2 + u1^2
expr = lam1 - 1 + (u0/rr)^2 + (u1/rr)^2
check("Lemma C: third-swap quantity = 1/r - 1", (expr - (1/rr - 1)).simplify_full() == 0)

# ----------------------------------------------------- head (first step) ---
Q2, R = var('Q2 R')   # |Q|^2, A^2/Delta
# C/A = |Q|^2 + Delta/A^2  =>  C^2/Delta = R (|Q|^2 + 1/R)^2 ; minus 4|Q|^2 is R(|Q|^2 - 1/R)^2 >= 0
check("head: C^2/Delta - 4|Q|^2 = R(|Q|^2 - 1/R)^2", ((R*(Q2 + 1/R)^2 - 4*Q2) - R*(Q2 - 1/R)^2).expand() == 0)

print("ALL OK" if ok else "SOME CHECKS FAILED")
