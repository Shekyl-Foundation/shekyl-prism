# Cornacchia trace harness: classical vs. randomized-basis (ePrint 2023/807)

Date: 2026-11-21. Status: exploratory research, not a security claim. Nothing
here is constant-time code; it is a step-counting model in Python.

## 1. What was compared

All variants solve x² + y² = M (d = 1) for a prime M ≡ 1 (mod 4) and share the
same square root u of −1 mod M, normalised to u < M/2.

| Id | Variant | Source |
|----|---------|--------|
| A  | Classical Cornacchia: Euclid on (M, u) until the remainder is below √M, then check that M − b² is a square | Cohen, Alg. 1.5.2 |
| L  | Lagrange (Gauss) reduction of the lattice with basis (1, u), (0, M), no masking | 2023/807 §4.2, Alg. 6 |
| B  | **Alg. 10, "(Randomized) LatticeReduction(M, d, u)"**: basis replaced by S·L with S = P_j·R_i·P_j⁻¹, R_i = [[1, r_i], [0, 1]], r_i a fresh 128-bit random integer, P_j from a pre-generated list of small-entry unimodular matrices (products of two list entries once the list is used up), then Alg. 6 | 2023/807 §4.2 and §4.2.1 |
| B0 | The S = [[1, r0], [r1, r0·r1 + 1]] construction that §4.2.1 rejects | 2023/807 §4.2.1 |

**How faithful B is.** The paper's randomized variant is not "Cornacchia with a
randomized unimodular basis". It replaces Cornacchia's Euclidean half-GCD with
2-D Lagrange reduction of the lattice ⟨(1, u), (0, M)⟩, whose shortest vector
is (x, y) with x² + y² = M, and masks the starting basis with a random det-1
matrix S. Our B follows Alg. 10 and §4.2.1. The paper does not fix the P_j
entry size or the list length, so we chose 4-bit entries and 64 matrices. That
keeps S's entries between 128 and 145 bits (measured mean about 140), matching
the paper's "∼128 bits". **The PDF itself was blocked by a Cloudflare check
from the box, so Alg. 10 and §4.2.1 were read from indexed full-text
excerpts of eprint.iacr.org/2023/807.pdf. Those excerpts must be checked
against the paper itself before anything here is cited.** The paper's first
CT proposal (fixed-iteration constant-time half-GCD) is **not** implemented
here.

**Parameter choices.** M is a uniformly random prime ≡ 1 (mod 4) of exactly 376,
384, 500 or 664 bits. 376, 500 and 664 match the Shekyl sets P376, P500 and
P664. 384 is a bit length only: P363 and P384 are **not** Shekyl parameter
sets (the repository sets are P248, P324, P376, P500 and P664). P248 and P324
were not run in this 2-D harness. We did not derive M from an actual PRISM
norm equation (p = c·2^e − 1). In SQIsign/PRISM the Cornacchia input is a
cofactor of roughly that size, which the generator retries until it is prime,
and the "random" stream below models that retry. d = 1 throughout, the norm form
of O₀ for p ≡ 3 mod 4.

Streams per size: 5,000 primes (A, L, B, B0 on each); 100,000 random b-bit
integers run through the SQIsign-style "retry until prime ≡ 1 mod 4" filter;
a fixed panel of 30 primes × 1,000 fresh S draws each.

**Correctness.** Every run of A, L, B and B0 returned a valid (x, y).
`sage_crosscheck.sage` matched A, L and B against Sage `two_squares` and PARI
`qfbcornacchia` on 200 fresh primes per size, with 0 mismatches.

## 2. Key numbers (step counts, primes stream, n = 5,000 per size)

"Steps" means Euclidean divisions for A and µ computations (Alg. 6 step 2 plus
each while-loop pass) for L, B and B0.

