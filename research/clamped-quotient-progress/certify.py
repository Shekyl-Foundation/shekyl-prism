# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Exact rational (gmpy2.mpq) interval branch-and-bound certifier for the
Z[i]-Lagrange fixed-schedule worst-case step bound.

Zero floating-point approximations: every bound is proved using exact
GMP rational arithmetic over dyadic boxes in [0, lam_max] x [-1/2, 1/2]^2.
"""
import time
from gmpy2 import mpq

ZERO = mpq(0)
ONE = mpq(1)
HALF = mpq(1, 2)
TWO = mpq(2)
QTR = mpq(1, 4)


def floor_half_up(f: mpq) -> int:
    """Exact floor(f + 1/2) == (2*num + den) // (2*den) for positive den."""
    num = f.numerator
    den = f.denominator
    return int((2 * num + den) // (2 * den))


def iv_sq(lo: mpq, hi: mpq):
    if lo >= ZERO:
        return lo * lo, hi * hi
    if hi <= ZERO:
        return hi * hi, lo * lo
    return ZERO, max(lo * lo, hi * hi)


def iv_div(alo: mpq, ahi: mpq, blo: mpq, bhi: mpq):
    """Exact interval [alo, ahi] / [blo, bhi] assuming blo > 0."""
    c1 = alo / blo
    c2 = alo / bhi
    c3 = ahi / blo
    c4 = ahi / bhi
    return min(c1, c2, c3, c4), max(c1, c2, c3, c4)


def certify_1step_exact(c_lyap: mpq, target: mpq, lam_max: mpq):
    """Certify that for all lam in [0, lam_max] and (x, y) in [-1/2, 1/2]^2
    such that r = lam + x^2 + y^2 < 1 and step 2 swaps (lam/r^2 + |ep|^2 < 1),
    we have r * (1 + c_lyap * |ep|^2) / (1 + c_lyap * (x^2 + y^2)) <= target.
    """
    stack = [(ZERO, lam_max, -HALF, HALF, -HALF, HALF)]
    nodes = 0
    max_ub = ZERO
    min_dx = mpq(1, 1 << 22)
    while stack:
        ll, lh, xl, xh, yl, yh = stack.pop()
        nodes += 1
        x2l, x2h = iv_sq(xl, xh)
        y2l, y2h = iv_sq(yl, yh)
        rl = ll + x2l + y2l
        rh = lh + x2h + y2h
        if rl >= ONE:
            continue
        rh_c = min(rh, ONE)
        ve_lo = ONE + c_lyap * (x2l + y2l)
        ub_triv = rh_c * (ONE + c_lyap * HALF) / ve_lo
        if ub_triv <= target:
            if ub_triv > max_ub:
                max_ub = ub_triv
            continue
        if rl > ZERO:
            q0l, q0h = iv_div(xl, xh, rl, rh_c)
            q1l, q1h = iv_div(-yh, -yl, rl, rh_c)
            m0l, m0h = floor_half_up(q0l), floor_half_up(q0h)
            m1l, m1h = floor_half_up(q1l), floor_half_up(q1h)
            if (m0h - m0l + 1) * (m1h - m1l + 1) <= 25:
                box_ub = ZERO
                for m0 in range(m0l, m0h + 1):
                    for m1 in range(m1l, m1h + 1):
                        if m0 * m0 + m1 * m1 <= 1:
                            continue
                        ep0l = max(-HALF, q0l - m0)
                        ep0h = min(HALF, q0h - m0)
                        ep1l = max(-HALF, q1l - m1)
                        ep1h = min(HALF, q1h - m1)
                        if ep0l > ep0h or ep1l > ep1h:
                            continue
                        ep2l = iv_sq(ep0l, ep0h)[0] + iv_sq(ep1l, ep1h)[0]
                        ep2h = iv_sq(ep0l, ep0h)[1] + iv_sq(ep1l, ep1h)[1]
                        if ll / (rh_c * rh_c) + ep2l >= ONE:
                            continue
                        val_ub = rh_c * (ONE + c_lyap * min(HALF, ep2h)) / ve_lo
                        if val_ub > box_ub:
                            box_ub = val_ub
                if box_ub <= target:
                    if box_ub > max_ub:
                        max_ub = box_ub
                    continue
        dx, dy, dl = xh - xl, yh - yl, lh - ll
        if dx < min_dx and dy < min_dx and dl < min_dx:
            return False, nodes, (ll, lh, xl, xh, yl, yh)
        if dx >= dy and dx >= dl * 100:
            xm = (xl + xh) / TWO
            stack.append((ll, lh, xl, xm, yl, yh))
            stack.append((ll, lh, xm, xh, yl, yh))
        elif dy >= dl * 100:
            ym = (yl + yh) / TWO
            stack.append((ll, lh, xl, xh, yl, ym))
            stack.append((ll, lh, xl, xh, ym, yh))
        else:
            lm = (ll + lh) / TWO
            stack.append((ll, lm, xl, xh, yl, yh))
            stack.append((lm, lh, xl, xh, yl, yh))
    return True, nodes, max_ub


def certify_2step_exact(c_lyap: mpq, target: mpq, lam_max: mpq):
    """Certify 2-step main-phase decay <= target when lam0, lam1, lam2 < lam_max."""
    stack = [(ZERO, lam_max, -HALF, HALF, -HALF, HALF)]
    nodes = 0
    max_ub = ZERO
    min_dx = mpq(1, 1 << 22)
    while stack:
        l0l, l0h, x0l, x0h, y0l, y0h = stack.pop()
        nodes += 1
        x2l, x2h = iv_sq(x0l, x0h)
        y2l, y2h = iv_sq(y0l, y0h)
        r1l = l0l + x2l + y2l
        r1h = l0h + x2h + y2h
        if r1l >= ONE:
            continue
        r1h_c = min(r1h, ONE)
        ve0l = ONE + c_lyap * (x2l + y2l)
        ok_box = False
        if r1h_c * (ONE + HALF * c_lyap) / ve0l <= target:
            ok_box = True
        elif r1l > ZERO:
            l1l = l0l / (r1h_c * r1h_c)
            l1h = l0h / (r1l * r1l)
            if l1l >= lam_max:
                continue
            q10l, q10h = iv_div(x0l, x0h, r1l, r1h_c)
            q11l, q11h = iv_div(-y0h, -y0l, r1l, r1h_c)
            m10l, m10h = floor_half_up(q10l), floor_half_up(q10h)
            m11l, m11h = floor_half_up(q11l), floor_half_up(q11h)
            if (m10h - m10l + 1) * (m11h - m11l + 1) <= 16:
                box_ub = ZERO
                too_many = False
                for m10 in range(m10l, m10h + 1):
                    for m11 in range(m11l, m11h + 1):
                        if m10 * m10 + m11 * m11 <= 1:
                            continue
                        ep10l = max(-HALF, q10l - m10)
                        ep10h = min(HALF, q10h - m10)
                        ep11l = max(-HALF, q11l - m11)
                        ep11h = min(HALF, q11h - m11)
                        if ep10l > ep10h or ep11l > ep11h:
                            continue
                        ep1_2l = iv_sq(ep10l, ep10h)[0] + iv_sq(ep11l, ep11h)[0]
                        ep1_2h = iv_sq(ep10l, ep10h)[1] + iv_sq(ep11l, ep11h)[1]
                        r2l = l1l + ep1_2l
                        r2h = l1h + ep1_2h
                        if r2l >= ONE:
                            continue
                        r2h_c = min(r2h, ONE)
                        ub1 = r1h_c * r2h_c * (ONE + HALF * c_lyap) / ve0l
                        if ub1 <= target:
                            if ub1 > box_ub:
                                box_ub = ub1
                            continue
                        if r2l <= ZERO:
                            too_many = True
                            break
                        l2l = l1l / (r2h_c * r2h_c)
                        if l2l >= lam_max:
                            continue
                        q20l, q20h = iv_div(ep10l, ep10h, r2l, r2h_c)
                        q21l, q21h = iv_div(-ep11h, -ep11l, r2l, r2h_c)
                        m20l, m20h = floor_half_up(q20l), floor_half_up(q20h)
                        m21l, m21h = floor_half_up(q21l), floor_half_up(q21h)
                        if (m20h - m20l + 1) * (m21h - m21l + 1) > 16:
                            too_many = True
                            break
                        for m20 in range(m20l, m20h + 1):
                            for m21 in range(m21l, m21h + 1):
                                if m20 * m20 + m21 * m21 <= 1:
                                    continue
                                ep20l = max(-HALF, q20l - m20)
                                ep20h = min(HALF, q20h - m20)
                                ep21l = max(-HALF, q21l - m21)
                                ep21h = min(HALF, q21h - m21)
                                if ep20l > ep20h or ep21l > ep21h:
                                    continue
                                ep2_2l = iv_sq(ep20l, ep20h)[0] + iv_sq(ep21l, ep21h)[0]
                                ep2_2h = iv_sq(ep20l, ep20h)[1] + iv_sq(ep21l, ep21h)[1]
                                if l2l + ep2_2l >= ONE:
                                    continue
                                ub2 = r1h_c * r2h_c * (ONE + c_lyap * min(HALF, ep2_2h)) / ve0l
                                if ub2 > box_ub:
                                    box_ub = ub2
                    if too_many:
                        break
                if not too_many and box_ub <= target:
                    ok_box = True
                    if box_ub > max_ub:
                        max_ub = box_ub
        if ok_box:
            continue
        dx, dy, dl = x0h - x0l, y0h - y0l, l0h - l0l
        if dx < min_dx and dy < min_dx and dl < min_dx:
            return False, nodes, (l0l, l0h, x0l, x0h, y0l, y0h)
        if dx >= dy and dx >= dl * 100:
            xm = (x0l + x0h) / TWO
            stack.append((l0l, l0h, x0l, xm, y0l, y0h))
            stack.append((l0l, l0h, xm, x0h, y0l, y0h))
        elif dy >= dl * 100:
            ym = (y0l + y0h) / TWO
            stack.append((l0l, l0h, x0l, x0h, y0l, ym))
            stack.append((l0l, l0h, x0l, x0h, ym, y0h))
        else:
            lm = (l0l + l0h) / TWO
            stack.append((l0l, lm, x0l, x0h, y0l, y0h))
            stack.append((lm, l0h, x0l, x0h, y0l, y0h))
    return True, nodes, max_ub


def certify_tail_exact(lam_min: mpq, target_swaps: int = 5):
    """Certify that starting from lam >= lam_min, >= target_swaps swaps is impossible."""
    def can_reach(l0l, l0h, x0l, x0h, y0l, y0h):
        def rec(ll, lh, xl, xh, yl, yh, d):
            x2l, x2h = iv_sq(xl, xh)
            y2l, y2h = iv_sq(yl, yh)
            rl = ll + x2l + y2l
            rh = lh + x2h + y2h
            if rl >= ONE:
                return False
            if d >= target_swaps:
                return True
            if rl <= ZERO:
                return True
            rh_c = min(rh, ONE)
            if ll >= HALF:
                return False
            if ll >= QTR and (d + 1) < target_swaps:
                return False
            q0l, q0h = iv_div(xl, xh, rl, rh_c)
            q1l, q1h = iv_div(-yh, -yl, rl, rh_c)
            m0l, m0h = floor_half_up(q0l), floor_half_up(q0h)
            m1l, m1h = floor_half_up(q1l), floor_half_up(q1h)
            if (m0h - m0l + 1) * (m1h - m1l + 1) > 25:
                return True
            nl_l = ll / (rh_c * rh_c)
            nl_h = lh / (rl * rl)
            for m0 in range(m0l, m0h + 1):
                for m1 in range(m1l, m1h + 1):
                    m2 = m0 * m0 + m1 * m1
                    if m2 <= 1:
                        continue
                    dot_hi = (m0 * xh if m0 >= 0 else m0 * xl) - (m1 * yl if m1 >= 0 else m1 * yh)
                    if (m2 - 1) * rl >= TWO * dot_hi - ONE:
                        continue
                    ep0l = max(-HALF, q0l - m0)
                    ep0h = min(HALF, q0h - m0)
                    ep1l = max(-HALF, q1l - m1)
                    ep1h = min(HALF, q1h - m1)
                    if ep0l > ep0h or ep1l > ep1h:
                        continue
                    if rec(nl_l, nl_h, ep0l, ep0h, ep1l, ep1h, d + 1):
                        return True
            return False

        return rec(l0l, l0h, x0l, x0h, y0l, y0h, 1)

    stack = [(lam_min, QTR, -HALF, HALF, -HALF, HALF)]
    nodes = 0
    min_dx = mpq(1, 1 << 18)
    while stack:
        box = stack.pop()
        nodes += 1
        if not can_reach(*box):
            continue
        ll, lh, xl, xh, yl, yh = box
        dx, dy, dl = xh - xl, yh - yl, lh - ll
        if dx < min_dx and dy < min_dx and dl < min_dx:
            return False, nodes, box
        if dx >= dy and dx >= dl * 10:
            xm = (xl + xh) / TWO
            stack.append((ll, lh, xl, xm, yl, yh))
            stack.append((ll, lh, xm, xh, yl, yh))
        elif dy >= dl * 10:
            ym = (yl + yh) / TWO
            stack.append((ll, lh, xl, xh, yl, ym))
            stack.append((ll, lh, xl, xh, ym, yh))
        else:
            lm = (ll + lh) / TWO
            stack.append((ll, lm, xl, xh, yl, yh))
            stack.append((lm, lh, xl, xh, yl, yh))
    return True, nodes, None


if __name__ == '__main__':
    t0 = time.time()
    ok1, n1, ub1 = certify_1step_exact(TWO, mpq(31, 100), mpq(1, 1000))
    print(f'Exact 1-step (<= 0.31, lam<=0.001): ok={ok1}, nodes={n1}, max_ub={float(ub1):.8f}, dt={time.time()-t0:.2f}s')

    t0 = time.time()
    ok2, n2, ub2 = certify_2step_exact(TWO, mpq(90, 1000), mpq(1, 1000))
    print(f'Exact 2-step (<= 0.090, lam<=0.001): ok={ok2}, nodes={n2}, max_ub={float(ub2) if ok2 else ub2}, dt={time.time()-t0:.2f}s')

    t0 = time.time()
    ok2b, n2b, ub2b = certify_2step_exact(TWO, mpq(86, 1000), mpq(1, 1000))
    print(f'Exact 2-step (<= 0.086, lam<=0.001): ok={ok2b}, nodes={n2b}, max_ub={float(ub2b) if ok2b else ub2b}, dt={time.time()-t0:.2f}s')

    t0 = time.time()
    ok3, n3, fail3 = certify_tail_exact(mpq(1, 1000), 5)
    print(f'Exact Tail (>= 5 swaps impossible for lam>=0.001): ok={ok3}, nodes={n3}, fail={fail3}, dt={time.time()-t0:.2f}s')
