# Sage accept-path model report

Rust status, 2026-11-12: the adapted two-chain path is now implemented.
All 113 honest fixtures pass the public verifier and every intermediate
null point of both chains matches the six traces below. The recorded Sage
runs remain historical evidence; fresh Sage differential execution requires
Sage and the pinned submodule. See `docs/SECURITY_REVIEW.md`.

Pin: `external/Theta_dim4` @ `4770b24a61e6aa15da4cac1b9d82fd7b9a05f459`.

## Accept path Ok (regenerated vectors)

On the regenerated `s5_lvl1.txt` the model runs both chains and confirms:

- Derived: `C1.zero() == C2.hadamard().zero()` (step 5 / Codomain).
- Derived: step 8 images `Φ(X, 0) = (±[s]X, ±[s]Y)` for `X` in
  `{P_vk, Q_vk, P_vk − Q_vk}`.
- Ran: accept path Ok.

All six vectors confirm the algebra: Weil `e(R1,R2)=e(P1,P2)^q` with no
sign repair, `VkDx = x(VkP − VkQ)`, product basis symplectic after `[s]`,
`Φ̂Φ = 2^{2e′−1} I`, `K″1`/`K″2` isotropic, symplectic matrices.

## The earlier Codomain refutation was the vk export, not the claim

The first model run refuted `C1.zero() == C2.hadamard().zero()` on the
then-committed NIST-I vectors (both keys). Three findings resolved it:

1. **Model wiring.** The first model completed `M1` from scratch. In
   `starting_two_symplectic_matrices` the matrix `M1` is never re-completed:
   it is the block matrix `[complement of M1_0 | −Φ̂(complement of M2)]`,
   and its symplecticity is a checked theorem. The chain kernels are the
   kernel-half columns of `M1`, `M2` evaluated on the basis (`kernel_basis`),
   not the hand-written `K″` generators: the theta base change `N_i` is
   adapted to exactly that basis. Both fixes are in `s5_model.sage`; small
   dim-2 instances (nine `(s, q, e′)` cases, both exporter sign classes,
   basis and difference second points — 72 runs) and Dartois's own dim-4
   `KaniEndoHalf` (where `F1.codomain.zero() == F2_dual.codomain.hadamard()
   .zero()` holds on the nose) confirm the construction.

2. **The refutation survived the wiring fix** on both NIST-I keys, while
   every small synthetic instance met. Pairings cannot distinguish the
   exported pair from any unimodular transform of the true preimages, so
   the vectors were suspected.

3. **Instrumented exporter.** Rebuilding `s5_lvl1.txt` from its committed
   seeds with a debug exporter and solving 2-adic discrete logs in the
   image basis gave, exactly: `[s]P_sig = σ(P)`, `[s]Q_sig = σ(Q)` for the
   C's internal canonical pair `(P, Q)` — the signing pipeline is correct —
   while the exported points were `VkP = −P`, `VkQ = Q − P`. The exporter's
   Q↔PmQ swap emitted the pair `(P, P−Q)`: determinant `−1` with a shear,
   invisible to every pairing check, fatal to the chains. Running the
   chains with the true preimages `(−VkP, VkQ − VkP)` met immediately.

The fix removes the swap from `scripts/c/export_s5_vectors.c`: the export
is now the preimage pair itself (up to one global sign from `lift_basis`,
which composes `[−1]` into `σ` and is harmless), with `VkDx = x(P − Q)`
(= `x(R)`, the cleared `−x(P)−A` point, Rust's `even_basis.q`). All six
vectors are regenerated from the same seeds; only `VkQx`, `VkQy`, `VkDx`
changed.

## Known-answer files

Per vector: `*_matrices.json` (`Φ`, `M1`, `M2`, mod-4 reductions, theta
base changes `N1`, `N2`, chain kernel bases `BK1`, `BK2`). For every
vector, both NIST-I keys and both keys at III and V: `*_nulls.json`
(theta-null point after every step of both chains) and `*_step8.json`
(step 8 images). Codomain and Image are confirmed on all six.

## Limit of a pairing check (for the §7 arguments)

`e(R₁, R₂) = e(P₁, P₂)^q` holds for every determinant-±1 image of the
true preimages, not only for the preimage pair. The vk export's Q↔PmQ
swap emitted `VkP = −P`, `VkQ = Q − P`, a determinant −1 shear. Every
algebraic check in this model accepted it. The chains did not. A
pairing-level validation of an exported basis does not identify the pair
the signature corresponds to. The chain is the check that does.

## Index search

The first search tested `1 + b^2` as an integer square. Under that test
`s5_lvl3.txt` and `s5_lvl5.txt` (`A` a square) named an abscissa on the
twist. The verifier tests `1 + b^2` in `F_p`. With that test every
committed vector's abscissa is on the curve (`s5_lvl3.txt` index 10,
`s5_lvl5.txt` index 4), and an abscissa off the curve is a failure.

