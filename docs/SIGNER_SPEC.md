# Signer specification

Status: begun 2026-11-21. Section 1 (the constants) is written; no signer code exists yet. This is a research specification, not an adopted standard, and it makes no security claim.

## What this document is

It is the specification our signer is held to. It is written one layer at a time, and each layer's section is written and reviewed before that layer's code (ruling G3 below). A section says what each object is in mathematical terms, what fixes it uniquely, and how a wrong value would be caught. The code is then built to the section, and a difference between the two is a defect in one of them.

It is the signing-side companion of [VERIFIER_SPEC.md](VERIFIER_SPEC.md). That document fixes the protocol: the five primes, the challenge `s`, the degree `q = 2^(2e−5) − s²`, the bytes of a key and of a signature, and every refusal. This document does not restate them. A signer conforms when the verifier of that specification accepts what it emits, at all five sets, and when each section's defining properties hold.

The paper is ePrint 2026/443 (`docs/papers/2026-443_PRISM_with _salt.pdf`). Our protocol is its Section 5 variant, the prime-degree response of Algorithm 2 (`GenIsogenyStrong`), with the departures the ledger lists (D1 to D15). For key generation and for the subroutines beneath signing the paper gives a sketch (Sections 2.3 and 3.3) and defers to its reference [1], the SQIsign round-2 submission document, by algorithm number (4.1, 3.10, 2.1). That document is not in this repository. The SQIsign specification of 2026-09-01 (`docs/papers/sqisign-20260901.pdf`, called "the 3.0 specification" below) is, and where it defines an object this document cites it by its own numbers.

### Labels

Every statement of fact carries one of five labels. **Ran**: executed, with the output committed; nothing in this section is, because this part commits no script and no output. **Read**: read in the pinned source or document at the place named, and checkable by reading it or by one `grep` or `diff` there. **Executed, not committed**: computed once on the development machine by a scratch script while this section was written; this part adds no script and no data (its brief forbids both), so each such statement is listed in 1.8 as a test that stage 1b must commit. **Derived**: argued, not machine-checked. **Open**: not established.

### Rulings (Rick, 2026-11-21)

- **G1.** The signer is new layer crates beneath the front crate, one per mathematical layer, re-exported under `hazmat`. The front crate gains `SecretKey<S>`, key generation and `sign`. `prism-signer` becomes the reference oracle at cutover. Per-set signer constants are not in `prism-field`.
- **G2.** Each new crate has the library's standards from its first commit: no `unsafe`, documentation required, its surface decided, the no-upward-edge check, and a place in the nightly's mutation job.
- **G3.** Specification first. Each stage's pull request opens with that layer's section of this document, reviewed before code is written.
- **G4.** Constant-time hardening is inside the beta gate. It is the last stage. There is no timeline for it or for anything else here.
- **G5.** The experiments on the second basis point (ledger O21) are their own part, directly after key generation and before signing.
- **G6.** Stage 1 opens with this section.

Standing from Round 0 (2026-11-14): we are not transcribing the C; other code bases are oracles for values and not sources of structure; five sets, one algorithm.

### The layers, in the order they are built

