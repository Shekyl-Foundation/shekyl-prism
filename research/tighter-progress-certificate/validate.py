# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Empirical validation of the new counts (results/step_counts_new.csv).

Every input runs
  * the exact contract for T_exact steps (coordinates),
  * exp64 with the carried Gram on the O0 numerator (or model) Gram for
    T_exp64 steps, and
  * exp64 on the normalized Gram (entries / 2T, determinant p) for valid
    O0 ideals, or on the Gram divided by its content for the other bases,
and must end reduced.  It also records the steps each run needs (variable
time, including the final size reduction), and the margin to the count.
Writes results/validate.csv and results/validate.txt."""
import csv, math, os, random, sys
import gmpy2
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, 'vendor'))
from q5common import SETS, params, N4, herm
import gen_inputs as G
from trace_counterexample import crafted, crafted_k1
from adversarial import chains
import ngram as NG

OUT = []


def say(s):
    print(s, flush=True); OUT.append(s)


def load_new():
    c = {}
    for r in csv.DictReader(open(os.path.join(HERE, 'results', 'step_counts_new.csv'))):
        c[(r['set'], r['cls'])] = (int(r['T_exact']), int(r['T_exp64']))
    return c


def phys_need(res):
    """Physical steps until the last step that changes the pair."""
    need = 0
    for t, (m0, m1, s, big, sw) in enumerate(res['phys']):
        if m0 or m1 or sw:
            need = t + 1
    return need


def evaluate(u0, v0, p, Tex, Texp, T_ideal=None):
    need_ex = NG.exact_steps_needed(u0, v0, p)
    r = NG.run_exact(u0, v0, p, Tex)
    A, C, h0, h1 = N4(r['u'], p), N4(r['v'], p), *herm(r['v'], r['u'], p)
    ok_ex = NG.is_reduced(r['u'], r['v'], (A, C, h0, h1))
    out = dict(need_exact=need_ex, ok_exact=ok_ex)
    grams = [('num', NG.num_gram(u0, v0, p))]
    if T_ideal is not None:
        grams.append(('norm', NG.normalize(u0, v0, p, T_ideal)))
    else:
        g = NG.num_gram(u0, v0, p)
        d = math.gcd(*g)
        grams.append(('content', tuple(x // d for x in g)))
    for tag, Gm in grams:
        big = NG.run_gram(u0, v0, Gm, Texp + 64, coords=False)
        res = NG.run_gram(u0, v0, Gm, Texp, coords=False)
        out[f'need_exp64_{tag}'] = phys_need(big)
        out[f'nbig_{tag}'] = big['nbig']
        out[f'ok_exp64_{tag}'] = NG.is_reduced(None, None, res['G']) and res['pending'] == (0, 0)
        out['scale2'] = tag
    return out


def main(scale=1):
    c = load_new()
    rows = []
    for s in SETS:
        name = s[0]; P = params(name); p, e, B = P['p'], P['e'], P['B']
        rng = random.Random(0x7167 ^ int.from_bytes(name.encode(), 'big'))
        qb = 2 * e - 5

        def go(fam, cls, u0, v0, T_ideal=None):
            Tex, Texp = c[(name, cls)]
            o = evaluate(u0, v0, p, Tex, Texp, T_ideal)
            row = dict(set=name, family=fam, cls=cls, T_exact=Tex, T_exp64=Texp, **o)
            rows.append(row)
            if not (o['ok_exact'] and all(v for k, v in o.items() if k.startswith('ok_exp64'))) \
                    or o['need_exact'] > Tex or max(v for k, v in o.items() if k.startswith('need_exp64')) > Texp:
                say(f'  FAIL {name} {fam} {row}')

        qpool = [G.sample_q(e, p, rng) for _ in range(3)] + [G.sample_q(e, p, rng, smallest=True)]
        qroots = [(q, G.root_pair(q, p, rng)) for q, _ in qpool]
        for i in range(12 * scale):
            q, (xq, yq) = qroots[i % len(qroots)]
            N = G.sample_N(p, B, rng, at_bound=(i % 2 == 0), avoid=(q,))
            xN, yN = G.root_pair(N, p, rng)
            x, y = G.crt_pair(N, xN, yN, q, xq, yq)
            go('honest_sign_O0', 'sign-generic', *G.o0_basis(N * q, x, y), T_ideal=N * q)
        for sd in range(3):
            T, k, x, y = crafted(P, seed=77 + sd)
            go('crafted_q5_O0', 'sign-generic', *G.o0_basis(T, x, y), T_ideal=T)
            T, k, x, y = crafted_k1(P, seed=78 + 10 * sd)
            go('crafted_k1_O0', 'sign-generic', *G.o0_basis(T, x, y), T_ideal=T)
        # Rolletschek chains at each generic class's bound (R = T^2/p = 4 Tmax^2/p)
        D = int(gmpy2.next_prime(1 << (2 * P['W'])))
        for cls, tgt in (('sign-generic', 1 << (P['Tbits'] + 1)), ('challenge', 1 << (qb + 1)),
                         ('keygen-Isec', 1 << (B + 1)), ('keygen-D', 2 * D)):
            for mode in ('conj_alt', 'i_rot'):
                T, g, h, k = G.make_rolletschek(tgt, mode=mode)
                go(f'rolletschek_{mode}', cls, *G.model_basis(T, g, h))
        # Rolletschek chains at the pre-reduced classes' bound: R = T^2/p <= 4 q^2 F^2
        for Fg in (256,):
            tgt = math.isqrt(4 * (1 << (2 * qb)) * Fg * Fg * p)
            for mode in ('conj_alt', 'i_rot'):
                T, g, h, k = G.make_rolletschek(tgt, mode=mode)
                go(f'rolletschek_{mode}', 'prered-guard256', *G.model_basis(T, g, h))
        # big-quotient chains, tail stress, ping-pong, half ties (generic signing)
        for fam, T, g, h in chains(P, rng):
            go(fam, 'sign-generic', *G.model_basis(T, g, h))
        Tt = (1 << (P['Tbits'] - 1)) + 1234567
        K = max(1, math.isqrt(max(1, Tt // max(1, int(30.386 * math.isqrt(p))))))
        Ut = Tt // K
        go('tail_stress', 'sign-generic', [Ut, 0, 0, 0], [-int(0.386 * Ut), -int(0.381 * Ut), K, 0])
        U = 1 << (P['L'] // 2 - 2)
        go('pingpong', 'sign-generic', [U, 0, 0, 0], [(U >> 1) + 1, 0, 1, 0])
        Tm = 1 << P['Tbits']
        for fam, x, y in (('half_tie_pp', Tm // 2, Tm // 2), ('half_tie_pm', Tm // 2, -Tm // 2), ('half_tie_p0', Tm // 2, 0)):
            go(fam, 'sign-generic', *G.model_basis(Tm, x, y))
        # challenge, keygen
        for i in range(3 * scale):
            q, (xq, yq) = qroots[i % len(qroots)]
            go('challenge_O0', 'challenge', *G.o0_basis(q, xq, yq), T_ideal=q)
            xD, yD = G.root_pair(D, p, rng)
            go('keygen_D_O0', 'keygen-D', *G.o0_basis(D, xD, yD), T_ideal=D)
            N = G.sample_N(p, B, rng, at_bound=True)
            xN, yN = G.root_pair(N, p, rng)
            go('keygen_Isec_O0', 'keygen-Isec', *G.o0_basis(N, xN, yN), T_ideal=N)
        # pre-reduced (guard 256), model coordinates (ratios are coordinate-free)
        sp_ = math.isqrt(p)
        keys = []
        for i in range(3 * scale):
            N = G.sample_N(p, B, rng, at_bound=(i % 2 == 0))
            xN, yN = G.root_pair(N, p, rng)
            us, vs, nus, nvs, _ = G.exact_lagrange(*G.model_basis(N, xN, yN), p)
            keys.append(('sampled', N, us, vs, nvs))
        N = G.sample_N(p, B, rng, at_bound=True)
        for kind, kt in (('crafted_skew_edge', sp_ * 55 // 100), ('crafted_skew_256', sp_ * 120)):
            x, y, k = G.crafted_skew_key(N, p, kt)
            us, vs, nus, nvs, _ = G.exact_lagrange(*G.model_basis(N, x, y), p)
            keys.append((kind, N, us, vs, nvs))
        for kind, N, us, vs, nvs in keys:
            if nvs * nvs > 256 * 256 * p * N * N:
                say(f'  ({name} prered {kind}: outside guard 256, skipped)')
                continue
            q, (xq, yq) = qroots[rng.randrange(len(qroots))]
            w1, w2, _ = G.inter_prereduced_zi(us, vs, xq, yq, q)
            go('prered_' + kind, 'prered-guard256', w1, w2)
        # summary
        say(f'== {name}')
        fams = {}
        for r in rows:
            if r['set'] != name:
                continue
            k = (r['cls'], r['family'])
            ne = r['need_exact']; nx = max(v for kk, v in r.items() if kk.startswith('need_exp64'))
            a = fams.setdefault(k, [0, 0, 0, r['T_exact'], r['T_exp64']])
            a[0] += 1; a[1] = max(a[1], ne); a[2] = max(a[2], nx)
        for (cls, fam), (n, ne, nx, Tex, Texp) in sorted(fams.items()):
            say(f'  {cls:16s} {fam:26s} n={n:2d}  exact need {ne:4d} / {Tex:4d} (margin {Tex-ne:3d})   '
                f'exp64 need {nx:4d} / {Texp:4d} (margin {Texp-nx:3d})')
    fails = [r for r in rows if not (r['ok_exact'] and r.get('ok_exp64_num') and (r.get('ok_exp64_norm', True) and r.get('ok_exp64_content', True)))]
    say(f'TOTAL: {len(rows)} inputs, {len(rows) - len(fails)} reduced under all rules and scalings')
    keys = sorted({k for r in rows for k in r})
    with open(os.path.join(HERE, 'results', 'validate.csv'), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=keys); w.writeheader(); w.writerows(rows)
    open(os.path.join(HERE, 'results', 'validate.txt'), 'w').write('\n'.join(OUT) + '\n')
    assert not fails


if __name__ == '__main__':
    main(int(sys.argv[1]) if len(sys.argv) > 1 else 1)
