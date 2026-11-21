# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Run variants A, L, B, B0 on shared input streams and write per-sample CSVs.

Streams per bit size b:
  primes : random primes M = 1 mod 4 of exactly b bits (always solvable, d = 1)
  random : uniform random b-bit integers, SQIsign-style "retry until prime":
           non-prime or 3 mod 4 inputs are rejected before Cornacchia (logged)
  fixed  : K fixed primes, B (and B0) re-run `draws` times each with fresh S
"""
import argparse, csv, secrets, time, os
import gmpy2
from cornacchia_variants import *

def rand_prime_1mod4(b):
    while True:
        m = secrets.randbits(b) | (1 << (b - 1))
        m = m - (m % 4) + 1
        if m.bit_length() == b and gmpy2.is_prime(m):
            return m

def feats(M, u):
    return dict(m_hw=gmpy2.popcount(M), m_mod8=M % 8, m_mod3=M % 3, m_mod5=M % 5,
                m_mod7=M % 7, u_bits=u.bit_length(), u_hw=gmpy2.popcount(u))

def qsummary(prefix, q):
    aq = [abs(x) for x in q]
    return {f'{prefix}_steps': len(q),
            f'{prefix}_qsumbits': sum(x.bit_length() for x in aq),
            f'{prefix}_qmaxbits': max((x.bit_length() for x in aq), default=0),
            f'{prefix}_q1': sum(1 for x in aq if x == 1),
            f'{prefix}_q2': sum(1 for x in aq if x == 2),
            f'{prefix}_q3': sum(1 for x in aq if x == 3),
            f'{prefix}_q4p': sum(1 for x in aq if x >= 4),
            f'{prefix}_q0': sum(1 for x in aq if x == 0)}

def timed(f, *a):
    t = time.perf_counter_ns(); r = f(*a); return r, time.perf_counter_ns() - t

def one(M, gen, b):
    (u, att), _ = timed(sqrt_minus_one, M)
    row = dict(bits=b, sqrt_attempts=att, **feats(M, u))
    (sa, ta), tA = timed(cornacchia_classical, M, u)
    (sl, tl), tL = timed(lattice_plain, M, u)
    S = gen.next()
    (sb, tb), tB = timed(lattice_randomized, M, u, S)
    (sb0, tb0), tB0 = timed(lattice_randomized, M, u, naive_S())
    assert sa and sl and sb and sb0, (M, u)
    row.update(qsummary('A', ta['quotients'])); row['A_ok'] = int(ta['square_ok']); row['A_ns'] = tA
    row.update(qsummary('L', tl['quotients'])); row['L_ns'] = tL
    row.update(qsummary('B', tb['quotients'])); row['B_ns'] = tB; row['B_Sbits'] = tb['S_maxbits']
    row['B_swaps'] = tb['swaps']; row['B_draws'] = 1
    row['B0_steps'] = tb0['steps']
    return row

def write(path, rows):
    with open(path, 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sizes', type=int, nargs='+', default=[384, 500, 664])
    ap.add_argument('--n', type=int, default=3000)
    ap.add_argument('--nrandom', type=int, default=20000)
    ap.add_argument('--fixed', type=int, default=24)
    ap.add_argument('--draws', type=int, default=500)
    ap.add_argument('--out', default='results')
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    for b in a.sizes:
        gen = SGenerator()
        t0 = time.time()
        rows = [one(rand_prime_1mod4(b), gen, b) for _ in range(a.n)]
        write(f'{a.out}/primes_{b}.csv', rows)
        # random stream: SQIsign-style rejection until prime = 1 mod 4
        acc = rej_comp = rej_3mod4 = 0; solved = []
        for _ in range(a.nrandom):
            m = secrets.randbits(b) | (1 << (b - 1))
            if not gmpy2.is_prime(m): rej_comp += 1; continue
            if m % 4 != 1: rej_3mod4 += 1; continue
            acc += 1; solved.append(one(m, gen, b))
        write(f'{a.out}/random_{b}_accepted.csv', solved) if solved else None
        with open(f'{a.out}/random_{b}_summary.csv', 'w') as f:
            f.write('bits,n,accepted,rejected_composite,rejected_3mod4\n')
            f.write(f'{b},{a.nrandom},{acc},{rej_comp},{rej_3mod4}\n')
        # fixed-m panel
        frows = []
        for k in range(a.fixed):
            M = rand_prime_1mod4(b)
            u, _ = sqrt_minus_one(M)
            _, ta = cornacchia_classical(M, u)
            _, tl = lattice_plain(M, u)
            # A twice with the other root and a fresh sqrt: determinism check
            u2, _ = sqrt_minus_one(M)
            _, ta2 = cornacchia_classical(M, u2)
            for dr in range(a.draws):
                _, tb = lattice_randomized(M, u, gen.next())
                _, tb0 = lattice_randomized(M, u, naive_S())
                aq = [abs(x) for x in tb['quotients']]
                frows.append(dict(bits=b, m_id=k, draw=dr, A_steps=ta['steps'],
                                  A_steps_recheck=ta2['steps'], L_steps=tl['steps'],
                                  B_steps=tb['steps'], B_qsumbits=sum(x.bit_length() for x in aq),
                                  B_q1=sum(1 for x in aq if x == 1), B_swaps=tb['swaps'],
                                  B0_steps=tb0['steps'], **feats(M, u)))
        write(f'{a.out}/fixed_{b}.csv', frows)
        print(f'{b} bits done in {time.time()-t0:.1f}s', flush=True)

if __name__ == '__main__':
    main()
