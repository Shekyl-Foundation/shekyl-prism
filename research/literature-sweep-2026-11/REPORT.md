# Literature sweep, November 2026: what applies to PRISM and the research lane

Date: 2026-11-26 (America/New_York). Branch `exp/literature-sweep-2026-11`
off `research` @ `113312c`. Requested by Rick (t54u), widened to pure
mathematics and other areas of cryptography (t55u).

PRISM is ePrint 2026/443 (Basso, Borin, Castryck, Corte-Real Santos,
Invernizzi, Leroux, Maino, Vercauteren, Wesolowski). Shekyl uses `p500`.
Nothing here is a constant-time or security claim, and nothing here changes
`dev`.

## How this was done, and its limits

- Web searches covered PRISM and its variants, SQIsign and its constant-time
  work, ideal-to-isogeny and norm equations, dimension-2 and dimension-4
  reduction, HNF, constant-time gcd and inversion, big-integer arithmetic,
  timing and stack-residue testing on ARM, quaternion-order algorithms, sums
  of squares, binary-form reduction, lattice signatures (Falcon, HAWK) and
  class-group work (CTIDH, binary quadratic forms).
- **Every item below was opened this session.** ePrint items were read from
  their abstract pages. **No ePrint PDF could be read.** The box's IP is
  blocked by eprint.iacr.org (its page says the block is for automated
  fetches; I sent one burst of about 20 requests before I saw it, then
  stopped). The fetch service gets the abstract pages but a 403 on the PDFs.
  Statements below that go beyond an abstract are marked *(search excerpt)*:
  they come from search-engine excerpts of the PDF. They have to be checked
  against the paper before anyone relies on them.
