# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Width-bound calculator for the Stage 2c prism-int mapping (SHE-25).

Every limb count below is derived from a proven bound, not from an
observation. The bounds are those of research/normalized-gram
(bounds_ng.py, lemmas W1 to W4), restated here in sign-magnitude terms
(prism-int holds a magnitude of N limbs and the sign beside it, so a value
of magnitude below 2^(64N) fits N limbs; no sign bit is spent):

  M0   public bound on the larger starting normalized diagonal, per class
         generic HNF:  A0 = 2T, C0 = (x^2+y^2+p)/(2T) < 4T + p/(2T)
                       -> M0 = 4 Tmax + floor(p/2) + 1
         guarded pre-reduced (F = 256): M0 = 2 F ceil(sqrt p) (qmax + 3)
  W1   A, C <= M0 and |h_j| < M0 at every step (det = p > 0)
  W2   quotient on both paths, used or discarded: |m_w A| <= 2 M0,
       X = 2h + A - 2 m_w A and X - 2A below 9 M0
  W3   carried update: |m 2^s A| <= 1.5 M0, |h + r| <= 2 M0,
       |m (h + r)| 2^s <= 2.25 M0
  W4   coordinates (numerator units) below 4 sqrt(2 Tmax M0)
  W5   leading words (this note): bl(A>>k) <= 64; the window numerator
       |h| >> (k+s) < 2^125 on both paths; the quotient word < 2^62; the
       128-by-64 division's precondition hi < n holds (NOTES.md 2.4)

Constructor and decode buffers:
  generic HNF numerator  n = x^2 + y^2 + p < 8 Tmax^2 + p
  general numerator Gram |N4|, |D| <= 2 Tmax M0 (numerator = 2T x normalized)
  determinant check      A' C' <= M0^2
  Hensel quotient        Q in [0, 2^(64 W)); product back Q T < 2^(64 W) Tmax
  decode guard           N4(v_s)^2 <= p (1024 N)^2 with N < 2^B, compared at
                         a width fixed by the key encoding (here: the bound
                         itself, plus one bit for a value that fails)

