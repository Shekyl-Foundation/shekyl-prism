# LatticePrism 4-D hash-expanded embedding: trace experiment

Date: 2026-11-21. Status: exploratory research for Shekyl PRISM. This is a
step-counting model in Python. It makes no constant-time claim and is not
a security argument. The 2-D experiment in `../cornacchia-harness/` is the
negative control.

## 1. Question

Does embedding PRISM targets in a **4-D** lattice whose basis comes from a
**hash expansion** (SHAKE-256 or BLAKE3 XOF of seed, target and a domain tag)
break the target-to-trace dependence seen in 2-D? There, the 2023/807
randomized-basis variant's trace was 85–91% explained by the target.
If it does break, does some other invariant survive? (ePrint 2023/807 was
read from indexed excerpts because the PDF was blocked from the box; it
must be checked against the paper itself before anything here is cited.)

## 2. Setup

**Parameters.** All five repository sets are taken from
`prism-field/src/lib.rs` (`parameter_set!`: e, f) and
`docs/PARAMETER_SETS.md`, with p = f·2^e − 1: P248 (5·2^248 − 1), P324
(3·2^324 − 1), P376 (65·2^376 − 1), P500 (27·2^500 − 1) and P664
(17·2^664 − 1). The challenge degree is q = 2^(2e−5) − s² with s odd and
e − 4 free bits (`docs/VERIFIER_SPEC.md`). P363 and P384 are **not** Shekyl
parameter sets (the repository sets are the five above) and were not run.

**Target streams (600 per set per stream):**
- **T1** consists of prime challenge degrees q of 491, 643, 747, 995 and
  1,323 bits. q ≡ 7 (mod 8), so x² + y² = q has no solution. Classical
  Cornacchia (A) therefore runs RepresentInteger-style: M = q − p(z² + w²),
  with (z, w) from a public PRNG keyed by q, is retried until M is a prime
  ≡ 1 (mod 4), and then Cornacchia is run on M.
- **T2** consists of primes N ≡ 1 (mod 4) whose bit length is uniform in
  [bits(p) − 16, bits(p)], i.e. the norm-equation cofactor scale. A is
  Cornacchia on N directly.

**Lattice.** The quaternion algebra is (−1, −p) with order O = ℤ⟨1, i, j, k⟩
and norm form x² + y² + p(z² + w²). The lattice is the left ideal
I = Oα + O·T with α = x + yi + j and x² + y² ≡ −p (mod T): rank 4,
determinant T² in O's coordinates, reduced with the norm form. The first
LLL vector β satisfies T | nrd(β) and nrd(β)/T ≈ √p. This is the SQIsign
and PRISM "equivalent ideal of small norm" step, the 4-D problem that the
TCHES 2025 constant-time LLL targets. I picked this ideal lattice over a
generic ℤ⁴ or q-ary lattice because it is the only one of these that still
encodes the actual problem. The q-ary forms were run as well, as controls.

| Id | Embedding | Encodes the problem? |
|---|---|---|
| D0 | ideal above, y = smallest value that works: a pure function of T (deterministic baseline) | yes |
| H1 | D0's ideal, basis multiplied by a unimodular U = Perm·L·R from XOF(seed‖T‖"H1"): 128-bit triangular entries and an XOF permutation. The 4-D analogue of 2023/807's mask | yes, same lattice |
| H2 | ideal *chosen* by the XOF: y = XOF(seed‖T‖"H2"‖ctr) mod T, root sign from the XOF. One of the T + 1 ideals of norm T, chosen pseudorandomly | yes, a different valid ideal |
| H3 | q-ary {v ∈ ℤ⁴ : ⟨a, v⟩ ≡ 0 mod T}, a from the XOF, Euclidean norm, covolume T | **no** |
| H3s | same with small modulus 3329, covolume constant | **no**; depends on T only through the hash |

**Seeds.** I ran (a) a fixed seed; (b) seed = SHAKE-256(simulated public key)
with a fresh pk-sized random key, the LatticePrism design; and (c) BLAKE3
in place of SHAKE-256 (H1 and H2, fixed seed).

**Reduction.** Cohen Alg. 2.6.7 integral LLL on the Gram matrix with δ = 0.99,
instrumented to count main-loop iterations, Lovász checks, swaps and
size-reductions. `crosscheck_4d.sage` checks it against Sage's
QuaternionAlgebra and PARI `qflllgram`: 0 mismatches in 500 checks.
The control is a sweep LLL run in fixed tours (6 size-reductions plus 3
Lovász checks and conditional swaps per tour), in the spirit of TCHES 2025.