| bits | A mean ± sd [range] | L mean ± sd [range] | B mean ± sd [range] | B0 |
|-----:|---------------------|---------------------|---------------------|----|
| 376 | 108.9 ± 8.2 [83, 140]  | 76.7 ± 5.1 [60, 94]   | 83.5 ± 5.5 [65, 101]  | L + 2 exactly |
| 384 | 111.4 ± 8.1 [82, 141]  | 78.5 ± 5.1 [58, 97]   | 85.1 ± 5.4 [62, 107]  | L + 2 exactly |
| 500 | 145.5 ± 9.3 [112, 177] | 102.1 ± 5.7 [82, 124] | 108.9 ± 6.1 [89, 133] | L + 2 exactly |
| 664 | 193.4 ± 11.0 [151, 230]| 135.3 ± 6.7 [108, 157]| 142.3 ± 7.1 [118, 166]| L + 2 exactly |

Square-root attempts (random v until v^((M−1)/4) is a root of −1): mean 2.0,
range 1 to 13, which is geometric with p = 1/2 as expected. Cofactor-square
check: 100% success on primes. Random stream acceptance: 114 to 178 per
100,000. All rejections happen before Cornacchia runs.

### Does B depend on the input?

| bits | corr(B, L) | corr(B, A) | fixed panel: ICC (share of B variance due to M) | slope of mean B on L (r) | B − L per draw | Kruskal-Wallis across M |
|-----:|-----:|-----:|-----:|-----:|-----|-----|
| 376 | 0.918 | 0.781 | 0.91 | 0.99 (0.998) | 6.7 ± 2.1 [0, 13] | H = 25,953, p ≈ 0 |
| 384 | 0.935 | 0.794 | 0.86 | 1.01 (0.998) | 6.8 ± 1.8 [0, 13] | H = 24,882, p ≈ 0 |
| 500 | 0.922 | 0.783 | 0.91 | 1.00 (0.999) | 6.8 ± 2.3 [0, 13] | H = 25,617, p ≈ 0 |
| 664 | 0.951 | 0.816 | 0.85 | 1.01 (0.997) | 6.9 ± 2.2 [0, 13] | H = 24,901, p ≈ 0 |

Chi-square homogeneity of binned B steps across the 30 fixed M: χ² ≈ 47,000 to
58,000 on 145 dof, p ≈ 0 at every size. Two-sample KS between the fixed M with
the lowest and highest L: D = 1.0 (the distributions do not overlap) at every
size. Mutual information, using 8-quantile bins with a 300-permutation null:
MI(B; L) has an excess of 1.14 to 1.39 bits (p = 0.003, the floor). MI(B; m
Hamming weight, m mod 3/5/7/8, u Hamming weight) is not significant. MI(B;
bit length of u) is small but significant at 376 and 384 (about 0.005 bits),
and that same effect is present in A.

**Mask-size scaling** (`s_scaling.py`, 384 bits, 30 M × 400 draws, S a
uniform-ish SL₂(ℤ) element with t-bit entries; this goes beyond the paper):

| S bits | 32 | 128 | 256 | 384 | 768 |
|---|---|---|---|---|---|
| ICC | 0.86 | 0.57 | 0.40 | 0.29 | 0.18 |
| slope of mean B on L (r) | 1.00 (0.9998) | 1.00 (0.999) | 1.00 (0.998) | 1.00 (0.997) | 1.03 (0.998) |
| mean steps | 95 | 134 | 186 | 238 | 393 |

## 3. Verdict

**A** is a deterministic function of (M, u). The fixed panel gave exactly one
step count per M, the same with a freshly computed root, and the count
correlates with the lattice count L (r ≈ 0.85). This is the leak 2023/807
describes.

**B is not flat in this model.** Its step count behaves like
`L(M, u) + noise(S)`. The noise has mean about 6.8 and sd about 2, is roughly
independent of M, and is mostly the continued-fraction length of S itself,
while L is fully determined by the lattice of (M, u). Per-M means sit on L with
slope 1.00 and r ≥ 0.997 at every size. 85–91% of B's step-count variance comes
from which M is being solved, and a single trace carries about 1.1–1.4 bits of
L. Larger masks lower the ICC, but the slope stays at 1. The mask adds
independent noise; it does not remove the dependence, so averaging or
template-matching recovers L.

