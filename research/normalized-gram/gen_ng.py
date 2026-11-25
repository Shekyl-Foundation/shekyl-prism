# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Inputs, Python references and observed widths for the normalized Gram
experiment. Writes rust/vectors_ng/<set>.txt, rust/vectors_ng/toy.txt and
results/gen_<set>.json.

Inputs per set:
  * every one of the 375 research vectors (rust/vectors, the a60fd06 set):
    the 66 distinct O0 ideals among them, lifted as check_native.py lifts
    them, and the 9 non-ideal bases, which the constructor must reject;
  * the other admitted classes: key generation at norm D, secret ideal
    pre-reduction, the challenge alone, and the guarded pre-reduced
    intersection (factor 256);
  * adversarial chains on real signing ideals at the O0 bound: Rolletschek
    (1 +- i) chains, big-quotient chains and mixed chains, built by
    running reduction steps backwards from a reduced basis of a genuine
    norm-T ideal, so the normalized Gram stays integral;
  * toy half-tie cases (small p): full runs containing exact +1/2 and
    -1/2 quotient components.

Line format (hex integers, '-' for negative):
  p cls kind shape T U0 U1 U2 U3 V0 V1 V2 V3 steps accept
  [A C h0 h1 nbig_num nbig_norm phys_num phys_norm logical
   fu0 fu1 fu2 fu3 fv0 fv1 fv2 fv3]