| Stage | Layer | Section here |
| --- | --- | --- |
| 1 | The constants: what each is (1a, this section) and the generator that produces them (1b) | 1 |
| 2 | Integers and the quaternion algebra: the algebra, the maximal order, lattices, ideals, lattice reduction | to be written |
| 3 | Norm equations and sampling: Cornacchia, the norm equation of the ideal-to-isogeny translation, random ideals of a given norm, primality | to be written |
| 4 | Ideal to isogeny: the starting curve, its basis, the action of its endomorphisms, the dimension-two chain from `E0 × E0` | to be written |
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
| 1, 2 | `BASIS_E0_PX`, `BASIS_E0_QX` | `x(P0)` and `x(Q0)` of the starting basis (item 4), a second copy. | Conv | The deterministic-basis routine, only when the curve it is given is `E0` (`ec_basis_E0_2f`). | **No**, as separate data: equal to item 4 at all three sets (**Executed, not committed**). A key is never `E0` (the verifier refuses `A = 0`, D10). A response curve is `E0` only with negligible probability; what the signer does then is for stage 5's section. | none | as item 4 | as item 4 |
| 3 | `curve` | The starting curve `E0: y² = x³ + x` over `F_{p²}`, `j = 1728`, Montgomery coefficient 0, with `(A + 2)/4 = 1/2` stored beside it. | Det | `K`, `S`, `S5`: ideal to isogeny. Stage 4. | **Yes**, as a definition. No generated data. | none | `C` (**Executed, not committed**: `A = 0` at all three) | Supersingular: `p ≡ 3 (mod 4)`. |
| 4 | `basis_even` | `(P0, Q0)`, a basis of `E0[2^e]`, as `x(P0)`, `x(Q0)`, `x(P0 − Q0)`. The paper fixes "a basis" and no rule (Section 3.3); the rule is Algorithm B.1 of the 3.0 specification (1.4.1). | Conv | `K`, `S`, `S5`: the kernel of the chain and the points pushed through it (`dim2id2iso_ideal_to_isogeny_clapotis`). Stage 4. | **Yes** | secret (the matrix of the secret key is written in it); a signature does not depend on it (1.6). | `C`; `R` `E0_BASIS_PX`, `E0_BASIS_QX`, `E0_BASIS_PMQX`. Same points; the reference stores Montgomery-form machine words, `sqisign-rs` canonical little-endian bytes. Equal at `p500`, all three coordinates (**Executed, not committed**). | Both points on `E0`, of order exactly `2^e`; `[2^(e−1)]P0 ≠ [2^(e−1)]Q0`; `[2^(e−1)]Q0 = (0, 0)`; the third coordinate is of `P0 − Q0` and not of `P0 + Q0`; and the rule of 1.4.1 run again gives the same points. |
| 5 | `action_i` | The matrix modulo `2^e` of `i: (x, y) ↦ (−x, i·y)` on `(P0, Q0)`: column one is the coordinates of `i(P0)`, column two of `i(Q0)`. | Det, given item 4 and 1.4.2 | Only a function with no caller. | **No** as shipped data. The generator computes it: items 8 to 10 are tested against it. | none | `C`; `R` `ACTION_I`, same layout, equal at `p500` (**Executed, not committed**) | `M² = −1`; trace 0, determinant 1; and on points: `[M₁₁]P0 + [M₂₁]Q0 = i(P0)`, likewise for `Q0`. |
| 6 | `action_j` | The matrix of the Frobenius `π: (x, y) ↦ (x^p, y^p)`. | Det, as item 5 | Only a function with no caller. | **No**, as item 5 | none | `C`; `R` `ACTION_J`, equal at `p500` (**Executed, not committed**) | `M² = −p`; trace 0, determinant `p`; and on points. |
| 7 | `action_k` | The matrix of `k = i∘π`. | Det | Nothing. | **No** | none | `C`; `R` `ACTION_K`, equal at `p500` (**Executed, not committed**) | `M_k = M_i·M_j = −M_j·M_i`. |
| 8 | `action_gen2` | The matrix of the second generator of the maximal order (item 13), which is `i`. | Det | `K`, `S`, `S5`: `endomorphism_application_even_basis`. Stage 4. | **Yes** | none | `C`; `R` `ACTION_GEN2`, equal at `p500` (**Executed, not committed**) | Equal to item 5. |
| 9 | `action_gen3` | The matrix of `(i + π)/2`. The endomorphism `i + π` kills `E0[2]`, so it is twice another one. | Det | as item 8 | **Yes** | none | `C`; `R` `ACTION_GEN3`, equal at `p500` (**Executed, not committed**) | On points, with a half of each basis point (1.4.3). The identities `2M = M_i + M_j`, `M² = −(p+1)/4`, trace 0 and determinant `(p+1)/4` are necessary and **not sufficient**: 4 of the 16 matrices that differ from the true one only in the top bit of each entry pass all of them, at each of the three sets (**Executed, not committed**). |
| 10 | `action_gen4` | The matrix of `(1 + k)/2`. | Det | as item 8 | **Yes** | none | `C`; `R` `ACTION_GEN4`, equal at `p500` (**Executed, not committed**) | On points, as item 9. Necessary: `2M = 1 + M_k`, trace 1, determinant `(p+1)/4`, `M_i·M₉ = M − 1`. |
| 11 | `NUM_ALTERNATE_STARTING_CURVES` | The number of further curves with known endomorphism ring: 0. | Tun | Nothing. | **No.** One starting curve; the reference's code is written for several and is built with one. | — | `C` | — |

#### `quaternion_data`