**Why this happens.** Every lattice here is a rotated √M·ℤ², so the geometry is
the same for all M. What differs is the basis. Lagrange on S·L computes the
reduction of the matrix S·U, where U carries L's basis to the reduced one. The
continued-fraction (SL₂(ℤ)) word of a product is essentially the
concatenation of the two words, with a few cancellations at the junction. So
steps(S·U) ≈ steps(S) + steps(U) − O(1), whatever the size of S. **Left
multiplication by a random unimodular matrix shifts the trace; it does not
hide it.** B0 is the extreme case: its S is undone in exactly 2 µ-steps, so
B0 = L + 2 for every input, which reproduces the paper's own §4.2.1
observation.

This agrees with the TCHES 2025 dim-4 paper's remark that [JMKR23, Alg. 10]'s
runtime "is not actually constant, but at best independent from the input
integers". We see the weaker case here: it is not independent either.

**Caveats.**
- These are step counts, not cycles. Real timing also depends on operand
  sizes; B's operands are 128 to 145 bits larger than L's.
- d = 1 and random primes M only, not true PRISM norm-equation cofactors.
- Our P_j sizes are a choice the paper leaves open.
- L(M, u) is a small integer (σ ≈ 5–7 steps). How much it reveals about a
  signing key in SQIsign or PRISM is not established here. The paper's attack
  needs the inputs themselves, not a CF length.
- In SQIsign each M is used once per signature, so the averaging attack needs
  a different repeated observable, but single-trace leakage (above) remains.
- Python timing (`*_ns` columns) was recorded but is not analysed; it is too
  noisy to matter.

## 4. Implications for LatticePrism

1. If the map's randomization is seeded from H(pk) and is therefore public, a
   B-style mask hides nothing: anyone can recompute S, and the trace
   becomes a deterministic function of the input, as with A.
2. Even a secret, fresh mask is additive in the trace. Getting constant time
   "for free" needs an algorithm whose iteration count does not depend on the
   data at all. Examples are a fixed-iteration constant-time half-GCD (the
   paper's first proposal), constant-time dim-4 LLL (TCHES 2025), or the
   constant-time quaternion routines of ePrint 2025/2192. A per-input random
   mask is not enough.
3. All d = 1 lattices are isometric, so the "geometry" carries no per-prime
   information. Only the basis representation does, and that is what a
   canonical, constant-time reduction removes.

## 5. Suggested next experiments

- Implement 2023/807's first variant (fixed-iteration CT half-GCD) and the
  TCHES 2025 CT dim-4 LLL step schedule; check that the trace has zero
  variance and measure the cost against B.
- Feed real PRISM norm-equation cofactors from the Shekyl sets (P248, P324,
  P376, P500 and P664 in `prism-field`) instead of random primes. P363 and
  P384 are not Shekyl parameter sets.
- Quantify how many bits of the secret ideal or key L(M, u) carries, for
  example by simulating the 2023/807 key-recovery with a CF-length oracle.
- Try a mask applied on both sides (S·L·T) or a randomized-reduction order, to
  see whether any masking breaks the additive CF structure.
- Measure operand-size-aware cost (sum of limb products) and cycle counts in a
  Rust or C port instead of step counts.

## 6. Files

`cornacchia_variants.py` (A, L, B, B0, instrumented), `run_experiments.py`
(streams and CSVs), `analyze.py` (statistics and plots), `s_scaling.py`
(mask-size scaling), `sage_crosscheck.sage`, `pari_crosscheck.gp`,
`results/summary.json`, `results/summary_table.csv`,
`results/s_scaling_384.json`, `results/random_*_summary.csv`,
`results/sage_crosscheck.txt`, and `plots/steps_<bits>.png` and
`plots/b_minus_l_<bits>.png`. Raw per-sample CSVs (about 10 MB) are
regenerated by `run_experiments.py` and are git-ignored.
