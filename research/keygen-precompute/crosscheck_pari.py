# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Verify kpc.py algebra against PARI (mathnf, qfminim) across all 5 sets."""
from pathlib import Path

from hdl import (
    SETS, shake, prime_of, fs_challenge, select_xy, ideal_basis,
    secret_class, ideal_from_classes, hnf_rows, in_span, pari,
)
from kpc import (
    PARAMS, key_sizes_bytes, find_ideal_generator, sim_quat_lideal_create,
    sim_inter_hnf_q3, inter_prereduced_zi, basis_from_zi, gram_from_zi,
    zi_gauss_uv,
)

pari.set_real_precision(1200)
OUT = Path(__file__).resolve().parent / 'results'
OUT.mkdir(parents=True, exist_ok=True)


def pari_lam1(G):
    M = pari.matrix(4, 4, [G[r][c] for r in range(4) for c in range(4)])
    U = pari.qflllgram(M)
    Mr = U.mattranspose() * M * U
    res = pari.qfminim(Mr, None, 4, 2)
    V = res[2]
    Gm = [[int(Mr[r, t]) for t in range(4)] for r in range(4)]
    lam1 = None
    for j in range(int(pari.matsize(V)[1])):
        x = [int(V[r, j]) for r in range(4)]
        nx = sum(x[r] * Gm[r][t] * x[t] for r in range(4) for t in range(4))
        lam1 = nx if lam1 is None else min(lam1, nx)
    return lam1


def main():
    lines = []
    lines.append("Key sizes (bytes):")
    for name, _, _ in SETS:
        ks = key_sizes_bytes(PARAMS[name])
        lines.append(
            f"  {name}: cref_sk={ks['cref_sk']} base_spec={ks['base_spec_sk']} "
            f"exp_hnf={ks['exp_hnf_with_can']} exp_red={ks['exp_red_compact_with_can']} "
            f"(no_can={ks['exp_red_compact_no_can']}, compat={ks['exp_red_compat']}, "
            f"gram={ks['exp_red_gram']})"
        )

    n_per_set = 100
    hnf_checks = 0
    zi_checks = 0
    bound_checks = 0
    for name, e, f in SETS:
        p = prime_of(e, f)
        L = p.bit_length()
        base_nb = (L + 1) // 2
        B = base_nb + 32
        T_sec = int(0.32 * B) + 40
        T_prod = int(0.32 * (2 * e - 5)) + 30
        for kid in range(n_per_set):
            sk = shake(b'xchk-sk', name.encode(), kid.to_bytes(4, 'little'), n=32)
            u_off = (kid * 7) % 33
            nb = base_nb + u_off
            N, xN, yN = secret_class(sk, p, nb)
            B_sec = ideal_basis(N, p, (xN, yN, 1, 0))

            u_sec, v_sec, nu_sec, nv_sec, _, _ = zi_gauss_uv(
                [N, 0, 0, 0], B_sec[2], p, T_sec, keep_trace=False
            )
            assert nu_sec % N == 0 and nu_sec >= N
            assert nv_sec <= 2 * N * p
            bound_checks += 1

            msg = shake(b'xchk-msg', name.encode(), kid.to_bytes(4, 'little'), n=32)
            salt = shake(b'xchk-salt', sk, msg, n=32)
            _, q, _ = fs_challenge(e, b'pk', salt, msg)
            xq, yq, _ = select_xy(q, p, sk)
            B_q = ideal_basis(q, p, (xq, yq, 1, 0))

            B_prod, T_prod_norm = ideal_from_classes(p, [(xN, yN, N), (xq, yq, q)])
            assert T_prod_norm == N * q

            if kid < 20:
                gen = find_ideal_generator(B_sec, N, p)
                B_rec, _ = sim_quat_lideal_create(gen, N, p)
                assert B_rec == B_sec, f"HNF mismatch on create for {name} {kid}"
                B_q3, _ = sim_inter_hnf_q3(B_sec, N, B_q, q)
                assert B_q3 == B_prod, f"HNF mismatch on Q3 inter for {name} {kid}"
                hnf_checks += 1

            w1, w2, _ = inter_prereduced_zi(u_sec, v_sec, xq, yq, q)
            B_exp = basis_from_zi(w1, w2)
            assert hnf_rows(B_exp) == B_prod, f"EXP_RED lattice mismatch for {name} {kid}"

            u_out, _, nu_out, _, _, _ = zi_gauss_uv(w1, w2, p, T_prod, keep_trace=False)
            assert nu_out % (N * q) == 0 and nu_out > 0
            assert in_span(u_out, B_prod)
            G_exp = gram_from_zi(w1, w2, p, type('C', (), {'mul': 0})())
            lam1_p = pari_lam1(G_exp)
            assert nu_out == lam1_p, f"Z[i] missed lam1 at {name} {kid}: {nu_out} vs {lam1_p}"
            zi_checks += 1
        print(f"crosscheck {name}: done ({n_per_set} keys)", flush=True)

    lines.append(
        f"PARI crosscheck PASS: hnf_checks={hnf_checks}, zi_checks={zi_checks}, "
        f"bound_checks={bound_checks} (0 mismatches)."
    )
    text = "\n".join(lines) + "\n"
    (OUT / 'pari_crosscheck.txt').write_text(text)
    print(text)


if __name__ == '__main__':
    main()