| # | Item | What it is | Class | Read by | Needed | Bytes | Oracle | Property |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 12 | `QUATALG_PINFTY` | The quaternion algebra over `Q` ramified at `p` and infinity, with `i² = −1`, `j² = −p`, `k = ij`. The stored datum is `p`. | Det | `K`, `S`, `S5`: every norm and product. Stage 2. | **Yes**; the datum is the prime, which `prism-field` defines. | none | `C` (**Executed, not committed**: the stored integer is `f·2^e − 1` at all three) | `p ≡ 3 (mod 4)`, so the algebra `(−1, −p)` is ramified exactly at `p` and infinity. |
| 13 | `EXTREMAL_ORDERS[0].order` | The maximal order `O0` with basis `1, i, (i+j)/2, (1+k)/2`, stored as a `4×4` integer matrix over the denominator 2. It is the endomorphism ring of `E0` under `i ↦ i`, `j ↦ π`. | Conv: the order is determined; the basis and its order are a choice. | `K`, `S`, `S5`: creating ideals, and writing an endomorphism in this basis before items 8 to 10 are applied. Stage 2. | **Yes** | secret (an ideal's coordinates), given the choice of basis; path otherwise | `C`, identical at the three sets (**Executed, not committed**); `R` `O0_BASIS_NUM`, `O0_DENOM`, written by hand there and not generated, same four elements, rows where the reference has columns. | Closed under multiplication, contains 1, and discriminant `p²` (reduced discriminant `p`), which makes it maximal. |
| 14 | `EXTREMAL_ORDERS[0].z`, `.t`, `.q` | `z = i`, `t = j`, `q = 1`: an element generating the quadratic suborder `Z[i]`, an element of norm `p` orthogonal to it, and the norm form's coefficient. | Det | `S` (`quat_represent_integer`) and a function with no caller. Not `K`, not `S5`. | **No**: our path does not represent integers (1.2). **Yes** if a later stage adds a step that does; that stage's section says so. | — | `C` | `z² = −q`, `t² = −p`, `zt = −tz`. |
| 15 | `NUM_ALTERNATE_EXTREMAL_ORDERS` | 0. | Tun | Tests only. | **No** | — | `C` | — |
| 16 | `CONNECTING_IDEALS`, `CONJUGATING_ELEMENTS` | For each alternate order, an ideal connecting it to `O0` and an element conjugating one to the other. With no alternate order: `O0` itself, and 1. | Det, for the trivial case | Nothing. | **No** | — | `C` (**Executed, not committed**: the unit ideal and 1 at all three) | — |
| 17 | `QUAT_small_primes_where_neg_q_square` | 2 and the first 100 primes that are 1 modulo 4: the small primes modulo which `−1` is a square (2, 5, 13, …, 1237). | Conv (1.4.4); its length is item 23 | `K`, `S`, `S5`: the sum-of-two-squares step of the norm equation (`ibz_cornacchia_extended`). Stage 3. | **Yes** | path, +draws | `C`, identical at the three sets, and equal element for element to the list as defined (**Executed, not committed**). Not prime-dependent. | Each entry is prime, and is 2 or 1 modulo 4; the list is increasing and misses no such prime below its last. |
| 18 | `QUAT_prods_of_bad_primes` | The product of the 102 primes that are 3 modulo 4 and below 1237: primes that cannot divide a sum of two coprime squares. | Conv | Nothing: the parameters the library builds pass 1 in its place. | **No** | — | `C` (**Executed, not committed**: it factors as exactly those 102 primes, the same integer at all three) | — |
| 19 | `QUAT_prime_cofactor` | A prime for the composite-norm sampling: the next prime after `2^⌈log₂ p⌉`. `2^251 + 65`, `2^383 + 369`, `2^505 + 51`. | Conv | Two benchmarks. Not `S`: see 1.2. | **No** | — | `C` (**Executed, not committed**: each is that next prime). `R` has `RI_COFACTOR_LE`, by another rule (Algorithm B.3) and for another protocol; not comparable. | Prime. |

#### `quaternion_constants`

| # | Item | Value | What it is | Class | Read by | Needed | Bytes | Oracle | Property |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 20 | `QUAT_primality_num_iter` | 32 | Rounds of the probabilistic primality test inside the quaternion layer. | Tun | `K`: the reduced ideal's norm. `K`, `S`, `S5`: the norm equation. `S`: `quat_represent_integer`. Stage 3. | **Yes**, as a decision: 1.5. | path; +draws through a changed verdict only | `C`. `R` has `PRIMALITY_NUM_ITER = ⌈λ/2⌉` (64, 96, 128), a different value at the shared prime (96). | — |
| 21 | `QUAT_repres_bound_input` | 20 / 21 / 21 | Bits by which a dimension-two length exceeds `log₂ p − log₂ u` in the fixed-degree isogeny. | Tun | Only a function with no caller. | **No** | — | `C`. No rule for 20, 21, 21 was looked for. | — |
| 22 | `QUAT_equiv_bound_coeff` | 64 | Half-width of the box of coefficients, on a reduced basis, searched for an equivalent ideal of prime norm; the search gives up after `(2·64 + 1)⁴` draws. | Tun | `K` only (`quat_lideal_prime_norm_reduced_equivalent`). Stage 3. | **Yes** | secret (which representative of the class the secret key holds), +draws | `C`. `R` has `EQUIV_BOUND_COEFF` by Algorithm B.2 (15, 19, 22), a different value at the shared prime (19). | — |
| 23 | `QUAT_good_prime_lists_length` | 101 | The length of item 17. `primes_per_order` of `prism_parameters.txt` is its source. | Tun | as item 17 | **Yes** | path, +draws | `C` | Equal to the length of item 17. |
| 24 | `QUAT_qlapoti_gen_bound_bits` | 17 | Bit length of the random coefficients from which a generator `α` of the ideal is drawn, on a reduced basis. | Tun | `K`, `S`, `S5`: `quat_qlapoti`. Stage 3. | **Yes** | path, +draws | `C`. `R` has none: its norm equation is the 3.0 specification's, a different algorithm whose bounds are computed from the ideal. | — |
| 25 | `QUAT_qlapoti_used_power_of_two` | 246 / 374 / 498 | The exponent of the power of two the two equivalent ideals' norms sum to, and the length of the translation's chain: `e − 2`. The length is one less when the norm equation returns its transformed case. The paper says only that the exponent is at most the 2-valuation of `p + 1` (Section 2.3, equation 2). The 3.0 specification sets `f − 2` in its notation "for technical reasons related to 2-dimensional isogeny computations" (its Section 4.6), and its chain takes kernel points of order `2^(length+2)` (Section 4.5.4). The pinned reference calls the translation's chain *without* that extra torsion, on a kernel of order exactly `2^(e−2)`. So why its value is `e − 2` and not larger is **not understood**: this row is stated in the reference's terms. | Conv | `K`, `S`, `S5`. Stages 3 and 4. | **Yes** | path, +draws | `C`; `R` `QLAPOTY_USED_POWER_OF_TWO = f − 2` in its notation (322, 498, 662), equal at `p500` (**Read**: both files). | Equal to `e − 2`. |
| 26 | `QUAT_qlapoti_iteration_bound` | 20,000 | Multiples `λ·α` of one generator tried before another is drawn. The whole search stops after 20 times this many steps; the 20 is written in the function and not generated. | Tun | as item 24 | **Yes** | path, +draws | `C`. `R` none, as item 24. | — |
| 27, 28 | `FINDUV_box_size`, `FINDUV_cube_size` | 2, 624 / 3, 2400 / 3, 2400 | Parameters of a search the library declares and does not define. `cube = (2·box + 1)⁴ − 1`. | Tun | Nothing: there is no code. | **No** | — | `C` | — |

#### `torsion_constants` and `ec_params`

| # | Item | Value | What it is | Class | Read by | Needed | Bytes | Oracle | Property |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 29 | `TORSION_2POWER_BYTES` | 32 / 48 / 63 | `⌈(e + 1)/8⌉`: bytes of the integer `2^e`, used for a matrix entry in the reference's secret-key encoding. | Fmt | The reference's secret-key encoder. | **No**: the secret key's format is ours to specify (stage 5). An entry modulo `2^e` needs `⌈e/8⌉`. | — | `C`; `R` `TORSION_2POWER_BYTES`, 63 at `p500` | — |
| 30 | `TWO_TO_SECURITY_BITS` | `2^128` / `2^192` / `2^256` | A power of two named for a level. | Det | Nothing. | **No** | — | `C` | — |
| 31 | `TORSION_PLUS_2POWER` | `2^e` | The modulus of the matrices. | Det | `S`, `S5`: the response matrix. Stage 5. | **Has**: it is `2^E` of `ParameterSet::E` (`prism-field`). No generated data. | none | `C` | — |
| 32 | `TWO_POW_A` | `2^192` / `2^256` / `2^320` | `2^a` of salt-PRISM. | Det | Nothing. | **No** | — | `C` | — |
| 33 | `SEC_DEGREE` | `2^512 + 75` / `2^768 + 183` / `2^1024 + 643` | The prime norm of the ideal key generation first samples. Each is the next prime after a power of two (**Executed, not committed**). The paper asks for a norm that makes the key's distribution statistically close to uniform and gives no value (Section 3.3). | Tun | `K`. Stage 5. | **Yes** | public and secret: another prime gives other keys from the same randomness. +draws | `C`. `R` has `DEG_MIX_LE = next_prime(p·2^(2λ))`, the 3.0 specification's rule (Table 8): a different integer at the shared prime. | Prime; at least `p·2^(2λ)` for the `λ` one claims. |
| 34 | `COM_DEGREE` | equal to item 33 | A commitment degree: SQIsign's, which PRISM does not have. | Tun | Nothing. | **No** | — | `C` | — |
| 35 | `TORSION_EVEN_POWER` | 248 / 376 / 500 | `e`. | Det | everywhere | **Has**: `ParameterSet::E`. | none | `C`; `R` `TWO_ADIC_EXPONENT` | — |
| 36 | `A_POWER` | 192 / 256 / 320 | salt-PRISM's `a`: the challenge prime has `a` bits (the paper's Table 2). | Tun | `S` and the reference's verifier. Not `S5`. | **No**: our degree is `2^(2e−5) − s²` and has no `a` (VERIFIER_SPEC). | — | `C` | — |
| 37 | `SALT_SIZE` | 256 / 384 / 512 | The reference's salt size in bits, as a parameter of a function that returns 32 bits. | Tun | The reference's challenge hash. | **No**: our salt is `ParameterSet::SALT_LEN` bytes. | — | `C` | — |
| 38 | `p_cofactor_for_2f` and its bit length | 5 / 65 / 27 | `f = (p + 1)/2^e`. | Det | The deterministic basis of a codomain. | **Has**: `ParameterSet::COFACTOR`. | none | `C`; `R` `COFACTOR_BITLENGTH` | — |

