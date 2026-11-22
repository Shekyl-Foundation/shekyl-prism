# Signer specification

Status: begun 2026-11-21. Section 1 (the constants) is written, and its values are generated and committed (stage 1b, `scripts/signer_constants.py`, `prism-vectors/data/signer/`); no signer code exists yet. This is a research specification, not an adopted standard, and it makes no security claim.

## What this document is

It is the specification our signer is held to. It is written one layer at a time, and each layer's section is written and reviewed before that layer's code (ruling G3 below). A section says what each object is in mathematical terms, what fixes it uniquely, and how a wrong value would be caught. The code is then built to the section, and a difference between the two is a defect in one of them.

It is the signing-side companion of [VERIFIER_SPEC.md](VERIFIER_SPEC.md). That document fixes the protocol: the five primes, the challenge `s`, the degree `q = 2^(2e−5) − s²`, the bytes of a key and of a signature, and every refusal. This document does not restate them. A signer conforms when the verifier of that specification accepts what it emits, at all five sets, and when each section's defining properties hold.

The paper is ePrint 2026/443 (`docs/papers/2026-443_PRISM_with _salt.pdf`). Our protocol is its Section 5 variant, the prime-degree response of Algorithm 2 (`GenIsogenyStrong`), with the departures the ledger lists (D1 to D15). For key generation and for the subroutines beneath signing the paper gives a sketch (Sections 2.3 and 3.3) and defers to its reference [1], the SQIsign round-2 submission document, by algorithm number (4.1, 3.10, 2.1). That document is not in this repository. The SQIsign specification of 2026-09-01 (`docs/papers/sqisign-20260901.pdf`, called "the 3.0 specification" below) is, and where it defines an object this document cites it by its own numbers.

### Labels

Every statement of fact carries one of four labels. **Ran**: executed, with the output committed. For this section that means one of three commands unless another is named: `python3 scripts/signer_constants.py --check`, which regenerates every value at the five sets, compares it with the committed files and with both oracles, and tests each defining property; `python3 scripts/signer_constants.py --selftest`, which runs the rules at toy primes; and the test `prism-isogeny/tests/signer_constants.rs`. CI runs all three. **Read**: read in the pinned source or document at the place named, and checkable by reading it or by one `grep` or `diff` there. **Derived**: argued, not machine-checked. **Open**: not established.

Stage 1a wrote this section before any script existed and marked what it had computed by hand "Executed, not committed". Stage 1b committed a test for each such statement, and they are now **Ran**. The ledger keeps the earlier label where a row records what was true at the time.

### Rulings (Rick, 2026-11-21)

- **G1.** The signer is new layer crates beneath the front crate, one per mathematical layer, re-exported under `hazmat`. The front crate gains `SecretKey<S>`, key generation and `sign`. `prism-signer` becomes the reference oracle at cutover. Per-set signer constants are not in `prism-field`.
- **G2.** Each new crate has the library's standards from its first commit: no `unsafe`, documentation required, its surface decided, the no-upward-edge check, and a place in the nightly's mutation job.
- **G3.** Specification first. Each stage's pull request opens with that layer's section of this document, reviewed before code is written.
- **G4.** Constant-time hardening is inside the beta gate. It is the last stage. There is no timeline for it or for anything else here.
- **G5.** The experiments on the second basis point (ledger O21) are their own part, directly after key generation and before signing.
- **G6.** Stage 1 opens with this section.

Rulings on this section, the same day:

- **R1.** The secret ideal's first norm is the next prime after `2^(2·WIDTH_BITS)` at all five sets (1.7).
- **R2.** The four bounds of the quaternion layer (32 primality rounds, a box of 64, 17 generator bits, 20,000 steps per generator) are the values at all five sets: starting values, fixed at the three reference primes for replay, provisional at `p324` and `p664`, gated on stage 3's measured failure rates.
- **R3, not ruled.** The Montgomery model of a key (1.6) is looked at again when the work reaches stage 3.

Standing from Round 0 (2026-11-14): we are not transcribing the C; other code bases are oracles for values and not sources of structure; five sets, one algorithm.

### The layers, in the order they are built

| Stage | Layer | Section here |
| --- | --- | --- |
| 1 | The constants: what each is (1a, this section) and the generator that produces them (1b) | 1 |
| 2 | Integers and the quaternion algebra: the algebra, the maximal order, lattices, ideals, lattice reduction | to be written |
| 3 | Cornacchia; the norm equation of the ideal-to-isogeny translation; sampling an ideal of prime norm, with the equivalent prime-norm ideal for key generation. No step that represents an integer by the order's norm form (1.2) | to be written |
| 4 | Ideal to isogeny: first the engine's forward chain as the signer needs it (1.9), then the starting curve, its basis, the action of its endomorphisms, and the translation | to be written |
| 5 | Key generation and signing | to be written |
| last | Constant-time hardening (G4) | to be written |

The table of 1.3 names the stage that consumes each constant by these numbers. The plan of 2026-11-21 divides the stages further; this section does not use the finer names.

## 1. The constants

### 1.1 What was read

