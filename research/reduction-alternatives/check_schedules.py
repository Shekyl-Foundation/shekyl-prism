#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Exercise each native count and the two Rolletschek orientations at its signing bound."""

import random

from native_counts import OLD, THREE, STEPS, input_bounds, params, schedule, secret_norm
from gen_inputs import (
    sample_N,
    sample_q,
    root_pair,
    crt_pair,
    model_basis,
    o0_basis,
    exact_lagrange,
    inter_prereduced_zi,
    make_rolletschek,
)
from variants import run
from trace_counterexample import crafted_k1


def check_set(name):
    params_set = params(name)
    p, e, bits = (params_set[k] for k in ("p", "e", "B"))
    rng = random.Random(0x5EC0 + e)
    counts = {}
    for label, phi, norm in input_bounds(name):
        counts[label] = schedule(phi, norm, (*OLD, THREE))
    checked = 0

    def check(label, u, v):
        nonlocal checked
        target_u, target_v, *_ = exact_lagrange(u, v, p)
        for rule, steps in zip(("exact", "exp64"), counts[label]):
            result = run(u, v, p, steps, rule)
            if (result["u"], result["v"]) != (target_u, target_v):
                raise AssertionError((name, label, rule, steps))
        checked += 1

    q, _ = sample_q(e, p, rng)
    xq, yq = root_pair(q, p, rng)
    check("challenge", *o0_basis(q, xq, yq))
    d = secret_norm(name)
    check("norm-D", *o0_basis(d, *root_pair(d, p, rng)))
    guarded = 0
    for i in range(12):
        n = sample_N(p, bits, rng, at_bound=(i % 2 == 0), avoid=(q,))
        xn, yn = root_pair(n, p, rng)
        secret = o0_basis(n, xn, yn)
        check("secret-pre", *secret)
        check("signing", *o0_basis(n * q, *crt_pair(n, xn, yn, q, xq, yq)))
        u, v, _, nv, _ = exact_lagrange(*secret, p)
        if nv**2 <= p * (1024 * n) ** 2:
            w, z, _ = inter_prereduced_zi(u, v, xq, yq, q)
            check("guard-256", w, z)
            guarded += 1
    if not guarded:
        raise AssertionError((name, "no guarded intersections"))
    for seed in (78, 88, 98):
        t, _, x, y = crafted_k1(params_set, seed=seed)
        check("signing", *o0_basis(t, x, y))
    for mode in ("conj_alt", "i_rot"):
        t, g, h, _ = make_rolletschek(1 << (params_set["Tbits"] + 1), mode)
        check("signing", *model_basis(t, g, h))
    print(name, checked, "pairs matched the exact fixed point,", guarded, "guarded")


if __name__ == "__main__":
    for name in STEPS:
        check_set(name)
