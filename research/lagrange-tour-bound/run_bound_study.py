#!/usr/bin/env python3
"""
Numerical validation, adversarial testing, bounded-precision analysis, and
Lehmer/half-GCD batching study for fixed-schedule Z[i]-Lagrange reduction.
"""
import argparse
import csv
import json
import math
import os
import random
import sys
import time

import gmpy2
from gmpy2 import mpq, mpz

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS_DIR = os.path.join(HERE, "results")
sys.path.insert(0, HERE)
import certify  # noqa: E402

SETS = [
    ("p248", 248, 5, 189, 640.0),
    ("p324", 324, 3, 239, 825.0),
    ("p376", 376, 65, 283, 955.0),
    ("p500", 500, 27, 359, 1255.0),
    ("p664", 664, 17, 465, 1660.0),
]


def set_params(name, e, f):
    p = f * (1 << e) - 1
    L = p.bit_length()
    B = (L + 1) // 2 + 32
    q_bits = 2 * e - 5
    T_bits = B + q_bits
    return p, L, B, q_bits, T_bits


def zi_gauss_trace(u, v, p, track_ops=False):
    u0, u1, u2, u3 = u
    v0, v1, v2, v3 = v
    nu = u0 * u0 + u1 * u1 + p * (u2 * u2 + u3 * u3)
    nv = v0 * v0 + v1 * v1 + p * (v2 * v2 + v3 * v3)
    swaps = 0
    if nv < nu:
        u0, u1, u2, u3, v0, v1, v2, v3 = v0, v1, v2, v3, u0, u1, u2, u3
        nu, nv = nv, nu
    steps = 0
    max_mbits = 0
    mul_work = 0  # sum of bit_length products as proxy for big-int ops
    while True:
        steps += 1
        h0 = v0 * u0 + v1 * u1 + p * (v2 * u2 + v3 * u3)
        h1 = v1 * u0 - v0 * u1 + p * (v3 * u2 - v2 * u3)
        m0 = (2 * h0 + nu) // (2 * nu)
        m1 = (2 * h1 + nu) // (2 * nu)
        if track_ops:
            ub = max(u0.bit_length(), u1.bit_length(), u2.bit_length(), u3.bit_length(), 1)
            vb = max(v0.bit_length(), v1.bit_length(), v2.bit_length(), v3.bit_length(), 1)
            mul_work += 8 * ub * vb + 2 * (nu.bit_length() ** 2)
        mb = max(abs(m0).bit_length(), abs(m1).bit_length())
        if mb > max_mbits:
            max_mbits = mb
        if m0 == 0 and m1 == 0 and nu <= nv:
            return steps, swaps, nu, nv, max_mbits, mul_work
        v0 -= m0 * u0 - m1 * u1
        v1 -= m0 * u1 + m1 * u0
        v2 -= m0 * u2 - m1 * u3
        v3 -= m0 * u3 + m1 * u2
        if track_ops:
            ub = max(u0.bit_length(), u1.bit_length(), u2.bit_length(), u3.bit_length(), 1)
            mul_work += 4 * ub * max(mb, 1) + 4 * (ub * ub)
        nv = v0 * v0 + v1 * v1 + p * (v2 * v2 + v3 * v3)
        if nv < nu:
            u0, u1, u2, u3, v0, v1, v2, v3 = v0, v1, v2, v3, u0, u1, u2, u3
            nu, nv = nv, nu
            swaps += 1