- **The reference.** `external/PRISM_v2` at `87fad7d`: the generated files of `src/precomp/ref/lvl1`, `lvl3` and `lvl5` (the reference's names for `p248`, `p376` and `p500`), and the code that reads them, followed from `protocols_keygen` (`src/signature/ref/lvlx/keygen.c`) and `protocols_sign` (`sign.c`) down. Its build file names eight Sage scripts under `scripts/precomp/` that produce these files. That directory is not in the tree (**Read**).
- **Our research signer.** `scripts/c/export_s5_vectors.c`, the program `scripts/export_s5_vectors.sh` builds on the patched reference and the committed salted vectors are regenerated through. It is the only existing implementation of *our* signing path, and it differs from the reference's: see 1.2.
- **`sqisign-rs`.** <https://github.com/anchorageoss/sqisign-rs> at `7a4a01b`, its generator `tools/gen-precomp` and the constant files that generator wrote. It is an implementation of SQIsign 3.0 at `p324`, `p500` and `p664`, and not of PRISM. Its sources point at a `PRECOMP.md` for the derivations; no file of that name is in its tree at that commit.
- **The 3.0 specification**, Section 4.7 (Tables 8 and 9), Section 9.3 and Appendix B (Algorithms B.1 to B.3).

### 1.2 Two signing paths, and which one is ours

The reference signs salt-PRISM (the paper's Section 6): the response ideal has norm `q(2^a − q)` for an `a`-bit prime `q`, a composite norm. Our protocol signs the Section 5 variant: the response ideal has the prime norm `q = 2^(2e−5) − s²`. Below the protocol the two walk different code, and that decides which constants our signer needs. (**Read**; each function is in the reference at the pin.)

| Step | Reference key generation | Reference signing | Our signing (the exporter) |
| --- | --- | --- | --- |
| A random left ideal of the maximal order with a given norm (`quat_sampling_random_ideal_O0_given_norm`) | prime norm: a random trace-zero element and a square root modulo the norm | composite norm: an element of norm `(cofactor)·(norm)` from `quat_represent_integer` | prime norm, as in key generation |
| An equivalent ideal of small prime norm (`quat_lideal_prime_norm_reduced_equivalent`) | yes | no | no |
| Intersection with the secret ideal (`quat_lideal_inter`) | no | yes | yes |
| Ideal to isogeny (`dim2id2iso_arbitrary_isogeny_evaluation`: the norm equation `quat_qlapoti`, the action of an endomorphism on the starting basis, a dimension-two chain from `E0 × E0` with a random Montgomery model of the codomain) | yes | yes | yes |
| The deterministic basis of the codomain and a change-of-basis matrix | yes | yes | yes |

So `quat_represent_integer` is on the reference's signing path only. It is not reached in key generation, and it is not reached in our signing. Three functions that read generated constants have no caller in the library at all: the two translations between a kernel and an ideal (`id2iso_ideal_to_kernel_dlogs_even`, `id2iso_kernel_dlogs_to_ideal_even`) and `fixed_degree_isogeny_and_eval`. A fourth, `find_uv`, is declared and never defined. (**Read**: `grep` over `src`, tests and benchmarks excluded.)

One fact about the reference's signing is recorded because the constant table shows it: `protocols_sign` passes the challenge degree `q` itself where the sampling function expects a precomputed prime cofactor, so it represents `q · q(2^a − q)`, and the precomputed cofactor (item 19) is read by nothing but two benchmarks. Whether that changes the distribution of the reference's responses is **Open** (ledger O30) and is not on our path.

### 1.3 The inventory

An **item** is one numbered row below: one named constant of the generated files (a preprocessor constant, an exported symbol, a field of the two generated structures, a line of `prism_parameters.txt`), or a few names that are one object (the three fields of item 14, the two symbols of item 16, item 38 with its bit length). There are 62. The three reference sets have the same 62 items in the same structure; the only structural fact that varies is the number of machine words (**Read**: the nine headers and six sources compared file by file).

Columns:

- **Value** (third and fourth tables, whose items are small numbers): at `p248` / `p376` / `p500`; one value where the three agree. In the other tables a value that fits is in the first cell.
- **Class**: **Det** (determined by the prime alone), **Conv** (determined once a stated convention is fixed; the convention is in 1.4), **Tun** (a bound or count someone chose; what it trades is in 1.5), **Fmt** (a size or layout of the reference's own encodings).
- **Read by**: the reference function that reads it. `K` reference key generation, `S` reference signing, `S5` our signing; "nothing" means no reader on any of the three. Then the stage of ours that would consume it.
- **Needed**: **Yes**; **No** with the reason; **Has** (the verifier's crates already hold it, at the place named); **Open** (a design question decides).
- **Bytes**: what a different *valid* choice would change. **public** (a byte of a key or signature), **secret** (the secret key's bytes only), **path** (neither), **none** (there is no other valid choice). **+draws** marks an item that changes how many random values are consumed or in what order; a replay test that compares bytes with the reference under one seed depends on every such item, whatever its class. All entries in this column are **Derived** unless marked.
- **Oracle**: where a value to compare with exists. `C`: the reference, at `p248`, `p376`, `p500`. `R`: `sqisign-rs`, at `p324`, `p500`, `p664`, with the name it emits.
- **Property**: a test that catches a wrong value with no oracle.

#### `e0_basis` and `endomorphism_action`

| # | Item | What it is | Class | Read by | Needed | Bytes | Oracle | Property |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1, 2 | `BASIS_E0_PX`, `BASIS_E0_QX` | `x(P0)` and `x(Q0)` of the starting basis (item 4), a second copy. | Conv | The deterministic-basis routine, only when the curve it is given is `E0` (`ec_basis_E0_2f`). | **No**, as separate data: equal to item 4 at all three sets (**Ran**). A key is never `E0` (the verifier refuses `A = 0`, D10). A response curve is `E0` only with negligible probability; what the signer does then is for stage 5's section. | none | as item 4 | as item 4 |
| 3 | `curve` | The starting curve `E0: y² = x³ + x` over `F_{p²}`, `j = 1728`, Montgomery coefficient 0, with `(A + 2)/4 = 1/2` stored beside it. | Det | `K`, `S`, `S5`: ideal to isogeny. Stage 4. | **Yes**, as a definition. No generated data. | none | `C` (**Ran**: `A = 0` at all three) | Supersingular: `p ≡ 3 (mod 4)`. |
| 4 | `basis_even` | `(P0, Q0)`, a basis of `E0[2^e]`, as `x(P0)`, `x(Q0)`, `x(P0 − Q0)`. The paper fixes "a basis" and no rule (Section 3.3); the rule is Algorithm B.1 of the 3.0 specification (1.4.1). | Conv | `K`, `S`, `S5`: the kernel of the chain and the points pushed through it (`dim2id2iso_ideal_to_isogeny_clapotis`). Stage 4. | **Yes** | secret (the matrix of the secret key is written in it); a signature does not depend on it (1.6). | `C`; `R` `E0_BASIS_PX`, `E0_BASIS_QX`, `E0_BASIS_PMQX`. Same points; the reference stores Montgomery-form machine words, `sqisign-rs` canonical little-endian bytes. Equal at `p500`, all three coordinates (**Ran**). | Both points on `E0`, of order exactly `2^e`; `[2^(e−1)]P0 ≠ [2^(e−1)]Q0`; `[2^(e−1)]Q0 = (0, 0)`; the third coordinate is of `P0 − Q0` and not of `P0 + Q0`; and the rule of 1.4.1 run again gives the same points. |
| 5 | `action_i` | The matrix modulo `2^e` of `i: (x, y) ↦ (−x, i·y)` on `(P0, Q0)`: column one is the coordinates of `i(P0)`, column two of `i(Q0)`. | Det, given item 4 and 1.4.2 | Only a function with no caller. | **No** as shipped data. The generator computes it: items 8 to 10 are tested against it. | none | `C`; `R` `ACTION_I`, same layout, equal at `p500` (**Ran**) | `M² = −1`; trace 0, determinant 1; and on points: `[M₁₁]P0 + [M₂₁]Q0 = i(P0)`, likewise for `Q0`. |
| 6 | `action_j` | The matrix of the Frobenius `π: (x, y) ↦ (x^p, y^p)`. | Det, as item 5 | Only a function with no caller. | **No**, as item 5 | none | `C`; `R` `ACTION_J`, equal at `p500` (**Ran**) | `M² = −p`; trace 0, determinant `p`; and on points. |
| 7 | `action_k` | The matrix of `k = i∘π`. | Det | Nothing. | **No** | none | `C`; `R` `ACTION_K`, equal at `p500` (**Ran**) | `M_k = M_i·M_j = −M_j·M_i`. |
| 8 | `action_gen2` | The matrix of the second generator of the maximal order (item 13), which is `i`. | Det | `K`, `S`, `S5`: `endomorphism_application_even_basis`. Stage 4. | **Yes** | none | `C`; `R` `ACTION_GEN2`, equal at `p500` (**Ran**) | Equal to item 5. |
| 9 | `action_gen3` | The matrix of `(i + π)/2`. The endomorphism `i + π` kills `E0[2]`, so it is twice another one. | Det | as item 8 | **Yes** | none | `C`; `R` `ACTION_GEN3`, equal at `p500` (**Ran**) | On points, with a half of each basis point (1.4.3). The identities `2M = M_i + M_j`, `M² = −(p+1)/4`, trace 0 and determinant `(p+1)/4` are necessary and **not sufficient**: 4 of the 16 matrices that differ from the true one only in the top bit of each entry pass all of them, for this item and for item 10, at each of the five sets (**Ran**); the test on points refuses the three wrong ones, and so does the comparison with an oracle. |
| 10 | `action_gen4` | The matrix of `(1 + k)/2`. | Det | as item 8 | **Yes** | none | `C`; `R` `ACTION_GEN4`, equal at `p500` (**Ran**) | On points, as item 9. Necessary: `2M = 1 + M_k`, trace 1, determinant `(p+1)/4`, `M_i·M₉ = M − 1`. |
| 11 | `NUM_ALTERNATE_STARTING_CURVES` | The number of further curves with known endomorphism ring: 0. | Tun | Nothing. | **No.** One starting curve; the reference's code is written for several and is built with one. | — | `C` | — |

#### `quaternion_data`

| # | Item | What it is | Class | Read by | Needed | Bytes | Oracle | Property |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 12 | `QUATALG_PINFTY` | The quaternion algebra over `Q` ramified at `p` and infinity, with `i² = −1`, `j² = −p`, `k = ij`. The stored datum is `p`. | Det | `K`, `S`, `S5`: every norm and product. Stage 2. | **Yes**; the datum is the prime, which `prism-field` defines. | none | `C` (**Ran**: the stored integer is `f·2^e − 1` at all three) | `p ≡ 3 (mod 4)`, so the algebra `(−1, −p)` is ramified exactly at `p` and infinity. |
| 13 | `EXTREMAL_ORDERS[0].order` | The maximal order `O0` with basis `1, i, (i+j)/2, (1+k)/2`, stored as a `4×4` integer matrix over the denominator 2. It is the endomorphism ring of `E0` under `i ↦ i`, `j ↦ π`. | Conv: the order is determined; the basis and its order are a choice. | `K`, `S`, `S5`: creating ideals, and writing an endomorphism in this basis before items 8 to 10 are applied. Stage 2. | **Yes** | secret (an ideal's coordinates), given the choice of basis; path otherwise | `C`, identical at the three sets (**Ran**); `R` `O0_BASIS_NUM`, `O0_DENOM`, written by hand there and not generated, same four elements, rows where the reference has columns. | Closed under multiplication, contains 1, and discriminant `p²` (reduced discriminant `p`), which makes it maximal. |
| 14 | `EXTREMAL_ORDERS[0].z`, `.t`, `.q` | `z = i`, `t = j`, `q = 1`: an element generating the quadratic suborder `Z[i]`, an element of norm `p` orthogonal to it, and the norm form's coefficient. | Det | `S` (`quat_represent_integer`) and a function with no caller. Not `K`, not `S5`. | **No**: our path does not represent integers (1.2). **Yes** if a later stage adds a step that does; that stage's section says so. | — | `C` | `z² = −q`, `t² = −p`, `zt = −tz`. |
| 15 | `NUM_ALTERNATE_EXTREMAL_ORDERS` | 0. | Tun | Tests only. | **No** | — | `C` | — |
| 16 | `CONNECTING_IDEALS`, `CONJUGATING_ELEMENTS` | For each alternate order, an ideal connecting it to `O0` and an element conjugating one to the other. With no alternate order: `O0` itself, and 1. | Det, for the trivial case | Nothing. | **No** | — | `C` (**Ran**: the unit ideal and 1 at all three) | — |
| 17 | `QUAT_small_primes_where_neg_q_square` | 2 and the first 100 primes that are 1 modulo 4: the small primes modulo which `−1` is a square (2, 5, 13, …, 1237). | Conv (1.4.4); its length is item 23 | `K`, `S`, `S5`: the sum-of-two-squares step of the norm equation (`ibz_cornacchia_extended`). Stage 3. | **Yes** | path, +draws | `C`, identical at the three sets, and equal element for element to the list as defined (**Ran**). Not prime-dependent. | Each entry is prime, and is 2 or 1 modulo 4; the list is increasing and misses no such prime below its last. |
| 18 | `QUAT_prods_of_bad_primes` | The product of the 102 primes that are 3 modulo 4 and below 1237: primes that cannot divide a sum of two coprime squares. | Conv | Nothing: the parameters the library builds pass 1 in its place. | **No** | — | `C` (**Ran**: it factors as exactly those 102 primes, the same integer at all three) | — |
| 19 | `QUAT_prime_cofactor` | A prime for the composite-norm sampling: the next prime after `2^⌈log₂ p⌉`. `2^251 + 65`, `2^383 + 369`, `2^505 + 51`. | Conv | Two benchmarks. Not `S`: see 1.2. | **No** | — | `C` (**Ran**: each is that next prime). `R` has `RI_COFACTOR_LE`, by another rule (Algorithm B.3) and for another protocol; not comparable. | Prime. |

#### `quaternion_constants`

| # | Item | Value | What it is | Class | Read by | Needed | Bytes | Oracle | Property |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 20 | `QUAT_primality_num_iter` | 32 | Rounds of the probabilistic primality test inside the quaternion layer. | Tun | `K`: the reduced ideal's norm. `K`, `S`, `S5`: the norm equation. `S`: `quat_represent_integer`. Stage 3. | **Yes**, as a decision: 1.5. | path; +draws through a changed verdict only | `C`. `R` has `PRIMALITY_NUM_ITER = ⌈λ/2⌉` (64, 96, 128), a different value at the shared prime (96). | — |
| 21 | `QUAT_repres_bound_input` | 20 / 21 / 21 | Bits by which a dimension-two length exceeds `log₂ p − log₂ u` in the fixed-degree isogeny. | Tun | Only a function with no caller. | **No** | — | `C`. No rule for 20, 21, 21 was looked for. | — |
| 22 | `QUAT_equiv_bound_coeff` | 64 | Half-width of the box of coefficients, on a reduced basis, searched for an equivalent ideal of prime norm; the search gives up after `(2·64 + 1)⁴` draws. | Tun | `K` only (`quat_lideal_prime_norm_reduced_equivalent`). Stage 3. | **Yes** | secret (which representative of the class the secret key holds), +draws | `C`. `R` has `EQUIV_BOUND_COEFF` by Algorithm B.2 (15, 19, 22), a different value at the shared prime (19). | — |
| 23 | `QUAT_good_prime_lists_length` | 101 | The length of item 17. `primes_per_order` of `prism_parameters.txt` is its source. | Tun | as item 17 | **Yes** | path, +draws | `C` | Equal to the length of item 17. |
| 24 | `QUAT_qlapoti_gen_bound_bits` | 17 | Bit length of the random coefficients from which a generator `α` of the ideal is drawn, on a reduced basis. | Tun | `K`, `S`, `S5`: `quat_qlapoti`. Stage 3. | **Yes** | path, +draws | `C`. `R` has none: its norm equation is the 3.0 specification's, a different algorithm whose bounds are computed from the ideal. | — |
| 25 | `QUAT_qlapoti_used_power_of_two` | 246 / 374 / 498 | The exponent of the power of two the two equivalent ideals' norms sum to, and the length of the translation's chain: `e − 2`. The length is one less when the norm equation returns its transformed case. The paper says only that the exponent is at most the 2-valuation of `p + 1` (Section 2.3, equation 2). The 3.0 specification sets `f − 2` in its notation "for technical reasons related to 2-dimensional isogeny computations" (its Section 4.6), and its chain takes kernel points of order `2^(length+2)` (Section 4.5.4). The pinned reference calls the translation's chain *without* that extra torsion, on a kernel of order exactly `2^(e−2)`. So why its value is `e − 2` and not larger is **not understood**: this row is stated in the reference's terms. The value is kept, because replay needs it; whether `e` would do, and what that does to the failure rate, is for stage 4's section, with ledger O29. | Conv | `K`, `S`, `S5`. Stages 3 and 4. | **Yes** | path, +draws | `C`; `R` `QLAPOTY_USED_POWER_OF_TWO = f − 2` in its notation (322, 498, 662), equal at `p500` (**Read**: both files). | Equal to `e − 2`. |
| 26 | `QUAT_qlapoti_iteration_bound` | 20,000 | Multiples `λ·α` of one generator tried before another is drawn. The search's outer loop ends after 20 times this many passes; the 20 is written in the function and not generated. That is a budget of passes and not a bound on the work: inside a pass, drawing a generator is a rejection loop with no cap (`quat_lideal_generator_small_coprime`), and so is the step to the next multiple coprime to the norm. | Tun | as item 24 | **Yes** | path, +draws | `C`. `R` none, as item 24. | — |
| 27, 28 | `FINDUV_box_size`, `FINDUV_cube_size` | 2, 624 / 3, 2400 / 3, 2400 | Parameters of a search the library declares and does not define. `cube = (2·box + 1)⁴ − 1`. | Tun | Nothing: there is no code. | **No** | — | `C` | — |

#### `torsion_constants` and `ec_params`

| # | Item | Value | What it is | Class | Read by | Needed | Bytes | Oracle | Property |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 29 | `TORSION_2POWER_BYTES` | 32 / 48 / 63 | `⌈(e + 1)/8⌉`: bytes of the integer `2^e`, used for a matrix entry in the reference's secret-key encoding. | Fmt | The reference's secret-key encoder. | **No**: the secret key's format is ours to specify (stage 5). An entry modulo `2^e` needs `⌈e/8⌉`. | — | `C`; `R` `TORSION_2POWER_BYTES`, 63 at `p500` | — |
| 30 | `TWO_TO_SECURITY_BITS` | `2^128` / `2^192` / `2^256` | A power of two named for a level. | Det | Nothing. | **No** | — | `C` | — |
| 31 | `TORSION_PLUS_2POWER` | `2^e` | The modulus of the matrices. | Det | `S`, `S5`: the response matrix. Stage 5. | **Has**: it is `2^E` of `ParameterSet::E` (`prism-field`). No generated data. | none | `C` | — |
| 32 | `TWO_POW_A` | `2^192` / `2^256` / `2^320` | `2^a` of salt-PRISM. | Det | Nothing. | **No** | — | `C` | — |
| 33 | `SEC_DEGREE` | `2^512 + 75` / `2^768 + 183` / `2^1024 + 643` | The prime norm of the ideal key generation first samples. Each is the next prime after a power of two (**Ran**). The paper asks for a norm that makes the key's distribution statistically close to uniform and gives no value (Section 3.3). | Tun | `K`. Stage 5. | **Yes** | public and secret: another prime gives other keys from the same randomness. +draws | `C`. `R` has `DEG_MIX_LE = next_prime(p·2^(2λ))`, the 3.0 specification's rule (Table 8): a different integer at the shared prime. | Prime, and greater than `p²`. |
| 34 | `COM_DEGREE` | equal to item 33 | A commitment degree: SQIsign's, which PRISM does not have. | Tun | Nothing. | **No** | — | `C` | — |
| 35 | `TORSION_EVEN_POWER` | 248 / 376 / 500 | `e`. | Det | everywhere | **Has**: `ParameterSet::E`. | none | `C`; `R` `TWO_ADIC_EXPONENT` | — |
| 36 | `A_POWER` | 192 / 256 / 320 | salt-PRISM's `a`: the challenge prime has `a` bits (the paper's Table 2). | Tun | `S` and the reference's verifier. Not `S5`. | **No**: our degree is `2^(2e−5) − s²` and has no `a` (VERIFIER_SPEC). | — | `C` | — |
| 37 | `SALT_SIZE` | 256 / 384 / 512 | The reference's salt size in bits, as a parameter of a function that returns 32 bits. | Tun | The reference's challenge hash. | **No**: our salt is `ParameterSet::SALT_LEN` bytes. | — | `C` | — |
| 38 | `p_cofactor_for_2f` and its bit length | 5 / 65 / 27 | `f = (p + 1)/2^e`. | Det | The deterministic basis of a codomain. | **Has**: `ParameterSet::COFACTOR`. | none | `C`; `R` `COFACTOR_BITLENGTH` | — |

#### `hd_splitting_transforms`, `encoded_sizes`, `fp_constants`, `prism_parameters.txt`

| # | Item | What it is | Class | Read by | Needed | Bytes | Oracle | Property |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 39 to 42 | `EVEN_INDEX`, `CHI_EVAL`, `FP2_CONSTANTS`, `SPLITTING_TRANSFORMS` | The ten even theta characteristics, the characters' signs, the five field constants `0, 1, i, −1, −i`, and the ten changes of theta basis that take a product structure to the one whose null point shows the splitting. | Det | The last step of every dimension-two chain, signing and verifying. Stage 4. | **Has**: `prism-isogeny/src/splitting.rs`, copied from the reference with its notice. | none | `C`, identical at the three sets (**Ran**: the lines of the three tables hash equal, and the five field constants decode to `0, 1, i, −1, −i` at each). | The engine's own tests of the splitting. |
| 43 | `NORMALIZATION_TRANSFORMS` | Six `4×4` matrices with entries in `0, ±1, ±i`: the identity, one permutation, two of Hadamard type and two with `i`. The signing chains multiply the splitting's change of basis by one of them, chosen by a random index (four random bytes a chain). The reference's header says this "selects a random Montgomery model of the codomain". That the six matrices correspond to the six models one to one was **not verified** here: this row is stated in the reference's terms. | Det | `K`, `S`, `S5`: `theta_chain_compute_and_eval_randomized`. Stage 4. | **Open**: needed if our signer keeps the random model, not needed if it does not (1.6). The engine does not have it: its port states that the six matrices are left out. | public (which coefficient names a key), +draws | `C`, identical at the three sets (**Ran**: the lines hash equal). Not prime-dependent. | **Open**: what the six must satisfy is to be stated by stage 4's section, once the row is understood. |
| 44 to 46 | `SECURITY_BITS`, `SQIsign_response_length`, `HASH_ITERATIONS` | A level (128 / 192 / 256), the length of SQIsign's response (126 / 192 / 253), and a hash iteration count (64 / 256 / 512). | Tun | The first by the reference's challenge hash; the other two by nothing. | **No**: a level is not coded here (ruled 2026-11-14), and the other two are SQIsign's. | — | `C` | — |
| 47, 48 | `FP_ENCODED_BYTES`, `FP2_ENCODED_BYTES` | Bytes of an element of `F_p` (32 / 48 / 64) and of `F_{p²}`. | Det | encoders | **Has**: `ParameterSet::FP_LEN`, `FP2_LEN`. | none | `C` | Pinned by tests in `prism-field`. |
| 49 to 51 | `EC_CURVE_ENCODED_BYTES`, `EC_POINT_ENCODED_BYTES`, `EC_BASIS_ENCODED_BYTES` | Sizes of encodings. | Fmt | Nothing. | **No** | — | `C` | — |
| 52 to 54 | `PUBLICKEY_BYTES`, `SECRETKEY_BYTES`, `SIGNATURE_BYTES` | The reference's key and signature sizes (65, 353, 260 at `p248`). | Fmt | The reference's encoders. | **No**: our key and signature lengths are in `prism-field`; our secret key's is stage 5's. | — | `C` | — |
| 55 to 58 | `NWORDS_FIELD`, `NWORDS_ORDER`, `BITS`, `LOG2P` | Machine words of a field element and of an integer modulo `2^e`, a rounded bit size, and `⌈log₂ log₂ p⌉`. | Fmt | The reference's arithmetic; `LOG2P` by nothing. | **No**: a layout of the reference's integers. Our integer widths are stage 2's. | — | `C`; `R` `NWORDS_ORDER`, `LOG2P` | — |
| 59 to 62 | `lvl`, `p`, `num_orders`, `primes_per_order` | The inputs of the reference's generator: a level, the prime, one order, 101 primes. | — | The absent scripts. | `p`: **Has**. `primes_per_order`: item 23. `lvl`, `num_orders`: **No**. | — | `C` | — |

Every row is the same at the three reference sets except in the value shown. A dash in a cell means the column does not apply to an item that is not needed.

#### What the paper says of each needed item

| Items | In ePrint 2026/443 |
| --- | --- |
| 3, 12, 13, and the maps of 5 and 6 | Section 2.1, Example 1: `E0: y² = x³ + x`, `ι: (x, y) ↦ (−x, √−1·y)` for "a fixed square root of −1", `π` the Frobenius, and `O0 = Z⟨1, i, (i+j)/2, (1+k)/2⟩ ≅ End(E0)` by `i ↦ ι`, `j ↦ π`. This is the definition used here. Which square root is "fixed" is not said; 1.4.2 says it. |
| 4 | Section 3.3: "fix a basis `P0, Q0`" of the starting curve's torsion. No rule. |
| 8, 9, 10 | Nothing: the action of the order on the basis is not mentioned. |
| 25 | Section 2.3, equation (2): the two norms sum to `2^e` with `e` at most the 2-valuation of `p + 1`. No value. |
| 17, 23, 24, 26 | Section 2.3 sketches the norm equation (a solution modulo the norm, a lattice reduction, Cornacchia) and cites its reference [17]. No bound, no list. |
| 20, 22 | Nothing. |
| 33 | Section 3.3: a random ideal "of random prime norm" such that the secret isogeny's distribution is statistically close to uniform, by its reference [1], Algorithm 4.1. No value; and here the norm is one fixed prime, not a random one. |
| 43 | Nothing. |

For every other item the paper says nothing, except item 36 (its Table 2 gives `a`) and item 37 (a salt of `2λ` bits, Section 7.2). The reference is the only source for the items the table marks "nothing".

### 1.4 Conventions, written out

Where a search defines a constant, the search is the definition. This is the lesson of ledger O21, where a value turned out to be defined by the coordinates one routine returns.

#### 1.4.1 The starting basis (item 4)

It is Algorithm B.1 (`GenerateBasisB0`) of the 3.0 specification. In our terms:

1. Order `F_{p²}` by `a + bi ≤ c + di` when `(a, b) ≤ (c, d)` lexicographically, `a, b, c, d` being integers in `[0, p)`.
2. For `m = 1, 2, 3, …` in that order, let `x = 1 + m·i`. If `x³ + x` is not a square, go on to the next `m`.
3. Let `y` be the root of `y² = x³ + x` that is the smaller of the two in the order of step 1, and `R = (x, y)`.
4. Let `n` be the order of `R`. If `2^e` does not divide `n`, go on. Otherwise replace `R` by `[n/2^e]R`: the multiplier is the odd part of the order *of this point*, which divides `f` and need not equal it.
5. The first `R` to arrive is `P0`. After it, the first `R` with `[2^(e−1)]R ≠ [2^(e−1)]P0` is `Q0`. Algorithm B.1 states this test as a Weil pairing (its line 20: `e(P0, R)` raised to `2^(e−1)` is `−1`). The two are the same test: both points have order exactly `2^e`, so that power of the pairing is the pairing of the two points of order 2 above them, which is `−1` when those differ and `1` when they are equal; and the pair is a basis exactly when they differ.
6. Then: if `[2^(e−1)]Q0 = (0, 0)`, keep both. Otherwise, if `[2^(e−1)]P0 = (0, 0)`, exchange them. Otherwise replace `Q0` by `Q0 + P0`.
7. The third stored coordinate is `x(P0 − Q0)`.

**Ran.** A plain implementation of steps 1 to 7 gives the reference's `x(P0)`, `x(Q0)` and `x(P0 − Q0)` exactly, at `p248`, `p376` and `p500`. The values of `m` used are 11 and 12, 10 and 12, 4 and 5. At `p500` the reference's three coordinates equal the three `sqisign-rs` has checked in.

At all five sets (**Ran**; the reviewer's own implementation gave the same on 2026-11-21, uncommitted):

| Set | Candidates `m` | Branch of step 6 | Compared with |
| --- | --- | --- | --- |
| `p248` | 11, 12 | third (`Q0 + P0`) | the reference: equal |
| `p324` | 2, 3 | third | `sqisign-rs`, three coordinates: equal |
| `p376` | 10, 12 | third | the reference: equal |
| `p500` | 4, 5 | third | the reference and `sqisign-rs`: equal |
| `p664` | 9, then 19 refused as dependent on `P0`, then 20 | second (the exchange) | `sqisign-rs`, three coordinates: equal |

- The second and third branches of step 6 are each taken at a set with an oracle. The first branch is taken at none of the five, and at no prime at all: see below.
- In step 4, every accepted point at every set has odd part of order equal to `f` (5, 3, 65, 27, 17). So "multiply by the odd part of the point's order" and "multiply by `f`" give the same points at all five, and no value here tells them apart. The rule adopted is the first, which is Algorithm B.1 as printed (its line 17).
- Step 3's choice of `y` moves the result through step 6's third branch and through step 7. It does at the four sets that take that branch.

**Step 6's first case is never taken.** This is a finding of stage 1b, made when the toy primes that were to reach it could not be found. **Derived**, and held by the script at 65 toy primes and at the five sets (**Ran**):

- For `x = 1 + m·i` the three factors of `x³ + x = (x − i)·x·(x + i)` are `1 + (m−1)i`, `1 + m·i`, `1 + (m+1)i`, and `1 + k·i` is a square in `F_{p²}` exactly when its norm `1 + k²` is a square modulo `p`. Write the three as a word of `S` (square) and `N`.
- A candidate passes step 2 when the product is a square, so the word has no `N` or two. It passes step 4 unless the point is twice another point, which is the word `SSS`. So a candidate that passes both has one of the words `SNN`, `NSN`, `NNS`.
- The `S` marks the point of order 2 under the candidate: `x − x(T)` is a square exactly when `T = [f·2^(e−1)]R`. (The value `x(R) − x(T)` modulo squares is the Tate pairing of `R` with `T`; on `E0(F_{p²})`, where the Frobenius is multiplication by `−p`, that pairing is the Weil pairing of `T` with `[(p+1)/2]R`.) So `NSN` is a point over `(0, 0)`, and the three words are the three points of order 2.
- `1 + 0²` is a square. So the non-squares among `1 + k²`, `k = 0, 1, 2, …`, come in runs that each start after a square. A run of length one gives no passing candidate at its own index. A run of length two or more gives `SNN` at its first index and `NNS` at its last, and nothing between them passes (`NNN` is not on the curve). `NSN` is a square between two non-squares.
- Hence the first point is `NSN` or `SNN`, never `NNS`, whose run's start comes before it. If it is `NSN`, it lies over `(0, 0)` and step 6 exchanges: the second case. If it is `SNN`, the next candidate that passes is the end of the same run, `NNS`: it is independent, it is the second point, and neither lies over `(0, 0)`: the third case. The first case needs a second point over `(0, 0)` after a first point that is not, and the run's end always comes first.

The rule is kept as published, with its first case, and the script's self-test calls that case directly. Algorithm B.1's lines 23 and 24 are dead for the search of its lines 6 to 22; that is a remark on the SQIsign specification and changes no value.

The toy primes `f·2^e − 1` of the self-test, and what each is there for (**Ran**, `--selftest`):

| `e`, `f` | `p` | Reaches |
| --- | --- | --- |
| 3, 1 | 7 | a candidate refused at step 2; third case |
| 3, 9 | 71 | a point refused at step 4 because `2^e` does not divide its order; third case |
| 3, 25 | 199 | the exchange |
| 4, 5 | 79 | a dependent candidate, then the exchange |
| 3, 33 | 263 | both points' orders miss a prime of `f` (odd part 11 of 33): the two readings of step 4 give different bases |
| 3, 91 | 727 | one order misses a prime of `f` (13 of 91); the readings differ |
| 4, 57 | 911 | a dependent candidate and an order that misses a prime of `f`, with the exchange |
| 11, 3 | 6143 | eleven levels of the discrete logarithm |

At each, the basis has the properties of item 4, the six matrices are tested on points, and every one of the fifteen top-bit variants of each matrix is refused on points. Where an order misses a prime of `f`, the test requires the two readings of step 4 to give different points, so the case is a real one. The sweep behind "65 toy primes" is every prime with `e` from 2 to 7 and odd `f` below 64: step 6 takes its second case at 17 and its third at 48.

#### 1.4.2 The action matrices (items 5 to 10)

- `i` is the element of `F_{p²}` with `i² = −1` that the field crate calls `ext_i`, and the endomorphism `i` is `(x, y) ↦ (−x, i·y)`. The other square root of `−1` gives the conjugate convention and other matrices.
- `j` is the `p`-power Frobenius, which on `F_{p²}` is conjugation of both coordinates. `k = i∘j`.
- The matrix `M` of an endomorphism `α` has `α(P0) = [M₁₁]P0 + [M₂₁]Q0` and `α(Q0) = [M₁₂]P0 + [M₂₂]Q0`, entries in `[0, 2^e)`. With this layout the matrix of `α∘β` is `M_α·M_β`.
- These are the conventions of `prism-isogeny/src/endomorphism.rs`, which the verifier's tests already use.

**Ran.** With these conventions, applied on points to the basis of 1.4.1, the reference's `action_i` and `action_j` are reproduced at all three sets, and at each `M_i² = −1`, `M_j² = −p`, `M_k = M_i M_j = −M_j M_i`, `gen2 = M_i`, `2·gen3 = M_i + M_j`, `2·gen4 = 1 + M_k`.

#### 1.4.3 The halved generators (items 9 and 10)

`(i + j)/2` is the endomorphism `β` with `2β = i + j`. On `E0[2^e]` the relation `2M = M_i + M_j` fixes `M` only modulo `2^(e−1)`. The top bit of each entry is fixed by the definition on points: for `R` in the basis, take any `H` with `2H = R`; then `β(R) = (i + j)(H)`, and the result does not depend on which half is taken because `i + j` kills `E0[2]`. A half of a point of order `2^e` is not in `E0(F_{p²})`; `sqisign-rs` takes it in `E0(F_{p⁴})`. Likewise for `(1 + k)/2`.

How the halves are taken (**Ran**). `x³ + x = x(x − i)(x + i)`. For `R = (x, y)` take square roots `r₁² = x`, `r₂² = x − i`, `r₃² = x + i` in `F_{p⁴}`; every element of `F_{p²}` has two there. Then `x_H = x + r₁r₂ + r₂r₃ + r₃r₁` and `y_H = ±(r₁ + r₂)(r₂ + r₃)(r₃ + r₁)`, with the sign for which `2H = R`, found by doubling. Changing the signs of `r₁` and `r₂` together gives another half. `F_{p⁴}` is `F_{p²}(t)` with `t²` the first non-square of the form `k + i`; the Frobenius there sends `t` to `t·(t²)^((p−1)/2)`. On every half used, the script checks `i² = −1`, `π² = −p` and `iπ = −πi` before it trusts the maps, and it computes each image from two different halves and requires them equal.

The row's finding stands as a requirement: a test of items 9 and 10 by matrix identities alone passes four candidates out of sixteen. The test is on points.

#### 1.4.4 The prime list (items 17 and 23)

The primes `ℓ` for which `−1` is a square modulo `ℓ`, in increasing order, starting with 2, the first 101 of them. It does not depend on `p`. The norm equation divides its target by these primes before it tests what is left for primality and writes it as a sum of two squares.

#### 1.4.5 The maximal order's basis (item 13)

`1, i, (i + j)/2, (1 + k)/2`, in that order. Items 8, 9 and 10 are the actions of the second, third and fourth. An endomorphism is applied to a torsion basis by writing it in this basis and combining the three matrices, so the basis and the matrices are one convention.

#### 1.4.6 "The next prime"

Items 19 and 33 are "the next prime after" a power of two: the smallest prime strictly greater. The test is Baillie–PSW as `VERIFIER_SPEC.md` specifies it (departure D15), written in the script: the primes below 100, the strong test to base 2, the strong Lucas test with Selfridge's discriminant among 256 candidates. Every integer between the power of two and the result is refused by a leg that never refuses a prime (a factor, a failed base-2 test, a failed Lucas test); a refusal for want of a discriminant is not a proof and would stop the script. So "next" is exact, given that the result is prime. The result is a probable prime and not a proven one: a proof joins ledger O19. The nightly asks sympy for the same five primes (`scripts/ci/check_signer_primes.py`).

One refusal is worth knowing. `2^512 + 1` is the Fermat number `F₉`, a strong pseudoprime to base 2 with no factor below 100. Only the Lucas leg refuses it (**Ran**: `--selftest` holds it).

### 1.5 Tunables, and what each one trades

A tunable that changes the distribution of what the signer outputs, or how often it fails, is part of ledger O4 (the distribution of responses and failures) and is marked so.

| # | Item | Value | What it trades | Meaning |
| --- | --- | --- | --- | --- |
| 20 | Primality rounds in the quaternion layer | 32 at all three | Time against the chance a composite is taken for a prime. | **Completeness.** A composite taken as the reduced secret ideal's norm, or as the cofactor in a sum of two squares, makes a later step fail or loop; ledger C9 is a failure in exactly that step. No effect on what a verifier accepts. A deterministic test (the verifier's Baillie–PSW) removes the parameter; that is a decision for stage 3. |
| 22 | Box for the equivalent prime-norm ideal | 64 at all three | A larger box finds a prime norm more surely and gives a larger one. | **Completeness** of key generation, and the size and representative of the secret ideal. The key's curve is the same for any representative. |
| 23 | Length of the prime list | 101 | More primes: more targets written as two squares, more divisions per target. | **Completeness and O4**: it changes which solution of the norm equation is found first. |
| 24 | Bits of the generator's coefficients | 17 at all three | Larger: more generators to try, larger numbers. | **Completeness and O4.** |
| 26 | Steps per generator | 20,000 at all three; a budget of 400,000 passes of the outer loop | The failure rate of the translation against the number of passes spent before giving up. It bounds neither the running time nor termination: two loops inside a pass have no cap (item 26's row). | **Completeness.** The reference prints "SHOULDNT HAPPEN" when the bound is reached. No estimate of the rate is in the reference or the paper. **Open.** |
| 33 | Norm of the first secret ideal | next prime after `2^512`, `2^768`, `2^1024` | Larger: the key's curve is closer to uniform; larger integers in key generation. | **Security.** It is the parameter of the claim that a key is statistically close to a uniform supersingular curve. |
| 43 | Random model of a codomain | one of six, four random bytes | — | **Open**, 1.6. |
| 11, 15 | Number of starting curves and orders | one | More: other signing strategies the reference does not build. | Not ours. |
| 21, 27, 28, 34, 36, 37, 44 to 46 | — | — | — | Not needed; not assessed. |

Two of these are not SQIsign 3.0's values. `sqisign-rs` uses `⌈λ/2⌉` rounds (64, 96, 128; the rule is in its generator, and was not found in the specification's text) where the reference has 32 at every set. The specification's Algorithm B.2 gives a box of 14, 17 and 20 at the reference's three primes and levels (**Ran**), and its Table 9 prints 15, 19 and 22 at its own; the reference has 64. The reference's values are constants across its sets and follow no formula in `e` and `f`. Why they were chosen is not written in the reference or the paper: **Open**.

### 1.6 What a signature and a key depend on

**Derived.** A signature is the canonical model of `E_sig` (D5), and on it the points `[s⁻¹]σ(P_vk)` and `[s⁻¹]σ(Q_vk)` with one sign rule, where `σ: E_vk → E_sig` is the response isogeny and `(P_vk, Q_vk)` the key's deterministic basis. None of that mentions `E0`'s basis, the order's basis, or how the norm equation was solved: those cancel between the secret key's matrix and the response's. So for a fixed key and a fixed response ideal the signature's bytes do not depend on any item marked "path" or "secret". They depend on the ideal, and so on every item marked "+draws".

**Open, and a finding.** The same is not established for the **key**. A key is the Montgomery coefficient the key-generation chain returns, and that chain picks the codomain's model by a random index (item 43). The verifier's tests of a key (nonsingular, not 0, a basis index found inside the search bound, and the points it names a basis of `E[2^e]`; `VERIFIER_SPEC.md`, and ledger O15 for what that asks of key generation) do not include a canonical model: of the coefficients of one curve, it accepts each that passes them. So, in the reference and in our exporter, which of up to six coefficients names a key is a random choice, and an item marked "path" can change a key's bytes through the model even when it cannot change the curve. Two ways to settle it, for the section of stage 4 or 5 to choose between and Rick to rule on:

- key generation emits the canonical model, by the rule the response curve already has (D5). Then item 43 is not needed, no "path" item reaches a byte, and one curve has one key;
- or the random model is kept, item 43 is a needed constant, and the specification says why a key's model should be random.

What the 3.0 specification does (**Read**): it does not randomize the model. A curve there is always held by the largest of its six Montgomery coefficients in the specification's order on `F_{p²}`: at the end of a chain `ThetaToMontgomery` (its Algorithm 4.126) chooses, among six theta structures it lists, the one whose coefficient is largest, and `NormalizeMontgomery` (Algorithm 4.96) does the same from a point of order 4. Its six theta structures (the null point, its swap, two sums and differences, two with `i`) look like the six matrices of item 43; that they are the same six was not verified. Our D5 keeps the least. A rule for keys here would be D5's, so that a key and a response curve are normalized one way.

What the choice does to replay against the reference (**Derived**). The reference's keys carry a random model, and a key's bytes enter the challenge. If our key generation emits the canonical model, a key's bytes differ from the reference's for the same curve, and so does every challenge under it. Then a replay of signing has to run on a key imported from the reference, model and all, and a replay of key generation compares the two keys up to the change of model and not byte for byte.

This section does not choose. **Not ruled** (R3, 2026-11-21): it is looked at again when the work reaches stage 3. Ledger O28.

### 1.7 Totals, and the constants with no oracle at `p324` and `p664`

Of the 62 items (**Derived** from the tables above; the count is by item number):

| Needed | Count | Items |
| --- | ---: | --- |
| **Yes** | 15 | 3, 4, 8, 9, 10, 12, 13, 17, 20, 22, 23, 24, 25, 26, 33 |
| **Open** | 1 | 43 |
| **Has** (the verifier's crates hold it) | 10 | 31, 35, 38, 39, 40, 41, 42, 47, 48, 60 |
| **No** | 36 | the rest |

Of the 36, 22 have no reader on any of the three paths: 7, 11, 15, 16, 18, 19, 27, 28, 30, 32, 34, 45, 46, 49, 50, 51, 58, 59 and 61, which nothing in the library reads, and 5, 6 and 21, whose only reader is a function with no caller. The other 14 are read by the reference's own encodings, its arithmetic layout or its salt-PRISM path, or repeat an item counted elsewhere (1, 2 and 62).

By class, all 62: **Det** 22, **Conv** 8, **Tun** 17, **Fmt** 11, and the 4 generator inputs (59 to 62), which have no class. By class, the 16 that are needed or open: **Det** 6 (3, 8, 9, 10, 12, 43), **Conv** 4 (4, 13, 17, 25), **Tun** 6 (20, 22, 23, 24, 26, 33).

Oracles for the 16 at the two primes the reference does not have:

| Item | At `p324` and `p664` | Rule proposed |
| --- | --- | --- |
| 3, 12 | none needed | `E0` and the algebra are the same definition at every prime. |
| 4 | `sqisign-rs`, three coordinates | 1.4.1. |
| 8, 9, 10 | `sqisign-rs` | 1.4.2 and 1.4.3. |
| 13, 17, 23, 43 | not prime-dependent | The reference's value, identical at its three sets, is the value at every set. |
| 25 | `sqisign-rs` | `e − 2`. It reproduces 246, 374 and 498. |
| 20 | **none** | 32 at every set, the reference's constant. **Ruled 2026-11-21 (R2):** a starting value, fixed at the three reference primes for replay, provisional at `p324` and `p664`, and gated on stage 3's measured failure rates (ledger O26, O29). Stage 3 may replace the test with a deterministic one. |
| 22 | **none** | 64 at every set, the reference's constant. **Ruled 2026-11-21 (R2):** a starting value, fixed at the three reference primes for replay, provisional at `p324` and `p664`, and gated on stage 3's measured failure rates (ledger O26, O29). |
| 24 | **none** | 17 at every set, the reference's constant. **Ruled 2026-11-21 (R2):** a starting value, fixed at the three reference primes for replay, provisional at `p324` and `p664`, and gated on stage 3's measured failure rates (ledger O26, O29). |
| 26 | **none** | 20,000 at every set, the reference's constant. **Ruled 2026-11-21 (R2):** a starting value, fixed at the three reference primes for replay, provisional at `p324` and `p664`, and gated on stage 3's measured failure rates (ledger O26, O29). |
| 33 | **none** | The next prime after `2^(2W)`, `W` being `ParameterSet::WIDTH_BITS` (`e` rounded up to a multiple of 128). **Ruled 2026-11-21 (R1).** It reproduces the three known values (`2W` = 512, 768, 1024) and gives `2^768 + 183` at `p324` and `2^1536 + 75` at `p664` (**Ran**). `W` is at least the bit length of `p` at all five sets (256, 384, 384, 512, 768 against 251, 326, 383, 505, 669), so the norm exceeds `p²`, which is at least `p·2^(2λ)` for every `λ ≤ ½·log₂ p`. No level is coded. The reference's own rule is not written anywhere. |

For items 20, 22, 24 and 26 the three known values are one constant, so "the same constant" reproduces them trivially and says nothing about whether the constant is right at a larger prime. Nothing in the reference, the paper or `sqisign-rs` says how any of the four should grow with `p`. That is a finding (ledger O26, O29), and the reason R2 makes the values provisional: the failure rate of the translation at `p664` under these bounds is **Open**, and the first evidence will be stage 3's own measurements.

`sqisign-rs` is an oracle for the starting basis, the six matrices and `e − 2`, and for nothing else here. Its other constants (`PRIMALITY_NUM_ITER`, `EQUIV_BOUND_COEFF`, `DEG_MIX_LE`, `RI_COFACTOR_LE`) are SQIsign 3.0's, by that specification's formulas, and at the shared prime `p500` each differs from the reference's.

### 1.8 The generator (stage 1b)

`scripts/signer_constants.py`: one Python script, standard library only, Python 3.8 or later. Field arithmetic over `F_{p²}` and `F_{p⁴}`, the order of 1.4.1 step 1, square roots, point arithmetic, discrete logarithms and the primality test are written in it, so that no library's convention can enter a value. Its output is `prism-vectors/data/signer/`, under the manifest `prism-vectors/data.sha256`; `prism_vectors::signer` reads it. A later signer crate holds its own copy of what it needs and a test holds that copy equal to this data; how it embeds it is that stage's decision.

What it writes:

- `<set>.txt` at each of the five sets. Shipped to the signer: items 4, 8, 9, 10, 25 and 33. Test intermediates: items 5, 6 and 7. And the record of how 1.4.1 ran: the candidates, the case of step 6, the odd part of each order.
- `shared.txt`: items 17 and 23 (the prime list) and items 20, 22, 24 and 26 (the four bounds), all shipped. The file marks the four bounds **provisional** at `p324` and `p664` (ruling R2; ledger O26, O29).
- Items 3, 12 and 13 are definitions and have no generated data.

The requirements stage 1a set, and how each is met (**Ran** unless marked):

1. **From the definitions.** The generating functions take a prime and return text. They read no file. `--selftest` holds that the generated bytes are the same with the data directory out of reach.
2. **Deterministic and reproducible.** `--check` regenerates every value and requires the committed files byte for byte, on x86_64 and on aarch64. No value passes through a float; Algorithm B.2 is evaluated in integers.
3. **Checked in with a digest manifest.** `prism-vectors/data.sha256`.
4. **Compared element for element with every oracle.** The values of the C reference and of `sqisign-rs` are committed as extracted, in `oracle_c_reference.txt` and `oracle_sqisign_rs.txt`, each with its repository, its commit and the SHA-256 of every file a value was read from. The comparison is a separate function over two tables and counts what it compares: 17 values a set against the C reference at `p248`, `p376`, `p500` (11 that depend on the prime, 6 that do not), 10 a set against `sqisign-rs` at `p324`, `p500`, `p664`, and the two oracles with each other at `p500`. A corrupted oracle value fails it (`--selftest`), and so does a halved generator's matrix with a wrong top bit that passes every matrix identity.
5. **Every defining property a committed test**: the bullets stage 1a listed, each at every set it applies to, in `--check`.
6. **Every branch executed.** 1.4.1 has the toy primes and the case no search reaches.
7. **A second implementation.** `prism-isogeny/tests/signer_constants.rs` reads the committed files and checks, with the engine's own field, points and `endomorphism` maps, at five sets: both points on `E0` and of order exactly `2^e`, independent, `[2^(e−1)]Q0 = (0, 0)`; the third coordinate is `x(P0 − Q0)`, and read as `x(P0 + Q0)` the matrices are not the actions; the matrices of `i`, of the Frobenius and of `k` are their actions; `2·gen3 = M_i + M_j` and `2·gen4 = 1 + M_k`. It cannot see the top bit of the entries of items 9 and 10: the engine has no arithmetic over `F_{p⁴}`. Those bits rest on the script's test on points and on the oracles, of which every set has at least one.

Where the oracle files come from, and what holds them:

- The C reference's values are re-extracted by `signer_constants.py --extract-c` from the pinned submodule. CI's `c-reference-pin` job has the submodule and runs it on every pull request: the committed file must be what the pinned tree gives.
- `sqisign-rs`'s values are re-extracted by `--extract-sqisign-rs <checkout>` from a checkout at the commit the file names. **This is not run automatically**: CI has no checkout of that repository. The manifest holds the committed file to its bytes, and the file's own header says what it was read from.

What stage 1a did not know, and what was found:

- Plain Python is enough for everything, the halved generators included. `--check` takes about half a minute on the development machine, where `p664` is the slowest set at about 12 seconds, and 70 to 95 seconds on a build VM shared with other runs.
- The halves are taken over `F_{p⁴}`, by the formulas of 1.4.3.
- The primality test of "the next prime" is 1.4.6.
- The data is in `prism-vectors`.

### 1.9 What the generated files do not contain

Constants the signer's path reads that are written in the reference's code and not in its generated files (**Read**). Each belongs to a later section; they are listed so that the inventory is not mistaken for the whole.

- The extra torsion above a chain's length: 2 (`HD_extra_torsion`). The verifier's chain is given it. The translation's chain is not (item 25), and takes the branch of the reference's chain that works without it.
- The splitting's expected structure. With extra torsion the reference requires the product structure to be the ninth of the ten (index 8), which is what the engine's port requires; without it, as in the translation, any of the ten is accepted. So the signer's chain differs from the engine's verification chain in three ways the engine does not have today: no extra torsion, any splitting index, and the random change of basis of item 43. This is for stage 4's section.
- The factor 20 in the norm equation's total bound (item 26).
- The parameters of the sum-of-two-squares step: the form's coefficient 1, and 1 in place of the bad-primes product (item 18).
- The maximal order's basis a second time, written by hand in `quat_lattice_O0_set`.
- In our exporter: 16 attempts at a response and 64 draws of an ideal per attempt.
