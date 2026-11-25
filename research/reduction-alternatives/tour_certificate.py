#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Exact interval check for a block of consecutive main-phase contractions."""

from pathlib import Path
import sys

sys.path.insert(
    0, str(Path(__file__).resolve().parents[1] / "clamped-quotient-progress")
)
from native_counts import THREE, OLD
from certify import (
    iv_sq,
    iv_div,
    floor_half_up,
    certify_1step_exact,
    certify_2step_exact,
    certify_tail_exact,
)
from gmpy2 import mpq

ZERO, ONE, HALF = mpq(0), mpq(1), mpq(1, 2)
SPLIT = mpq(1, 1000)


def certify_block(length, target):
    # Coordinates are (Delta/A^2, real residual, imaginary residual).
    def covered(box, depth, product, initial):
        ll, lh, xl, xh, yl, yh = box
        x2l, x2h = iv_sq(xl, xh)
        y2l, y2h = iv_sq(yl, yh)
        low, high = x2l + y2l, x2h + y2h
        if ll > SPLIT or ll + low >= ONE:
            return True
        if depth == length:
            return product * (ONE + 2 * high) <= target * initial
        rl, rh = ll + low, min(ONE, lh + high)
        product *= rh
        # All remaining swaps have r < 1; the terminal weight is <= 2.
        if 2 * product <= target * initial:
            return True
        if rl == 0:
            return False
        next_l = ll / (rh * rh)
        if next_l > SPLIT:
            return True
        next_h = min(SPLIT, lh / (rl * rl))
        ax, bx = iv_div(xl, xh, rl, rh)
        ay, by = iv_div(-yh, -yl, rl, rh)
        x0, x1 = floor_half_up(ax), floor_half_up(bx)
        y0, y1 = floor_half_up(ay), floor_half_up(by)
        if (x1 - x0 + 1) * (y1 - y0 + 1) > 32:
            return False
        for i in range(x0, x1 + 1):
            for j in range(y0, y1 + 1):
                if i * i + j * j <= 1:
                    continue  # The next step swaps strictly; Lemma 1.
                child = (
                    next_l,
                    next_h,
                    max(-HALF, ax - i),
                    min(HALF, bx - i),
                    max(-HALF, ay - j),
                    min(HALF, by - j),
                )
                if child[2] <= child[3] and child[4] <= child[5]:
                    if not covered(child, depth + 1, product, initial):
                        return False
        return True

    stack = [(ZERO, SPLIT, -HALF, HALF, -HALF, HALF)]
    nodes = 0
    while stack:
        box = stack.pop()
        nodes += 1
        ll, lh, xl, xh, yl, yh = box
        initial = ONE + 2 * (iv_sq(xl, xh)[0] + iv_sq(yl, yh)[0])
        if covered(box, 0, ONE, initial):
            continue
        spans = [100 * (lh - ll), xh - xl, yh - yl]
        if max(spans) < mpq(1, 1 << 20):
            raise ValueError(f"unresolved box after {nodes} nodes: {box}")
        axis = max(range(3), key=spans.__getitem__) * 2
        mid = (box[axis] + box[axis + 1]) / 2
        stack.extend(
            (
                box[: axis + 1] + (mid,) + box[axis + 2 :],
                box[:axis] + (mid,) + box[axis + 1 :],
            )
        )
    return nodes


if __name__ == "__main__":
    # Recheck every rate used by the count generator, and the four-swap tail.
    for check, rate in zip((certify_1step_exact, certify_2step_exact), OLD):
        passed, nodes, _ = check(2, mpq(rate.numerator, rate.denominator), SPLIT)
        if not passed:
            raise RuntimeError(f"{check.__name__} failed after {nodes} boxes")
        print(check.__name__, nodes, "boxes certified", flush=True)
    target = mpq(THREE.numerator, THREE.denominator)
    print("three-link", certify_block(3, target), "boxes certified", flush=True)
    passed, nodes, _ = certify_tail_exact(SPLIT, 5)
    if not passed:
        raise RuntimeError(f"tail failed after {nodes} boxes")
    print("tail", nodes, "boxes certified", flush=True)