**Panels:**
- Target panels: 24 targets × 150 simulated public keys, per set and stream.
- Key panel (`run_secret.py`): 20 secret keys × 100 signatures per set. Each
  key is a secret ideal of prime norm N (about bits(p)/2 bits) plus its own
  pk seed. Each signature has a fresh public q, and the lattice is the ideal
  of norm N·q, joined by CRT, with D0's or H2's rule for the q-part.

Under the null hypothesis of no group effect, η² ≈ (K − 1)/(n − 1), which
is 0.0064 for the target panels and 0.0095 for the key panel.

## 3. Results

### 3.1 Trace length per set (T1, iterations, mean ± sd [range]; H\* with seed = H(pk))

| set | log2 q | A: Cornacchia steps (retries) | D0 | H1 | H2 | H3 | H3s |
|---|---|---|---|---|---|---|---|
| P248 | 491 | 142.0 ± 9.3 (787 ± 751) | 231.3 ± 14.8 [181, 280] | 1877 ± 401 [1112, 2715] | 743.2 ± 30.3 [659, 843] | 602.7 ± 25.4 | 22.8 ± 4.3 |
| P324 | 643 | 186.5 ± 10.5 (1097 ± 1108) | 300.8 ± 17.5 [253, 358] | 2165 ± 413 [1433, 3025] | 974.5 ± 35.3 [870, 1066] | 788.0 ± 30.1 | 23.0 ± 4.0 |
| P376 | 747 | 217.5 ± 10.6 (1239 ± 1278) | 346.5 ± 18.3 [265, 397] | 2289 ± 409 [1561, 3217] | 1123.7 ± 36.8 [1026, 1227] | 911.3 ± 31.6 | 23.0 ± 4.1 |
| P500 | 995 | 288.6 ± 13.5 (1546 ± 1569) | 460.3 ± 21.3 [385, 517] | 2668 ± 411 [1937, 3557] | 1499.3 ± 43.3 [1364, 1632] | 1210.2 ± 36.2 | 23.0 ± 4.3 |
| P664 | 1323 | 384.9 ± 15.1 (2177 ± 2297) | 610.2 ± 24.9 [511, 682] | 3186 ± 413 [2442, 4079] | 1997.5 ± 48.1 [1851, 2126] | 1607.6 ± 41.1 | 23.1 ± 4.1 |

T2 figures are in `results/summary_4d_table.csv`. All D0, H1 and H2 outputs
were valid at every set and stream (T | nrd(β), and log2(nrd(β)/T) ≈
log2 √p). H1's output norm matched D0's in 100% of cases.

### 3.2 Target identity → trace (target panels, η² = share of variance between targets)

**T1** (challenge degrees; all targets the same bit length):

| set | H1 η² (Kruskal p) | H2 η² (KW p; χ² p) | H3 η² (KW p) | H3s η² (KW p) |
|---|---|---|---|---|
| P248 | 0.212 (3e−133) | 0.0066 (0.47; 0.08) | 0.0072 (0.34) | 0.0037 (0.95) |
| P324 | 0.259 (2e−164) | 0.0067 (0.51; 0.20) | 0.0058 (0.64) | 0.0044 (0.87) |
| P376 | 0.245 (6e−160) | 0.0082 (0.21; 0.80) | 0.0034 (0.95) | 0.0068 (0.39) |
| P500 | 0.223 (1e−141) | 0.0027 (0.98; 0.63) | 0.0078 (0.07) | 0.0050 (0.56) |
| P664 | 0.221 (1e−137) | 0.0067 (0.29; 0.14) | 0.0057 (0.52) | 0.0059 (0.56) |

**T2** (bit length varies over 16 bits). H2's η² is 0.225, 0.212, 0.161,
0.205 and 0.115 across the five sets. Per-target mean H2 regresses on the
target's bit length with slope about **2.0 iterations per bit, r = 0.97 to
0.99**. After that linear dependence is removed, η² drops to 0.006–0.011
(KW p 0.09–0.45, plus one p = 0.035 at P500). H3 behaves the same way
(about 1.0–1.2 per bit; residual η² ≤ 0.012). H1 stays at η² 0.26–0.43
even after removing bit length.

**Correlation with the baselines.** In the streams, the per-target trace of
H1, H2 and H3 has |r| ≤ 0.14 against A's step count and ≤ 0.14 against D0
at every set, as does D0 against A (|r| ≤ 0.08). In the panels, the slope of
per-target mean against D0 or A is never significant at fixed bit length.
The 2-D signature, slope 1 with r ≥ 0.997, **does not appear** in 4-D.