Run: python3 -B widths.py  [--csv out.csv]   (stdlib only)
"""
import math
import sys

# name, e, f, W (WIDTH_BITS), LIMBS (SIGNER_SPEC 2.1.2), D - 2^(2W) (SIGNER_SPEC 2.1.4)
SETS = [
    ('p248', 248, 5, 256, 26, 75),
    ('p324', 324, 3, 384, 33, 183),
    ('p376', 376, 65, 384, 38, 183),
    ('p500', 500, 27, 512, 49, 643),
    ('p664', 664, 17, 768, 65, 75),
]
GUARD = 256


def is_probable_prime(n):
    if n < 2:
        return False
    for sp in (2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37):
        if n % sp == 0:
            return n == sp
    d, s = n - 1, 0
    while d % 2 == 0:
        d //= 2
        s += 1
    for a in (2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41):
        x = pow(a, d, n)
        if x in (1, n - 1):
            continue
        for _ in range(s - 1):
            x = x * x % n
            if x == n - 1:
                break
        else:
            return False
    return True


def isqrt_up(x):
    r = math.isqrt(x)
    return r if r * r == x else r + 1


def limbs(bits):
    return -(-bits // 64)


def params(name, e, f, wbits, LIMBS, doff):
    p = f * (1 << e) - 1
    L = p.bit_length()
    B = (L + 1) // 2 + 32
    qbits = 2 * e - 5
    D = (1 << (2 * wbits)) + doff
    assert is_probable_prime(D), name
    return dict(name=name, p=p, L=L, B=B, e=e, qmax=1 << qbits, Tbits=B + qbits, D=D, LIMBS=LIMBS)


def class_bounds(P):
    """(Tmax, M0) per admitted class, as research/normalized-gram/bounds_ng.py."""
    p, qmax, B = P['p'], P['qmax'], P['B']
    out = {}
    for cls, Tm in (('sign-generic', 1 << P['Tbits']), ('keygen-Isec', 1 << B), ('challenge', qmax)):
        out[cls] = (Tm, 4 * Tm + p // 2 + 1)
    D = P['D']
    out['keygen-D'] = (D + 1, 4 * D + p // (2 * D) + 1)
    out['prered-guard256'] = ((1 << B) * qmax, 2 * GUARD * isqrt_up(p) * (qmax + 3))
    return out


def table(P):
    cb = class_bounds(P)
    rows = {}
    for cls, (Tm, M0) in cb.items():
        rows[cls] = dict(
            M0=M0.bit_length(),
            gram=(9 * M0).bit_length(),                       # W1-W3 (9 M0 dominates)
            coord=(4 * isqrt_up(2 * Tm * M0)).bit_length(),   # W4
            hnf_num=(8 * Tm * Tm + P['p']).bit_length() if cls != 'prered-guard256' else 0,
            gen_num=(2 * Tm * M0).bit_length(),
            det=(M0 * M0).bit_length(),
            Tbits=Tm.bit_length(),
        )
    W = limbs(max(r['gram'] for r in rows.values()))
    C = limbs(max(r['coord'] for r in rows.values()))
    tmax_bits = max(r['Tbits'] for r in rows.values())
    num = max(max(r['hnf_num'], r['gen_num'], r['det']) for r in rows.values())
    back = 64 * W + tmax_bits
    W2 = limbs(max(num, back))
    # decode guard: N4(v_s) <= 1024 sqrt(p) N, squared, vs p (1024 N)^2
    Nmax = 1 << P['B']
    guard_rhs = (P['p'] * (1024 * Nmax) ** 2).bit_length()
    guard = limbs(guard_rhs + 1)
    return rows, dict(W=W, C=C, W2=W2, num_bits=num, back_bits=back, guard=guard, guard_bits=guard_rhs)


# ------------------------------------------------------------ stack census

def size_of(n):
    """Bytes of a SecretInt<n>: n limbs + sign byte + mark byte, 8-aligned."""
    return 8 * n + 8


def census(W, C, W2):
    """Simultaneously live secret values, by phase (NOTES.md 2.6). An estimate."""
    s1, s2 = size_of(1), size_of(2)
    state = 8 * size_of(C) + 4 * size_of(W)                         # U, V, Gram (caller-owned)
    keyed = size_of(W) + size_of(C)                                  # T^-1 (or cached N^-1), T
    quotient = 5 * size_of(W) + 2 * s2 + 12 * s1                     # |h0|,|h1|,m_w A,X,X-2A; windows; counts, words
    coord = 4 * size_of(C) + 2 * s1                                  # two products, sum, shifted; m0, m1
    gram = 6 * size_of(W) + 2 * s1                                   # mA, shifted, r, h+r, m(h+r), shifted
    swap = 2 * size_of(max(W, C))                                    # select results before assignment
    internal_red = 4 * 8 * max(W, C) + 64                            # prism-int Scratch inside one op (sum, 2 diffs, selected)
    step_peak = max(quotient, coord, gram, swap) + internal_red + size_of(W)   # + one by-value result in flight
    reducer = state + keyed + step_peak
    # constructors at W2 (double width)
    gen_hnf = 5 * size_of(W2) + 6 * size_of(W) + 4 * 8 * W2 + 64
    general = 8 * size_of(W2) + 6 * size_of(W) + 4 * 8 * W2 + 64
    constructor = state + keyed + max(gen_hnf, general)
    return dict(state=state, keyed=keyed, step_peak=step_peak, reducer=reducer,
                constructor=constructor, peak=max(reducer, constructor))


def main(argv):
    out = []
    print('Per-class bounds (bits of magnitude):')
    print(f'{"set":5} {"class":16} {"M0":>5} {"9M0":>5} {"coord":>5} {"HNF n":>6} {"gen num":>7} {"M0^2":>5}')
    summary = []
    for s in SETS:
        P = params(*s)
        rows, t = table(P)
        for cls, r in rows.items():
            print(f'{P["name"]:5} {cls:16} {r["M0"]:5} {r["gram"]:5} {r["coord"]:5} {r["hnf_num"]:6} {r["gen_num"]:7} {r["det"]:5}')
        summary.append((P, t))
    print()
    print('Limbs per buffer (from the bounds above):')
    hdr = f'{"set":5} {"Gram W":>6} {"coord C":>7} {"ctor 2W":>7} {"num bits":>8} {"Q*T bits":>8} {"guard":>5} {"LIMBS":>5}'
    print(hdr)
    for P, t in summary:
        assert t['W'] == t['C'], 'the prototype relies on W == C'
        assert t['W2'] == 2 * t['W']
        assert t['W2'] <= P['LIMBS'], 'double width must fit the single 2.1.2 width'
        print(f'{P["name"]:5} {t["W"]:6} {t["C"]:7} {t["W2"]:7} {t["num_bits"]:8} {t["back_bits"]:8} {t["guard"]:5} {P["LIMBS"]:5}')
        out.append(dict(set=P['name'], W=t['W'], C=t['C'], W2=t['W2'], LIMBS=P['LIMBS'], guard=t['guard'],
                        spare_gram=64 * t['W'] - max(r for r in [table(P)[0][c]['gram'] for c in table(P)[0]]),
                        spare_coord=64 * t['C'] - max(table(P)[0][c]['coord'] for c in table(P)[0])))
    print()
    print('Spare bits above the binding bound: ' + ', '.join(f'{o["set"]} Gram {o["spare_gram"]} coord {o["spare_coord"]}' for o in out))
    print()
    print('Stack census (bytes; estimate, NOTES.md 2.6). Split widths vs the single 2.1.2 width:')
    print(f'{"set":5} {"state":>6} {"step":>6} {"reducer":>8} {"ctor":>6} {"PEAK":>6} | {"single-width PEAK":>17}')
    for P, t in summary:
        c = census(t['W'], t['C'], t['W2'])
        L = P['LIMBS']
        cs = census(L, L, L)
        print(f'{P["name"]:5} {c["state"]:6} {c["step_peak"]:6} {c["reducer"]:8} {c["constructor"]:6} {c["peak"]:6} | {cs["peak"]:17}')
        o = next(o for o in out if o['set'] == P['name'])
        o.update(peak=c['peak'], peak_single=cs['peak'], state=c['state'], reducer=c['reducer'], ctor=c['constructor'])
    if '--csv' in argv:
        path = argv[argv.index('--csv') + 1]
        keys = list(out[0])
        with open(path, 'w') as fh:
            fh.write(','.join(keys) + '\n')
            for o in out:
                fh.write(','.join(str(o[k]) for k in keys) + '\n')
    # Cross-check against research/normalized-gram/results/bounds.txt (W) and its RUST_C.
    assert [t['W'] for _, t in summary] == [11, 14, 16, 21, 27]
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