## §6 after joint normalization and the pairing check

Both keys at each level. Sign is decided on `P` after negating both
points or neither. `Pairing` is `e(R_1, R_2) = e(P_1, P_2)^q` before
either chain. No row differs from the expected set.

| # | All six vectors |
| --- | --- |
| 1 | Composite |
| 2 | Image |
| 3 | Sign |
| 4 | Pairing |
| 5 | Pairing |
| 6 | Kernel |
| 7 | order 2: 6 Pairing and 3 Codomain; order 4: 30 Pairing and 6 Codomain; never Ok |
| 8 | Kernel |
| 9 | `[3]` Pairing, `[2^{e-1}+1]` Image |
| 10 | Model, five times |
| 11 | Image, at a prime index |
| 12 | Pairing, at a prime index for the other key |
| 13 | Ok five times, with Model off |
| 14 | Codomain for `(P, Q+P)`, `(P+Q, Q)`, `(Q, -P)` |

## Basis rule

Over 101 keys at each level (the committed `A` plus 100 samples), Rust's
`P` and `R` match the C basis, and the C `Q` is `difference(P, R)` after
the cofactor on every key. `EvenBasis.difference` is that point.
`[f]` of the difference of the uncleared points matches `Q` on 44 / 50 /
50 keys at I / III / V, and it is not the field. `c-vectors` asserts the
cleared one.

## Accept vectors

`prism-verify/testdata/accept/` holds further honest signatures the model
returned Ok on: 20 at NIST-I, 47 at NIST-III, and 40 at NIST-V. A name
containing `_r` is a repeated-byte seed. A name containing `_p` is one of
seeds 0 through 19 under
`SHA-256("shekyl-prism-accept-positive|{III|V}|{counter}")`, expanded to
the exporter's 48 bytes by appending the digest's first 16 bytes, and
signed with the positive-cofactor patch below. The other names are the
earlier SHA-256 seeds. The original six files in the parent testdata
directory stay as they are. `accept_verdicts.tsv` lists every file in
that directory with the verdict from a fresh run of the model, each Ok.

The earlier NIST-III rows, the ones whose names are not `_p`, are the
seeds on which keygen returned before that patch. In a sample of 24
SHA-256 seeds, keygen aborted on all 24, so that earlier set is the
survivors of that fault.

## Exporter aborts

A sample of 24 SHA-256 seeds at each level, classified by the exporter's
stderr. Every abort is `mpz_div_qr: Divide by zero`. The zero operand is
the remainder `r1` in the Euclidean loop of `ibz_cornacchia_prime`
(`integers.c` in the pinned tree). The coefficient `n` is `1`. The other
argument `p`, the cofactor `ibz_cornacchia_extended` passes through, is
negative (185 bits at III, 249 bits at V). The loop test is
`remainder² >= p`, which stays true for a zero remainder when `p` is
negative, and the next iteration divides by that remainder.
`mpz_probab_prime_p` uses the absolute value, so the negative cofactor is
treated as prime and handed to Cornacchia. `quat_qlapoti` reaches that
call from `dim2id2iso_ideal_to_isogeny_clapotis` during
`protocols_keygen`. The exporter only calls `protocols_keygen`; it does
not pass the zero. The fault is in the pinned tree. No attempt printed
the exporter's own retry lines (no response, image not a `2^e` basis,
basis application failed).

| Level | Attempts | Signatures | Keygen abort | Sign abort |
| --- | --- | --- | --- | --- |
| I | 24 | 17 | 2 | 5 |
| III | 24 | 0 | 24 | 0 |
| V | 24 | 2 | 14 | 8 |

`scripts/c/cornacchia-positive.patch` rejects a non-positive cofactor as
the first statement of `ibz_cornacchia_prime`, returning 0 before the
Euclidean loop. `scripts/export_s5_vectors.sh` applies it to a copy of
`integers.c`, compiles that object, and links the exporter against a
copy of the quaternion archive. The submodule and the reference build
are not modified. With that exporter, seeds 0 through 19 at NIST-III and
NIST-V each produced a signature, and the closed-form model returned Ok
on all 40.

## Closed form

`M1` and `M2` are the closed form in `docs/VERIFIER_SPEC.md`, with
`t = s^{-1}` modulo `2^e`. On the 73 honest signatures present before
the `_p` batch, and on every negative row of the six original vectors,
the chain verdict is the same as Dartois's `complete_symplectic_matrix_dim2`.
The matrix, null-point, and step-8 files are regenerated from the closed
form. `base_change_theta_dim2` in `prism-isogeny/src/theta_base.rs` matches
`N1` and `N2` in those files. The Rust accept path now consumes these adapted structures and compares
both image components.