**H1's own structure.** H1's histograms are multimodal (`plots/hist_*`).
`h1_modes.py` (P500, 3 targets × 300 keys) shows that the XOF-chosen row
permutation of the mask explains η² = 0.26, and the target 0.21. The
geometry of the hash basis is itself a visible invariant.

**Seeds and hash function.** Fixed seed versus H(pk) and SHAKE-256 versus
BLAKE3 make no difference to the distributions: KS p ≥ 0.09 in all but one
of 60 comparisons, H3 P324-T1 at p = 0.0014. That is consistent with
chance across that many tests.

**Recomputability.** With the public seed, the trace for (pk, T) was
recomputed exactly in 100% of 6,000 cases. With a known seed, each trace
is a deterministic function of T carrying 4–9 bits of entropy (A
5.2–5.8, D0 4.6–5.2, H1 8.7–8.8, H2 6.7–7.3).

### 3.3 Key identity → trace (key panel, fresh public q per signature)

| set | D0 η² (KW p) | H2 η² (KW p) |
|---|---|---|
| P248 | 0.016 (0.10) | 0.006 (0.91) |
| P324 | 0.007 (0.75) | 0.008 (0.65) |
| P376 | 0.016 (0.06) | 0.012 (0.36) |
| P500 | 0.012 (0.36) | 0.011 (0.45) |
| P664 | 0.014 (0.09) | 0.009 (0.79) |

Every value is near the null level of 0.0095, and every reduced vector was
valid. Because a new public q re-randomizes the lattice on every signature,
even D0 shows no key dependence at this resolution. That resolution is
limited: the test detects η² of roughly 0.02 or more. D0's p = 0.06 to
0.10 at three sets deserves a larger run.

### 3.4 Covolume / bit length (cross-set invariant)

Mean T1 trace length is linear in log2 q, with r = 1.000 for every variant
except H3s:

| variant | iterations per bit |
|---|---|
| A | 0.29 |
| D0 | 0.455 |
| H2 | 1.506 |
| H3 | 1.207 |
| H1 | 1.548, plus an intercept of about 1,137 |

H3s does not depend on size at all (0.0002 per bit). The H2 lattice has
covolume T², and its trace tracks log T. See `plots/scaling_T1.png`.

### 3.5 Fixed-schedule control

Tours needed to stabilize (300 T1 targets per set):

| set | tours: mean ± sd [range] | fixed-T ops (12 × max) |
|---|---|---|
| P248 | 77.5 ± 5.0 [63, 91] | 1,092 |
| P324 | 100.1 ± 5.5 [82, 113] | 1,356 |
| P376 | 116.1 ± 6.1 [99, 133] | 1,596 |
| P500 | 153.8 ± 6.7 [136, 169] | 2,028 |
| P664 | 204.0 ± 8.3 [183, 227] | 2,724 |

Tours needed correlate with D0 iterations at r = 0.997–0.999. Run with a
fixed T at or above the maximum, the trace has zero variance by
construction, costing about 10–17% more tours than the mean. A real
constant-time schedule should set T from a proven bound in log2 T, which
is public per set, not from an empirical maximum.

## 4. Verdict

1. **The 2-D dependence breaks only when the hash picks a new problem
   instance (H2), not when it masks the basis of a fixed instance (H1).**
   - H2 at fixed target size: η² 0.003–0.008, at the null level, with
     KW p 0.21–0.98 at all five sets.
   - H1: target identity still explains 21–26% of the variance at fixed
     size (26–43% when size varies), and the mask's own permutation adds a
     further 26%.
   - The 2-D lesson repeats in 4-D. Masking a fixed lattice shifts the
     trace; re-randomizing the lattice is what flattens it.
2. **One invariant survives in every variant that encodes the problem:
   target size, i.e. the covolume.** It is about 1.5 iterations per bit
   across sets and about 2 per bit within a set. At a fixed size, nothing
   else about the target was detectable in H2, using η², Kruskal-Wallis,
   χ², MI against Hamming weight and residues, or correlation with A and D0.
3. **Not constant time.** H2's count still varies by about ±45 iterations
   per execution. A flat distribution is not a constant trace. Only the
   fixed schedule gives zero variance.
4. **Hashing gives no secrecy.** H(pk) is public, so anyone who knows the
   target recomputes the exact trace. Whatever H2 achieves comes from
   choosing a fresh instance uniformly; a public seed does not hide which
   instance was chosen.