def tonelli_shanks(n, p):
    n %= p
    if n == 0:
        return 0
    if p % 4 == 3:
        return int(gmpy2.powmod(n, (p + 1) // 4, p))
    Q, S = p - 1, 0
    while Q % 2 == 0:
        Q //= 2
        S += 1
    z = 2
    while gmpy2.powmod(z, (p - 1) // 2, p) != p - 1:
        z += 1
    M = S
    c = gmpy2.powmod(z, Q, p)
    t = gmpy2.powmod(n, Q, p)
    R = gmpy2.powmod(n, (Q + 1) // 2, p)
    while t != 1:
        i = 1
        t2 = (t * t) % p
        while t2 != 1:
            t2 = (t2 * t2) % p
            i += 1
        b = gmpy2.powmod(c, 1 << (M - i - 1), p)
        c = (b * b) % p
        M = i
        t = (t * c) % p
        R = (R * b) % p
    return int(R)


def sample_prism_ideal(p, e, B, rng, at_bound=False):
    q_base = 1 << (2 * e - 5)
    while True:
        s = rng.randrange(1, 1 << 20)
        q = q_base - s * s
        if q > 2 and gmpy2.is_prime(q) and gmpy2.legendre(-p % q, q) == 1:
            break
    lo = (1 << (B - 1)) if at_bound else (1 << max(20, B - 32))
    hi = 1 << B
    while True:
        N = rng.randrange(lo, hi) | 1
        if gmpy2.is_prime(N) and N != q and gmpy2.legendre(-p % N, N) == 1:
            break
    while True:
        yN = rng.randrange(N)
        rN = (-(yN * yN + p)) % N
        if rN != 0 and gmpy2.legendre(rN, N) == 1:
            break
    while True:
        yq = rng.randrange(q)
        rq = (-(yq * yq + p)) % q
        if rq != 0 and gmpy2.legendre(rq, q) == 1:
            break
    xN = tonelli_shanks(rN, N)
    if rng.getrandbits(1):
        xN = N - xN
    xq = tonelli_shanks(rq, q)
    if rng.getrandbits(1):
        xq = q - xq
    T = N * q
    inv_N = int(gmpy2.invert(N, q))
    y = yN + N * (((yq - yN) * inv_N) % q)
    x = xN + N * (((xq - xN) * inv_N) % q)
    return T, x, y, N, q


def make_rolletschek(target, mode="conj_alt"):
    wp0, wp1 = 0, 0
    wc0, wc1 = 1, 0
    k = 0
    while True:
        if mode == "conj_alt":
            m0, m1 = (1, 1) if (k % 2 == 0) else (1, -1)
            wn0 = (m0 * wc0 - m1 * wc1) + wp0
            wn1 = (m0 * wc1 + m1 * wc0) + wp1
        else:
            # w_{k+1} = (1+i) w_k + i w_{k-1}
            wn0 = (wc0 - wc1) - wp1
            wn1 = (wc0 + wc1) + wp0
        if wn0 * wn0 + wn1 * wn1 >= target:
            break
        wp0, wp1, wc0, wc1 = wc0, wc1, wn0, wn1
        k += 1
    T = wc0 * wc0 + wc1 * wc1
    g = (wp0 * wc0 + wp1 * wc1) % T
    h = (wp1 * wc0 - wp0 * wc1) % T
    return T, g, h, k


def make_tail_stress(p, T_target_bits):
    # Lift a worst-case 4-swap tail seed (lam = 0.001083, eps = (-0.386, -0.381))
    # Wait: Delta = p * T^2, A_0 = sqrt(Delta / 0.001083) ~ 30.39 * sqrt(p) * T.
    # For a direct Z[i] basis with lam ~ 0.001083, we can set u = [A_sqrt, 0, 0, 0]
    # Wait: valid basis with Delta = p*T^2 needs u_0 = [T, 0, 0, 0], v_0 = [g, h, 1, 0] where
    # h0/nu = m0 + eps0. Pick T ~ 2^(T_target_bits - 1), and make A_0 = T^2 > sqrt(1000*p)*T if T > 32*sqrt(p).
    # Actually at lam0 = 0.001083, A_0 = sqrt(p/0.001083) * T -> T ~ 30.39*sqrt(p) so u = [T,0,0,0] has A_0 = T^2!
    # Wait: that only occurs when T ~ 30.39*sqrt(p)! At large T, A_0 = T^2 >> sqrt(p)*T,
    # and lam reaches 0.001083 near the END of reduction.
    # So to stress the tail at full scale, construct an intermediate basis directly:
    # u = [T, 0, 0, 0], v = [g, h, 1, 0] with T = isqrt(round(p / 0.001083)) -> wait that has smaller T.
    # For general basis with det = p * T^2 at full T: pick u = [U, 0, 0, 0], v = [x, y, K, 0]
    # Wait: here K*U = T -> Delta = p * (K*U)^2! Set U = isqrt(round(p * T^2 / 0.001083))? No, K=1 gives Delta = p*U^2.
    # If U ~ 30.39 * sqrt(p) * S, then K = T // U gives Delta ~ p*T^2 AND lam0 = p*K^2/U^2 ~ 0.001083!
    T = (1 << (T_target_bits - 1)) + 1234567
    sp = math.isqrt(p)
    # Want (p * K^2) / U^2 ~ 0.001083 => U / K ~ 30.386 * sqrt(p)
    # With U * K = T => K^2 ~ T / (30.386 * sqrt(p)), U = T // K
    K = max(1, math.isqrt(max(1, T // max(1, int(30.386 * sp)))))
    U = T // K
    T_eff = U * K
    # Set x = round(-0.386 * U), y = round(-0.381 * U) -> wait: if we want immediate swap,
    # set u = [U, 0, 0, 0], v = [round(-0.386*U), round(-0.381*U), K, 0] (has m=(0,0) and nv < nu!)
    x = -int(0.386 * U)
    y = -int(0.381 * U)
    return T_eff, U, K, x, y


def provable_bound_for(T_ub_log2, p, rho1=0.31, rho2=0.087, lam_split=0.001):
    # From Main-phase theorem:
    # Phi_S <= rho2^((S-1)//2) * (rho1 if (S-1)%2 else 1) * 2 * T^2
    # And Phi_S >= A_{S-1} >= sqrt(Delta / lam_split) = sqrt(p / lam_split) * T
    # Largest S such that LHS <= log2(2 * T / sqrt(p / lam_split)):
    rhs_log2 = T_ub_log2 + 1.0 - 0.5 * math.log2(p / lam_split)
    S_max = 1
    while True:
        S_next = S_max + 1
        m = S_next - 1
        lhs = (m // 2) * math.log2(1.0 / rho2) + (math.log2(1.0 / rho1) if (m % 2) else 0.0)
        if lhs <= rhs_log2:
            S_max = S_next
        else:
            break
    return S_max, S_max + 5


def panel_A():
    print("=== Panel A: Exact Certification & Bounds Table ===")
    t0 = time.time()
    ok1, n1, ub1 = certify.certify_1step_exact(mpq(2), mpq(31, 100), mpq(1, 1000))
    ok2, n2, ub2 = certify.certify_2step_exact(mpq(2), mpq(87, 1000), mpq(1, 1000))
    okt, nt, _ = certify.certify_tail_exact(mpq(1, 1000), 5)
    assert ok1 and ok2 and okt, (ok1, ok2, okt)

    rows = []
    for name, e, f, T_emp, lll_avg in SETS:
        p, L, B, q_bits, T_bits = set_params(name, e, f)
        S_max_sec, T_prov_sec = provable_bound_for(T_bits, p)
        S_max_q, T_prov_q = provable_bound_for(q_bits, p)
        # Asymptotic Rolletschek estimate: log_{2+sqrt(3)}(T / sqrt(p)) + 1
        r_base = 2.0 + math.sqrt(3.0)
        rol_asymp_sec = (T_bits - 0.5 * L) / math.log2(r_base) + 1.0
        rol_asymp_q = (q_bits - 0.5 * L) / math.log2(r_base) + 1.0
        rows.append({
            "set": name,
            "e": e,
            "f": f,
            "L_bits_p": L,
            "B": B,
            "q_bits": q_bits,
            "T_max_bits": T_bits,
            "T_emp": T_emp,
            "T_prov": T_prov_sec,
            "S_main_max": S_max_sec,
            "T_prov_Iq": T_prov_q,
            "rolletschek_asymp": round(rol_asymp_sec, 2),
            "rolletschek_asymp_Iq": round(rol_asymp_q, 2),
            "margin_vs_emp_pct": round(100.0 * (T_prov_sec - T_emp) / T_emp, 1),
            "margin_vs_rol_pct": round(100.0 * (T_prov_sec - rol_asymp_sec) / rol_asymp_sec, 1),
            "cost_ratio_vs_emp": round(T_prov_sec / T_emp, 3),
            "lll_avg_steps": lll_avg,
            "cost_ratio_vs_lll": round(T_prov_sec / lll_avg, 3),
        })
        print(f"  {name}: T_max_bits={T_bits}, T_emp={T_emp}, Rol~{rol_asymp_sec:.1f}, T_prov={T_prov_sec} (+{100*(T_prov_sec-T_emp)/T_emp:.1f}% vs emp, {T_prov_sec/lll_avg:.2f}x LLL)")

    with open(os.path.join(RESULTS_DIR, "bounds_table.csv"), "w", newline="") as fp:
        w = csv.DictWriter(fp, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)

    cert_info = {
        "c_lyap": 2.0,
        "lam_split": 0.001,
        "rho1_exact": "31/100",
        "rho1_nodes": n1,
        "rho1_ub": float(ub1),
        "rho2_exact": "87/1000",
        "rho2_nodes": n2,
        "rho2_ub": float(ub2),
        "tail_max_swaps": 4,
        "tail_nodes_ruled_out_5": nt,
        "elapsed_s": round(time.time() - t0, 2),
        "sets": rows,
    }
    with open(os.path.join(RESULTS_DIR, "cert_summary.json"), "w") as fp:
        json.dump(cert_info, fp, indent=2)
    print(f"Panel A done in {time.time()-t0:.2f}s")


def panel_B():
    print("=== Panel B: Observed Distribution (300 PRISM ideals/set) ===")
    t0 = time.time()
    rng = random.Random(20261122)
    out_path = os.path.join(RESULTS_DIR, "observed_dist.csv")
    fields = ["set", "regime", "idx", "N_bits", "q_bits", "T_bits", "steps", "swaps", "max_mbits", "lam1_ok"]
    summary = {}
    with open(out_path, "w", newline="") as fp:
        w = csv.DictWriter(fp, fieldnames=fields)
        w.writeheader()
        for name, e, f, T_emp, _ in SETS:
            p, L, B, q_bits, T_bits = set_params(name, e, f)
            _, T_prov = provable_bound_for(T_bits, p)
            st_all, st_bnd = [], []
            for idx in range(300):
                at_bound = idx >= 200
                regime = "bound_B" if at_bound else "stream_32"
                T, x, y, N, q = sample_prism_ideal(p, e, B, rng, at_bound=at_bound)
                steps, swaps, nu, nv, mb, _ = zi_gauss_trace([T, 0, 0, 0], [x, y, 1, 0], p)
                lam1_ok = (nu * nu <= 2 * p * T * T)
                assert steps <= T_prov, f"Violation on {name}: {steps} > {T_prov}"
                w.writerow({
                    "set": name,
                    "regime": regime,
                    "idx": idx,
                    "N_bits": N.bit_length(),
                    "q_bits": q.bit_length(),
                    "T_bits": T.bit_length(),
                    "steps": steps,
                    "swaps": swaps,
                    "max_mbits": mb,
                    "lam1_ok": int(lam1_ok),
                })
                st_all.append(steps)
                if at_bound:
                    st_bnd.append(steps)
            st_all.sort()
            st_bnd.sort()
            summary[name] = {
                "n": len(st_all),
                "mean": round(sum(st_all) / len(st_all), 2),
                "p95": st_all[int(0.95 * len(st_all))],
                "max": max(st_all),
                "bound_max": max(st_bnd),
                "T_emp": T_emp,
                "T_prov": T_prov,
            }
            print(f"  {name}: mean={summary[name]['mean']}, p95={summary[name]['p95']}, max={summary[name]['max']} (bound_max={summary[name]['bound_max']}, T_emp={T_emp}, T_prov={T_prov})")
    with open(os.path.join(RESULTS_DIR, "observed_summary.json"), "w") as fp:
        json.dump(summary, fp, indent=2)
    print(f"Panel B done in {time.time()-t0:.2f}s")


def panel_C():
    print("=== Panel C: Adversarial Bases & Toy Exhaustive Valid-Ideal Scan ===")
    t0 = time.time()
    adv_rows = []
    for name, e, f, T_emp, _ in SETS:
        p, L, B, q_bits, T_bits = set_params(name, e, f)
        _, T_prov = provable_bound_for(T_bits, p)
        target = 1 << T_bits

        # 1. Rolletschek conj_alt and i_rot
        for mode in ["conj_alt", "i_rot"]:
            T, g, h, k = make_rolletschek(target, mode=mode)
            steps, swaps, nu, nv, mb, _ = zi_gauss_trace([T, 0, 0, 0], [g, h, 1, 0], p)
            assert steps <= T_prov, f"Rolletschek violation {name} {mode}: {steps} > {T_prov}"
            adv_rows.append({
                "set": name, "family": f"rolletschek_{mode}", "T_bits": T.bit_length(),
                "steps": steps, "swaps": swaps, "T_emp": T_emp, "T_prov": T_prov,
                "lam1_ok": int(nu * nu <= 2 * p * T * T),
            })

        # 2. Corner cases: x=y=0; A0 = T^2; A0 << T^2 (1,0); half-tie (x = T//2)
        T_full = (1 << (T_bits - 1)) + 13579
        corners = [
            ("zero_xy", [T_full, 0, 0, 0], [0, 0, 1, 0]),
            ("half_tie", [T_full, 0, 0, 0], [T_full // 2, T_full // 2, 1, 0]),
            ("skew_large_q", [T_full, 0, 0, 0], [1, 0, 1, 0]),
            ("equal_norms", [T_full, 0, 0, 0], [T_full - 1, 0, 1, 0]),
        ]
        for cname, u_in, v_in in corners:
            steps, swaps, nu, nv, mb, _ = zi_gauss_trace(u_in, v_in, p)
            assert steps <= T_prov
            adv_rows.append({
                "set": name, "family": cname, "T_bits": T_full.bit_length(),
                "steps": steps, "swaps": swaps, "T_emp": T_emp, "T_prov": T_prov,
                "lam1_ok": int(nu * nu <= 2 * p * T_full * T_full),
            })

        # 3. Tail-stress direct basis at lam0 ~ 0.001083
        T_eff, U, K, x_ts, y_ts = make_tail_stress(p, T_bits)
        steps, swaps, nu, nv, mb, _ = zi_gauss_trace([U, 0, 0, 0], [x_ts, y_ts, K, 0], p)
        assert steps <= 6, f"Tail stress exceeded 6 steps: {steps}"
        adv_rows.append({
            "set": name, "family": "tail_stress_0.001083", "T_bits": T_eff.bit_length(),
            "steps": steps, "swaps": swaps, "T_emp": T_emp, "T_prov": T_prov,
            "lam1_ok": int(nu * nu <= 2 * p * T_eff * T_eff),
        })

    with open(os.path.join(RESULTS_DIR, "adversarial.csv"), "w", newline="") as fp:
        w = csv.DictWriter(fp, fieldnames=list(adv_rows[0].keys()))
        w.writeheader()
        w.writerows(adv_rows)

    for r in adv_rows:
        if r["family"].startswith("rolletschek") or r["family"] == "tail_stress_0.001083":
            print(f"  {r['set']} {r['family']}: steps={r['steps']} (T_emp={r['T_emp']}, T_prov={r['T_prov']})")

    # Toy exhaustive scan: p = 101, all odd T in [200, 1000] (plus Rolletschek Ts up to 2^18)
    # Compare max steps for VALID ideals (T | x^2+y^2+p) vs GENERAL bases (all 0<=x,y<T)
    toy_rows = []
    for p_toy, T_max_toy in [(101, 400), (1009, 600)]:
        max_valid = 0
        max_gen = 0
        n_valid = 0
        n_gen = 0
        # For T in [T_max_toy//2, T_max_toy], sample/exhaust
        for T in range(T_max_toy // 2, T_max_toy + 1):
            sq = {}
            for x in range(T):
                r = (x * x) % T
                sq.setdefault(r, []).append(x)
            # valid pairs: x^2 + y^2 + p_toy == 0 mod T => y^2 == -(x^2 + p_toy) mod T
            for x in range(T):
                need = (-x * x - p_toy) % T
                if need in sq:
                    for y in sq[need]:
                        n_valid += 1
                        st, _, _, _, _, _ = zi_gauss_trace([T, 0, 0, 0], [x, y, 1, 0], p_toy)
                        if st > max_valid:
                            max_valid = st
            # random sample of 200 general bases at same T (or full if T<=50)
            rng_toy = random.Random(T + p_toy)
            for _ in range(250):
                x = rng_toy.randrange(T)
                y = rng_toy.randrange(T)
                n_gen += 1
                st, _, _, _, _, _ = zi_gauss_trace([T, 0, 0, 0], [x, y, 1, 0], p_toy)
                if st > max_gen:
                    max_gen = st
        # Also test exact Rolletschek at T <= T_max_toy
        Tr, gr, hr, _ = make_rolletschek(T_max_toy + 1, "conj_alt")
        st_rol, _, _, _, _, _ = zi_gauss_trace([Tr, 0, 0, 0], [gr, hr, 1, 0], p_toy)
        _, T_prov_toy = provable_bound_for(math.log2(T_max_toy), p_toy)
        toy_rows.append({
            "p": p_toy, "T_max": T_max_toy, "n_valid_tested": n_valid,
            "max_valid_ideal": max_valid, "max_random_general": max_gen,
            "rolletschek_at_Tmax": st_rol, "T_prov": T_prov_toy,
        })
        print(f"  Toy p={p_toy}, T<={T_max_toy}: max_valid={max_valid} (over {n_valid} ideals), max_rand_gen={max_gen}, rol={st_rol}, T_prov={T_prov_toy}")

    with open(os.path.join(RESULTS_DIR, "toy_exhaustive.csv"), "w", newline="") as fp:
        w = csv.DictWriter(fp, fieldnames=list(toy_rows[0].keys()))
        w.writeheader()
        w.writerows(toy_rows)
    print(f"Panel C done in {time.time()-t0:.2f}s")


def fsr_round_q(h, nu, prec):
    if prec == 0:
        return (2 * h + nu) // (2 * nu)
    shift = max(0, nu.bit_length() - prec)
    h_s = h >> shift
    nu_s = nu >> shift
    if nu_s == 0:
        return 0
    return (2 * h_s + nu_s) // (2 * nu_s)


def run_bounded_prec(u, v, p, prec, max_steps, one_exact_cleanup=False):
    u0, u1, u2, u3 = u
    v0, v1, v2, v3 = v
    nu = u0 * u0 + u1 * u1 + p * (u2 * u2 + u3 * u3)
    nv = v0 * v0 + v1 * v1 + p * (v2 * v2 + v3 * v3)
    if nv < nu:
        u0, u1, u2, u3, v0, v1, v2, v3 = v0, v1, v2, v3, u0, u1, u2, u3
        nu, nv = nv, nu
    steps = 0
    fixed_pt = False
    for s in range(1, max_steps + 1):
        h0 = v0 * u0 + v1 * u1 + p * (v2 * u2 + v3 * u3)
        h1 = v1 * u0 - v0 * u1 + p * (v3 * u2 - v2 * u3)
        m0 = fsr_round_q(h0, nu, prec)
        m1 = fsr_round_q(h1, nu, prec)
        if m0 == 0 and m1 == 0 and nu <= nv:
            steps = s
            fixed_pt = True
            break
        v0 -= m0 * u0 - m1 * u1
        v1 -= m0 * u1 + m1 * u0
        v2 -= m0 * u2 - m1 * u3
        v3 -= m0 * u3 + m1 * u2
        nv = v0 * v0 + v1 * v1 + p * (v2 * v2 + v3 * v3)
        if nv < nu:
            u0, u1, u2, u3, v0, v1, v2, v3 = v0, v1, v2, v3, u0, u1, u2, u3
            nu, nv = nv, nu
    else:
        steps = max_steps

    if one_exact_cleanup and not fixed_pt:
        h0 = v0 * u0 + v1 * u1 + p * (v2 * u2 + v3 * u3)
        h1 = v1 * u0 - v0 * u1 + p * (v3 * u2 - v2 * u3)
        m0 = (2 * h0 + nu) // (2 * nu)
        m1 = (2 * h1 + nu) // (2 * nu)
        if m0 == 0 and m1 == 0 and nu <= nv:
            fixed_pt = True
        else:
            v0 -= m0 * u0 - m1 * u1
            v1 -= m0 * u1 + m1 * u0
            v2 -= m0 * u2 - m1 * u3
            v3 -= m0 * u3 + m1 * u2
            nv = v0 * v0 + v1 * v1 + p * (v2 * v2 + v3 * v3)
            if nv < nu:
                u0, u1, u2, u3, v0, v1, v2, v3 = v0, v1, v2, v3, u0, u1, u2, u3
                nu, nv = nv, nu
            h0 = v0 * u0 + v1 * u1 + p * (v2 * u2 + v3 * u3)
            h1 = v1 * u0 - v0 * u1 + p * (v3 * u2 - v2 * u3)
            m0 = (2 * h0 + nu) // (2 * nu)
            m1 = (2 * h1 + nu) // (2 * nu)
            fixed_pt = (m0 == 0 and m1 == 0 and nu <= nv)
    return steps, fixed_pt, nu


def panel_D():
    print("=== Panel D: Bounded-Precision Quotients & Ping-Pong Study ===")
    t0 = time.time()
    rng = random.Random(777)
    rows = []
    for name, e, f, T_emp, _ in SETS:
        p, L, B, q_bits, T_bits = set_params(name, e, f)
        _, T_prov = provable_bound_for(T_bits, p)
        # Generate 80 random PRISM ideals + 1 Rolletschek + 1 ping-pong half-integer adversary + 1 large-q adversary
        ideals = [sample_prism_ideal(p, e, B, rng, at_bound=(i < 30)) for i in range(80)]
        exact_res = [zi_gauss_trace([T, 0, 0, 0], [x, y, 1, 0], p) for T, x, y, _, _ in ideals]
        # Half-integer boundary ping-pong input: nu = 2^65, v = [nu//2 - 1, 0, 0, 0] + [0,0,1,0] -> h0/nu = 1/2 - 1/nu
        # When truncated to prec <= 64, h0 >> shift == (nu >> shift)//2 => rounds to 0 or 1, and wait:
        # specifically h0 = nu//2 + 1 gives +1/2 + 1/nu -> rounds to 1, leaving h0' = -nu//2 + 1 -> rounds to -1!
        U_pp = 1 << 70
        # Wait: ensure nu = U_pp^2? No, make u = [U_pp, 0, 0, 0] with nu = U_pp^2 ~ 2^140,
        # and h0 = nu//2 + 1 -> v = [U_pp//2, 0, 1, 0]? Wait: h0 = v0 * U_pp = (U_pp//2) * U_pp = nu//2 exactly!
        # Add 1 to v0 => v0 = U_pp//2 + 1 => h0 = nu//2 + U_pp = nu*(1/2 + 1/U_pp) within 2^-70 of 1/2!
        # Wait: for nv > nu, we need p*1^2 + (U_pp//2)^2 > U_pp^2 -> so pick U_pp = isqrt(p) + 100!
        # Better: make U_pp small enough that p > U_pp^2 so nv = v0^2 + p > nu = U_pp^2!
        # Wait: at p248, p ~ 2^250, so U_pp = 1 << 124 gives nu = 2^248 < p ~ 5*2^248!
        U_pp = 1 << (L // 2 - 2)
        v0_pp = (U_pp >> 1) + 1

        for prec in [8, 16, 24, 32, 48, 64]:
            lam_hits = 0
            fp_raw = 0
            fp_clean = 0
            max_extra = -999
            max_st = 0
            for idx, (T, x, y, _, _) in enumerate(ideals):
                ex_steps, _, ex_nu, _, _, _ = exact_res[idx]
                st, fp, nu_bp = run_bounded_prec([T, 0, 0, 0], [x, y, 1, 0], p, prec, T_prov, one_exact_cleanup=False)
                _, fp_c, nu_c = run_bounded_prec([T, 0, 0, 0], [x, y, 1, 0], p, prec, T_prov, one_exact_cleanup=True)
                if nu_bp == ex_nu:
                    lam_hits += 1
                if fp:
                    fp_raw += 1
                if fp_c:
                    fp_clean += 1
                max_extra = max(max_extra, st - ex_steps)
                max_st = max(max_st, st)

            # Test ping-pong adversary (raw vs 1-step cleanup):
            st_pp_raw, fp_pp_raw, _ = run_bounded_prec([U_pp, 0, 0, 0], [v0_pp, 0, 1, 0], p, prec, 50, one_exact_cleanup=False)
            st_pp_cln, fp_pp_cln, _ = run_bounded_prec([U_pp, 0, 0, 0], [v0_pp, 0, 1, 0], p, prec, 50, one_exact_cleanup=True)

            # Test large-quotient adversary (u=[1<<10,0,0,0], v=[1<<80, 0, 1, 0]):
            u_lq = [1 << 10, 0, 0, 0]
            v_lq = [1 << 80, 0, 1, 0]
            ex_lq, _, ex_nu_lq, _, _, _ = zi_gauss_trace(u_lq, v_lq, p)
            st_lq, fp_lq, _ = run_bounded_prec(u_lq, v_lq, p, prec, 200, one_exact_cleanup=False)

            rows.append({
                "set": name, "prec": prec, "n": len(ideals),
                "lam1_match_pct": round(100.0 * lam_hits / len(ideals), 2),
                "fp_raw_pct": round(100.0 * fp_raw / len(ideals), 2),
                "fp_clean_pct": round(100.0 * fp_clean / len(ideals), 2),
                "max_steps": max_st,
                "max_extra_vs_exact": max_extra,
                "pp_raw_steps": st_pp_raw, "pp_raw_fp": int(fp_pp_raw),
                "pp_clean_fp": int(fp_pp_cln),
                "lq_exact_steps": ex_lq, "lq_bp_steps": st_lq,
            })
            print(f"  {name} prec={prec:2d}: lam1={100*lam_hits/len(ideals):.1f}%, fp_raw={100*fp_raw/len(ideals):.1f}%, fp_cln={100*fp_clean/len(ideals):.1f}%, max_extra={max_extra}, pp_raw={st_pp_raw}(fp={fp_pp_raw})->cln_fp={fp_pp_cln}, lq={ex_lq}->{st_lq}")

    with open(os.path.join(RESULTS_DIR, "prec_study.csv"), "w", newline="") as fp:
        w = csv.DictWriter(fp, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"Panel D done in {time.time()-t0:.2f}s")


def zi_gauss_lehmer(u, v, p, W=128, m_inner=16, max_batches=1000):
    u0, u1, u2, u3 = u
    v0, v1, v2, v3 = v
    nu = u0 * u0 + u1 * u1 + p * (u2 * u2 + u3 * u3)
    nv = v0 * v0 + v1 * v1 + p * (v2 * v2 + v3 * v3)
    if nv < nu:
        u0, u1, u2, u3, v0, v1, v2, v3 = v0, v1, v2, v3, u0, u1, u2, u3
        nu, nv = nv, nu

    batches = 0
    exact_fallbacks = 0
    inner_steps_total = 0
    mul_work = 0
    for _ in range(max_batches):
        h0 = v0 * u0 + v1 * u1 + p * (v2 * u2 + v3 * u3)
        h1 = v1 * u0 - v0 * u1 + p * (v3 * u2 - v2 * u3)
        m0 = (2 * h0 + nu) // (2 * nu)
        m1 = (2 * h1 + nu) // (2 * nu)
        ub = max(u0.bit_length(), u1.bit_length(), u2.bit_length(), u3.bit_length(), 1)
        vb = max(v0.bit_length(), v1.bit_length(), v2.bit_length(), v3.bit_length(), 1)
        mul_work += 8 * ub * vb  # inner product h0, h1
        if m0 == 0 and m1 == 0 and nu <= nv:
            break
        batches += 1
        nb = max(nu.bit_length(), nv.bit_length())
        shift = max(0, nb - W)
        nu_s = nu >> shift
        nv_s = nv >> shift
        h0_s = h0 >> shift
        h1_s = h1 >> shift
        a0, a1, b0, b1 = 1, 0, 0, 0
        c0, c1, d0, d1 = 0, 0, 1, 0
        step_taken = 0
        if nu_s > 16 and shift > 0:
            for _j in range(m_inner):
                ab = abs(a0) + abs(a1) + abs(b0) + abs(b1)
                cd = abs(c0) + abs(c1) + abs(d0) + abs(d1)
                err_nu = ab * ab
                err_h = ab * cd
                if err_nu * 8 >= nu_s:
                    break
                q0 = (2 * h0_s + nu_s) // (2 * nu_s)
                q1 = (2 * h1_s + nu_s) // (2 * nu_s)
                margin0 = nu_s - abs(2 * h0_s - 2 * q0 * nu_s)
                margin1 = nu_s - abs(2 * h1_s - 2 * q1 * nu_s)
                req0 = 2 * err_h + (2 * abs(q0) + 1) * err_nu + 4
                req1 = 2 * err_h + (2 * abs(q1) + 1) * err_nu + 4
                if margin0 <= req0 or margin1 <= req1 or abs(q0) > (1 << 20) or abs(q1) > (1 << 20):
                    break
                if q0 == 0 and q1 == 0 and nv_s + cd * cd >= nu_s - err_nu:
                    break
                nc0 = c0 - (q0 * a0 - q1 * a1)
                nc1 = c1 - (q0 * a1 + q1 * a0)
                nd0 = d0 - (q0 * b0 - q1 * b1)
                nd1 = d1 - (q0 * b1 + q1 * b0)
                q2 = q0 * q0 + q1 * q1
                nv_new = nv_s + q2 * nu_s - 2 * (q0 * h0_s + q1 * h1_s)
                nh0 = h0_s - q0 * nu_s
                nh1 = h1_s - q1 * nu_s
                c0, c1, d0, d1 = nc0, nc1, nd0, nd1
                nv_s = nv_new
                h0_s, h1_s = nh0, nh1
                step_taken += 1
                inner_steps_total += 1
                cd_new = abs(c0) + abs(c1) + abs(d0) + abs(d1)
                if nv_s + cd_new * cd_new < nu_s - err_nu:
                    a0, a1, b0, b1, c0, c1, d0, d1 = c0, c1, d0, d1, a0, a1, b0, b1
                    nu_s, nv_s = nv_s, nu_s
                    h0_s, h1_s = h0_s, -h1_s
                else:
                    break
        mul_work += m_inner * (W * W)  # fixed W-bit inner cost
        if step_taken == 0:
            exact_fallbacks += 1
            mul_work += 2 * (nu.bit_length() ** 2)
            mb = max(abs(m0).bit_length(), abs(m1).bit_length(), 1)
            v0 -= m0 * u0 - m1 * u1
            v1 -= m0 * u1 + m1 * u0
            v2 -= m0 * u2 - m1 * u3
            v3 -= m0 * u3 + m1 * u2
            nv = v0 * v0 + v1 * v1 + p * (v2 * v2 + v3 * v3)
            mul_work += 4 * ub * mb + 4 * (ub * ub)
            if nv < nu:
                u0, u1, u2, u3, v0, v1, v2, v3 = v0, v1, v2, v3, u0, u1, u2, u3
                nu, nv = nv, nu
        else:
            cb = max(abs(a0).bit_length(), abs(a1).bit_length(), abs(b0).bit_length(), abs(b1).bit_length(),
                     abs(c0).bit_length(), abs(c1).bit_length(), abs(d0).bit_length(), abs(d1).bit_length(), 1)
            nu0 = a0 * u0 - a1 * u1 + b0 * v0 - b1 * v1
            nu1 = a0 * u1 + a1 * u0 + b0 * v1 + b1 * v0
            nu2 = a0 * u2 - a1 * u3 + b0 * v2 - b1 * v3
            nu3 = a0 * u3 + a1 * u2 + b0 * v3 + b1 * v2
            nv0 = c0 * u0 - c1 * u1 + d0 * v0 - d1 * v1
            nv1 = c0 * u1 + c1 * u0 + d0 * v1 + d1 * v0
            nv2 = c0 * u2 - c1 * u3 + d0 * v2 - d1 * v3
            nv3 = c0 * u3 + c1 * u2 + d0 * v3 + d1 * v2
            u0, u1, u2, u3, v0, v1, v2, v3 = nu0, nu1, nu2, nu3, nv0, nv1, nv2, nv3
            nu = u0 * u0 + u1 * u1 + p * (u2 * u2 + u3 * u3)
            nv = v0 * v0 + v1 * v1 + p * (v2 * v2 + v3 * v3)
            ub2 = max(u0.bit_length(), u1.bit_length(), u2.bit_length(), u3.bit_length(), 1)
            vb2 = max(v0.bit_length(), v1.bit_length(), v2.bit_length(), v3.bit_length(), 1)
            mul_work += 16 * max(ub, vb) * cb + 4 * (ub2 * ub2 + vb2 * vb2)
            if nv < nu:
                u0, u1, u2, u3, v0, v1, v2, v3 = v0, v1, v2, v3, u0, u1, u2, u3
                nu, nv = nv, nu
    return batches, exact_fallbacks, inner_steps_total, nu, mul_work


def panel_E():
    print("=== Panel E: Constant-Time Lehmer / Half-GCD Batching Study ===")
    t0 = time.time()
    rng = random.Random(999)
    rows = []
    for name, e, f, T_emp, _ in SETS:
        p, L, B, q_bits, T_bits = set_params(name, e, f)
        _, T_prov = provable_bound_for(T_bits, p)
        ideals = [sample_prism_ideal(p, e, B, rng, at_bound=(i < 15)) for i in range(40)]
        T_rol, g_rol, h_rol, _ = make_rolletschek(1 << T_bits, "conj_alt")

        # Measure unbatched baseline bit-product work (with track_ops=True)
        ex_works = []
        ex_steps_list = []
        for T, x, y, _, _ in ideals:
            st, _, nu_ex, _, _, mw = zi_gauss_trace([T, 0, 0, 0], [x, y, 1, 0], p, track_ops=True)
            ex_works.append(mw)
            ex_steps_list.append(st)
        st_rol, _, nu_rol, _, _, mw_rol = zi_gauss_trace([T_rol, 0, 0, 0], [g_rol, h_rol, 1, 0], p, track_ops=True)
        avg_ex_work = sum(ex_works) / len(ex_works)

        for W, m_in in [(64, 8), (64, 16), (128, 16), (128, 24)]:
            bs, fbs, works = [], [], []
            ok_cnt = 0
            for idx, (T, x, y, _, _) in enumerate(ideals):
                b, fb, ins, nu_l, mw = zi_gauss_lehmer([T, 0, 0, 0], [x, y, 1, 0], p, W=W, m_inner=m_in)
                bs.append(b)
                fbs.append(fb)
                works.append(mw)
                if nu_l * nu_l <= 2 * p * T * T:
                    ok_cnt += 1
            b_rol, fb_rol, ins_rol, nu_lrol, mw_rol_l = zi_gauss_lehmer([T_rol, 0, 0, 0], [g_rol, h_rol, 1, 0], p, W=W, m_inner=m_in)
            rows.append({
                "set": name, "W": W, "m_inner": m_in,
                "rand_batches_mean": round(sum(bs) / len(bs), 2),
                "rand_batches_max": max(bs),
                "rand_fb_max": max(fbs),
                "rol_batches": b_rol,
                "rol_fb": fb_rol,
                "lam1_ok_pct": round(100.0 * ok_cnt / len(ideals), 1),
                "work_ratio_vs_exact_emp": round((sum(works) / len(works)) / avg_ex_work, 3),
                "rol_work_ratio": round(mw_rol_l / mw_rol, 3),
                "T_emp": T_emp, "T_prov": T_prov,
            })
            print(f"  {name} W={W:3d} m={m_in:2d}: rand_batches={sum(bs)/len(bs):.1f} (max={max(bs)}, fb_max={max(fbs)}), rol_batches={b_rol} (fb={fb_rol}), work_ratio={rows[-1]['work_ratio_vs_exact_emp']}")

    with open(os.path.join(RESULTS_DIR, "lehmer_study.csv"), "w", newline="") as fp:
        w = csv.DictWriter(fp, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"Panel E done in {time.time()-t0:.2f}s")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--panels", default="A,B,C,D,E")
    args = parser.parse_args()
    panels = set(p.strip().upper() for p in args.panels.split(","))
    if "A" in panels:
        panel_A()
    if "B" in panels:
        panel_B()
    if "C" in panels:
        panel_C()
    if "D" in panels:
        panel_D()
    if "E" in panels:
        panel_E()


if __name__ == "__main__":
    main()