#### `hd_splitting_transforms`, `encoded_sizes`, `fp_constants`, `prism_parameters.txt`

| # | Item | What it is | Class | Read by | Needed | Bytes | Oracle | Property |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 39 to 42 | `EVEN_INDEX`, `CHI_EVAL`, `FP2_CONSTANTS`, `SPLITTING_TRANSFORMS` | The ten even theta characteristics, the characters' signs, the five field constants `0, 1, i, −1, −i`, and the ten changes of theta basis that take a product structure to the one whose null point shows the splitting. | Det | The last step of every dimension-two chain, signing and verifying. Stage 4. | **Has**: `prism-isogeny/src/splitting.rs`, copied from the reference with its notice. | none | `C`, identical at the three sets (**Executed, not committed**: the lines of the three tables hash equal, and the five field constants decode to `0, 1, i, −1, −i` at each). | The engine's own tests of the splitting. |
| 43 | `NORMALIZATION_TRANSFORMS` | Six `4×4` matrices with entries in `0, ±1, ±i`: the identity, one permutation, two of Hadamard type and two with `i`. The signing chains multiply the splitting's change of basis by one of them, chosen by a random index (four random bytes a chain). The reference's header says this "selects a random Montgomery model of the codomain". That the six matrices correspond to the six models one to one was **not verified** here: this row is stated in the reference's terms. | Det | `K`, `S`, `S5`: `theta_chain_compute_and_eval_randomized`. Stage 4. | **Open**: needed if our signer keeps the random model, not needed if it does not (1.6). The engine does not have it: its port states that the six matrices are left out. | public (which coefficient names a key), +draws | `C`, identical at the three sets (**Executed, not committed**: the lines hash equal). Not prime-dependent. | **Open**: what the six must satisfy is to be stated by stage 4's section, once the row is understood. |
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
5. The first `R` to arrive is `P0`. After it, the first `R` with `[2^(e−1)]R ≠ [2^(e−1)]P0` is `Q0`.
6. Then: if `[2^(e−1)]Q0 = (0, 0)`, keep both. Otherwise, if `[2^(e−1)]P0 = (0, 0)`, exchange them. Otherwise replace `Q0` by `Q0 + P0`.
7. The third stored coordinate is `x(P0 − Q0)`.