## 5. Structural flags

- **Correctness.** D0, H1 and H2 encode the real 4-D step: a short element
  of an ideal of norm T, giving an equivalent ideal of norm about √p, and
  100% of outputs were valid. Two limits apply:
  - LLL does not solve the norm equation x² + y² + p(z² + w²) = T. The
    Cornacchia/RepresentInteger step still follows in a KLPT-style
    pipeline, so 4-D embedding does not remove Cornacchia's channel.
  - The model uses O = ℤ⟨1, i, j, k⟩ (index 4 in O₀), not the maximal order
    and not the secret O_pk.
- **H3 and H3s solve nothing.** The q-ary lattices don't encode the norm
  equation. H3s's perfectly flat trace (about 23 iterations at every size)
  is flat because it ignores the target. That is exactly the trap of a
  hash basis that doesn't depend on the target's arithmetic.
- **Predictability.** With seed = H(pk), XOF(seed‖T) is a public
  deterministic function. Against a timing adversary who knows pk, hashing
  helps only to the extent the secret enters the lattice, and then the
  secret, not the hash, is what hides the trace.
  - In PRISM, q is public: the verifier computes it. So "leaking the
    target" is not the threat; the threat is dependence on the secret
    ideal.
  - In the key panel, the per-signature public q already re-randomizes the
    lattice enough that no key dependence was visible, even for D0.
  - If unpredictability is wanted, seed the choice from the signer's own
    secret: H(sk‖msg), the deterministic-nonce pattern of EdDSA and
    RFC 6979. That needs no pre-shared secret.
- **Covolume leak.** LLL's iteration count tracks log of the basis norms
  and the determinant: about 1.5–2 iterations per bit here. That is
  harmless while every lattice of a set has the same, public covolume. If
  a secret-dependent norm of variable size ever enters the lattice, its
  bit length leaks at that rate. Equalize the covolume, for example by
  always reducing at a fixed total norm size, or use a fixed schedule.
- **A's real channel is the retry loop.** In T1, RepresentInteger needs a
  mean of 790–2,180 tries, geometric with sd ≈ mean, before Cornacchia
  runs. That dwarfs Cornacchia's ±10–15 Euclid steps, and it is the
  primality-search pattern the outer PRISM loop also has.
- **Cost.** H2 runs 3.3× D0's iterations, because D0's small y gives a
  structured basis. H1 runs 4–8× and is multimodal.
- **Model limits.** These are step counts, not cycles, and operand sizes
  differ between variants. The secret-ideal model is simplified. There are
  many statistical tests, so isolated p ≈ 0.01–0.04 hits are expected.

## 6. Suggested next constructions

1. **H2-style instance re-randomization plus a fixed schedule.** Use a fresh
   ideal (or a fresh equivalent lattice) per signature, chosen from
   H(sk‖msg‖q), and reduce it with a constant-time dimension-4 LLL whose
   tour count is set by a proven bound in log2 q. The first part removes
   structure; the second removes the count.
2. **Covolume equalization.** Embed every target so that all lattices of a
   set have identical determinant size, then check that the surviving
   bit-length invariant disappears.
3. **Real PRISM lattices.** Rerun the key panel on ideals in O_pk reached
   through I_sec, with the signer specification's norms, at P376 and P500
   with more keys (power to detect η² ≈ 0.005).
4. **Constant-time downstream steps.** Replace RepresentInteger's retry
   loop and Cornacchia with the fixed-iteration routines of ePrint
   2025/2192, or show that their inputs are independent of the secret.
5. **Rust or C port with cycle counts.** Use fixed-width limbs to see
   whether operand-size effects add a channel that step counts miss.

## 7. Files

- **Code:** `lp4d.py` (embeddings, instrumented LLL, fixed schedule),
  `run_4d.py` (streams and target panels), `run_secret.py` (key panel),
  `analyze_4d.py`, `h1_modes.py`, `crosscheck_4d.sage`.
- **Results** (`results/`): `summary_4d.json`, `summary_4d_table.csv`,
  `h1_modes_p500.json`, `sage_crosscheck_4d.txt`.
- **Histogram bin counts** (`results/histograms/`): `hist_<set>_<T1|T2>.csv`
  and `panel_hist_<set>_<T1|T2>.csv`, raw counts for recomputing χ².
- **Plots** (`plots/`): `hist_*`, `panel_*`, `scaling_T1.png`.
- Raw per-sample CSVs, about 4.6 MB in `results/raw/`, are regenerated by
  the run scripts and are git-ignored.
