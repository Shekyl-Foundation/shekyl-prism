"""Empirical cross-check of the recomputed O0-coordinate counts (t37u).

For each set: honest signing ideals and crafted_k1 in O0 numerators, challenge
and keygen ideals in O0 numerators, and the two Rolletschek worst-case chains
built at size 2^(Tbits+1) in model shape (A0 = 4T^2, Delta = 4 p T^2: exactly
the O0 signing bounds). Every input runs the exact contract for the O0
T_exact and exp64 for the O0 T_exp64 (any-basis count for the chains), and
must end reduced. Writes results/verify_o0_counts.txt."""
import os, random, sys
import gmpy2
from q5common import SETS, params
import gen_inputs as G
from trace_counterexample import crafted_k1
from adversarial import load_counts, run_rule, G_exact_es

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = []


def say(s):
    print(s, flush=True); OUT.append(s)


def main():
    c = load_counts()
    for s in SETS:
        name = s[0]; P = params(name); p, e, B = P['p'], P['e'], P['B']
        rng = random.Random(0x5EC0 ^ (hash(name) & 0xffff))
        worst = {}
        bad = 0; n = 0

        def go(fam, cls, u0, v0, ideal=True):
            nonlocal bad, n
            Tex, Texp, Tany = c[(name, cls + '@O0')]
            steps, _, _ = G_exact_es(u0, v0, p)
            need = steps - 1
            worst[fam] = max(worst.get(fam, (0, Tex))[0], need), Tex
            r1 = run_rule(u0, v0, p, Tex, 'exact', False)
            r2 = run_rule(u0, v0, p, Texp if ideal else Tany, 'exp64', False)
            n += 1
            if not (r1['reduced'] and r2['reduced']):
                bad += 1
                say(f'  FAIL {name} {fam}')

        q, _ = G.sample_q(e, p, rng); xq, yq = G.root_pair(q, p, rng)
        for i in range(12):
            N = G.sample_N(p, B, rng, at_bound=(i % 2 == 0), avoid=(q,))
            xN, yN = G.root_pair(N, p, rng)
            x, y = G.crt_pair(N, xN, yN, q, xq, yq)
            go('honest_sign_O0', 'sign-generic', *G.o0_basis(N * q, x, y))
        for sd in range(3):
            T, k, x, y = crafted_k1(P, seed=78 + 10 * sd)
            go('crafted_k1_O0', 'sign-generic', *G.o0_basis(T, x, y))
        for i in range(3):
            qq, _ = G.sample_q(e, p, rng); xa, ya = G.root_pair(qq, p, rng)
            go('challenge_O0', 'challenge', *G.o0_basis(qq, xa, ya))
            N = G.sample_N(p, B, rng, at_bound=True)
            xa, ya = G.root_pair(N, p, rng)
            go('keygen_Isec_O0', 'keygen-Isec', *G.o0_basis(N, xa, ya))
        D = int(gmpy2.next_prime(1 << (2 * P['W'])))
        xD, yD = G.root_pair(D, p, rng)
        go('keygen_D_O0', 'keygen-D', *G.o0_basis(D, xD, yD))
        for mode in ('conj_alt', 'i_rot'):
            T, g, h, k = G.make_rolletschek(1 << (P['Tbits'] + 1), mode=mode)
            go('rolletschek_' + mode + '_at_O0_bound', 'sign-generic', *G.model_basis(T, g, h), ideal=False)
        say(f'== {name}: {n - bad}/{n} inputs reduced under exact (O0 T_exact) and exp64 (O0 T_exp64)')
        for fam, (w, Tex) in worst.items():
            say(f'  {fam:34s} worst steps needed (excl. final size reduction) {w:4d} of O0 T_exact {Tex}')
        assert bad == 0
    open(os.path.join(HERE, 'results', 'verify_o0_counts.txt'), 'w').write('\n'.join(OUT) + '\n')


if __name__ == '__main__':
    main()
