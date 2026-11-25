# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Empirical check of the Q5 bounds.

For each set, runs the instrumented fixed-schedule reduction (instrumented.py)
on sampled PRISM ideals and on adversarial inputs, records the largest |value|
of every named intermediate, and checks it against
  (a) the Lemma-4 bound computed from that run's own starting M0 (the lemma
      itself, input by input), and
  (b) the proven bound of the input's variant (the width claim).
Ideal inputs are also checked to end at lambda_1 (exact Z[i]-Lagrange oracle),
and a subset against fsr.py zi_gauss_fixed(prec=64) step for step.

Usage: python3 run_empirical.py SET [SET...] [--scale K]
Per-set output: results/raw/<set>.json (resumable: done sets are skipped).
"""
import json
import math
import os
import random
import sys
import time

from q5common import params, N4
from instrumented import Rec, reduce_instrumented
from bounds import start_bounds, intermediate_bounds
import gen_inputs as G

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, 'results', 'raw')

FSR = None
for d in (os.environ.get('FSR_DIR', ''), os.path.join(HERE, '..', 'fixed-schedule-reduction'),
          '/workspace/q5-ref/research/fixed-schedule-reduction'):
    if d and os.path.exists(os.path.join(d, 'fsr.py')):
        sys.path.insert(0, d)
        import fsr as FSR  # noqa
        break


class Fam:
    def __init__(self, name, rep, variant):
        self.name, self.rep, self.variant = name, rep, variant
        self.rec = Rec(); self.n = 0; self.rel_viol = []; self.conv_fail = 0
        self.conv_checked = 0; self.nsat = 0; self.max_growth = 0.0; self.fsr_checked = 0
        self.fsr_mismatch = 0; self.M0_bits = 0; self.setup_bits = 0

    def to_json(self):
        return dict(name=self.name, rep=self.rep, variant=self.variant, n=self.n,
                    maxbits={k: v.bit_length() for k, v in self.rec.mx.items()},
                    rel_viol=self.rel_viol[:20], n_rel_viol=len(self.rel_viol),
                    conv_checked=self.conv_checked, conv_fail=self.conv_fail, nsat=self.nsat,
                    max_log2_growth=self.max_growth, fsr_checked=self.fsr_checked,
                    fsr_mismatch=self.fsr_mismatch, M0_bits=self.M0_bits,
                    setup_bits=self.setup_bits)


def run_one(fam, P, u0, v0, steps, oracle=False, fsr_check=False):
    p = P['p']
    rec = Rec()
    u, v, nu, nv, info = reduce_instrumented(u0, v0, p, steps, rec)
    Ms, b = intermediate_bounds(P, info['M0'], steps + 1)
    for k, val in rec.mx.items():
        if k in b and val > b[k]:
            fam.rel_viol.append((k, val.bit_length(), b[k].bit_length()))
    fam.rec.merge(rec); fam.n += 1; fam.nsat += info['nsat']
    fam.M0_bits = max(fam.M0_bits, info['M0'].bit_length())
    g = math.log2(info['Mmax'] / info['M0']) if info['Mmax'] > info['M0'] else 0.0
    fam.max_growth = max(fam.max_growth, g)
    if oracle:
        _, _, nu_ex, _, _ = G.exact_lagrange(u0, v0, p)
        fam.conv_checked += 1
        if nu != nu_ex:
            fam.conv_fail += 1
    if fsr_check and FSR is not None and info['nsat'] == 0:
        r2 = Rec()
        uu, vv, _, _, _ = reduce_instrumented(u0, v0, p, steps, r2, cleanup=False)
        # fsr.zi_gauss_uv equivalent: zi_gauss_fixed starts from (T,0,0,0); use the
        # general start through its herm/_round_q primitives
        fu, fv = list(u0), list(v0)
        fnu, fnv = N4(fu, p), N4(fv, p)
        if fnv < fnu:
            fu, fv, fnu, fnv = fv, fu, fnv, fnu
        for _ in range(steps):
            h0, h1 = FSR.herm(fv, fu, p, FSR.Ctr())
            m0, m1 = FSR._round_q(h0, fnu, 64), FSR._round_q(h1, fnu, 64)
            iu = (-fu[1], fu[0], -fu[3], fu[2])
            fv = [fv[r] - m0 * fu[r] - m1 * iu[r] for r in range(4)]
            fnv = N4(fv, p)
            if fnv < fnu:
                fu, fv, fnu, fnv = fv, fu, fnv, fnu
        fam.fsr_checked += 1
        if fu != uu or fv != vv:
            fam.fsr_mismatch += 1
    return u, nu, info


def run_set(name, scale):
    P = params(name); p, e, B, L = P['p'], P['e'], P['B'], P['L']
    rng = random.Random(hash(name) & 0xffffffff ^ 0x51515151)
    fams = {}

    def fam(n, rep, var):
        if n not in fams:
            fams[n] = Fam(n, rep, var)
        return fams[n]

    t0 = time.time()
    qpool = [G.sample_q(e, p, rng) for _ in range(6)] + [G.sample_q(e, p, rng, smallest=True)]
    qroots = [(q, G.root_pair(q, p, rng)) for q, _ in qpool]
    Ssec, Sq, Spre, Spreu = P['tprov_sec'], P['tprov_q'], P['tprov_pre'], P['tprov_pre_ung']
    print(name, 'q pool ready', round(time.time() - t0, 1), flush=True)

    def signing_ideal(at_bound, largest=False, q_idx=None):
        q, (xq, yq) = qroots[rng.randrange(len(qroots)) if q_idx is None else q_idx]
        N = G.sample_N(p, B, rng, at_bound=at_bound, largest=largest, avoid=(q,))
        xN, yN = G.root_pair(N, p, rng)
        x, y = G.crt_pair(N, xN, yN, q, xq, yq)
        return N * q, x, y

    # 1. sampled signing ideals (model and O0)
    for i in range(40 * scale):
        T, x, y = signing_ideal(False)
        run_one(fam('model_nvar', 'model', 'sign_generic'), P, *G.model_basis(T, x, y), Ssec,
                oracle=(i % 4 == 0), fsr_check=(i < 10))
    for i in range(40 * scale):
        T, x, y = signing_ideal(True)
        run_one(fam('model_nbound', 'model', 'sign_generic'), P, *G.model_basis(T, x, y), Ssec,
                oracle=(i % 4 == 0), fsr_check=(i < 10))
        if i < 30 * scale:
            run_one(fam('O0_nbound', 'O0', 'sign_generic'), P, *G.o0_basis(T, x, y), Ssec,
                    oracle=(i % 4 == 0))
    print(name, 'sampled done', round(time.time() - t0, 1), flush=True)
    # 2. max-norm secrets at the bound B, largest q (smallest s)
    for i in range(6 * scale):
        T, x, y = signing_ideal(True, largest=(i % 2 == 0), q_idx=len(qroots) - 1)
        run_one(fam('model_maxnorm', 'model', 'sign_generic'), P, *G.model_basis(T, x, y), Ssec, oracle=True)
        run_one(fam('O0_maxnorm', 'O0', 'sign_generic'), P, *G.o0_basis(T, x, y), Ssec, oracle=True)
    # 3. adversarial bases (model units unless named O0)
    Tmax = (1 << P['Tbits']) - 1
    for mode in ('conj_alt', 'i_rot'):
        T, g, h, k = G.make_rolletschek(1 << P['Tbits'], mode=mode)
        run_one(fam('adv_rolletschek', 'model', 'sign_generic'), P, *G.model_basis(T, g, h), Ssec, oracle=True)
        run_one(fam('adv_rolletschek_O0scaled', 'O0', 'sign_generic'), P,
                [2 * T, 0, 0, 0], [2 * g, 2 * h, 1, 0], Ssec, oracle=True)
    Tf = (1 << (P['Tbits'] - 1)) + 13579
    for (x, y) in ((0, 0), (Tf // 2, Tf // 2), (1, 0), (Tf - 1, 0)):
        run_one(fam('adv_corners', 'model', 'sign_generic'), P, *G.model_basis(Tf, x, y), Ssec, oracle=True)
    run_one(fam('adv_max_entries', 'model', 'sign_generic'), P, [Tmax, 0, 0, 0], [Tmax - 1, Tmax - 1, 1, 0], Ssec, oracle=True)
    run_one(fam('adv_max_entries_O0', 'O0', 'sign_generic'), P, [2 * Tmax, 0, 0, 0], [2 * Tmax - 2, 2 * Tmax - 1, 1, 0], Ssec, oracle=True)
    U = 1 << (L // 2 - 2)
    run_one(fam('adv_pingpong', 'model', 'sign_generic'), P, [U, 0, 0, 0], [(U >> 1) + 1, 0, 1, 0], Ssec, oracle=True)
    Tt = (1 << (P['Tbits'] - 1)) + 1234567
    K = max(1, math.isqrt(max(1, Tt // max(1, int(30.386 * math.isqrt(p))))))
    Ut = Tt // K
    run_one(fam('adv_tail_stress', 'model', 'sign_generic'), P, [Ut, 0, 0, 0],
            [-int(0.386 * Ut), -int(0.381 * Ut), K, 0], Ssec, oracle=True)
    # saturation stress: tiny u, v at the maximal size, so every quotient clamps
    for i in range(5 * scale):
        X, Y = rng.randrange(Tmax), rng.randrange(Tmax)
        u0 = [rng.choice([1, -1]), rng.randrange(0, 2), 0, 0]
        run_one(fam('adv_saturation', 'model', 'sign_generic'), P, u0, [X, Y, 0, 1], Ssec)
        run_one(fam('adv_saturation_skewj', 'model', 'sign_generic'), P, [1, 0, 0, 0],
                [X, Y, rng.randrange(1, 3), rng.randrange(0, 2)], Ssec)
    print(name, 'adversarial done', round(time.time() - t0, 1), flush=True)
    # 4. challenge ideal alone; key generation (norm D) and keygen pre-reduction
    for i in range(10 * scale):
        q, (xq, yq) = qroots[i % len(qroots)]
        run_one(fam('Iq_only', 'model', 'Iq_only'), P, *G.model_basis(q, xq, yq), Sq, oracle=(i % 2 == 0))
    import gmpy2
    D = int(gmpy2.next_prime(1 << (2 * P['W'])))
    for i in range(10 * scale):
        xD, yD = G.root_pair(D, p, rng)
        run_one(fam('keygen_D', 'model', 'keygen_D'), P, *G.model_basis(D, xD, yD), Ssec, oracle=(i % 2 == 0))
        if i % 2 == 0:
            run_one(fam('keygen_D_O0', 'O0', 'keygen_D'), P, *G.o0_basis(D, xD, yD), Ssec, oracle=True)
    # 5. keygen pre-reduced signing (model), guarded and crafted keys
    keys = []
    for i in range(10 * scale):
        N = G.sample_N(p, B, rng, at_bound=(i % 2 == 0))
        xN, yN = G.root_pair(N, p, rng)
        run_one(fam('keygen_prereduce_Isec', 'model', 'keygen_prereduce_Isec'), P,
                *G.model_basis(N, xN, yN), Ssec, oracle=True)
        us, vs, nus, nvs, _ = G.exact_lagrange(*G.model_basis(N, xN, yN), p)
        keys.append(('sampled', N, us, vs, nvs))
    sp_ = math.isqrt(p)
    for j in range(3 * scale):
        N = G.sample_N(p, B, rng, at_bound=True)
        for kind, kt in (('crafted_skew_max', 1), ('crafted_edge_fail', sp_ * 45 // 100),
                         ('crafted_edge_pass', sp_ * 55 // 100)):
            x, y, k = G.crafted_skew_key(N, p, kt)
            us, vs, nus, nvs, _ = G.exact_lagrange(*G.model_basis(N, x, y), p)
            keys.append((kind, N, us, vs, nvs))
    guard_stats = {}
    for kind, N, us, vs, nvs in keys:
        guard = nvs * nvs <= 4 * p * N * N
        guard_stats.setdefault(kind, [0, 0])[0 if guard else 1] += 1
        for t in range(3):
            q, (xq, yq) = qroots[(t + rng.randrange(len(qroots))) % len(qroots)]
            w1, w2, setup = G.inter_prereduced_zi(us, vs, xq, yq, q)
            if guard:
                f = fam('prered_guarded_' + kind, 'model', 'sign_prereduced_guarded')
                run_one(f, P, w1, w2, Spre, oracle=True)
            else:
                f = fam('prered_unguarded_' + kind, 'model', 'sign_prereduced_unguarded_generic')
                run_one(f, P, w1, w2, Spreu, oracle=True)
            f.setup_bits = max(f.setup_bits, setup.bit_length())
    print(name, 'pre-reduced done', round(time.time() - t0, 1), flush=True)
    # 6. keygen guard rejection rate on sampled keys (random ideals of norm N, B-bit
    #    and nvar), for guard factors c in nrd(v_s) <= c sqrt(p) N
    gfac = (2, 4, 16, 256, 2 ** 16)
    grej = {c: 0 for c in gfac}; ng = 0; worst_ratio = 0.0
    for i in range(100 * scale):
        N = G.sample_N(p, B, rng, at_bound=(i % 2 == 0))
        xN, yN = G.root_pair(N, p, rng)
        us, vs, nus, nvs, _ = G.exact_lagrange(*G.model_basis(N, xN, yN), p)
        ng += 1
        worst_ratio = max(worst_ratio, nvs / (math.sqrt(p) * N))
        for c in gfac:
            if nvs * nvs > c * c * p * N * N:
                grej[c] += 1
    guard_rate = dict(n=ng, rejected={str(c): grej[c] for c in gfac},
                      max_nrd_v_over_sqrtp_N=worst_ratio)
    out = dict(set=name, scale=scale, seconds=round(time.time() - t0, 1), guard_stats=guard_stats,
               guard_rate=guard_rate,
               families=[f.to_json() for f in fams.values()], fsr_available=FSR is not None)
    os.makedirs(RAW, exist_ok=True)
    tmp = os.path.join(RAW, name + '.json.tmp')
    json.dump(out, open(tmp, 'w'), indent=1)
    os.replace(tmp, os.path.join(RAW, name + '.json'))
    print(name, 'DONE', out['seconds'], 's', flush=True)


def main():
    args = sys.argv[1:]
    scale = 10
    if '--scale' in args:
        i = args.index('--scale'); scale = int(args[i + 1]); del args[i:i + 2]
    force = '--force' in args
    args = [a for a in args if a != '--force']
    for name in args:
        if not force and os.path.exists(os.path.join(RAW, name + '.json')):
            print(name, 'already done, skipping'); continue
        run_set(name, scale)


if __name__ == '__main__':
    main()
