# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Word-level bounds of the shared-division exp64 step (NOTES.md 2.4, W5).

Models Rick's shared division (a60fd06, `zi_exp::quotient_exp`) on
sign-magnitude values, as a prism-int port would hold them: one 128-by-64
division per component feeds both the window correction and the big
undershoot, and both paths are always computed. Asserts, on random and
edge-case operands at every Gram width of the five sets:

  (a) n = A >> k fits one word, and n >= 2^63 once bl(A) >= 64;
  (b) the numerator |h| >> (k + s) fits two words (< 2^125);
  (c) the division precondition hi < n holds, so the fixed 64-iteration
      division never needs to saturate, and the quotient word is < 2^62;
  (d) on the window path the truncated-magnitude estimate is within one
      of the half-up quotient, and the +-1 correction from X makes it exact
      (both half ties included);
  (e) used or discarded, |m_w A| <= 2 max(A, |h|) and |X|, |X - 2A| < 9 max(A, |h|),
      i.e. W2 holds on the discarded path too, so the sticky mark is never
      set by the path that is thrown away;
  (f) every secret shift count lies in [0, 64 W).

Run: python3 -B check_exp64_words.py [samples]   (stdlib only)
"""
import random
import sys

T0 = 60


def bl(x):
    return abs(x).bit_length()


def half_up(h, a):
    return (2 * h + a) // (2 * a)


def step_words(h0, h1, a, wbits):
    k = max(0, bl(a) - 64)
    n = a >> k
    assert 0 < n < 1 << 64                                   # (a)
    if bl(a) >= 64:
        assert n >= 1 << 63
    e = max(bl(h0), bl(h1)) - bl(a)
    big = e > T0
    s = e - T0 if big else 0
    assert 0 <= k + s < wbits                                # (f)
    out = []
    for h in (h0, h1):
        num = abs(h) >> (k + s)
        assert num < 1 << 125                                # (b)
        assert (num >> 64) < n                               # (c) precondition
        qtop = num // n
        assert qtop < 1 << 62                                # (c)
        sg = -1 if h < 0 else 1
        # window path: estimate, then the exact +-1 correction from X
        mw = sg * qtop
        X = 2 * h + a - 2 * mw * a
        lim = 9 * max(a, abs(h0), abs(h1))
        assert abs(mw * a) <= 2 * max(a, abs(h0), abs(h1))   # (e)
        assert abs(X) < lim and abs(X - 2 * a) < lim         # (e)
        mw_c = mw - (X < 0) + (X >= 2 * a)
        # big path: undershoot by one unit of 2^s
        mb = sg * (qtop - 1 if qtop >= 2 else 0)
        if not big:
            target = half_up(h, a)
            assert abs(mw - target) <= 1, (h, a, mw, target)  # (d)
            assert mw_c == target                             # (d)
            out.append((mw_c, 0))
        else:
            out.append((mb, s))
    return out


def main(argv):
    samples = int(argv[0]) if argv else 200000
    rng = random.Random(20261125)
    widths = [64 * w for w in (11, 14, 16, 21, 27)]
    count = 0
    for wbits in widths:
        top = wbits - 4                                      # 9 M0 < 2^(64W): M0 < 2^(64W - 4)
        for _ in range(samples // len(widths)):
            abits = rng.choice([1, 2, 3, 63, 64, 65, 127, 128, rng.randint(1, top)])
            a = rng.getrandbits(abits) | (1 << (abits - 1))
            if rng.random() < 0.1:
                a = 1 << (abits - 1)                         # power of two: n exact
            mode = rng.random()
            hs = []
            for _ in range(2):
                if mode < 0.3:                               # exact half ties, either sign
                    m = rng.getrandbits(rng.randint(0, min(60, top - abits)))
                    h = m * a + a // 2 if a % 2 == 0 else m * a + (a - 1) // 2
                    h = h if rng.random() < 0.5 else -h
                elif mode < 0.6:                             # near the window edge e = 60, 61
                    hb = min(top, abits + rng.choice([59, 60, 61, 62]))
                    h = rng.getrandbits(hb) | (1 << (hb - 1))
                    h = h if rng.random() < 0.5 else -h
                else:
                    hb = rng.randint(0, top)
                    h = rng.getrandbits(hb) if hb else 0
                    h = h if rng.random() < 0.5 else -h
                hs.append(h)
            step_words(hs[0], hs[1], a, wbits)
            count += 1
    # deterministic edge cases
    for wbits in widths:
        for a in (1, 2, 3, (1 << 63) - 1, 1 << 63, (1 << 64) - 1, 1 << 64, (1 << 64) + 1):
            for h in (0, 1, -1, a // 2, -(a // 2), a, -a, (a << 60) - 1, -((a << 60) - 1), a << 60, a << 61):
                step_words(h, -h, a, wbits)
                count += 1
    print(f'exp64 word bounds (a)-(f) hold on {count} operand pairs at Gram widths 11/14/16/21/27 limbs')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
