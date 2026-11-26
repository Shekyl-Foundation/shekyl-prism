"""Exact-integer checks of the inequalities the equivalence argument and the width claims rest on."""
from sage.all import isqrt, ZZ
from gen_corpus import SETS, params

LIMBS = {"p248": 11, "p324": 14, "p376": 16, "p500": 21, "p664": 27}
out = []
def log(s):
    print(s); out.append(s)

for name in SETS:
    f, e, p, L, B = params(name)
    W = LIMBS[name]
    log(f"== {name}: e={e} f={f} p prime={ZZ(p).is_prime()} p mod 4={p % 4} L={L} B={B} limbs={W}")
    # 1. q = 2^(2e-5) - s^2 with s odd is 7 mod 8, so q = 3 mod 4: Z[i]/q is a field for prime q.
    assert (2 * e - 5) >= 3
    for s in (1, 3, 5, 99, (1 << (e - 3)) - 1 | 1):
        assert ((1 << (2 * e - 5)) - s * s) % 8 == 7
    log("  q = 2^(2e-5) - s^2, s odd  =>  q = 7 (mod 8)  =>  q = 3 (mod 4)           [checked on samples; proof: s^2 = 1 mod 8, 8 | 2^(2e-5)]")
    # 2. guard bound below the smallest admissible q: nrd(v) <= 256 sqrt(p) N < 2^(2e-6) <= q
    lhs = (256 * (1 << B)) ** 2 * p      # (256 * 2^B)^2 p  >=  (256 sqrt(p) N)^2 for N < 2^B
    rhs = (1 << (2 * e - 6)) ** 2
    assert lhs < rhs
    log(f"  (256*2^B)^2 * p < (2^(2e-6))^2 : yes, margin {rhs.bit_length() - lhs.bit_length()} bits"
        "  =>  nrd(v) < 2^(2e-6) <= q for every admitted v")
    # 3. max N admitted by Q4 is < 2^B and q > 2^B so gcd(N, q) = 1 for prime q
    assert (1 << B) < (1 << (2 * e - 6))
    log(f"  N < 2^B = 2^{B} <= 2^(2e-6) = 2^{2*e-6} <= q: q prime does not divide N, coprime")
    # 4. widths: T = N q, |U_k|, |V_k| <= q sqrt(Cmax), Cmax = N4(v) bound = 1024 sqrt(p) N
    Nmax = (1 << B) - 1
    qmax = (1 << (2 * e - 5)) - 1
    Cmax = 1024 * (isqrt(p) + 1) * Nmax
    Tbits = (Nmax * qmax).bit_length()
    cb = (qmax * (isqrt(Cmax) + 1)).bit_length()
    Aprime = (qmax * Cmax // 2) // 1  # A' = q^2 C /(2 N q) = q C/(2N) <= 512 sqrt(p) q (using C <= 1024 sqrt(p) N)
    Ap = (512 * (isqrt(p) + 1) * qmax).bit_length()
    # N4(V)^(1/2) <= N4(u)^(1/2) + |gamma| N4(v)^(1/2) <= C^(1/2) (1 + (q-1)/sqrt 2), so N4(V) < q^2 C and C' < q C/(2N) <= 512 sqrt(p) q
    Cp = Ap
    log(f"  width bits: T=Nq <= {Tbits}; |U_k|,|V_k| <= q sqrt(Cmax) <= {cb}; A' <= 512 sqrt(p) q: {Ap}; C' < 512 sqrt(p) q: {Cp}; limb capacity {64*W}")
    assert max(Tbits, cb, Ap, Cp) < 64 * W
    log(f"  -> all below 64*W with margin {64*W - max(Tbits, cb, Ap, Cp)} bits")
open("results/bounds.txt", "w").write("\n".join(out) + "\n")