**Executed, not committed.** A plain implementation of steps 1 to 7 gives the reference's `x(P0)`, `x(Q0)` and `x(P0 − Q0)` exactly, at `p248`, `p376` and `p500`. The values of `m` used are 11 and 12, 10 and 12, 4 and 5. At `p500` the reference's three coordinates equal the three `sqisign-rs` has checked in.

What the three known values do **not** exercise:

- Step 6 took its third branch (`Q0 + P0`) at all three. The first two branches are untested by any reference value.
- In step 4 every accepted point had odd part of order equal to `f` (5, 65, 27). So the three values cannot tell the rule "multiply by the odd part of the point's order" from the rule "multiply by `f`", which differ when a point's order misses a prime of `f`.
- Step 3's choice of `y` moves the result only through step 6's third branch and through step 7. It did at all three.

The rule above is the one adopted. Whether `p324` and `p664` reach the untested branches is **Open** (ledger O27) until stage 1b runs it there and compares with `sqisign-rs`.

#### 1.4.2 The action matrices (items 5 to 10)

- `i` is the element of `F_{p²}` with `i² = −1` that the field crate calls `ext_i`, and the endomorphism `i` is `(x, y) ↦ (−x, i·y)`. The other square root of `−1` gives the conjugate convention and other matrices.
- `j` is the `p`-power Frobenius, which on `F_{p²}` is conjugation of both coordinates. `k = i∘j`.
- The matrix `M` of an endomorphism `α` has `α(P0) = [M₁₁]P0 + [M₂₁]Q0` and `α(Q0) = [M₁₂]P0 + [M₂₂]Q0`, entries in `[0, 2^e)`. With this layout the matrix of `α∘β` is `M_α·M_β`.
- These are the conventions of `prism-isogeny/src/endomorphism.rs`, which the verifier's tests already use.

