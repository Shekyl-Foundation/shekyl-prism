# Copyright (c) 2025-2026, The Shekyl Foundation.  All rights reserved.  BSD-3-Clause
# Identities behind cellmap.c (re-derived for the tail proof), and the
# degenerate double tie that stalled the earlier certifier.
ok = True
def check(name, cond):
    global ok
    print(("OK   " if cond else "FAIL ") + name); ok = ok and cond

lam, ex, ey, mx, my = var('lam ex ey mx my')
r = lam + ex^2 + ey^2
qx, qy = ex/r, -ey/r                       # q = conj(eps)/r
lam1 = lam/r^2
r1 = lam1 + (qx - mx)^2 + (qy - my)^2      # ratio of the step after a swap with quotient m
# Lemma 1 identity: r' = (1 - 2 Re(m eps) + |m|^2 r)/r, Re(m eps) = mx ex - my ey
check("Lemma 1 identity r' = (1 - 2(mx ex - my ey) + |m|^2 r)/r",
      (r1 - (1 - 2*(mx*ex - my*ey) + (mx^2+my^2)*r)/r).simplify_full() == 0)
# hence r' < 1 <=> 1 + (|m|^2-1) r < 2(mx ex - my ey) <= |mx| + |my| on the CLOSED square;
# for |m|^2 <= 1 the right side is <= 1 <= left side: no swap (ties irrelevant).

# Lemma C: swap t had r_t = r >= 1/2 from eps_{t-1} = e, so |q_t,j| <= 1, Lemma 1 forces
# |m_t,j| = 1 and |eps_t,j| = 1 - u_j/r (u_j = |e_j|).  If r_{t+1} > 1/3 then |q_{t+1,j}| < 3/2,
# so |m_{t+1,j}| = 1 with the signs of q, and by the identity step t+2 swaps iff
#   1 + r_{t+1} < 2(|eps_t,x| + |eps_t,y|)  <=>  lam_t + (1-|eps_t,x|)^2 + (1-|eps_t,y|)^2 < 1.
a, b, L, u0, u1, rr = var('a b L u0 u1 rr')
check("Lemma C: 1 + (L+a^2+b^2) - 2(a+b) = L + (1-a)^2 + (1-b)^2 - 1",
      ((1 + L + a^2 + b^2 - 2*(a+b)) - (L + (1-a)^2 + (1-b)^2 - 1)).expand() == 0)
lprev = var('lprev'); R = lprev + u0^2 + u1^2
expr = lprev/R^2 + (u0/R)^2 + (u1/R)^2          # lam_t + sum (u_j/r)^2
check("Lemma C: lam_t + sum (u_j/r_t)^2 = 1/r_t  (so step t+2 swapping needs r_t > 1)",
      (expr - 1/R).simplify_full() == 0)

# Degenerate double tie (why "fewer than 4 swaps from 2^-7.6" cannot be closed by intervals):
# on the face eps_x = -1/2 with first quotient m = (-1, 1), the second step's q_y is EXACTLY 1/2.
e = var('e')
R0 = lam + 1/4 + e^2
Q1x, Q1y = (-1/2)/R0, -e/R0
E1x, E1y = Q1x + 1, Q1y - 1
L1 = lam/R0^2
R1 = L1 + E1x^2 + E1y^2
check("tie identity: r_2 = 2(r_1 + e)/r_1 on eps_x = -1/2, m_1 = (-1,1)", (R1 - 2*(R0 + e)/R0).simplify_full() == 0)
check("tie identity: q_2,y = -eps_1,y / r_2 = 1/2 exactly", ((-E1y)/R1 - 1/2).simplify_full() == 0)
# Half-up takes m_2,y = 1 (eps_2,y = -1/2); the closed box also admits m_2,y = 0 (eps_2,y = +1/2).
# With eps_2,y = -1/2 and a later unit quotient (0,1), Lemma 1 holds with equality r = 1 exactly
# (strict swap: no swap).  This is a closure artefact for the 4-swap bound; cellmap never needs
# that bound, because G_5 is proved over the closed square with r <= 1 counted as a swap.
print("ALL OK" if ok else "SOME CHECKS FAILED")