- Non-ePrint sources were read in full or in part. These were the
  EUROCRYPT 2026 safegcd paper (PDF from the author's page), the WOOT'25
  Cortex-A72 paper, Pollack–Treviño, arXiv 0707.0644, arXiv 2509.04010, the
  `class-groups` crate's source, the TCHES zeroization page, the LLVM RFC,
  the NIST and HAWK pages, and the USENIX RTLF page.
- Items already in our records (ePrint 2026/1486, 2026/1821, 2026/906,
  2026/2312, 2026/1169, 2025/1602, 2025/2192, 2025/027, 2026/1700,
  2024/1180, 2023/807, arXiv 1806.03113) are mentioned only where something
  new bears on them.
- One dropped lead: a TU Graz "HNF for memory-constrained devices" PDF turned
  out to be a thesis advertisement, not a result.

## Ranked table (by expected value to our experiments or the signer)

| # | Item | Area | Touches | Verdict | Effort |
|---|------|------|---------|---------|--------|
| 1 | Czuprynko, Mukherjee, Sinha Roy, *Tighter and Friendlier Integer Bounds for Quaternion Algorithms – Application to SQIsign*, [ePrint 2026/2049](https://eprint.iacr.org/2026/2049) | isogeny / lattice | `fixed-schedule-reduction`, `lagrange-tour-bound`, `tail-proof-and-search`, `fixed-modulus-hnf(-followup)`, `normalized-gram`, Stage 2b/2d | **Independent corroboration of our two central design choices**: rank-2 Z[i] Lagrange instead of 4-D LLL, and an HNF modulus of 2·nrd(I) with bounded intermediates. Their iteration bound is asymptotic, O(1 + log‖b₂‖ − log λ₁) *(search excerpt)*. Our certified exact counts (533/546 at P500) are the stronger result. Cite it as concurrent prior art. Their exact-division inverse may help Stage 2b/2d. | Read the paper: 0.5 day. Compare constants and lemmas: 1–2 days. |
| 2 | Gilchrist, Lai, Meyer, *Fault Injection Attacks on Torsion Masking*, [ePrint 2026/1944](https://eprint.iacr.org/2026/1944) | isogeny SCA | the signer's threat model (no fault model in `docs/` today) | **A new threat class for any PRISM implementation.** Key recovery from one or two first-order faults on the torsion-masking scalars. It is not a mathematical break and does not touch `p500`. It needs a written fault position and a countermeasure decision (for example, verify before release, or recompute the masking), which is a design round with Rick. | Spec note: 0.5 day. Countermeasure cost: measure it. |
| 3 | Leroux, Schaeffler, *New algorithms for quaternion ideals in SQIsign*, [ePrint 2026/2153](https://eprint.iacr.org/2026/2153), with Leroux, *Efficient quaternion algorithms for the Deuring correspondence…*, [ePrint 2026/185](https://eprint.iacr.org/2026/185) | quaternion algorithms | `stage2c-design-notes` (admission, prism-int widths), `keygen-precompute`, `shortest-equivalent-shortcut`, `q5-width-proof` | A normalized ("inert") representation for p ≡ 3 mod 4 (every Shekyl prime is c·2ᵃ − 1). Concrete O(p⁴) integer bounds. Used in SQIsign 3.0 to drop GMP. **The abstract names PRISM as a direct beneficiary.** It could shrink our width budget and give a canonical key form. It needs a comparison with our N′, U′, V′ shortcut form, and any key-format change needs a design round. | 2–4 days. |
| 4 | Bernstein, Chen, Harrison, Huang, Maxwell, Wang, Wuille, Yang, *Accelerating and verifying constant-time modular inversion*, EUROCRYPT 2026 ([PDF](https://homepage.iis.sinica.edu.tw/~bywang/papers/eurocrypt26.pdf), [ACM](https://dl.acm.org/doi/abs/10.1007/978-3-032-25336-1_21)) | CT gcd | `prism-int` `inv_mod` (now Euclidean xgcd, variable-time), the inverse mod q in the 2d setup, the CRT root | A drop-in fixed-count inverse for any odd modulus, with a HOL Light proof. It needs ⌈(9437b+1)/4096⌉ divsteps instead of Bernstein–Yang's ⌊(49b+80)/17⌋: for q at P500 (b ≈ 995), 2,293 against 2,872. It improves on what we had planned ("e.g. safegcd" in `fsne.py`). | Prototype: 1–2 days, plus mutants. |
| 5 | Anne, *Phantom Timing Leaks on Cortex-M4*, [ePrint 2026/2744](https://eprint.iacr.org/2026/2744); Dunsche et al., *RTLF*, [USENIX Sec '24](https://www.usenix.org/conference/usenixsecurity24/presentation/dunsche); Adjonyo et al., [arXiv 2509.04010](https://arxiv.org/abs/2509.04010) | SCA methodology | `p248-arm-timing` (+ `quiet/`), G7 | **Corroborates our reading of the −6.77 that never reproduced.** Deterministic differences can come from the harness alone. Anne's A/A test (both classes the same bytes) still gave a large \|t\|, and a matched control removed the effect in 7 of 9 cases. Our same-data null contrast already acts as an A/A arm and showed excursions of the same size. What this adds is RTLF, which bounds type-1 error where dudect's static t-threshold does not, plus pre-registration and matched pre-call code. Together these would put G7 on a published, defensible footing. | 1 day on the existing Pi harness. |
| 6 | Kim et al., *Power Analysis on SQIsign via Ideal Reconstruction from Quaternion Leakages*, [ePrint 2026/2508](https://eprint.iacr.org/2026/2508); Mukherjee et al., *Simple Power Analysis Attack on SQIsign*, [ePrint 2025/830](https://eprint.iacr.org/2025/830) | isogeny SCA | `qlapoti-redesign`, `fixed-schedule-norm-equation`, `cornacchia-harness`, G7 | **Reinforces our strictness.** Five round-3 SQIsign signing leaks give polynomial-time key recovery, **even from partial bits**. The SPA attack reads Cornacchia's secret-derived modular-exponentiation exponent. For us the Cornacchia/modular-sqrt step in qlapoti needs a fixed-length, fixed-window exponentiation. A list of sensitive intermediates is worth carrying over to PRISM's signer. | Mapping their five leak points onto our pipeline: 1 day. |
| 7 | Kouider, Mukherjee, Jacquemin, Kutas, *Constant-time Integer Arithmetic for SQIsign*, [ePrint 2025/832](https://eprint.iacr.org/2025/832) (AFRICACRYPT 2025); Kim, Lee, Yoo, *Compact Quaternion Algorithms for SQIsign*, [ePrint 2026/1031](https://eprint.iacr.org/2026/1031) | CT bigint | `prism-int`, `fixed-modulus-hnf` (Barrett), `rust-lagrange-ct` (restoring division) | 2025/832 gives CT Euclidean division, exponentiation, integer square root and **modular reduction for changing moduli**, our Barrett-with-per-call-reciprocal situation, checked with TimeCop. It is a reference to compare against, not a better result. 2026/1031 brings SQIsign fixed precision to 1665/2521/3319 bits; our q5 widths can be checked against its method. | 1 day to compare. |
| 8 | Lai, *Qlapoti+ and More*, [ePrint 2026/1640](https://eprint.iacr.org/2026/1640); Borin et al., *Qlapoti*, [ePrint 2025/1604](https://eprint.iacr.org/2025/1604) | norm equations | `qlapoti-redesign` | 2026/1640 reports PRISM signing 1.47/1.90/1.60× faster than the JoC Qlapoti baseline. These are variable-time optimizations, so their value to us is whether any of them **lowers the worst case** that sizes our fixed budget (the work-unit model). It may not; check each technique. | 1–2 days. |
| 9 | David, Espitau, Hosoyamada, *Quantum binary quadratic form reduction*, [ePrint 2022/466](https://eprint.iacr.org/2022/466), and the Rust [`class-groups`](https://docs.rs/class-groups/latest/class_groups/crypto_bigint/reduction/index.html) crate (`kayabaNerve/trout`, 0.0.2-alpha) | class groups / binary forms | `rust-lagrange-ct`, `clamped-quotient-progress`, `ct-barrier-policy` | **The most interesting idea from outside isogenies.** Gauss reduction with only power-of-two transvections (shift and add, no division). The crate does it in constant time in Rust with crypto-bigint, one call per bit, each call taking at least one bit off \|b\|. A Z[i] analogue (m ∈ {±2ᵏ, ±i·2ᵏ}) could remove our restoring division entirely. It needs a new step certificate (more steps, cheaper each), so it is speculative. | 1 week for a prototype and certificate. |
| 10 | De Almeida Braga, Fouque, Lachguel, Prest, *Toward a Secure Fixed-Point Implementation of Falcon*, [ePrint 2026/1531](https://eprint.iacr.org/2026/1531) (CRYPTO 2026, revised) | lattice signatures | `secret-ideal-rerandomization`, keygen, T_prov sizing | A transferable technique: **keygen-time rejection on a few quantities gives sharp unconditional bounds** for every intermediate (they reject fewer than 50% of keys). The analogue for us would reject keys whose reduced secret basis is badly skewed, to shorten the certified schedule. It changes the key distribution and so needs a design round. | Measurement only: 1–2 days. |
| 11 | Arranz Olmos et al., *High-assurance zeroization*, [TCHES 2024(1)](https://tches.iacr.org/index.php/TCHES/article/view/11256); LLVM [RFC: Function-Boundary Zeroization](https://discourse.llvm.org/t/rfc-function-boundary-zeroization/91693) (2026-08-31) | compilers | `p248-arm-timing` stack residuals, `ct-barrier-policy` | It confirms that source-level zeroize cannot reach spill slots, which is our open stack-word finding. The LLVM RFC proposes backend stack and register zeroization on x86-64, AArch64 and ARM32. Watch it for a Rust front end. In the meantime a caller-side stack scrub is the only option. | Track only. |
| 12 | Pollack, Treviño, *Finding the four squares in Lagrange's theorem*, [Integers 18 (2018)](https://www.pollack-math.net/finding4squares.pdf) | pure mathematics | `cornacchia-harness`, qlapoti's two-square step | A two-square representation as a **Gaussian gcd** (Lemma 1: n \| N(a+bi), gcd(a, b) = 1). Cornacchia is then a rank-1 Z[i] / rank-2 Z reduction, so our cell-map certificate method could give it a fixed schedule. Loosely related. It reframes an existing step rather than giving a new algorithm. | 2–3 days if pursued. |
| 13 | Akhavi, Moreira Dos Santos, *A rewrite based analysis of algorithms* ([arXiv 0707.0644](https://arxiv.org/abs/0707.0644)); Damer, *Computing the HNF: A Survey* ([ePrint 2024/2089](https://eprint.iacr.org/2024/2089)) | pure mathematics / complexity | `lagrange-tour-bound`, `fixed-modulus-hnf` | Background only. These are worst-case Gauss reduction (Vallée's bound extended to determinant ±1) and the DKT87 modulo-determinant and Micciancio–Warinschi HNF. Our fixed-modulus HNF is the DKT approach specialized. Nothing new for us. | None. |
| 14 | Straznickas, Weis, *HAWK-n Key Recovery Reduces to SVP in Dimension n/2+1*, [ePrint 2026/1593](https://eprint.iacr.org/2026/1593); [HAWK site](https://hawk-sign.info/); [NIST round 3](https://www.nist.gov/news-events/news/2026/05/nine-candidates-advance-third-round-additional-digital-signatures-pqc) | lattice signatures | none directly | HAWK was withdrawn on 2026-07-29. The attack finds a public lattice automorphism and descends. **It does not transfer to PRISM**, whose hardness is not a lattice problem. The Z[i] and j structure we use is public and is used only for speed. A cautionary note: structure in a key lattice can be an attack surface. | None. |
| 15 | den Hollander et al., *SPRINT*, [ePrint 2026/364](https://eprint.iacr.org/2026/364) (Asiacrypt 2026) | isogeny signatures | context for G7 | An alternative that is constant time by construction: PIOP plus PCS, relying only on the ℓ-isogeny path problem, with much larger signatures. It states that PRISM and SQIsign "cannot be made constant-time without significant effort". Not a drop-in; context only. | None. |
| 16 | Boyapally et al., *Reality Check on Side-Channels… Cortex-A72*, [WOOT '25](https://www.usenix.org/system/files/woot25-boyapally.pdf) | physical SCA | `p248-arm-timing` host | EM key recovery on a **Raspberry Pi 4B (BCM2711)**, our exact board. Our G7 covers timing, not EM, so this is out of scope unless Rick widens the threat model. | None. |
| 17 | dCTIDH, [TCHES](https://tches.iacr.org/index.php/TCHES/article/view/12226); *Hardened CTIDH* ([arXiv 2509.12877](https://arxiv.org/abs/2509.12877)); Ray, *Dimension-4 SQIsign at Round-3 Parameters* ([ePrint 2026/2221](https://eprint.iacr.org/2026/2221)); CT-KAT ([ePrint 2026/1418](https://eprint.iacr.org/2026/1418)) | class groups, verification, tooling | loose | The dCTIDH work makes CSIDH constant time with fixed public batch schedules, the same philosophy we adopted, but there is no algorithm to borrow. The dimension-4 format is verifier-side. CT-KAT is a C-only tool. | None. |

## Anything that threatens PRISM or `p500`?

- **No new cryptanalysis of PRISM's hard problem was found** beyond what our
  records already carry: ePrint 2026/1486 (Wesolowski), 2026/1821 (Mamah),
  2026/906, and 2025/1602.
- The SQIsign 3.0 primes (about 326/505/669 bits) match our post-1486 table.
  `docs/PARAMETER_SETS.md` already records `p500` as NIST category III
  under SQIsign specification 3.0 §5.2. Nothing found moves that.
- **Implementation-level threats are new.**
  - ePrint 2026/1944: faults on torsion masking give PRISM key recovery
    from one or two first-order faults.
  - ePrint 2026/2508: power analysis on SQIsign's quaternion layer, with
    key recovery from partial leaks.
  - ePrint 2025/830: SPA on the Cornacchia exponent.
- None of these weakens the scheme. They raise the bar for the signer: fault
  handling is not in our spec at all, and the partial-leak results argue
  against any leniency in G7.

## Per-item notes

### 1. ePrint 2026/2049: Z[i] dimension 2 and the 2·nrd modulus

What the abstract says:

- All SQIsign lattices satisfy a containment property, so the canonical form
  can use a modulus of half the previous bit length.
- An inverse using only exact divisions keeps intermediates within twice the
  modulus.
- SQIsign's triangular convention saves a factor O(p).
- These lattices can be represented in dimension 2 over Z[i], so Lagrange
  reduction replaces LLL. Its iteration count is bounded by a multiple of
  the bit size of the longest vector, and it provably outputs a shortest
  basis without floating point.
- The implementation is in the GMP C reference, so it is not constant time.

*(search excerpt)*:

- nrd(I)·Z⁴ ⊂ I gives the modulus 2·nrd(I), with intermediates bounded by
  (2·nrd(I))².
- Lemma 7 bounds termination by O(1 + log‖b₂‖ − log λ₁(L)), with a
  contraction factor 1+η, 0 < η < √2−1.

Against our work:

- This is our C1–C3 fixed-modulus 2N HNF and our Z[i]-Lagrange contract,
  reached independently.
- It neither contradicts nor supersedes us. We have exact,
  machine-checked counts that include the final size reduction (533/546 at
  P500). We have the tail proof (`cellmap.c`, Lemma C), and we have an
  explicit containment criterion, Δ(G) = ∏d_c, with the pivot-product
  erratum.
- An O(·) bound cannot size a fixed schedule. Ours can.
- To do:
  - Cite it in the reduction and HNF reports as concurrent work.
  - Check whether its Lemma 7 constant and our ρ = 537/2000 per step agree.
  - Look at the exact-division inverse for the 2d setup's inverse and the
    Gram division by T in `normalized-gram`.
  - Check whether their "shortest basis, no floating point" claim implies a
    simpler rounding than our exp64 ±1-corrected 64-bit quotient.

### 2. ePrint 2026/1944: faults on masked torsion

PRISM's response publishes torsion images multiplied by a secret masking
matrix. Faulting one scalar during the multiplication gives Psig − P′sig =
[Δ]φ(P0). With a small fault model, two to four candidates per Δ remain, and
φ_sk is recovered and checked by a higher-dimensional representation. They
simulate the attack on deterministic PRISM. The paper discusses the
probabilistic version in its Remark 6 *(search excerpt)*. The code is at
[github.com/michael-meyer/fault-torsion-masking](https://github.com/michael-meyer/fault-torsion-masking).

For Shekyl:

- The salt makes signing probabilistic, but repeated signing of a chosen
  message is still the threat a wallet faces.
- Candidate countermeasures:
  - verify the signature before releasing it (we already have a verifier);
  - recompute the masked points and compare them;
  - check the point orders.
- Which to use is Rick's call.

### 3. ePrint 2026/2153 and 2026/185: the inert, normalized ideal representation

- For p ≡ 3 mod 4, a left O0-ideal of odd norm N is encoded by
  (x + iy + j)/2 with x, y < 2N *(search excerpt, 2026/185)*.
- 2026/2153 generalizes this to a normalized representation with simple
  algorithms and an O(p⁴) worst-case integer bound. That bound rests on an
  experimentally verified assumption about the lattice-reduction step of
  response sampling.
- It is used in SQIsign 3.0 to replace GMP with fixed-size integers.

This is close to our Stage 2c admission and our shortcut formulas, N′ =
A′/2, U′ = (A′,0,0,0) and V′ = (h0′, h1′, Re e, Im e). Comparing the two may:

- give a canonical form with smaller stored integers;
- simplify the det(U,V)/(2T) unit argument;
- tighten `q5-width-proof`.

Any change to the stored key is a format change and needs a design round.

### 4. EUROCRYPT 2026 safegcd

From the PDF:

- Theorem 1: f odd and 0 ≤ g ≤ f ≤ 2ᵇ imply g_n = 0 for n ≥
  ⌈(9437b+1)/4096⌉, starting from δ = 1/2. It is checked in HOL Light.
- The software is under 4,000 Skylake cycles mod 2²⁵⁵−19.

Divsteps for an inverse mod q (b = bits(q) as recorded in `q5-width-proof`
§4.4):

| set | b | Bernstein–Yang 2019 | EUROCRYPT 2026 |
|-----|---|---------------------|----------------|
| P248 | 491 | 1,419 | 1,132 |
| P324 | 643 | 1,858 | 1,482 |
| P376 | 747 | 2,157 | 1,722 |
| P500 | 995 | 2,872 | 2,293 |
| P664 | 1,323 | 3,818 | 3,049 |

- `prism-int::Int::inv_mod` is today an Euclidean xgcd with
  data-dependent iteration. That is fine for the reference replay, but it is
  not a CT primitive.
- For the Jacobi symbol, the secp256k1 safegcd variant has only an empirical
  bound (per search results). Pornin's binary GCD (ePrint 2020/972) is an
  alternative. Neither is verified here.

### 5. Timing methodology: phantom leaks and RTLF

- Anne pre-registered 44 campaigns of 100,000 trials each.
- Fresh valid ciphertexts flagged all nine implementations with differences
  of 4 to 15 cycles. A matched control removed the difference in seven of
  nine. An A/A test still gave |t| = 12,864 for one implementation. The
  differences came from the harness and memory system, not the code under
  test.
- Our `p248-arm-timing` result already found that the same-data null
  contrast (a reference copy, in effect an A/A arm) shows excursions as
  large as the real fixtures. That is Anne's finding on a different core.
- Not checked here: whether the published −6.77 run's two classes ran
  identical code before the timed call. Anne's matched control is the
  test for that.
- RTLF (bootstrap decile test with bounded α) is the published fix for
  dudect's false-positive and power trade-off.
- arXiv 2509.04010 automated TIMECOP, Binsec/Rel2, dudect and RTLF over the
  NIST PQDSS round-1 and round-2 code and reported 26 issues.
- Concrete changes for the next Pi run:
  - keep the null arm;
  - identical pre-call code in both classes;
  - a pre-registered stopping rule;
  - RTLF next to Welch t.

### 6. Power analysis on SQIsign (round 3) and SPA on Cornacchia

- Five leak points in round-3 SQIsign signing, each with a polynomial-time
  algebraic key recovery, including from partial bits, demonstrated on the
  source code.
- The SPA paper recovers the ephemeral exponent of the modular
  exponentiation inside Cornacchia (the square root of −1 mod n), then the
  key.
- For our qlapoti redesign, n is secret-derived, so the square root needs a
  fixed exponent length and a fixed-window ladder. Masking the modulus is
  not enough: our `cornacchia-harness` result already showed masking hides
  nothing.

### 7. CT integer arithmetic and fixed precision for SQIsign

- 2025/832's reduction for changing moduli is the case our Barrett with a
  restoring-division reciprocal handles. It is a cross-check, not an
  improvement.
- 2026/1031 cut fixed precision by about 76%. That is the same exercise as
  our `q5-width-proof`. Their method of uniform worst-case bounds per
  routine is comparable to ours.

### 8. Qlapoti+ (2026/1640) and Qlapoti (2025/1604)

- The Qlapoti abstract names PRISM: ideal-to-isogeny is 2.2 to 2.6× faster,
  with negligible failure and no rerandomization.
- 2026/1640 adds a PRISM-tailored optimization, for signing 1.47/1.90/1.60×
  faster.
- Our redesign is sized by the worst case. The question for each technique
  is whether it lowers the bound on passes or n_min, not the mean.

### 9. Division-free (binary) form reduction

- 2022/466 Algorithm 1 reduces [A,B,C] with transvections S_m, m = ±2ᵏ, so
  each step is shifts and adds only.
- The `class-groups` crate (Rust, crypto-bigint, `Choice`/`CtSelect`)
  implements it as `reduce_to_next_bit`, called once per bit of the bound,
  each call guaranteed to remove at least one bit from |b|. It also shrinks
  the active limb count as values shrink, which is a public, size-dependent
  schedule and a pattern we would have to justify.
- For us, a Z[i] version on the Hermitian Gram would replace the 64-bit
  quotient and the restoring division with a fixed bit-by-bit schedule.
- Costs:
  - The step count becomes about the bit length, much larger than 533.
  - Each step is far cheaper.
  - It needs a new certificate.
- This is the item that could change `rust-lagrange-ct` the most. It is
  also the least certain.

### 10. Keygen-time rejection (fixed-point Falcon)

- Almost every Falcon intermediate is bounded by four quantities fixed at
  keygen.
- Rejecting fewer than 50% of keys on those four gives unconditional
  fixed-point bounds.
- The PRISM analogue: bound the starting skew of the secret ideal's
  Z[i]-reduced basis at keygen, so the signing schedule (T_prov) and widths
  shrink.
- It changes the key distribution. The security cost would have to be
  argued, and it would need a design round.

### 11. Zeroization

- TCHES 2024: zeroization is surprisingly hard to guarantee in C, C++ or
  Rust. They solve it in the Jasmin compiler, with overhead below 2% in
  most cases.
- The LLVM RFC (31 August 2026, active in October) proposes
  `zeroize_on_return`-style backend stack and register clearing on AArch64
  and other targets, building on `-fzero-call-used-regs`.
- This bears directly on our residual stack words. A Rust attribute would
  follow LLVM support, and a Rust internals pre-pre-RFC exists per search
  results (not read).

### 12–17

See the table. They were read for completeness. Each is loosely related at
most, and none changes an experiment.

## Recommended next steps (none started; for Rick to pick)

1. Read ePrint 2026/2049 in full (from a machine whose IP ePrint does not
   block). Write a short comparison note against `tail-proof-and-search` and
   `fixed-modulus-hnf`.
2. Add a fault-attack position to the signer spec, for decision.
3. Re-run the G7 Pi contrast with matched pre-call code and RTLF next to
   the t-test, keeping the null arm and pre-registering the stopping rule.
4. Prototype a divstep `inv_mod` with the 2026 bound in a research crate.
5. Compare the normalized representation of 2026/2153 with our Stage 2c
   admission form.