**Executed, not committed.** With these conventions, applied on points to the basis of 1.4.1, the reference's `action_i` and `action_j` are reproduced at all three sets, and at each `M_i² = −1`, `M_j² = −p`, `M_k = M_i M_j = −M_j M_i`, `gen2 = M_i`, `2·gen3 = M_i + M_j`, `2·gen4 = 1 + M_k`.

#### 1.4.3 The halved generators (items 9 and 10)

`(i + j)/2` is the endomorphism `β` with `2β = i + j`. On `E0[2^e]` the relation `2M = M_i + M_j` fixes `M` only modulo `2^(e−1)`. The top bit of each entry is fixed by the definition on points: for `R` in the basis, take any `H` with `2H = R`; then `β(R) = (i + j)(H)`, and the result does not depend on which half is taken because `i + j` kills `E0[2]`. A half of a point of order `2^e` is not in `E0(F_{p²})`; `sqisign-rs` takes it in `E0(F_{p⁴})`. Likewise for `(1 + k)/2`.

The row's finding stands as a requirement: a test of items 9 and 10 by matrix identities alone passes four candidates out of sixteen. The test is on points.

#### 1.4.4 The prime list (items 17 and 23)

The primes `ℓ` for which `−1` is a square modulo `ℓ`, in increasing order, starting with 2, the first 101 of them. It does not depend on `p`. The norm equation divides its target by these primes before it tests what is left for primality and writes it as a sum of two squares.

#### 1.4.5 The maximal order's basis (item 13)

`1, i, (i + j)/2, (1 + k)/2`, in that order. Items 8, 9 and 10 are the actions of the second, third and fourth. An endomorphism is applied to a torsion basis by writing it in this basis and combining the three matrices, so the basis and the matrices are one convention.

#### 1.4.6 "The next prime"

Items 19 and 33 are "the next prime after" a power of two: the smallest prime strictly greater. The reference's own test of primality there is probabilistic, and the rule is only reproducible with a stated test (1.8).

### 1.5 Tunables, and what each one trades

A tunable that changes the distribution of what the signer outputs, or how often it fails, is part of ledger O4 (the distribution of responses and failures) and is marked so.