"""
import json
import os
import random
import sys
from multiprocessing import Pool

import gmpy2

import ngram as n
from q5common import params, N4
import gen_inputs as G
from trace_counterexample import crafted, crafted_k1  # noqa: F401  (documented provenance)

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'rust', 'vectors_ng')
RES = os.path.join(HERE, 'results')


def hx(v):
    return ('-' if v < 0 else '') + format(abs(v), 'x')


def zmul(a0, a1, w):
    return [a0 * w[0] - a1 * w[1], a0 * w[1] + a1 * w[0], a0 * w[2] - a1 * w[3], a0 * w[3] + a1 * w[2]]


def research_inputs(P):
    p, name = P['p'], P['name']
    ideals, rejects = {}, []
    path = os.path.join(HERE, 'rust', 'vectors', name + '.txt')
    for line in open(path).read().splitlines():
        kind, t, x, y, *_ = line.split()
        t, x, y = (int(z, 16) for z in (t, x, y))
        ok = t > 0 and t % 2 == 1 and t < (1 << P['Tbits']) and ((x % t) ** 2 + (y % t) ** 2 + p) % t == 0
        if ok:
            u, v = n.o0_lift(t, x % t, y % t)
            ideals.setdefault((t, x % t, y % t), (kind, u, v))
        else:
            # The same lift applied to a non-ideal: the constructor must refuse.
            tt = max(t, 1)
            u, v = [2 * tt, 0, 0, 0], [(x % tt) * 2 % (2 * tt), (2 * (y % tt) + 1) % (2 * tt), 1, 0]
            rejects.append((kind, tt, u, v))
    return [(k, t, u, v) for (t, _, _), (k, u, v) in sorted(ideals.items())], rejects


def honest_ideal(P, rng, q, xq, yq, at_bound=True):
    p, B = P['p'], P['B']
    N = G.sample_N(p, B, rng, at_bound=at_bound, avoid=(q,))
    xN, yN = G.root_pair(N, p, rng)
    x, y = G.crt_pair(N, xN, yN, q, xq, yq)
    T = N * q
    return T, N, (xN, yN), n.o0_lift(T, x % T, y % T)


def phi(w, xq, yq, q):
    """Gaussian residue of W modulo the O0 ideal of norm q with HNF lift
    (2q,0,0,0), (xq,yq,1,0): (W0 + W1 i) - (W2 + W3 i)(xq + yq i) mod q."""
    a = w[0] - (w[2] * xq - w[3] * yq)
    b = w[1] - (w[2] * yq + w[3] * xq)
    return a % q, b % q


def cmod(x, q):
    x %= q
    return x - q if x > q // 2 else x


def prereduced(P, rng, F=256):
    """I_sec reduced exactly, guard N4(v_s)^2 <= p (4 F N)^2 checked, then
    the intersection with I_q as U = q v_s, V = u_s + gamma v_s."""
    p, e = P['p'], P['e']
    while True:
        q, _ = G.sample_q(e, p, rng)
        xq, yq = G.root_pair(q, p, rng)
        N = G.sample_N(p, P['B'], rng, at_bound=True, avoid=(q,))
        xN, yN = G.root_pair(N, p, rng)
        us, vs, nu, nv, _ = G.exact_lagrange(*n.o0_lift(N, xN, yN), p)
        if nv * nv > p * (4 * F * N) ** 2:
            continue
        _, (lq0, lq1, _, _) = n.o0_lift(q, xq, yq)
        ru, rv = phi(us, lq0, lq1, q), phi(vs, lq0, lq1, q)
        nrv = (rv[0] ** 2 + rv[1] ** 2) % q
        if nrv == 0:
            continue
        inv = int(gmpy2.invert(nrv, q))
        # gamma = -r_u conj(r_v) / |r_v|^2
        t0 = -(ru[0] * rv[0] + ru[1] * rv[1])
        t1 = -(ru[1] * rv[0] - ru[0] * rv[1])
        g0, g1 = cmod(t0 * inv, q), cmod(t1 * inv, q)
        U = [q * c for c in vs]
        gv = zmul(g0, g1, vs)
        V = [us[r] + gv[r] for r in range(4)]
        assert phi(V, lq0, lq1, q) == (0, 0)
        T = N * q
        n.normalize(U, V, p, T)  # integral with det p, or raises
        return T, U, V


def chain_from(u0, v0, mus):
    """Run reduction backwards: (a, b) -> (b, a + mu b). The exact
    reduction of the result replays the mus in reverse when the rounding
    cells allow; the runs are measured, not assumed."""
    a, b = list(u0), list(v0)
    for m0, m1 in mus:
        mb = zmul(m0, m1, b)
        a, b = b, [a[r] + mb[r] for r in range(4)]
    return a, b


def chains(P, rng, q, xq, yq):
    """Families as in clamped-quotient-progress/adversarial.py chains(),
    applied to reduced bases of genuine signing ideals and capped at the
    generic O0 starting bound: max(N4)/(2T) <= 2^(Tbits+1)."""
    p = P['p']
    lim = 1 << (P['Tbits'] + 1)
    out = []

    def base():
        T, _, _, (u, v) = honest_ideal(P, rng, q, xq, yq, at_bound=True)
        us, vs, _, _, _ = G.exact_lagrange(u, v, p)
        return T, us, vs

    def fits(T, a, b):
        return max(N4(a, p), N4(b, p)) // (2 * T) <= lim

    def grow(T, us, vs, gen):
        mus = []
        k = 0
        while True:
            mu = gen(k)
            a, b = chain_from(us, vs, mus + [mu])
            if not fits(T, a, b):
                return mus
            mus.append(mu)
            k += 1

    for mode in ('conj_alt', 'i_rot'):
        T, us, vs = base()
        if mode == 'conj_alt':
            mus = grow(T, us, vs, lambda k: (1, 1) if k % 2 == 0 else (1, -1))
        else:
            mus = grow(T, us, vs, lambda k: (1, 1))
        out.append(('rolletschek_' + mode, T, *chain_from(us, vs, mus)))
    for trial in range(2):
        T, us, vs = base()

        def g(k):
            b = rng.randrange(61, 201)
            return (rng.randrange(1 << (b - 1), 1 << b), rng.randrange(-(1 << (b - 1)), 1 << (b - 1)))
        mus = grow(T, us, vs, g)
        out.append(('chain_allbig', T, *chain_from(us, vs, mus)))
    for frac in (0.25, 0.5, 0.75):
        T, us, vs = base()
        mus = []
        k = 0
        while True:
            mu = (1, 1) if k % 2 == 0 else (1, -1)
            a, b = chain_from(us, vs, mus + [mu])
            if max(N4(a, p), N4(b, p)) // (2 * T) > (1 << int(frac * (P['Tbits'] + 1))):
                break
            mus.append(mu)
            k += 1
        # one big quotient taking most of the remaining room
        a, b = chain_from(us, vs, mus)
        cur = max(N4(a, p), N4(b, p)) // (2 * T)
        room = (lim.bit_length() - cur.bit_length()) // 2 - 2
        while room > 61:
            a2, b2 = chain_from(us, vs, mus + [(1 << room, 1)])
            if fits(T, a2, b2):
                mus.append((1 << room, 1))
                break
            room -= 1
        out.append((f'chain_roll{int(frac * 100)}_big', T, *chain_from(us, vs, mus)))
    T, us, vs = base()
    mus = grow(T, us, vs, lambda k: ((1 << 62) + 1, 1) if k % 3 == 2 else ((1, 1) if k % 2 == 0 else (1, -1)))
    out.append(('chain_roll_62', T, *chain_from(us, vs, mus)))
    # (d) quotients just above the window, as many as fit: the most big
    # sub-steps per unit of size
    for trial in range(2):
        T, us, vs = base()
        mus = grow(T, us, vs, lambda k: (rng.randrange(1 << 61, 1 << 66), rng.randrange(-(1 << 20), 1 << 20)))
        out.append(('chain_big61', T, *chain_from(us, vs, mus)))
    return out


def reference(p, cls, kind, shape, T, u, v, steps, tex, rec):
    """Python references for one accepted input."""
    G_ = n.normalize(u, v, p, T)
    if shape == 'hnf':
        assert G_ == n.gram_hnf(T, v[0], v[1], p)
    Gn = n.num_gram(u, v, p)
    rn = n.run_gram(u, v, Gn, steps)
    rz = n.run_gram(u, v, G_, steps, rec=rec)
    ex = n.run_exact(u, v, p, tex)
    need = n.exact_steps_needed(u, v, p)
    assert need + 1 <= tex, (cls, kind, need, tex)
    assert (rn['u'], rn['v']) == (ex['u'], ex['v']) == (rz['u'], rz['v']), (cls, kind)
    assert n.trim(rn['logical']) == n.trim(ex['logical']) == n.trim(rz['logical']), (cls, kind)
    assert rn['pending'] == rz['pending'] == (0, 0)
    assert n.is_reduced(rz['u'], rz['v'], rz['G'])
    # scaled numerator Gram of the final pair equals 2T times the carried one
    assert n.num_gram(rz['u'], rz['v'], p) == tuple(2 * T * g for g in rz['G'])
    diff = sum(1 for a, b in zip(rn['phys'], rz['phys']) if a != b)
    groups = lambda r: _groups(r['phys'])
    gdiff = sum(1 for a, b in zip(groups(rn), groups(rz)) if a != b)
    fields = [hx(p), cls, kind, shape, hx(T)] + [hx(c) for c in u + v] + [str(steps), '1']
    fields += [hx(g) for g in G_] + [str(rn['nbig']), str(rz['nbig']), str(n.phys_digest(rn['phys'])),
                                     str(n.phys_digest(rz['phys'])), str(n.logical_digest(rz['logical']))]
    fields += [hx(c) for c in rz['u'] + rz['v']]
    info = dict(cls=cls, kind=kind, need=need, tex=tex, steps=steps, nbig_num=rn['nbig'], nbig_norm=rz['nbig'],
                phys_diff=diff, logical_steps=len(n.trim(ex['logical'])), big_group_diff=gdiff,
                ties=_ties(ex), M0_norm_bits=n.bl(max(G_[0], G_[1])), M0_num_bits=n.bl(max(Gn[0], Gn[1])),
                setup_num_bits=n.bl(v[0] ** 2 + v[1] ** 2 + p * (v[2] ** 2 + v[3] ** 2)))
    return ' '.join(fields), info


def _groups(phys):
    """Physical steps grouped by logical step: the tuple of big sub-steps
    (mantissas, shift) preceding each window step."""
    out, cur = [], []
    for m0, m1, s, big, sw in phys:
        if big:
            cur.append((m0, m1, s))
        else:
            out.append(tuple(cur))
            cur = []
    return out


def _ties(ex):
    return 0  # ties are counted on the toy runs, where they occur


def do_set(name):
    P = params(name)
    p, e = P['p'], P['e']
    rng = random.Random(0x6EA4 + e)
    lines, infos, rec = [], [], n.Rec()
    recs = {}
    ideals, rejects = research_inputs(P)
    tex, texp = n.counts('sign-generic', name)
    for kind, t, u, v in ideals:
        r = recs.setdefault('sign-generic', n.Rec())
        line, info = reference(p, 'sign-generic', 'lift_' + kind, 'hnf', t, u, v, texp, tex, r)
        lines.append(line); infos.append(info)
    for kind, t, u, v in rejects:
        try:
            n.normalize(u, v, p, t)
            raise AssertionError('a non-ideal passed normalize')
        except n.NotIntegral:
            pass
        lines.append(' '.join([hx(p), 'reject', kind, 'hnf', hx(t)] + [hx(c) for c in u + v] + ['0', '0']))
        infos.append(dict(cls='reject', kind=kind))
    # other classes
    D = int(gmpy2.next_prime(1 << (2 * P['W'])))
    for i in range(6):
        xD, yD = G.root_pair(D, p, rng)
        u, v = n.o0_lift(D, xD, yD)
        tex_, texp_ = n.counts('keygen-D', name)
        line, info = reference(p, 'keygen-D', 'keygen_D', 'hnf', D, u, v, texp_, tex_, recs.setdefault('keygen-D', n.Rec()))
        lines.append(line); infos.append(info)
    for i in range(6):
        N = G.sample_N(p, P['B'], rng, at_bound=(i % 2 == 0))
        xN, yN = G.root_pair(N, p, rng)
        u, v = n.o0_lift(N, xN, yN)
        tex_, texp_ = n.counts('keygen-Isec', name)
        line, info = reference(p, 'keygen-Isec', 'keygen_Isec', 'hnf', N, u, v, texp_, tex_, recs.setdefault('keygen-Isec', n.Rec()))
        lines.append(line); infos.append(info)
    for i in range(6):
        q, _ = G.sample_q(e, p, rng)
        xq, yq = G.root_pair(q, p, rng)
        u, v = n.o0_lift(q, xq, yq)
        tex_, texp_ = n.counts('challenge', name)
        line, info = reference(p, 'challenge', 'challenge', 'hnf', q, u, v, texp_, tex_, recs.setdefault('challenge', n.Rec()))
        lines.append(line); infos.append(info)
    for i in range(6):
        T, U, V = prereduced(P, rng)
        tex_, texp_ = n.counts('prered-guard256', name)
        line, info = reference(p, 'prered-guard256', 'prereduced_F256', 'gen', T, U, V, texp_, tex_, recs.setdefault('prered-guard256', n.Rec()))
        lines.append(line); infos.append(info)
    q, _ = G.sample_q(e, p, rng)
    xq, yq = G.root_pair(q, p, rng)
    for kind, T, U, V in chains(P, rng, q, xq, yq):
        line, info = reference(p, 'sign-generic', kind, 'gen', T, U, V, texp, tex, recs.setdefault('chains', n.Rec()))
        lines.append(line); infos.append(info)
    os.makedirs(OUT, exist_ok=True)
    open(os.path.join(OUT, name + '.txt'), 'w').write('\n'.join(lines) + '\n')
    json.dump(dict(set=name, inputs=infos, widths={k: dict(v) for k, v in recs.items()}),
              open(os.path.join(RES, f'gen_{name}.json'), 'w'), indent=1)
    acc = [i for i in infos if i['cls'] != 'reject']
    return f"{name}: {len(acc)} accepted, {len(infos) - len(acc)} rejects; " \
           f"max big sub-steps num/norm {max(i['nbig_num'] for i in acc)}/{max(i['nbig_norm'] for i in acc)}; " \
           f"inputs with differing physical decisions {sum(1 for i in acc if i['phys_diff'])}"


def toy():
    """Small-p full runs with exact half ties, both signs."""
    rng = random.Random(7)
    out, pos, neg = [], 0, 0
    for p in (3, 7, 11, 19, 23, 31, 43, 47, 59, 67, 71, 79, 83):
        for T in range(1, 400, 2):
            sols = [(x, y) for x in range(0, 2 * T, 2) for y in range(1, 2 * T, 2) if (x * x + y * y + p) % (4 * T) == 0]
            rng.shuffle(sols)
            for x, y in sols[:6]:
                u, v = [2 * T, 0, 0, 0], [x, y, 1, 0]
                ex = n.run_exact(u, v, p, 40)
                # replay to find ties
                Gz = n.normalize(u, v, p, T)
                tp = tn = 0
                A, C, h0, h1 = Gz
                uu, vv = list(u), list(v)
                if C < A:
                    A, C, h1 = C, A, -h1
                r = n.run_gram(u, v, Gz, 40)
                # count ties along the exact run on the Gram
                Az, Cz, a0, a1 = Gz
                if Cz < Az:
                    Az, Cz, a1 = Cz, Az, -a1
                for (m0, m1, sw) in r['logical']:
                    for h in (a0, a1):
                        if (2 * h + Az) % (2 * Az) == 0:
                            tp += h > 0; tn += h < 0
                    f0, f1 = m0, m1
                    r0, r1 = a0 - f0 * Az, a1 - f1 * Az
                    C2 = Cz - f0 * (a0 + r0) - f1 * (a1 + r1)
                    if C2 < Az:
                        Az, Cz, a0, a1 = C2, Az, r0, -r1
                    else:
                        Cz, a0, a1 = C2, r0, r1
                if (tp and pos < 40) or (tn and neg < 40):
                    pos += bool(tp); neg += bool(tn)
                    assert (r['u'], r['v']) == (ex['u'], ex['v'])
                    fields = [hx(p), 'toy', f'tie_p{p}_T{T}_pos{tp}_neg{tn}', 'hnf', hx(T)] + [hx(c) for c in u + v] + ['40', '1']
                    rn = n.run_gram(u, v, n.num_gram(u, v, p), 40)
                    fields += [hx(g) for g in Gz] + [str(rn['nbig']), str(r['nbig']), str(n.phys_digest(rn['phys'])),
                                                    str(n.phys_digest(r['phys'])), str(n.logical_digest(r['logical']))]
                    fields += [hx(c) for c in r['u'] + r['v']]
                    out.append(' '.join(fields))
            if pos >= 40 and neg >= 40:
                break
        if pos >= 40 and neg >= 40:
            break
    open(os.path.join(OUT, 'toy.txt'), 'w').write('\n'.join(out) + '\n')
    return f'toy: {len(out)} runs, {pos} with a +1/2 tie, {neg} with a -1/2 tie'


if __name__ == '__main__':
    os.makedirs(RES, exist_ok=True); os.makedirs(OUT, exist_ok=True)
    names = sys.argv[1:] or n.NAMES
    with Pool(len(names) + 1) as pool:
        jobs = [pool.apply_async(do_set, (nm,)) for nm in names] + [pool.apply_async(toy)]
        msgs = [j.get() for j in jobs]
    for m in msgs:
        print(m)
    open(os.path.join(RES, 'gen_summary.txt'), 'w').write('\n'.join(msgs) + '\n')
