# Signer specification

Status: begun 2026-11-21. Section 1 (the constants) is written, and its values are generated and committed (stage 1b, `scripts/signer_constants.py`, `prism-vectors/data/signer/`). Section 2.1 (the integers, stage 2a) is written and its crate, `prism-int`, exists. Section 2.3 (Gaussian basis reduction, stage 2c) is a design for review: `scripts/stage2c_oracle.py` recomputes the schedules, and the signer-feature example `stage2c` is the native prototype. The lattice and ideal sections (2b and 2d) are pending. This is a research specification, not an adopted standard, and it makes no security claim.

## What this document is

It is the specification our signer is held to. It is written one layer at a time, and each layer's section is written and reviewed before that layer's code (ruling G3 below). A section says what each object is in mathematical terms, what fixes it uniquely, and how a wrong value would be caught. The code is then built to the section, and a difference between the two is a defect in one of them.

It is the signing-side companion of [VERIFIER_SPEC.md](VERIFIER_SPEC.md). That document fixes the protocol: the five primes, the challenge `s`, the degree `q = 2^(2e−5) − s²`, the bytes of a key and of a signature, and every refusal. This document does not restate them. A signer conforms when the verifier of that specification accepts what it emits, at all five sets, and when each section's defining properties hold.

The paper is ePrint 2026/443 (`docs/papers/2026-443_PRISM_with _salt.pdf`). Our protocol is its Section 5 variant, the prime-degree response of Algorithm 2 (`GenIsogenyStrong`), with the departures the ledger lists (D1 to D15). For key generation and for the subroutines beneath signing the paper gives a sketch (Sections 2.3 and 3.3) and defers to its reference [1], the SQIsign round-2 submission document, by algorithm number (4.1, 3.10, 2.1). That document is not in this repository. The SQIsign specification of 2026-09-01 (`docs/papers/sqisign-20260901.pdf`, called "the 3.0 specification" below) is, and where it defines an object this document cites it by its own numbers.

### Labels

Every statement of fact carries one of four labels. **Ran**: executed, with the output committed. For this section that means one of three commands unless another is named: `python3 scripts/signer_constants.py --check`, which regenerates every value at the five sets, compares it with the committed files and with both oracles, and tests each defining property; `python3 scripts/signer_constants.py --selftest`, which runs the rules at toy primes; and the test `prism-isogeny/tests/signer_constants.rs`. CI runs all three. **Read**: read in the pinned source or document at the place named, and checkable by reading it or by one `grep` or `diff` there. **Derived**: argued, not machine-checked. **Open**: not established.

Stage 1a wrote this section before any script existed and marked what it had computed by hand "Executed, not committed". Stage 1b committed a test for each such statement, and they are now **Ran**. The ledger keeps the earlier label where a row records what was true at the time.

### Rulings (Rick, 2026-11-21)

- **G1.** The signer is new layer crates beneath the front crate, one per mathematical layer, re-exported under `hazmat`. The front crate gains `SecretKey<S>`, key generation and `sign`. `prism-signer` becomes the reference oracle at cutover. Per-set signer constants are not in `prism-field`.
- **G2.** **Amended 2026-11-25.** Each new signer crate documents its public surface, obeys the dependency order and has a place in nightly mutation testing from its first commit. Signer crates forbid unsafe code, with one exception for the audited register barrier in `prism-ct-barrier` (section 0).
- **G3.** Specification first. Each stage's pull request opens with that layer's section of this document, reviewed before code is written.
- **G4.** Constant-time hardening is inside the beta gate. It is the last stage. There is no timeline for it or for anything else here.
- **G5.** The experiments on the second basis point (ledger O21) are their own part, directly after key generation and before signing.
- **G6.** Stage 1 opens with this section.
- **G7 (2026-11-25).** Before finalizing the Stage 2c reduction or the Stage 3 norm equation and ideal sampling, establish constant-time feasibility under section 0. G4 remains the final full-signer hardening gate.

Rulings on this section, the same day:

- **R1.** The secret ideal's first norm is the next prime after `2^(2·WIDTH_BITS)` at all five sets (1.7).
- **R2.** The four bounds of the quaternion layer (32 primality rounds, a box of 64, 17 generator bits, 20,000 steps per generator) are the values at all five sets: starting values, fixed at the three reference primes for replay, provisional at `p324` and `p664`, gated on stage 3's measured failure rates.
- **R3, not ruled.** The Montgomery model of a key (1.6) is looked at again when the work reaches stage 3.

Standing from Round 0 (2026-11-14): we are not transcribing the C; other code bases are oracles for values and not sources of structure; five sets, one algorithm.

### The layers, in the order they are built

| Stage | Layer | Section here |
| --- | --- | --- |
| 1 | The constants: what each is (1a, this section) and the generator that produces them (1b) | 1 |
| 2 | Integers and the quaternion algebra: the integers (2a), then the algebra, the maximal order, lattices, ideals, lattice reduction (2b to 2d) | 2.1 for 2a; the rest to be written |
| 3 | Cornacchia; the norm equation of the ideal-to-isogeny translation; sampling an ideal of prime norm, with the equivalent prime-norm ideal for key generation. No step that represents an integer by the order's norm form (1.2) | to be written |
| 4 | Ideal to isogeny: first the engine's forward chain as the signer needs it (1.9), then the starting curve, its basis, the action of its endomorphisms, and the translation | to be written |
| 5 | Key generation and signing | to be written |
| last | Constant-time hardening (G4) | to be written |

The table of 1.3 names the stage that consumes each constant by these numbers. The plan of 2026-11-21 divides the stages further; section 1 does not use the finer names, and section 2 does (2a to 2d).

## 0. Constant-time feasibility before algorithm selection

Under G3, the Stage 2c and Stage 3 design reviews must establish the following for the algorithm they intend to implement.

- Identify which operands, addresses, loop counts, retries and failures depend on a key or a derived ideal. Review each exit from a secret type in the PR introducing it, including its downstream uses and the information conveyed by failure (2.1.3). Test logs may expose counts and intermediate invariants on synthetic fixtures only when the recorded values are safe to disclose.
- Derive a public bound for each loop and for every intermediate at each of the five sets. For reduction, prove completion under the actual rounding rule and the input bounds enforced by key generation and import (Q4/Q5). For a randomized search, give a separate bound on exhausting the fixed schedule. A fixed instruction count alone says nothing about the probability of returning a result.
- Give the estimated cost at all five sets and measure the candidate on representative hardware, including the P500 deployment floor, before accepting its cost. Record the toolchain, build settings, input class and oracle. G4 still covers the integrated signer's timing, failure behavior and erasure (O6).

### Stage 2c reduction and the G2 exception