| # | Item | Value | What it trades | Meaning |
| --- | --- | --- | --- | --- |
| 20 | Primality rounds in the quaternion layer | 32 at all three | Time against the chance a composite is taken for a prime. | **Completeness.** A composite taken as the reduced secret ideal's norm, or as the cofactor in a sum of two squares, makes a later step fail or loop; ledger C9 is a failure in exactly that step. No effect on what a verifier accepts. A deterministic test (the verifier's Baillie–PSW) removes the parameter; that is a decision for stage 3. |
| 22 | Box for the equivalent prime-norm ideal | 64 at all three | A larger box finds a prime norm more surely and gives a larger one. | **Completeness** of key generation, and the size and representative of the secret ideal. The key's curve is the same for any representative. |
| 23 | Length of the prime list | 101 | More primes: more targets written as two squares, more divisions per target. | **Completeness and O4**: it changes which solution of the norm equation is found first. |
| 24 | Bits of the generator's coefficients | 17 at all three | Larger: more generators to try, larger numbers. | **Completeness and O4.** |
| 26 | Steps per generator | 20,000 at all three; 400,000 in all | The failure rate of the translation against its worst-case time. | **Completeness.** The reference prints "SHOULDNT HAPPEN" when the bound is reached. No estimate of the rate is in the reference or the paper. **Open.** |
| 33 | Norm of the first secret ideal | next prime after `2^512`, `2^768`, `2^1024` | Larger: the key's curve is closer to uniform; larger integers in key generation. | **Security.** It is the parameter of the claim that a key is statistically close to a uniform supersingular curve. |
| 43 | Random model of a codomain | one of six, four random bytes | — | **Open**, 1.6. |
| 11, 15 | Number of starting curves and orders | one | More: other signing strategies the reference does not build. | Not ours. |
| 21, 27, 28, 34, 36, 37, 44 to 46 | — | — | — | Not needed; not assessed. |

Two of these are not SQIsign 3.0's values. `sqisign-rs` uses `⌈λ/2⌉` rounds (64, 96, 128; the rule is in its generator, and was not found in the specification's text) where the reference has 32 at every set. The specification's Algorithm B.2 gives a box of 14, 17 and 20 at the reference's three primes and levels (**Executed, not committed**), and its Table 9 prints 15, 19 and 22 at its own; the reference has 64. The reference's values are constants across its sets and follow no formula in `e` and `f`. Why they were chosen is not written in the reference or the paper: **Open**.

### 1.6 What a signature and a key depend on

**Derived.** A signature is the canonical model of `E_sig` (D5), and on it the points `[s⁻¹]σ(P_vk)` and `[s⁻¹]σ(Q_vk)` with one sign rule, where `σ: E_vk → E_sig` is the response isogeny and `(P_vk, Q_vk)` the key's deterministic basis. None of that mentions `E0`'s basis, the order's basis, or how the norm equation was solved: those cancel between the secret key's matrix and the response's. So for a fixed key and a fixed response ideal the signature's bytes do not depend on any item marked "path" or "secret". They depend on the ideal, and so on every item marked "+draws".

**Open, and a finding.** The same is not established for the **key**. A key is the Montgomery coefficient the key-generation chain returns, and that chain picks the codomain's model by a random index (item 43). The verifier accepts any nonsingular coefficient other than 0 as a key and has no canonical model for one. So, in the reference and in our exporter, which of up to six coefficients names a key is a random choice, and an item marked "path" can change a key's bytes through the model even when it cannot change the curve. Two ways to settle it, for the section of stage 4 or 5 to choose between and Rick to rule on:

- key generation emits the canonical model, by the rule the response curve already has (D5). Then item 43 is not needed, no "path" item reaches a byte, and one curve has one key;
- or the random model is kept, item 43 is a needed constant, and the specification says why a key's model should be random.

This section does not choose. Ledger O28.

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
| 20 | **none** | 32 at every set, the reference's constant, unless stage 3 replaces the test with a deterministic one. A design choice; no formula is involved. |
| 22 | **none** | 64 at every set, the reference's constant. A design choice. |
| 24 | **none** | 17 at every set, the reference's constant. A design choice. |
| 26 | **none** | 20,000 at every set, the reference's constant. A design choice. |
| 33 | **none** | The next prime after `2^(2W)`, `W` being `ParameterSet::WIDTH_BITS` (`e` rounded up to a multiple of 128). It reproduces the three known values (`2W` = 512, 768, 1024) and gives `2^768 + 183` at `p324` and `2^1536 + 75` at `p664` (**Executed, not committed**). It is at least `p·2^(2λ)` at each set for the level each prime was or is assessed at. The reference's own rule is not written anywhere; "the next prime after `2^(4λ)`" with the paper's levels gives the same three values and cannot be used here, because a level is not coded. A design choice with a security meaning: to be ruled. |

