"""Which hypotheses are necessary?  Controls where one admission check is dropped."""
import random, itertools, json
from sq import *

out = {}
# (a) Z[i]-lattice in O0 of index 4N^2 that is NOT j-closed (not an O0-ideal): D/(2N) need not be a unit/integer.
#     U=(2N,0,0,0), V=(x,y,1,0) with (x^2+y^2+p) not = 0 mod 4N  -> the lattice is not left-closed.
p = 7; bad = []
for N in range(1, 10):
    for x in range(0, 2 * N, 2):
        for y in range(1, 2 * N, 2):
            u, v = (2*N, 0, 0, 0), (x, y, 1, 0)
            S = gaussian_lattice([u, v])
            closed = all(lat_contains(S, qmul(p, e, w)) for e in O0 for w in (u, v, J(u), J(v)))
            e_ = gdiv_exact(detUV(u, v), 2 * N)
            # for these shapes det = 2N exactly, so the quotient is 1; the failure is in the Gram, not in det
            gram_ok = (N4(p, u) % (2*N) == 0 and Hform(p, v, u)[0] % (2*N) == 0 and Hform(p, v, u)[1] % (2*N) == 0
                       and N4(p, v) % (2 * N) == 0)
            if not closed and not gram_ok: bad.append((N, x, y))
out["nonclosed_generic_shape_gram_not_integral_examples"] = bad[:5]
out["nonclosed_generic_shape_count"] = len(bad)
# (b) non-closed lattices with |det|^2 = 4N^2 where D/(2N) is not even integral or not a unit:
#     index exactly 4N^2 in Z^4 but U,V arbitrary in O0 parity: D = random Gaussian with |D|^2 = 4N^2
rng = random.Random(1)
cnt = 0; nonint = 0; nonunit = 0
for _ in range(20000):
    u = tuple(rng.randint(-6, 6) for _ in range(4)); v = tuple(rng.randint(-6, 6) for _ in range(4))
    if not (in_O0(u) and in_O0(v)): continue
    D_ = detUV(u, v)
    n2 = gnorm(D_)
    if n2 == 0: continue
    r = isqrt(n2)
    if r * r != n2 or r % 2: continue       # need |D|^2 = 4N^2
    N = r // 2
    cnt += 1
    S = gaussian_lattice([u, v])
    closed = all(lat_contains(S, qmul(p, e, w)) for e in O0 for w in (u, v, J(u), J(v)))
    e_ = gdiv_exact(D_, 2 * N)
    if e_ is None: nonint += 1
    elif gnorm(e_) != 1: nonunit += 1
    if closed and e_ is None: raise SystemExit("COUNTEREXAMPLE with closed lattice")
out["random_index_4N2_lattices"] = dict(tested=cnt, quotient_nonintegral=nonint)
# (c) j-closed (and i-closed) lattice that is NOT in O0: 2*O1, O1 = Z<1,i,j,k>.  D/(2N) is a unit, Gram integral,
#     but A' is odd: N' = A'/2 is not an integer and the output is not in O0.
res = []
for p in (3, 7, 11):
    u, v = (2, 0, 0, 0), (0, 0, 2, 0)       # numerators of 1 and j, scaled: the lattice 2 O1 in numerator coords is Z^4*2
    D_ = detUV(u, v)                          # = 4, |D|^2 = 16 = 4 N^2 -> N = 2
    N = isqrt(gnorm(D_)) // 2
    ok, why = jidentities(p, N, u, v)
    A = N4(p, u) // (2 * N)
    res.append(dict(p=p, N=N, D_over_2N=gdiv_exact(D_, 2 * N), jid=ok, A_prime=A, in_O0=in_O0(u) and in_O0(v)))
out["j_closed_not_in_O0"] = res
# (d) the (1+i) case that the 2-adic step excludes: Gaussian D with conj(D)=eps*D and |D|^2 = 2 m^2: never equals 4N^2.
ex = [(m, gnorm(gm((m, 0), (1, 1)))) for m in range(1, 6)]
out["one_plus_i_multiples_norms"] = ex   # 2m^2: 2,8,18,32,50 -- never of the form 4N^2
assert all(n != 4 * k * k for m, n in ex for k in range(1, 200))
json.dump(out, open("results/negative_controls.json", "w"), indent=1, default=str)
print(json.dumps(out, indent=1, default=str))