Use fixed-schedule Gaussian-integer Lagrange reduction with exact quotients as the correctness baseline. **Read:** the Q5 research bounds intermediate width for its stated input shapes. On a valid ideal its bounded 64-bit quotient saturated, and the scheduled steps plus exact cleanup returned an unreduced basis. A sticky error would make saturation visible while leaving a secret-dependent failure. **Open (O33):** demonstrate adequate width and progress on every admitted input for the chosen quotient, including that fixture. A fast quotient can replace the baseline once it meets both obligations. [Q5 proof and counterexample](https://github.com/Shekyl-Foundation/shekyl-prism/blob/53e2eb4fdd772ff8a53708ba60314c3dabea9d05/research/q5-width-proof/REPORT.md)

The exception permits one dependency-free `no_std` crate with a safe value/mask API. Its only locally allowed unsafe operation is the register identity in `value_barrier_u64`, using one general-purpose input/output register on x86-64 and AArch64. The crate denies unsafe code elsewhere. Its implementation PR must pin this boundary in the workspace lint check, including a failing test for a second unsafe site. The existing benchmark counter has its own exception. Signing on an unaudited architecture must fail at build time. Verifier-only builds must remain portable, with the existing s390x big-endian verifier tests still executing. [Crate audit](https://github.com/Shekyl-Foundation/shekyl-prism/blob/53e2eb4fdd772ff8a53708ba60314c3dabea9d05/research/ct-barrier-policy/prism-ct-barrier/AUDIT.md)

**Read:** in the research reduction on Rust 1.97.1 the isolated barrier passed four build configurations on each supported architecture. On x86-64 the observed timing maxima were |t| = 2.18 at P248 and 1.80 at P500. The ARM64 traces were collected under QEMU. The implementation PR must integrate release assembly inspection, control-flow trace comparisons on both architectures, and x86-64 Valgrind secret-branch/address checks with a known-leaky control. Exercise the consuming operations, especially a shift by a secret amount. Repeat on toolchain, barrier, consumer or code-generation changes. Review shift-count and other instruction-latency findings on the target CPU. Require native timing evidence on each architecture before accepting its timing claim. [Barrier experiment](https://github.com/Shekyl-Foundation/shekyl-prism/blob/53e2eb4fdd772ff8a53708ba60314c3dabea9d05/research/ct-barrier-policy/REPORT.md)

### Stage 3 norm equation (O29)

| Decision | Design and acceptance evidence |
| --- | --- |
| Target exponent | Design the native signer for `e`. Retain the pinned reference and its `e − 2` fixtures for test-only byte replay. Test ideal equivalence where claimed, key/signature acceptance at all five sets, and byte equality for the designated replay path. **Read:** changing the exponent gave 300 successful keygen/sign/verify trials across the three C sets and changed every sampled key and signature. P324 and P664 still need native acceptance evidence. |
| Theta postcondition | Keep the square-root chain and the theta check that supplies odd `d1`. Apply the check as a mask. Include the transformed case's one-step-shorter chain in Stage 4's fixed-schedule design. Removing the check caused the reference to abort in most trials. |
| Even norms | Use the two-stage Qlapoti search for both parities, with a fixed generator pool and oblivious compaction. For the faster Qlapoty alternative, investigate a bounded conversion to an equivalent odd-norm ideal first. Adoption requires a proof of equivalence, width and completion for all accepted inputs, including its stuck first-loop cases. Secret parity must not select a visible path. |
| Failure budget | Use at most 2⁻⁶⁴ per complete norm-equation invocation as the design target. Budget jointly for small norms, pool exhaustion, correlated generator blocks and all other capped stages. State how the bound depends on the key, and compose it across invocations in keygen and signing. Final acceptance needs justified distributional bounds and measured cost. |

**Read:** at P500 the model prices two-stage Qlapoti at `e` at 88.6 times the reference mean work at `e − 2` for the 2⁻⁶⁴ target. The 35.2 figure for Qlapoty covers odd norms only and excludes unsuccessful first-loop searches. **Open:** those estimates extrapolate from simulated ideals. O29 stays open until Stage 3 supports its failure budget and measures its implementation at all five sets. [Norm-equation experiments and C trials](https://github.com/Shekyl-Foundation/shekyl-prism/blob/53e2eb4fdd772ff8a53708ba60314c3dabea9d05/research/qlapoti-redesign/REPORT.md)

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
| 25 | `QUAT_qlapoti_used_power_of_two` | 246 / 374 / 498 | The exponent of the power of two the two equivalent ideals' norms sum to, and the length of the translation's chain: `e − 2`. The length is one less when the norm equation returns its transformed case. The paper says only that the exponent is at most the 2-valuation of `p + 1` (Section 2.3, equation 2). The 3.0 specification sets `f − 2` in its notation "for technical reasons related to 2-dimensional isogeny computations" (its Section 4.6), and its chain takes kernel points of order `2^(length+2)` (Section 4.5.4). The pinned reference calls the translation's chain *without* that extra torsion, on a kernel of order exactly `2^(e−2)`. This row records the pinned reference value for replay. Section 0 selects `e` for the native design and records the C trials. Stage 3 owns the failure budget (O29) and Stage 4 the chain schedule. | Conv | `K`, `S`, `S5`. Stages 3 and 4. | **Yes** | path, +draws | `C`; `R` `QLAPOTY_USED_POWER_OF_TWO = f − 2` in its notation (322, 498, 662), equal at `p500` (**Read**: both files). | Equal to `e − 2`. |
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

Items 19 and 33 are "the next prime after" a power of two: the smallest prime strictly greater. The test is Baillie–PSW as `VERIFIER_SPEC.md` specifies it (departure D15), written in the script: the primes below 100, the strong test to base 2, the strong Lucas test with Selfridge's discriminant among 256 candidates. Every integer between the power of two and the result is refused by a leg that never refuses a prime (a factor, a failed base-2 test, a failed Lucas test); a refusal for want of a discriminant is not a proof and would stop the script. CI verifies pinned ECPP certificates for the four distinct secret norms and five cofactors, and finds a compositeness witness for every integer in each gap. The same checker verifies ECPP and Lucas N+1 certificates for all five field moduli (`scripts/ci/check_signer_prime_certificates.py`, ledger O19). The nightly also asks SymPy for the secret norm at each set (`scripts/ci/check_signer_primes.py`).

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
4. **Compared element for element with every oracle.** The values of the C reference and of `sqisign-rs` are committed as extracted, in `oracle_c_reference.txt` and `oracle_sqisign_rs.txt`, each with its repository, its commit and the SHA-256 of every file a value was read from. The comparison is a separate function over two tables and counts what it compares: 17 values a set against the C reference at `p248`, `p376`, `p500` (11 that depend on the prime, 6 that do not), 10 a set against `sqisign-rs` at `p324`, `p500`, `p664`, and the two oracles with each other at `p500`. A corrupted oracle value fails it (`--selftest`), and so does a halved generator's matrix with a wrong top bit that passes every matrix identity: `--check` plants each of the three wrong survivors of each generator at each set against every oracle that has the set, 36 in all, and requires each refused.
5. **Every defining property a committed test**: the bullets stage 1a listed, each at every set it applies to, in `--check`.
6. **Every branch executed.** 1.4.1 has the toy primes and the case no search reaches.
7. **A second implementation.** `prism-isogeny/tests/signer_constants.rs` reads the committed files and checks, with the engine's own field, points and `endomorphism` maps, at five sets: both points on `E0` and of order exactly `2^e`, independent, `[2^(e−1)]Q0 = (0, 0)`; the third coordinate is `x(P0 − Q0)`, and read as `x(P0 + Q0)` the matrices are not the actions; the matrices of `i`, of the Frobenius and of `k` are their actions; `2·gen3 = M_i + M_j` and `2·gen4 = 1 + M_k`. It cannot see the top bit of the entries of items 9 and 10: the engine has no arithmetic over `F_{p⁴}`. Those bits rest on the script's test on points and on the oracles, of which every set has at least one.

Where the oracle files come from, and what holds them:

- The C reference's values are re-extracted by `signer_constants.py --extract-c` from the pinned submodule. CI's `c-reference-pin` job has the submodule and runs it on every pull request: the committed file must be what the pinned tree gives.
- `sqisign-rs`'s values are re-extracted by `--extract-sqisign-rs <checkout>` from a checkout at the commit the file names. **This is not run automatically**: CI has no checkout of that repository. The manifest holds the committed file to its bytes, and the file's own header says what it was read from.

What stage 1a did not know, and what was found:

- Plain Python is enough for everything, the halved generators included. `--check` takes about a minute on the development machine: 23 seconds to generate the five sets, `p664` the slowest at about 12, and the rest to test the properties, most of it the fifteen top-bit variants of each matrix on points.
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

## 2. Integers and the quaternion algebra

Section 2.1 covers the integers (Stage 2a). Section 2.3 proposes Gaussian basis reduction (Stage 2c). The lattice and ideal sections (2b and 2d) are pending. Rulings that stand over 2.1, from Round 0 (2026-11-14) and the plan of 2026-11-21:

- **S5.** Integers are fixed-width. No value decides a length.
- **S6.** The first signer is correct, wipes its secrets, and classifies every step as secret or public. It makes no constant-time claim.
- **Plan 2.4.** Secret and public integers are different types from the first integer pull request, and a variable-time routine takes only the public type.

### 2.1 The integers (stage 2a)

Labels in this section. **Ran** means run on a build VM with the output committed: `scripts/build_traced_reference.sh` and `scripts/measure_integer_widths.py`, which wrote `docs/benchmarks/integer-widths-reference.tsv`. **Read**, **Derived** and **Open** are as above.

#### 2.1.1 What the layer is for

The signed integers under everything in stages 2b to 5: the coordinates of quaternions, the entries of lattice bases, norms, the numbers of the norm equation, and the matrices modulo `2^e` of the translation.

The reference uses arbitrary-precision integers: `ibz_t` is GMP's `mpz_t` (here its bundled `mini-gmp`), wrapped by 44 functions in `src/quaternion/ref/generic/intbig.c` and one more, `ibz_xgcd`, in `hnf/ibz_division.c` (**Read**). An `mpz_t` grows to fit its value, on the heap, and the time of every operation follows the value's length.

Ours do not grow. Every integer of a parameter set has the same number of 64-bit limbs, fixed by the set (S5). So the layer has to say how many, before the algorithms that fill them are written. That is 2.1.2, and it is the main question of this part.

#### 2.1.2 The width budget

**Notation.** For a set: `e` and `f` with `p = f·2^e − 1`; `L` the bit length of `p` (251, 326, 383, 505, 669); `W` its `WIDTH_BITS` (256, 384, 384, 512, 768); `D` the first norm of the secret ideal, the next prime after `2^(2W)` (ruling R1), of `2W + 1` bits; `q = 2^(2e−5) − s²` the challenge degree, below `2^(2e−5)`. For an integer `N`, `b(N)` is its bit length. A width is counted in bits of magnitude: 2.1.3 holds a sign beside the limbs.

`L` and `W` are kept apart in every formula. `2W/L` is 2.04, 2.01 and 2.03 at the three sets with a reference and 2.36 and 2.30 at `p324` and `p664`, so a formula in `L` alone that fits the three measured sets is wrong at the two that are not measured.

##### What the reference reaches (Ran)

The reference's key generation, and our exporter's signing (`scripts/c/export_s5_vectors.c`, the only implementation of our signing path, 1.2), were run with every `ibz_*` function that writes an integer recording the bit length of what it wrote and the address it was called from.

- **The instrumentation** is `scripts/c/ibz-trace.patch`, applied to a copy of the pinned tree by `scripts/build_traced_reference.sh`; the submodule is not changed. It renames 24 functions of `intbig.c` and `ibz_xgcd`, and adds a wrapper of each original name that calls the original and records each output. Calls between the functions of `intbig.c` are not recorded, so the temporaries inside the modular square root are not seen; they are below the square of its modulus. The copy also carries `cornacchia-positive.patch`, as the exporter's build does.
- **A call site is resolved to a function** with `addr2line -i`: the innermost inlined function where the build kept the inlining record, and otherwise the function the compiler emitted, which then stands for what was inlined into it. Two cases the tables below rely on were read in the built binary: it has no symbol for `quat_lattice_gram`, `quat_lideal_class_gram`, `quat_lideal_norm`, `quat_lideal_mul` or `quat_lideal_shortest_equivalent`, so each is inlined where it is called, and the Gram matrix's products are in the row of `quat_lideal_reduce_basis`; and the call that wrote the largest value of `quat_qlapoti`'s row is the multiplication between a call of `quat_alg_norm` and a call of `ibz_div`, which is the body of `quat_lideal_mul`.
- **The runs**, at `p248`, `p376` and `p500`: 256 key generations, and 8 signatures under each key, 2,048 a set. Every seed is derived from a fixed label. Every challenge scalar `s` is odd, of at most `e − 3` bits, with `q` prime (probable prime to 36 bases), as the protocol's is; the first under each key is the smallest such `s`, so the largest `q`. Every run returned 0: none failed, none was stopped by the time limit.
- **What it cost**: 43 minutes on two cores of a shared build VM.

The largest integer the reference wrote, in bits:

| | `p248` | `p376` | `p500` |
| --- | ---: | ---: | ---: |
| Key generation | 12,746 | 19,188 | 25,531 |
| Signing | 12,328 | 18,767 | 24,953 |
| The same, in units of `L` | 50.8, 49.1 | 50.1, 49.0 | 50.6, 49.4 |

These are not our budget. They are the reference's method, in three habits (**Read**, each at the place named):

- **A.** It brings a lattice to Hermite normal form modulo the determinant of a generating matrix (`quat_lattice_hnf`, `quat_lattice_add`, `quat_lattice_mul` in `lattice.c`), and the determinant of a `4×4` matrix of entries near `N` is near `N⁴`. It intersects two lattices through three duals, each an adjugate matrix (`quat_lattice_intersect`), and it finds an ideal's norm as the square root of a lattice index, a quotient of two determinants (`quat_lideal_norm` in `ideal.c`).
- **B.** It forms a generator as the product of two elements with coordinates below `N` and takes that product's norm, before anything is reduced modulo `N` (`quat_sampling_random_ideal_O0_given_norm` in `normeq.c`, `quat_lideal_create_principal`).
- **C.** Everything else: values that belong to the mathematical object and that any method holds.

Each function of the table below is put in one class by what its largest value is. A function that does both kinds of work is put in A or B, which can only lower the figure for C.

| Function | Class | Key generation: `p248` | `p376` | `p500` | Signing: `p248` | `p376` | `p500` |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `ibz_vec_4_linear_combination` | A | 12,746 | 19,188 | 25,531 | 12,328 | 18,767 | 24,953 |
| `ibz_mat_4xn_hnf_mod_core` | A | 10,197 | 15,350 | 20,425 | 9,863 | 15,013 | 19,962 |
| `ibz_xgcd_with_u_not_0` | A | 8,924 | 13,433 | 17,876 | 8,630 | 13,138 | 17,469 |
| `ibz_vec_4_linear_combination_mod` | A | 8,415 | 12,661 | 16,858 | 8,124 | 12,364 | 16,452 |
| `ibz_vec_4_scalar_mul_mod` | A | 7,647 | 11,509 | 15,317 | 7,394 | 11,257 | 14,972 |
| `ibz_mat_4x4_inv_with_det_as_denom` | A | 5,105 | 7,681 | 10,220 | 6,326 | 9,543 | 12,619 |
| `ibz_centered_mod` | A | 5,103 | 7,678 | 10,216 | 6,326 | 9,543 | 12,619 |
| `ibz_mod_not_zero` | A | 5,103 | 7,678 | 10,216 | 6,326 | 9,543 | 12,619 |
| `ibz_vec_4_copy_mod` | A | 5,103 | 7,678 | 10,216 | 6,326 | 9,543 | 12,619 |
| `quat_lattice_hnf` | A | 5,103 | 7,678 | 10,216 | 6,326 | 9,543 | 12,619 |
| `quat_lattice_add` | A | 2,552 | 3,840 | 5,109 | 3,810 | 5,741 | 7,586 |
| `ibz_mat_4x4_scalar_mul` | A | 2,552 | 3,840 | 5,109 | 2,468 | 3,756 | 4,993 |
| `ibz_mat_4x4_gcd` | A | 2,551 | 3,839 | 5,108 | 2,467 | 3,755 | 4,992 |
| `ibz_mat_4x4_scalar_div` | A | 2,551 | 3,839 | 5,108 | 2,467 | 3,755 | 4,992 |
| `quat_alg_coord_mul` | B | 2,551 | 3,838 | 5,107 | 2,466 | 3,754 | 4,991 |
| `quat_alg_norm` | B | 2,551 | 3,838 | 5,107 | 2,466 | 3,754 | 4,991 |
| `quat_lideal_create_principal` | B | 2,551 | 3,838 | 5,107 | 2,466 | 3,754 | 4,991 |
| `quat_lattice_mat_alg_coord_mul_without_hnf` | B | 1,277 | 1,920 | 2,555 | 1,234 | 1,878 | 2,497 |
| `quat_alg_conj` | B | 1,276 | 1,919 | 2,554 | 1,233 | 1,877 | 2,496 |
| `quat_sampling_random_ideal_O0_given_norm` | B | 1,276 | 1,919 | 2,554 | 1,234 | 1,877 | 2,496 |
| `quat_lattice_index` | A | 1,031 | 1,543 | 2,055 | 1,270 | 1,914 | 2,529 |
| `quat_lideal_reduce_basis` | C | 1,028 | 1,540 | 2,052 | 1,268 | 1,911 | 2,527 |
| `quat_alg_mul` | B | 1,027 | 1,539 | 2,051 | 1,266 | 1,910 | 2,525 |
| `ibz_mat_4x4_copy` | A | — | — | — | 1,266 | 1,910 | 2,525 |
| `ibz_mat_4x4_transpose` | A | — | — | — | 1,266 | 1,910 | 2,525 |
| `quat_lattice_dual_without_hnf` | A | — | — | — | 1,266 | 1,910 | 2,525 |
| `quat_lattice_reduce_denom` | A | 514 | 770 | 1,026 | 1,265 | 1,909 | 2,524 |
| `dim2id2iso_ideal_to_isogeny_clapotis` | C | 508 | 768 | 1,015 | 1,003 | 1,518 | 2,011 |
| `endomorphism_application_even_basis` | C | 741 | 1,125 | 1,497 | 933 | 1,415 | 1,879 |
| `quat_qlapoti` | C | 371 | 565 | 750 | 757 | 1,144 | 1,513 |
| `quat_lattice_contains` | C | 496 | 752 | 1,000 | 689 | 1,042 | 1,382 |
| `ibz_mat_4x4_eval` | C | 495 | 751 | 999 | 688 | 1,041 | 1,381 |
| `ibz_vec_4_copy` | C | 495 | 751 | 999 | 688 | 1,041 | 1,381 |
| `quat_lideal_mul` | C | 653 | 975 | 1,291 | — | — | — |
| `ibz_vec_4_copy_ibz` | C | 514 | 770 | 1,026 | 633 | 955 | 1,263 |
| `quat_lll_core` | C | 514 | 770 | 1,026 | 634 | 955 | 1,263 |
| `quat_lideal_norm` | C | 513 | 769 | 1,025 | 632 | 954 | 1,262 |
| `quat_lattice_alg_elem_mul` | C | 515 | 771 | 1,027 | 2 | 2 | 2 |
| `quat_lideal_prime_norm_reduced_equivalent` | C | 514 | 770 | 1,026 | — | — | — |
| `ibz_rounded_div` | C | 498 | 762 | 1,007 | 499 | 762 | 1,007 |
| `ibz_2x2_mul_mod` | C | — | — | — | 496 | 752 | 1,000 |
| `sign` | C | — | — | — | 496 | 752 | 1,000 |
| `ibz_vec_4_scalar_div` | C | 246 | 374 | 498 | 439 | 664 | 881 |
| `ibz_vec_4_scalar_mul` | C | 246 | 374 | 498 | 439 | 664 | 881 |
| `quat_alg_elem_copy` | C | 193 | 290 | 383 | 439 | 664 | 881 |
| `ibz_cornacchia_prime` | C | 254 | 378 | 502 | 255 | 378 | 506 |
| `matrix_application_even_basis` | C | 249 | 377 | 501 | 249 | 377 | 501 |
| `_change_of_basis_matrix_tate` | C | 248 | 376 | 500 | 248 | 376 | 500 |
| `ibz_from_bytes` | C | — | — | — | 248 | 376 | 500 |
| `set_decimal` | C | — | — | — | 245 | 373 | 497 |
| `ibz_mat_2x2_eval` | C | 191 | 290 | 381 | 192 | 290 | 383 |
| `ibz_vec_4_add` | C | 187 | 284 | 377 | 187 | 284 | 377 |
| `quat_alg_equal_denom` | C | 187 | 284 | 376 | 187 | 284 | 376 |
| `quat_qf_eval` | C | 145 | 209 | 270 | — | — | — |
| `ibz_to_bytes` | C | 142 | 208 | 267 | — | — | — |
| `quat_lideal_generator` | C | 141 | 207 | 267 | — | — | — |
| `ibz_vec_4_content` | C | 143 | 202 | 263 | 139 | 204 | 265 |
| `quat_alg_normalize` | C | 128 | 194 | 255 | 128 | 194 | 255 |
| `ibz_complex_mul` | C | 64 | 95 | 126 | 65 | 96 | 127 |

59 functions; 6 more wrote nothing above 64 bits and are left out. The file has every row, by operation.

| Largest value, bits | Key generation: `p248` | `p376` | `p500` | Signing: `p248` | `p376` | `p500` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Class A | 12,746 | 19,188 | 25,531 | 12,328 | 18,767 | 24,953 |
| Class B | 2,551 | 3,838 | 5,107 | 2,466 | 3,754 | 4,991 |
| Class C | 1,028 | 1,540 | 2,052 | 1,268 | 1,911 | 2,527 |

The largest value of class C is the Gram matrix of the ideal about to be reduced (`quat_lideal_reduce_basis`): 1,268, 1,911 and 2,527 bits, in signing. It is `2·b(N) + 4` at most for the ideal's norm `N`, at each set (the norm is the row `quat_lideal_norm`).

##### What section 2.1 commits the later stages to

The budget below is for these methods. Each is an obligation on the stage named, and that stage's review checks it. A stage that cannot meet one says so in its own section, and the width is derived again there.

- **Q1 (2b).** A left ideal of `O0` of norm `N` is held by a basis in Hermite normal form with numerators over the denominator 2 (the denominator of `O0`'s basis, 1.4.5). The lattice of numerators contains `2N·Z⁴`: `N·O0` lies in the ideal, and `2N·Z⁴` lies in `N·O0`. So the normal form is computed with the four vectors `2N·eᵢ` among the generators and every entry reduced modulo `2N` as it is made, which changes no lattice. The modulus is **fixed**: it is never divided by a pivot. No determinant is computed. This is not the reference's routine, nor the textbook modular algorithm: both work modulo a multiple of the lattice's determinant, here `4N²`, and divide it by each pivot as they go (`ibz_mat_4xn_hnf_mod_core`); given `2N` they give a wrong form (**Derived**, by hand through the reference's routine: the lattice `2Z ⊕ 2Z` with modulus 2 comes out as `diag(1, 2)`), and given `4N²` their intermediates are near `4b` bits, above the width.
- **Q2 (2b, 2d).** An ideal's norm is carried with it and never recovered from an index. An element that generates an ideal together with `N` is reduced modulo `N·O0` before it is used.
- **Q3 (2d).** The intersection of two left ideals of `O0` of coprime norms `N₁` and `N₂` is `N₂·I₁ + N₁·I₂`, one normal form of eight generators by the method of Q1 with the fixed modulus `2·N₁·N₂`, and no dual. (**Derived**: both terms lie in the intersection, and at each prime dividing `N₁` the sum is `I₁` and at each prime dividing `N₂` it is `I₂`.) Our signing intersects an ideal of prime norm `q` with the secret ideal of prime norm. The two norms are never equal: `s` has at most `e − 3` bits, so `q > 2^(2e−6)`, which is above every secret norm Q4 admits.
- **Q4 (stage 5).** Key generation accepts a secret ideal only if its norm is below `2^B`, `B = ⌈L/2⌉ + 32`, and otherwise goes on searching. The same bound is tested wherever a secret key enters, so also when one is decoded: a replay of signing runs on keys imported from the reference (1.6). See "the secret norm" below.
- **Q5 (2c).** The reduction of a basis states, in its own section, the largest intermediate it holds, as a formula. Section 2.3 proposes exact Gaussian reduction and identifies the starting-basis bounds needed to apply it.

##### The budget, by consumer (Derived)

For an ideal `I` of norm `N`, with `b = b(N)`, under Q1 and Q2. An element of `O0` is `(x₀ + x₁i + x₂j + x₃k)/2` with integer numerators, and its norm is `(x₀² + x₁² + p·(x₂² + x₃²))/4`.

| # | Quantity | Consumer | Bound on its bits | Proven? |
| --- | --- | --- | --- | --- |
| 1 | An entry of the basis | 2b, 2d | `b + 1`: every entry of the normal form is in `[0, 2N]` | Proven |
| 2 | An intermediate of the normal form with the fixed modulus `2N` (Q1) | 2b | `2b + 3`: a combination `u·x + v·y` of two entries below `2N` with cofactors of a greatest common divisor, at most `2·(2N)²`, before it is reduced | Proven for that method, and for no method that works modulo the determinant; 2b states its own |
| 3 | An entry of the Gram matrix of the basis, in the reference's scaling (`2·Σ wₖ·xₖ·yₖ`, weights 1, 1, `p`, `p`) | 2c | `2b + L + 4`: at most `16·(1 + p)·N²` | Proven |
| 4 | The same, when the normal form has its generic shape (two pivots near `N` in the coordinates of 1 and `i`, two pivots of 1 or 2) | 2c | `2b + 4` | Typical: the other shapes are a fraction near `1/N` of the ideals of a prime norm |
| 5 | An intermediate of the reduction | 2c | — | **Open** (Q5) |
| 6 | The norm of a sampled trace-zero element with numerators below `N`; a coordinate of the product of two elements with numerators below `2N` | stage 3 | `2b + L + 4` | Proven |
| 7 | A product in the square root modulo `N` | stage 3 | `2b` | Proven |
| 8 | The step to an equivalent ideal `I·β̄/N`, for `β` in `I` of norm `N·n'`: a coordinate of `x·β̄` for `x` in the basis, and the product `N·n'` | 2d, stage 3 | `1.5·b + 0.5·b(n') + 0.5·L + 5` | Proven; below row 3 |
| 9 | The norm equation: a coefficient of the generator, the target, the lattice of dimension 2, the values given to Cornacchia | stage 3 | `2.5·L + 80`, with the bounds of R2 (17 bits, 20,000 steps) and an ideal of norm below `2^(⌈L/2⌉ + 2)` | Proven, given a reduced basis in 2c's sense |
| 10 | The translation: a coordinate of an endomorphism of norm `d₁·d₂ < 2^(2e)`, its product with a matrix entry below `2^e`, a degree times a norm | stage 4 | `e + b + ⌈L/2⌉ + 4` | Proven, as row 9 |

Row 3 is the largest, and it is reached twice: by key generation, which reduces the ideal of norm `D`, and by signing, which reduces the intersection of the ideal of norm `q` with the secret ideal. So with `B` the bound on the secret norm's bits (Q4):

    need = max( 2·(2W + 1), 2·(2e − 5 + B) ) + L + 4,    B = ⌈L/2⌉ + 32

| Set | `L` | `W` | Key generation: `2·(2W + 1) + L + 4` | Signing: `2·(2e − 5 + B) + L + 4` | `need` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `p248` | 251 | 256 | 1,281 | 1,553 | 1,553 |
| `p324` | 326 | 384 | 1,868 | 2,006 | 2,006 |
| `p376` | 383 | 384 | 1,925 | 2,329 | 2,329 |
| `p500` | 505 | 512 | 2,559 | 3,069 | 3,069 |
| `p664` | 669 | 768 | 3,747 | 4,053 | 4,053 |

Signing binds at all five. Key generation comes nearest at `p324` and `p664`, where `W` is large beside `L`.

What was measured, beside the bound (**Ran**; bits):

| Quantity | Bound | `p248` | `p376` | `p500` |
| --- | --- | --- | --- | --- |
| `D`, key generation's first norm (`quat_lideal_norm`) | `2W + 1` | 513 of 513 | 769 of 769 | 1,025 of 1,025 |
| `q` (the exporter's `sign`, the subtraction) | `2e − 5` | 491 of 491 | 747 of 747 | 995 of 995 |
| The largest candidate for the secret norm | `B = ⌈L/2⌉ + 32` | 143 of 158 | 207 of 224 | 268 of 285 |
| The norm of signing's intersection (`quat_lideal_norm`) | `2e − 5 + B` | 632 of 649 | 954 of 971 | 1,262 of 1,280 |
| Gram matrix, key generation (`quat_lideal_reduce_basis`) | row 3: `2·(2W + 1) + L + 4` | 1,028 of 1,281 | 1,540 of 1,925 | 2,052 of 2,559 |
| Gram matrix, signing (`quat_lideal_reduce_basis`) | row 3: `2·(2e − 5 + B) + L + 4` | 1,268 of 1,553 | 1,911 of 2,329 | 2,527 of 3,069 |
| The reduction's integers (`quat_lll_core`), signing | row 5: none | 634 | 955 | 1,263 |
| The step to the shortest equivalent ideal and the norm equation (`quat_qlapoti`, whose row has `quat_lideal_shortest_equivalent` and a `quat_lideal_mul` inlined), signing | rows 8 and 9; row 8 is the larger | 757 of 1,168 | 1,144 of 1,750 | 1,513 of 2,305 |
| The same function in key generation, where the ideal's norm is small and the norm equation's own values are its largest | row 9: `2.5·L + 80` | 371 of 707 | 565 of 1,037 | 750 of 1,342 |
| The translation (`dim2id2iso_ideal_to_isogeny_clapotis`), signing | row 10: `e + b + ⌈L/2⌉ + 4` | 1,003 of 1,027 | 1,518 of 1,543 | 2,011 of 2,037 |

##### The secret norm

Key generation replaces the ideal of norm `D` by an equivalent one of small prime norm: an element `β` with coefficients in `[−64, 64]` (item 22) on a reduced basis, whose norm divided by `D` is prime. That prime is the secret ideal's norm, and signing's largest value grows with twice its length.

- **Typical.** The four reduced basis vectors have norm near `D·√p`, and a combination of them with coefficients up to 64 has up to `2^16` times that, so the secret norm is near `2^16·√p`: about `L/2 + 16` bits. Measured, the largest candidate tested for primality over 256 key generations a set has 143, 207 and 268 bits, which is `L/2` plus 17.5, 15.5 and 15.5.
- **Proven.** Only that the product of the four vectors' norms is at most `4·p²·D⁴` for an LLL-reduced basis (with the constant of the parameter 3/4; 2c fixes its own), and each is at least `D`, so one of them can be as long as `4·p²·D`, and the secret norm can have `2L + 18` bits. That happens when the ideal's class has very short elements, which is to say when the key's curve is joined to `E0` by an isogeny of very small degree. Its probability is not computed here.
- **The rule proposed (Q4).** Key generation refuses a candidate norm of more than `B = ⌈L/2⌉ + 32` bits and goes on to the next draw. `B` is 15 to 17 bits above the largest candidate seen. With it row 3 is a proven bound for signing; without it the proven figure would be `4e + 5L + 30` bits, about half as much again.
- **What it does to replay.** The reference has no such test. A replay of its key generation differs from it only at a seed where it accepts a norm above `2^B`, and none was seen. This is for stage 5's section to adopt or replace, and it is in the report of this part.

##### What `sqisign-rs` does (Read)

`sqisign-rs` at `7a4a01b` has one integer type, `Ibz<const N: usize>` (`crates/sqisign-rs/src/mp/mod.rs`), and **one `N` for each parameter set**, used by every layer: `IBZ_NLIMBS` is 30, 40 and 55 at `p324`, `p500` and `p664` (`crates/sqisign-rs/src/precomp/p324_3.rs`, `p500_27.rs`, `p664_17.rs`). A per-purpose length exists only as a bit bound carried inside each value, not as a type.

- **Its rule** (`tools/gen-precomp/src/emit.rs`, `ibz_nlimbs`): `5·⌈log₂ p / 64⌉` limbs, which its comment attributes to the SQIsign reference, "or more if a reduced ideal needs it. The constant-time reduction of an ideal of norm bound `nb` needs `2 (nb + 3) + log2 p + 3` bits". It takes the largest of three counts: the reference's, the intersection salt-PRISM reduces, and the ideal SQIsign's key generation reduces. The same comment says the rule of five falls short at the two round-2 primes it keeps for comparison, which are ours: "`p248_5`: 21, `p376_65`: 31", against 20 and 30.
- **The same dominant term.** Its `2·(nb + 3) + log₂ p + 3` is row 3 above with five bits more. So the two derivations agree on what binds: the Gram matrix of the ideal to be reduced. This section's row 3 was derived before that comment was read; the agreement is a check and the formula here is not taken from it.
- **Its figures are not ours.** Its norms are SQIsign's and salt-PRISM's: an ideal of norm about `p·2^(2λ)` in key generation, and an intersection with an ideal of norm below `2^(2λ + 128)`. Ours are `D`, above `p²`, and an intersection with an ideal of norm near `p²`. Its formula with our norms gives 25, 32, 37, 49, 64 limbs at the five sets, against 25, 32, 37, 48, 64 for `need`.
- **What its type does on overflow**: nothing reports it. `mul` drops the limbs above the container and caps the bound it carries, and `add` caps the bound likewise (`mp/arith.rs`); the container is sized "so no honest computation ever reaches the top of the container" (`mp/mod.rs`). Ours reports it (2.1.3).

Credited in `docs/lab/CONTRIBUTIONS.md`. An oracle for the choice, and not its source.

##### The rule proposed

**Default width for each parameter set**, public or secret. Stage 2c uses the narrower, statically selected widths specified in 2.3 for its normalized Gram and constructor.

    LIMBS = ⌈need / 64⌉ + 1

| Set | `need`, bits | `LIMBS` | Width, bits | Spare bits above `need` | Spare above the largest value of class C measured |
| --- | ---: | ---: | ---: | ---: | ---: |
| `p248` | 1,553 | 26 | 1,664 | 111 | 396 |
| `p324` | 2,006 | 33 | 2,112 | 106 | not measured |
| `p376` | 2,329 | 38 | 2,432 | 103 | 521 |
| `p500` | 3,069 | 49 | 3,136 | 67 | 609 |
| `p664` | 4,053 | 65 | 4,160 | 107 | not measured |

- **Why one default.** A second, narrower width for the values that are never multiplied together would make most operations cheaper and every signature of every function carry a choice. The first signer is to be correct (S6); the cost of one width is recorded here and is a question for the last stage, with the timings.
- **What one width costs.** A multiplication is `LIMBS²` limb products whatever the operands hold, so 676, 1,089, 1,444, 2,401, 4,225 limb products at the five sets, where most operands fill well under half of the width.
- **The spare limb** is the margin: between 67 and 111 bits above `need`. It is there for the constants of rows 2 and 8, which are stated to within a few bits, and it is all the reduction's intermediates (row 5) have until 2c states them.
- **At `p324` and `p664`** nothing is measured. The formula is the same derivation with their `e`, `L` and `W`, and the spare is 106 and 107 bits. What cannot be checked there until our own signer runs is the typical-case figures (row 4, the secret norm), which enter the formula only through `B`, a bound key generation enforces.
- **The width is the consumer's.** The integer crate is generic in the number of limbs and names no parameter set. A later stage that needs another width at a set changes one constant and says why in its section.
- **Exceeding the width is an error the caller sees.** It is never a wrap and never a truncation: 2.1.3 says how each type reports it.

##### Bounds not proven

1. **The reduction's intermediates** (row 5, Q5). The reference reduces with floating-point Gram–Schmidt coefficients (`lll/l2.c`) and exact integer updates of the basis and the Gram matrix. In 6,912 runs the largest integer it wrote was within two bits of the largest norm (`quat_lll_core` in signing: 634, 955, 1,263 bits, against norms of 632, 954, 1,262), so about the length of a basis entry. But no bound on a size-reduction coefficient is proven here, and ours is not written.
2. **The secret norm** (Q4): typical `L/2 + 16`, proven only `2L + 18`. The budget assumes the rule of Q4.
3. **The generic shape of a normal form** (row 4). Not used: the budget takes row 3, which is proven.
4. **Anything at `p324` and `p664`** that is typical and not proven, for want of a signer to measure. The same two items, 1 and 2.

Because of items 1 and 2 an overflow is, for now, a path the signer can take. That is temporary, and it is not the design. The aim: once stage 5 has Q4 and stage 2c has Q5, an overflow in signing is unreachable by proof. Abandoning the attempt and drawing again is then the defence if that proof is wrong, and nothing more. A retry caused by an overflow that depends on a secret is a channel: how often the signer retries would show something of the key. Until both obligations are met, the ledger holds this as open (O33).

#### 2.1.3 The two types

A public integer, `Int<N>`, and a secret integer, `SecretInt<N>`, of `N` limbs each. The crate proposed is `prism-int`.

**What they share.**

- **Sign and magnitude.** `N` limbs of 64 bits hold the magnitude, least significant first, and a sign is held beside them. The range is symmetric, `−(2^(64N) − 1)` to `2^(64N) − 1`, so negation and the absolute value cannot overflow, and zero has one representation, with the sign of a non-negative number. (Two's complement has one more negative value than positive, and negating it overflows.) In the secret type the sign is a secret bit. An addition does not look at the signs to choose its work: it computes the sum of the magnitudes and both orders of their difference, and selects among the three in fixed time. After every operation the sign of a zero magnitude is set to non-negative by a mask and without a branch; otherwise a negative zero would be unequal to zero.
- **Not `Copy`.** A value is 208 to 520 bytes, and a secret one must not be duplicated by an assignment. Both are `Clone`.
- **Wiped on drop, both.** See below.

**The public type** has the variable-time operations of 2.1.4, `PartialEq`, `Eq`, `Ord` and `Hash`. An operation whose result does not fit returns an error and no value.

**Neither type prints a value.** A declassified secret lives in `Int<N>`, and an `assert_eq!`, an `unwrap` or a `{:?}` on an error path would print it. So `Debug` of both types prints the type's name and no value, neither has `Display`, and no error of either type carries a value. A value the caller knows to be public (a constant, a test's operand) is formatted by a method with a name of its own, `Int::public_hex`, which the secret type does not have.

**The secret type.**

- A `Debug` that prints no value (above), no `Display`, no `PartialEq`, `Eq`, `Ord` or `Hash`, no `AsRef`, `Deref` or `From` into the public type, no serialization, and no method that formats its value.
- Its operations are those whose time depends on `N` alone: addition, subtraction, negation, schoolbook multiplication, shifts by a public amount, selection between two values by a secret bit, and comparison and equality that return a secret bit. Stage 2c adds secret bit length and shifts, fixed 128-by-64 division, signed word multiplication, explicitly modular low-magnitude operations, exact width conversions, and secret predicate composition. A secret bit is a type of this crate, `SecretBit`. It is not a `bool` and does not convert to one, because a caller who may branch on comparisons with public values recovers the integer one bit at a time.
- **Overflow, without a branch on the value.** A secret integer carries one more secret bit: *overflowed*. An operation whose true result does not fit sets it, and every operation passes it on from its operands. Multiplication forms all `N²` limb products and sets the bit if any of them lands above limb `N`. A `SecretBit` carries the mark too: a comparison or an equality of a marked integer gives a marked bit, and a selection or a swap by a marked bit marks what it returns. A selection passes on the mark of the value it selects and not of the other: the result is a copy of one operand, and it is as good as that operand. Bytes that do not fit `N` limbs set the mark as well: the secret type has no constructor that returns an error. The prototype also uses `mark_if` to fold a secret admission failure into the same mark. The mark is read at the exits below and nowhere else, and **each of the three** returns an error and no value when it is set. So a secret computation runs the same instructions whether it overflows or not, and a caller can obtain neither a wrapped value nor a clean bit computed from one. Step 2 has a test for each operation that the mark is passed on.
- **The error is itself a disclosure.** To learn that an overflow happened is to learn something about a secret. It has a treatment like any other exit: the one expected is **U**, unreachable by proof, once Q4 and Q5 hold (see "bounds not proven"). Until then each exit's row says so.
- **Three exits, each with one name, and no other.** `SecretInt::declassify` gives the public integer: every operation the secret type does not have (general division, a greatest common divisor, inversion, a square root, a valuation) is reached through it. `SecretBit::declassify` gives a `bool`: every branch on a secret comparison is reached through it. `SecretInt::export_le_bytes` writes the magnitude into a buffer the caller owns and the sign into a byte beside it: the secret key's encoding (stage 5) and a scalar handed to the curve layer (stage 4) are reached through it. It has a second refusal, which step 2 found it needs: a buffer shorter than the magnitude (`SecretError::DoesNotFit`). Cutting the value short would be a silent truncation, and requiring a buffer of the full width would make every caller cut it short themselves. That refusal shows something of the secret too, and it is treated as the overflow error is: the row's last column covers both. CI counts the call sites of the three names outside the crate and requires the count to equal the number of rows of the table below (`scripts/ci/check_secret_exits.py`; the count is by method name, and `declassify` is one name for two of the three). The brief asked for one conversion with one name; the other two are here because without them a secret would leave by a door the table does not count.

**The secret-treatment table.** One row for each call site of an exit: the file and function, which exit, what leaves, why the operation needs it, and the treatment it gets at the last stage (G4), one of four, and the treatment of its overflow error:

- **F**: a fixed-time routine replaces the variable-time one;
- **I**: an argument that the value, or what the routine's time shows of it, is independent of the secret key and of anything a signature does not already show;
- **B**: blinding, so that the declassified value is a masked one;
- **E**: export, for `export_le_bytes` only: the bytes go to a layer that holds them as secrets, and that layer's section says how.

And for the error an exit returns when the overflow mark is set, **U**: a proof that the mark cannot be set at that call site, and, for `export_le_bytes`, that the value fits the buffer it is given.

| # | Stage | Call site | Exit | What leaves | Operation | Treatment at the last stage | Its overflow error |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 2c fixture | `stage2c/driver.rs::check_bit` | SecretBit::declassify | Predicate on public oracle data | Compare the basis, form and half-tie control | I, public fixture only | Fail the fixture |
| 2 | 2c fixture | `stage2c/driver.rs::run` | SecretInt::declassify | Refusal on an invalid public input | Check every returned coordinate and form entry | I, public fixture only | Expected on this negative control |

Signing accumulates admission failures without a new exit. At the existing signature exits, an invalid invariant fails the operation. Only key generation may draw another candidate. Decoding must establish left-order stability, full span, Q4 and the same factor-256 guard before a key can supply an intersection. Use the Stage 2b canonical ideal HNF as the comparison target and verify closure under the maximal order generators. A Gram determinant alone certifies neither the source ideal nor this closure.

**Both types are wiped, and why this is more than the brief asked.** The brief asks that the secret type be wiped on drop. But in the first signer a declassified value is still a secret: it is a copy of one, held in the public type so that a variable-time routine may take it. If the public type were not wiped, S6's "wipes its secrets" would fail at every row of the table. So the proposal is that `Int<N>` is wiped on drop too. The cost is one pass over the limbs at each drop. A wipe on drop clears the place a value was dropped from. It does not clear the bytes a move left behind on the stack, and without `unsafe` nothing here can; step 2 states what its tests can show of this and what they cannot. What then separates the types is what S6 and plan 2.4 ask of them: which operations exist, and what may be printed or compared.

**What is not claimed.** This part does not claim constant time. The secret type's operations are *shaped* for it: no branch and no index on a secret limb is written. Whether the compiled code keeps that shape is measured at the last stage (ledger O6), where the claim is made or not.

#### 2.1.4 Operations and conventions

An operation with no consumer is not built. The consumers are the later stages, named as the reference names the work: each row gives the reference function that needs the operation on our path (1.2), read at the pin.

"Replay" marks a convention that a value compared with the reference in stage 5's replay depends on. The others are ours to choose.

**On both types** (fixed time in the secret type; the public type's may be variable):

| Operation | Convention | Consumer |
| --- | --- | --- |
| From a `u64` or an `i64`; zero and one; the secret type from a public value | They always fit | constants; every stage |
| From little-endian bytes of the magnitude | Public type: a value that does not fit `N` limbs is an error. Secret type: it sets the overflow mark. The public type is also made from little-endian limbs and a sign | stage 3 (sampling), stage 5 (the secret key) |
| Addition, subtraction, negation | Exact, or overflow | every stage |
| Multiplication | Schoolbook, exact, or overflow | every stage |
| Shift left by a public count | Exact, or overflow if a set bit leaves | powers of two (`ibz_pow` is called only with base 2) |
| Shift right by a public count | Of the magnitude: rounds toward zero, as `ibz_div_2exp` | 2c (`quat_lideal_reduce_basis` halves a diagonal) |
| Selection, and swap, by a bit | Secret type only: by a secret bit, fixed time. The public type needs neither: a branch and `core::mem::swap` | 2c, stage 3 |
| Equality, and comparison | Secret type: return a secret bit. Public type: `Eq`, `Ord` | every stage |

**On the public type only** (variable time):

| Operation | Convention | Replay | Consumer |
| --- | --- | --- | --- |
| To little-endian bytes of a stated length | Of the magnitude; an error if it does not fit. How a sign is encoded is the caller's format. (The secret type's is its exit, `export_le_bytes`, 2.1.3) | — | stage 5 (the secret key), stage 4 (scalars for the curve) |
| Sign, is zero, is odd, the low `k ≤ 64` bits of the magnitude | The low bits are of the magnitude, with the sign given separately. The reference's `ibz_get` returns the low 31 bits with the sign, and its callers then take `& 1`, `& 3` or `& 7`, which for a negative value in C's two's complement is the residue modulo 2, 4 or 8. Ours offers that residue directly: "modulo `2^k`, non-negative" (`residue_pow2`), beside the low bits of the magnitude (`low_bits`). A count above 64 is an error | Yes: `quat_qlapoti_check_mod_condition`, `get_endtype`, `quat_elem_is_odd_norm` (`qlapoti.c`) branch on these residues | stage 3 |
| Bit length | Of the magnitude; 0 for zero | — | 2c (if the reduction scales by a length), stage 3 |
| Division with remainder, truncating | Quotient rounded toward zero, remainder with the sign of the dividend (`ibz_div`, GMP's `tdiv`). Division by zero is an error | Yes: `quat_change_to_O0_basis` and the Euclidean loop of `ibz_cornacchia_prime` divide possibly negative values this way | 2b, 2d, stage 3 |
| Division with remainder, floor | Quotient rounded down, remainder with the sign of the divisor (`ibz_div_floor`) | Yes: `quat_qlapoti` (the target `m`); not in the normal form, whose result is canonical | 2b, stage 3 |
| Remainder, non-negative | In `[0, |m|)` whatever the signs (`ibz_mod`, GMP's `mpz_mod`) | Yes | 2b, stages 3 to 5 |
| Remainder, centred | In `(−m/2, m/2]` for `m > 0` (`ibz_centered_mod`: zero is 0, and the half is positive) | No: it is internal to the normal form | 2b |
| Division, rounded to nearest | The quotient nearest `a/b`, and a tie goes **toward zero** (`ibz_rounded_div`, `qlapoti.c`: it corrects the truncated quotient only when twice the remainder is strictly greater than the divisor) | Yes: the closest-vector step and the dimension-2 reduction of `quat_qlapoti` | stage 3 |
| Exact division | An error if the remainder is not zero | — | 2c (the Gram matrix divided by the norm), 2d |
| Greatest common divisor | Non-negative; `gcd(0, 0) = 0` | — | 2b, 2d, stage 3 |
| Extended greatest common divisor | `g = u·a + v·b`, `g ≥ 0`, with the cofactors of the Euclidean algorithm on the magnitudes, each given its operand's sign, so `|u| ≤ |b|/g` and `|v| ≤ |a|/g` when neither operand is zero (when `b` is zero they are `(±1, 0)`, which the first draft's bound did not allow for). The reference adds a normalization of its own, `ibz_xgcd_with_u_not_0`, on top of GMP's | No (**Derived**): the cofactors are used only inside the normal form, and the Hermite normal form of a lattice of full rank is unique, so its entries do not depend on them | 2b |
| Inverse modulo `m > 1` | In `[1, m)`; **none**, as a value the caller must handle, when `gcd(a, m) ≠ 1` (`ibz_invmod` returns 0 then) | Yes | stage 3 (`quat_qlapoti`), stages 4 and 5 (the inverses of `d₁` and of `s` modulo `2^e`) |
| Square root, floor | `⌊√a⌋` for `a ≥ 0`; an error for `a < 0` (`ibz_sqrt_floor`) | — | stage 3: `⌊√p⌋` in `quat_qlapoti`, once a set, which may become a constant |
| Square root, exact | The root, or none when `a` is not a perfect square, a negative `a` included (`ibz_sqrt` returns 0 and writes nothing then) | Yes | stage 3 (`ibz_cornacchia_prime`) |
| Jacobi symbol `(a/n)` | For `n` odd and positive; an error otherwise. −1, 0 or 1, and 0 exactly when `gcd(a, n) ≠ 1`. The reference's `ibz_legendre` is GMP's Legendre symbol, called once, with a probable prime below; the two agree there | Yes: one call, `ibz_cornacchia_extended` | stage 3 |
| Modular multiplication and exponentiation, modulus odd | Results in `[0, m)`. The modulus has at most half the width's bits, so that a product of two residues fits; a wider modulus is an error. The largest modulus on our path is `D`, of `2W + 1` bits | — | stage 3: the modular square root, and the primality test |
| Two-adic valuation | The number of trailing zero bits; **none** for zero. (The reference's `ibz_two_adic` has no caller. For zero, GMP's `mpz_scan1` returns the largest `unsigned long`, which the reference returns as an `int`: −1) | — | stage 3 (the strong probable-prime test) and stage 4 (the content of an endomorphism, which the reference removes by repeated halving, `id2iso.c`) |

**Not in this crate, and why:**

- **The modular square root** (`ibz_sqrt_mod_p`) is stage 3's, built on modular exponentiation here. Replay depends on **which root** it returns. In the reference: for a modulus that is 3 modulo 4 it is `a^((m+1)/4)`; for 5 modulo 8 it is `a^((m+3)/8)` when `a^((m−1)/4)` is 1, and `2a·(4a)^((m−5)/8)` otherwise; for 1 modulo 8, Tonelli–Shanks with the least non-residue from 2 up. For `a` divisible by the modulus it writes 0 and reports **no root**, so the sampler that calls it draws again (`intbig.c`, `ibz_sqrt_mod_p`). Both moduli of ideal sampling are 3 modulo 4 at every set: `D` is, at all five (**Derived**: each is `2^(2W)` plus 75, 183, 183, 643 or 75, 1.7, and each of those is 3 modulo 4), and `q = 2^(2e−5) − s²` with `s` odd is 7 modulo 8 (**Derived**). The other two branches are reached from Cornacchia, whose primes are 1 modulo 4.
- **Random integers** (`ibz_rand_interval` and its three relatives) are stage 3's. Replay depends on how bytes become an integer, draw by draw. From this crate they need: an integer from little-endian bytes, the bit length, a mask to that length, comparison, addition.
- **Primality** (`ibz_probab_prime`, item 20) is stage 3's. From this crate it needs: modular multiplication and exponentiation, the two-adic valuation, the Jacobi symbol, a remainder by a small integer, the exact square root. Whether it is the reference's randomized test or the verifier's Baillie–PSW is that stage's decision (1.5).
- **`ibz_pow`** with a base other than 2, **`ibz_pow_mod`** as a public function, **`ibz_mod_ui`**, **`ibz_two_adic`**, the string conversions and `ibz_conditional_assign`: no caller on our path (**Read**: `grep` over `src`, tests and benchmarks excluded).
- **Rationals** (`ibq_t`, `rationals.c`) and the floating-point type of the reduction (`dpe.h`): 2c decides whether it has either.

#### 2.1.5 The existing integers

`prism-verify/src/prime/mod.rs` has a private natural number, `Nat<W>`, for the verifier's Baillie–PSW test of the challenge degree: unsigned, of the width `ParameterSet::Degree` names, with Montgomery multiplication, a Jacobi symbol by a small integer, and loops that are all counted (**Read**).

Could this layer serve it later? In part.

- The storage and the ring operations of `Nat` are a subset of the public type's.
- Its Montgomery arithmetic and its test are what stage 3 needs for item 20, if stage 3 takes the deterministic test (1.5). Then there would be two implementations of one test, at two widths, or one.
- Against merging: the verifier's file is inside the reviewed verifier, pinned by operation counts and by the mutation record, generic over a width trait of `prism-field`, and it has no dependency. The new crate brings a wipe on drop the verifier does not need.

Not merged in this part, and not decided. Ledger O34 holds the question for stage 3's section.

#### 2.1.6 What the crate is (step 2 of this part)

**Ran** here means run on a build VM, x86_64, with Rust 1.97.1.

- **`prism-int`**: generic in the number of limbs, with one workspace dependency, `prism-ct-barrier`. `forbid(unsafe_code)`, `deny(missing_docs)`. Only the audited barrier precedes it in `scripts/ci/check_no_upward_edge.py`; it is in the research-boundary check; and the front crate re-exports it as `hazmat::int` behind a feature, `signer`, that is off by default (Rick's ruling, 2026-11-22). A default build of the front crate is the verifier alone: a node that only verifies compiles no signer code and none of its dependencies. `scripts/ci/check_consumer_build.py` holds that: the feature table is exactly `default = []` and `signer = ["dep:prism-int"]`, the default build graph holds none of `prism-int`, `zeroize` and `prism-ct-barrier`, and the graph with the feature holds all three. Signing and key generation will be behind the same feature. CI builds, lints, documents and tests the front crate in both configurations.
- **Dependencies.** `zeroize` 1.8.2 writes the zeros of the wipe so that the compiler may not remove them; it was already in the workspace's lock file. The register identity in `prism-ct-barrier` makes selection masks opaque on the two audited signing architectures. In the tests only: `num-bigint` 0.4.6 with `num-integer` and `num-traits`, the oracle. `cargo deny check` passes with all of them (**Ran**).
- **The limbs.** Carries are sums widened to `u128`. Every limb function (`limbs.rs`) does the same work whatever its operands hold, and the secret type is built on those functions alone: it calls no routine of the public type. A product is not held in `2N` limbs: every limb product that lands at or above limb `N` is gathered into one limb that is zero exactly when the product fits.
- **Division** is Knuth's algorithm D on limbs. Its correction of an estimated quotient limb is a loop counted to two, which is all the algorithm can need (each lowering adds the divisor's top limb, at least `2^63`, to the partial remainder, which then leaves its limb). A `while` would be the same on correct code and can spin for `2^64` rounds on a mutated one.
- **No loop ends on the word of the arithmetic alone** (the review of step 2). Four did: Euclid's algorithm in the greatest common divisor and in the extended one, Newton's iteration for the square root, and the reduction of the Jacobi symbol. Each is now counted to a bound from the width, and returns `IntError::StepBound` if the bound is reached first. Euclid and the Jacobi symbol: `2·BITS` steps, since from the second division on each remainder is below half of the one two before it, and the first divisor is below `2^BITS` (two neighbours of the Fibonacci sequence, the slowest operands, take about `1.44·BITS`). The square root: `BITS` steps, since the iteration starts within a factor of two above the root and at least halves its distance from it at each step, so `BITS/2 + 2` are enough (it takes about `log₂ BITS`). No operand reaches the refusal, so the public functions cannot test it. Each loop is in a private function that takes the count, and the crate's unit tests give it one too small: at one, two and three limbs and at the five widths they find the least count that answers, require the refusal at every count below it, and require that count to be the Fibonacci index less two for Euclid on the two largest neighbours the width holds, and within the bound for the others. A bound of `BITS + 2` in place of `2·BITS` fails that test at every width. Two signatures changed with it: the greatest common divisor returns a `Result`, and the exact square root a `Result` of an `Option`, so that a root that was not found is never reported as a number that has none. The error is a variant of its own and is not folded into another.
- **The surface** is `prism-int/surface.txt`, and `scripts/ci/check_int_surface.py` holds the sources to it and refuses any trait on the secret types that would let a value out.

**Tests** (**Ran**, debug with overflow checks, and release; CI runs both on x86_64 and aarch64).

- **Committed vectors**, `prism-int/tests/data/vectors.tsv`: 10,699 rows, written by `scripts/int_vectors.py` from Python's integers, with `--check` in CI. The file is 4.0 MB and is accepted on two conditions (Rick's ruling, 2026-11-22): it is left out of the crate's package, with the test that reads it (`exclude` in the manifest; `scripts/ci/check_consumer_build.py` reads `cargo package --list`), and it is written again only when an operation or a width is added, since each new copy is 4.0 MB more of history. Stage 2c appends 660 rows at the word, Gram and constructor widths. Every operation at one and two limbs, where the edges are taken in every combination, and at 26, 33, 38, 49 and 65 limbs. The test requires lines of every operation at every width, and a refused sum, difference, product, shift and byte string at every width. The division's lines include, at each of the five widths, operands that reach each of its corrections: the estimate of `2^64`, one lowering and two, the partial remainder leaving its limb, and the add-back. The script follows the crate's division step by step to find them, and takes nothing else from the Rust code. The secret type is run on the same lines.
- **Against an oracle**: both types against `num-bigint` on operands from a fixed stream, at one, two and three limbs and at the five widths.
- **Against the reference**: `scripts/c/int_conventions.c` reads the same vector file and gives each line's operands to the reference's `ibz_*` function for that operation, in the build the exporter uses: 26,299 comparisons over 18 functions, no difference (**Ran**; a step of CI's `c-vectors` job). That covers every convention marked "Replay" in 2.1.4, and `ibz_centered_mod`, `ibz_gcd`, `ibz_sqrt_floor` and the ring besides. The Jacobi symbol is compared with `ibz_legendre` at prime moduli only. So Python, the crate and the reference agree line by line, and the Rust tests link no C.
- **The overflow mark**: a test for each operation that it passes the mark on, from either operand and from both, and that each of the three exits refuses it (`tests/marks.rs`).
- **The secret type's promises** are `compile_fail` documentation tests, each beside a program that compiles and differs from it by the forbidden line alone: without that twin a `compile_fail` test passes on any error, a misspelt name included.
- **The wipe.** What safe code can show: that the wipe leaves zeros in the limbs, the sign and the mark, and that a drop of each of the three types runs the wipe (a counter that exists in test builds only). What it cannot show: the memory of a value after its drop, and what a move left behind on the stack. Scratch values of the division are wiped by their own wrappers.
- **The secret type's intermediates are wiped** (the review of step 2). An addition of secrets holds three magnitudes it selects from, and a product, a shift and a selection hold their results; none was wiped. Every value a secret operation names on its way to its result (limbs, a carry, a borrow, a flag) is now in a private wrapper that overwrites it with zeros when the operation returns, by any path; a selection writes into limbs its caller owns; and a swap copies into its operands in place and wipes the two values it selected. A unit test counts the wipes of each operation (ten for an addition, five for a comparison) and another shows the wrapper's wipe leaves zeros. **Not covered**, and stated in the type's documentation: what a limb function holds while it works and its result before the operation has hold of it (the limb functions are shared with the public type, return by value, and widen carries to 128 bits); the result of an operation in the place it was built before the move to its caller; a sign or a mark passed as an argument, and anything the compiler keeps in a register or spills; and the intermediates of the public operations run on a declassified value, but for the division's dividend. Safe code cannot clear these and the crate forbids the other kind; what the compiled code leaves is not measured.
- **Mutation** (**Ran**, cargo-mutants 27.1.0): 519 mutants in the first run, on two cores in 59 minutes: 432 caught, 17 missed, 17 timeouts, 53 unviable. Each miss and each timeout was removed by a test or by writing the code so that the mutant cannot exist (the notebook entry lists them), and one more miss of the second run likewise. On the final code: 497 mutants, 443 caught, 53 unviable, 1 missed, in 45 minutes (the second run, all six files), and that one removed and `division.rs` run again alone, 119 mutants, 113 caught, 6 unviable. So none is missed, none times out, and `docs/lab/MUTANTS.toml` has no entry for this crate. The nightly has the crate as a job of its own, `mutants-int`, dealt into four. The brief asked for it to join the layer job; there each of its mutants would run the tests of four packages that cannot reach it, and each verifier mutant would run its tests, so it is separate, and Rick ruled that it stays so (2026-11-22). The shard time is confirmed by the first nightly. After the review of step 2 (the counted loops and the wiped intermediates), the three files it changed were run again, `number.rs`, `secret.rs` and `error.rs`: 194 mutants, 144 caught, 50 unviable, none missed, none timed out, in 27 minutes on two cores. The crate's unit tests now run before the oracle's, which changes what catches a mutant of the other files first, so the whole crate was run again on the final code: 529 mutants, 455 caught, 74 unviable, none missed, none timed out, in 97 minutes with two at a time on a loaded VM.

**What the code showed of this section.** Four statements of 2.1.3 and 2.1.4 were incomplete, and are corrected above where they stand: the export needs a second refusal (a buffer shorter than the value); the cofactor bound of the extended greatest common divisor fails when an operand is zero; a selection passes on the mark of the value it selects, not of both; and the secret type has no constructor that can fail, so bytes that do not fit set the mark. No width and no convention changed.

The Stage 2c fixture exits are listed in 2.1.3 and counted by `scripts/ci/check_secret_exits.py`.

### 2.3 Gaussian-integer basis reduction (Stage 2c, design for review)

The reduction returns a basis of the same left ideal and a quadratic form for evaluating a linear combination of its columns. Key generation samples combinations of those columns. Shortest-equivalent-ideal construction needs the first column to attain the shortest nonzero norm. The mathematical oracle below uses variable-time integers on public fixtures. The exact rule defines the oracle. The production candidate is exp64 with the native schedules below. O33 remains open for the operand widths of the port and the construction of its admitted inputs. G7 also requires compiled-code evidence.

#### Native prototype boundary

The signer-feature example `stage2c` takes public fixtures at the output of the intersection layer. At each set the example and its compiled-code probe call Rust arithmetic directly through `prism-int`. The experimental end-to-end `prism-signer` uses a separate pinned C reference process. Generic HNF inputs have a shape-specific constructor. The guarded intersection uses all four entries of the Hermitian form. Both constructors verify exact division and the normalized determinant. In this normalization the raw numerator Gram is divided by `2T`, so its determinant changes from `4pT²` to `p`. The corresponding determinant in reduced-norm units before normalization is `pT²/4`.

The public class selects both the bounds and the schedule. Guarded pre-reduced signing is the native target, with 533 steps at P500. Generic HNF signing uses 546. The fixture driver also runs the PR #45 baselines of 594 and 609 for differential tests and paired measurements. The Gram and coordinates use 11 / 14 / 16 / 21 / 27 limbs across the sets, with double those widths in the constructor. The example instantiates each parameter set at its own widths and checks its reviewed schedule against the exact count oracle in CI. The original `stage2c_p500` entry still runs the P500 slice. Its narrowings verify representability except for explicitly modular operations in the Hensel inverse and quotient. Signing keeps a failed admission bit secret, propagates it to every returned value, and fails closed at the exit. No per-key inverse is cached.

The five-set prototype now has native x86-64 and ARM64 timing samples and a linked-code stack inspection. The stack scan finds residual input words after return, so production spill erasure remains open. See the [measurement report](benchmarks/stage2c/README.md). The end-to-end key producer and decoder remain outstanding. Both architecture jobs exercise secret shifts, bit length and fixed word division under QEMU. The x86-64 job also taints every input coordinate and the norm across the complete constructor and reviewed schedule on the public corpus. Hardware latency, coverage of other builds, and integration remain separate G7 obligations. [Width and admission derivations](https://github.com/Shekyl-Foundation/shekyl-prism/blob/35c59c6cca8c9d97701395802a308c9df6e06d01/research/stage2c-design-notes/NOTES.md)

#### Input contract with the lattice and ideal layers

Use the numerator coordinates of 2.1.2 throughout. Write `J(a,b,c,d) = (−b,a,−d,c)` for left multiplication by `i`. An ordered pair `U,V` represents the four columns `U, JU, V, JV`, each with denominator two. The caller supplies the parameter set and the positive ideal norm `T` it already carries under Q2. It must establish that these columns form a full basis of its ideal over the integers. Membership of the two columns alone permits a proper sublattice. Numerators in O0 satisfy `a ≡ d` and `b ≡ c` modulo two.

For a generic odd-norm ideal, the interface includes the basis `U=(2T,0,0,0)`, `V=(x,y,1,0)` from the HNF, with `x` even and `y` odd and both in `[0,2T)`. An HNF of another shape requires a spanning Gaussian pair and a bound for its starting norms. Its first and third columns cannot be assumed to be that pair. Stage 2b/2d owns this conversion and its width argument. With a pre-reduced key representation, the analogous obligation belongs to its constructor and decoder. For signing, Q4 gives `N < 2^B` and `T=Nq` with `q < 2^(2e−5)`. Key generation's starting norm is the separate item 33. The reducer takes typed internal inputs. Raw key validation belongs at import. The set and operation (e.g. signing or key generation) may select the public schedule. Ideal shape, norm, basis and quotients derived from a secret are secret.

#### Exact step and terminal state

Define `D(X,Y) = X₀Y₀ + X₁Y₁ + p(X₂Y₂ + X₃Y₃)` and `N4(X)=D(X,X)`. Its unit is four times the quaternion's reduced norm. Initially swap the pair precisely when `N4(V) < N4(U)`. Each logical exact step has these semantics:

1. Set `A=N4(U)`, `h₀=D(V,U)` and `h₁=D(V,JU)`. Full rank implies `A>0`.
2. For both components take `mⱼ=floor(hⱼ/A + 1/2)`. Ties go toward positive infinity. In particular `+1/2 ↦ 1` and `−1/2 ↦ 0`.
3. Replace `V` with `V − m₀U − m₁JU`. Swap precisely when the new `N4(V) < A`. Equal norms keep their order.

The subtraction and swap are invertible changes of Gaussian basis. At the fixed point `N4(U) ≤ N4(V)` and `−A ≤ 2hⱼ < A` for both updated inner products. Another exact step then leaves both columns unchanged. Within a deterministic run these conventions fix the output for its input basis. Different starting bases of one ideal can choose different shortest vectors of equal norm.

**Arithmetic width of the exact step.** Let `M` be the larger initial numerator norm. The exact Gaussian quotient minimizes the norm over all Gaussian multiples of the first vector, including zero. After the Gaussian multiple has been subtracted, both norms remain at most `M`, regardless of how many swaps occur. Cauchy–Schwarz bounds the sum of the absolute weighted products in either inner product by `M`. With `A=N4(U)`, each rounded component has magnitude at most `√(M/A)+1/2`. A product of that component and a coordinate of `U` or `JU` is therefore at most `3√M/2` in magnitude. Compute the new coordinate by subtracting one product and then the other. The first partial difference is at most `5√M/2`, and the final coordinate is at most `√M`. The norm-form numerator is at most `2M`, and the denominator `4T` is at most `M` for an integral ideal, since every nonzero element norm is a positive multiple of `T`. Thus `3M` bounds these mathematical intermediates, including when `M=1`. Applying that formula to a concrete fixed-width implementation requires the constructor's bound on `M` and a check of the limb operations in their actual evaluation order.

#### Production candidate and public schedule

Choose **exp64**. For `k=max(0,bl(A)−64)`, set `n=A >> k` and `eₕ=max(bl(|h₀|),bl(|h₁|))−bl(A)`, with `bl(0)=0`. Use the same exponent for both components.

- For `eₕ ≤ 60`, form `m=floor((2 floor(h/2ᵏ)+n)/(2n))` per component. Compute `X=2h+A−2mA`. Decrement the estimate for `X<0` or increment it for `X≥2A`. The result equals the exact half-up quotient, including for negative ties.
- For `eₕ > 60`, set `s=eₕ−60`. For each component set `qtop=floor((|h| >> (k+s))/n)` and use the signed multiple `sign(h)·max(qtop−1,0)·2ˢ`. This undershoots, leaves the first vector in place, and decreases the exponent by at least 58. Rounding commutes with subtracting these Gaussian integers. The next window step completes the same logical step as the exact oracle.

Execute both paths and select by a secret mask. Keep lengths and shift amounts inside secret arithmetic. The additions to `prism-int` are a secret magnitude bit length, masked left and right shifts by a secret count, fixed-iteration 128-by-64 division, and multiplication of an N-limb value by a secret scalar. The window calculation uses a floor shift on signed values; the large-quotient calculation shifts magnitudes. Bound the values used on both paths, including discarded correction products and shifts, and propagate the existing arithmetic marks correctly. Compute the exact same rounding with the 64-bit divisor `n`, for example as `floor((floor(h/2ᵏ)+floor(n/2))/n)`; `2n` itself can require 65 bits. G7 review includes release assembly, trace comparison, secret-dependent memory and branch checks, and timing on the supported targets. The research bench measured about 1.17 times the prototype cycles at P500 under the research schedule. That timing and its static assembly scan leave G7 open. The old clamp fails the P500 fixture and making saturation an abort introduces a reachable key-dependent exit. [Progress proof and measurements](https://github.com/Shekyl-Foundation/shekyl-prism/blob/2b10431bef686b8ebfe4f5bfbaccf86062a58f8e/research/clamped-quotient-progress/REPORT.md)

**Native input normalization.** In numerator units the Hermitian determinant is `Δ=A·C−h₀²−h₁²`, where `C=N4(V)`. Substitution of the generic O0 columns gives `Δ=4pT²`. More generally, the O0 numerator lattice has index four in the integer coordinate lattice, and a norm-T ideal has index `T²` in O0. Thus `|det B|=4T²` for a full basis. The real Gram determinant is both `p²(det B)²` and `Δ²`, giving the same identity for a pre-reduced ideal basis. Dividing all Gram entries by four gives **`Δ=pT²/4` in reduced-norm units**.[^o0-gram-units] The research count generator uses `pT²` in these units for its model-order lattice. The native input requires a factor of four in the upper bound for `Φ₁²/Δ`. The exact-rational contractions, strict swap, and half-up tie rule carry over. [Model count generator](https://github.com/Shekyl-Foundation/shekyl-prism/blob/2b10431bef686b8ebfe4f5bfbaccf86062a58f8e/research/clamped-quotient-progress/progress_check.py) · [O0 starting basis and width bounds](https://github.com/Shekyl-Foundation/shekyl-prism/blob/53e2eb4fdd772ff8a53708ba60314c3dabea9d05/research/q5-width-proof/REPORT.md)

[^o0-gram-units]: On the same O0 basis, the numerator Gram has determinant `4pT²`. Dividing each entry by four for the reduced norm divides the two-by-two determinant by sixteen, giving `pT²/4`. In either convention the certificate ratio `A₀/√Δ` is invariant under this rescaling. Its generic O0 upper bound is `2T/√p` in both conventions, one bit above the model-order bound `T/√p`.

Each cell gives **exact / exp64 steps** after the initial strict ordering, including the final size reduction after the last swap. The signer uses the exp64 count for the admitted class. These scale-free counts apply in O0 numerators, reduced-norm units and the normalized Gram.

| Native input class | P248 | P324 | P376 | P500 | P664 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Signing, generic HNF | 280 / 280 | 360 / 360 | 415 / 415 | 546 / 546 | 719 / 719 |
| Signing, pre-reduced, guard 256 | 267 / 267 | 347 / 347 | 402 / 402 | 533 / 533 | 706 / 706 |
| Key generation, norm D | 208 / 208 | 323 / 323 | 308 / 308 | 411 / 411 | 638 / 638 |
| Secret ideal pre-reduction, Q4 | 22 / 22 | 21 / 22 | 22 / 23 | 21 / 22 | 22 / 24 |
| Challenge alone | 197 / 197 | 257 / 257 | 297 / 297 | 396 / 396 | 526 / 526 |

Use the independently reviewed split at `λ=Δ/A²=2⁻⁹`. The 8,758 cells have maximum quadtree depth 14. Their potential bounds the main phase. The tail has at most four further swaps and the schedule includes the final size reduction. Exp64 charges large substeps to preceding progress. At the head, enumerate every positive `k` with `4^(58k+3) < R_C`, including cases that go straight to the tail. Here `R_C` bounds `C₀²/Δ` and `R₀` bounds `A₀²/Δ`. The standard-HNF-only refinement of the secret-ideal row requires a matching admission contract and a count derivation under this tail split. [Independent certificate review, §§1–4](https://github.com/Shekyl-Foundation/shekyl-prism/blob/c9a92cded9a0ce713570acdca3fa0b90d213e1fb/research/certificate-review/REPORT.md) · [Corrected head enumeration](https://github.com/Shekyl-Foundation/shekyl-prism/commit/df92507)

The native prototype retains this table as its five-set benchmark baseline. The subsequent independent tail proof extends the four-swap bound to λ ≥ 2⁻¹¹ and supports shorter schedules. At P500 the extra reduction would save one of 546 generic steps or one of 533 guarded steps. The general secret-ideal exp64 row still needs a class-specific count. The P664 search found an input requiring 22 steps. [Tail proof, counts and search witnesses](https://github.com/Shekyl-Foundation/shekyl-prism/blob/1d6e0cf/research/tail-proof-and-search/REPORT.md)

For the generic HNF, `R₀=4Tmax²/p`. The bound on `C₀²/Δ` at norm T is `(8T²+p)²/(4pT²)`. Its maximum on the admitted norm interval occurs at an endpoint. For key generation of fixed norm D, use T=D at both endpoints. For guard F=256 and challenge bound Q, the guarded ratios are at most `4F²Q²` and `4F²(1+Q)⁴/Q²` respectively. They follow from the starting columns, carried norm and guard below. The oracle uses an integer upper bound on √p in the guarded C₀ bound.

Key generation and decode both establish Q4 and check the same factor-256 guard for a stored pre-reduced basis, `0<nrd(uₛ)≤nrd(vₛ)≤256√p·N`. In numerator units the upper check is `N4(vₛ)²≤p·(1024N)²`; compare in a width justified for the accepted input encoding. Verify the ideal, its span and the basis invariants at construction or import. A decoder rejects a basis that fails these conditions before selecting the schedule. The operation and key format selecting a row must be public. Factor 256 had zero observed rejections in 10,000 sampled keys per set; valid inputs can still fail the guard.

#### Basis and norm form passed to a consumer

Let `B=(U,JU,V,JV)` in column order. Its numerator Gram matrix is `Wᵢⱼ=D(Bᵢ,Bⱼ)`. The upper-right two-by-two block has rows `(h₀,−h₁)` and `(h₁,h₀)`. The integral class norm form for the native consumer is lower triangular:

- `Fᵢᵢ = Wᵢᵢ/(4T)`.
- `Fᵢⱼ = 2Wᵢⱼ/(4T)` for `i>j`, and zero for `i<j`.

For every integer coefficient column `c`, `cᵀFc = N4(Bc)/(4T) = nrd(Bc/2)/T`. For an integral ideal of norm `T`, element norms are multiples of `T`; evaluating on one column and on each pairwise sum gives the exact divisions above. A remainder is an invariant failure. In the pinned C code, `quat_lideal_class_gram` divides the doubled numerator Gram by `denom²·T`. After reduction, `quat_lideal_reduce_basis` multiplies back by `denom²`, halves the diagonal and clears the upper triangle. Thus with the same basis over two, C's returned triangular form is `4F`. Differential tests using different bases must apply the corresponding change of coordinates before comparing values. [Pinned C class Gram](https://github.com/mariascrs/PRISM_v2/blob/87fad7da5f12f79112521677f4f5530672afe9ba/src/quaternion/ref/generic/ideal.c#L237) · [Pinned C reduced form](https://github.com/mariascrs/PRISM_v2/blob/87fad7da5f12f79112521677f4f5530672afe9ba/src/quaternion/ref/generic/lll/lll_applications.c#L6)

#### Executable acceptance cases

At toy prime `p=3` with ideal norm `T=5`, start with `U=(10,0,0,0)` and `V=(4,9,1,0)`. The first step subtracts `iU` and swaps. The new first vector `(4,−1,1,0)` already has the minimum numerator norm 20. The second vector still has projections `2` and `+1/2` on the first vector and its rotation (inner products divided by 20). The second step subtracts the multiple `2+i`, producing `(1,−2,−2,−1)`, also of norm 20. Equal norms preserve the order. A count of one swap therefore requires two active steps on this input.

Run `python3 -B -S scripts/stage2c_oracle.py`. It recomputes each schedule from the class bounds `A₀`, `Δ` and `C₀`. Exact reduction and exp64 are one Gaussian step: the quotients differ, and the driver takes a step count. For the variable-time exact oracle, let A₀ be the first numerator norm after initial ordering. Every swap strictly lowers this positive integer, so at most A₀−1 swaps and one final size reduction are possible. The oracle checks that the norm decreases at each reported swap and rejects a zero first norm. The P500 fixture also runs each published column as that count. The oracle checks both exp64 quotient paths, and it requires every in-window quotient on a six-bit box and on divisors wider than 64 bits to equal the exact half-up rounding. The window correction is the section's `X`. It also checks signed half ties, strict swaps, lattice span and the norm form on explicit combinations. Tiny lattices admit an exhaustive shortest-vector check. All five field weights are exercised. The P500 input in `scripts/fixtures/stage2c_p500.json` comes from the pinned width study's `crafted_saturation.py` (seed 77). It includes the model-order lattice where the bounded quotient stalls and the associated O0 numerator lattice. The two minimum values were computed independently with PARI/GP 2.17.2. For the input columns B, form `G=Bᵀ diag(1,1,p,p) B`, compute `L=qflllgram(G,1)`, then run `qfminim(LᵀGL,,1,2)` at 1000 decimal digits and evaluate the returned minimizing vector in the exact integer form. The model-order minimum is in integer coordinates; the O0 minimum is in numerator units.

The Rust implementation must agree with these conventions, preserve each lattice, obey the certified schedule, and provide release-build timing, control-flow and secret-address evidence on the targets it supports.