For items 20, 22, 24 and 26 the three known values are one constant, so "the same constant" reproduces them trivially and says nothing about whether the constant is right at a larger prime. Nothing in the reference, the paper or `sqisign-rs` says how any of the four should grow with `p`. That is a finding (ledger O26, O29): the failure rate of the translation at `p664` under these bounds is **Open**, and the first evidence will be stage 3's own measurements.

`sqisign-rs` is an oracle for the starting basis, the six matrices and `e − 2`, and for nothing else here. Its other constants (`PRIMALITY_NUM_ITER`, `EQUIV_BOUND_COEFF`, `DEG_MIX_LE`, `RI_COFACTOR_LE`) are SQIsign 3.0's, by that specification's formulas, and at the shared prime `p500` each differs from the reference's.

### 1.8 What the generator must satisfy (stage 1b)

Requirements, not a design.

1. **Scope.** It produces, at all five primes, items 4, 8, 9 and 10, and items 5, 6 and 7 as the intermediate values the tests of 8 to 10 need. Items 17, 20, 22 to 26 and 33 are written by rule (1.4.4, 1.7) and tested, not searched. Items 3, 12 and 13 are definitions, stated once.
2. **From the definitions.** Each value is computed from 1.4, not copied from an oracle.
3. **Deterministic and reproducible.** From a committed script, with the tool and its version pinned, the same bytes on every run. The obvious tool is Sage in the container the nightly already uses for the fixtures.
4. **Checked in with a digest manifest**, as `prism-vectors`' data is.
5. **Compared element for element with every oracle the table names**: the reference at `p248`, `p376`, `p500`; `sqisign-rs` at `p324`, `p500`, `p664`; and the two oracles with each other at `p500`.
6. **Every defining property a committed test**, and with them every statement this section marks "Executed, not committed":
   - the starting basis by 1.4.1 equals the reference's three coordinates at its three primes, and `sqisign-rs`'s at its three;
   - the two copies of the basis in the reference agree (items 1, 2 and 4);
   - the actions of `i` and `π` on points equal the matrices, and the matrix identities of 1.4.2 hold;
   - items 9 and 10 on points, by halving, and the count of wrong candidates the identities alone let through;
   - the prime list is what 1.4.4 says, and the reference's "bad primes" product is its complement below 1237;
   - items 19 and 33 are the next primes after their powers of two; the order and the unit ideal of items 13 and 16 are as stated; the five field constants of item 41 decode as stated;
   - the 3.0 specification's box (Algorithm B.2) at the five primes, beside the reference's 64.
7. **The branches the reference never took** (1.4.1): at each of the five primes the generator reports which branch of step 6 was taken and whether any accepted point had an order missing a prime of `f`, and a test fails if the two readings of step 4 give different points at a prime where no oracle decides between them.

Not yet known about doing this:

- whether Sage's own choices (which square root it returns, how it orders points, what `order()` costs at `p664`) can be kept out of the result. The rule of 1.4.1 has to be implemented as written, with the order of step 1 coded and not inherited;
- how the halves of 1.4.3 are best taken: in `F_{p⁴}` as `sqisign-rs` does, or by another construction; and the cost at `p664`;
- what primality test "the next prime" uses, so that the script and a later Rust check agree. A proof is cheap at these sizes and is what should be asked for;
- whether the generated data belongs in `prism-vectors` or beside the layer crate that reads it (G1 keeps it out of `prism-field`).

### 1.9 What the generated files do not contain

Constants the signer's path reads that are written in the reference's code and not in its generated files (**Read**). Each belongs to a later section; they are listed so that the inventory is not mistaken for the whole.

- The extra torsion above a chain's length: 2 (`HD_extra_torsion`). The verifier's chain is given it. The translation's chain is not (item 25), and takes the branch of the reference's chain that works without it.
- The splitting's expected structure. With extra torsion the reference requires the product structure to be the ninth of the ten (index 8), which is what the engine's port requires; without it, as in the translation, any of the ten is accepted. So the signer's chain differs from the engine's verification chain in three ways the engine does not have today: no extra torsion, any splitting index, and the random change of basis of item 43. This is for stage 4's section.
- The factor 20 in the norm equation's total bound (item 26).
- The parameters of the sum-of-two-squares step: the form's coefficient 1, and 1 in place of the bad-primes product (item 18).
- The maximal order's basis a second time, written by hand in `quat_lattice_O0_set`.
- In our exporter: 16 attempts at a response and 64 draws of an ideal per attempt.
