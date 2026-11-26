# Fixed-modulus, fixed-schedule four-row HNF modulo 2N (Rick's t45u, question 1)

Branch `experiment/fixed-modulus-hnf`, off `research` at `c94f25b`. Prototype and spec read at `origin/dev` `470d537`
(PRs #47, #48 and #49/#50 merged), not the PR branches. **Research notes, not adopted spec; no crate, no `dev`, no PR is touched.**

## 0. Verdict

**Yes, with three conditions that the decode path already establishes.** The general four-row HNF of a left `O0`-ideal lattice
can be computed by a fixed-schedule algorithm working modulo the fixed modulus `m = 2N`. Nothing divides the modulus by a pivot.
The algorithm has a proved public operation count and a proved width bound at all five sets. Zero pivots, composite norms,
even norms and non-generic ideals need no special case.

| Condition | Meaning | Who establishes it |
|---|---|---|
| **C1** containment | `m·Z⁴ ⊆ span(G)` for the generator rows `G`. For an ideal this is Q1. It holds when `G ⊂ I` and `[Z⁴ : span G] = 4N²`. | Decode's exact membership and index checks (`GuardedIdeal::admit`). Or, for any `G` and without ideal theory, the exact test `Δ(G) = ∏ d_c ≠ 0` (§3.3). The output's pivot product alone is **not** a certificate. |
| **C2** range | `2 ≤ m < 2^K`, with `K = B + 1` (Q4: `N < 2^B`), and generator entries `|x| < 2^(2K)`. | Q4 is already tested at admission. Entries of a guarded key are far below the bound (§5.3). |
| **C3** carried norm | `m` is exactly `2N` for the carried norm `N` (Q2). | The type that carries `N`. |

It does **not** need `N` odd, prime, square-free, coprime to anything, or generic. It does **not** need `m` to be a multiple of the
determinant `4N²`. If C1 fails the algorithm returns the HNF of the superlattice `span(G) + mZ⁴`; the exact test
`Δ(G) = ∏ d_c` fails (§6B), but the pivot product by itself can still equal `4N²`. If C2 fails the output is undefined and must be masked by the same check.

Per-set bounds for the decode class (`N < 2^B`, `k = 4` generator rows `u, Ju, v, Jv`; limbs are 64-bit; every number below is
checked by the instrumented model, not only derived):

| Set | `K` | `F = 2K−1` pair-steps per fold | Barrett reductions `12kK−2k+4` (k=4) | prism-int primitive calls `368kK+56K−100k+76` (k=4) | limb products (k=4) | widest value, schedule `3K` bits | widest value if entries reach `2^(2K)`, `4K−1` bits | dev coordinate width `W` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| P248 | 159 | 317 | 7,628 | 242,628 | 259,484 | 477 b = **8** limbs | 635 b = 10 | 11 |
| P324 | 196 | 391 | 9,404 | 299,164 | 611,516 | 588 b = **10** | 783 b = 13 | 14 |
| P376 | 225 | 449 | 10,796 | 343,476 | 864,000 | 675 b = **11** | 899 b = 15 | 16 |
| P500 | 286 | 571 | 13,724 | 436,684 | 1,455,164 | 858 b = **14** | 1,143 b = 18 | 21 |
| P664 | 368 | 735 | 17,660 | 561,980 | 3,179,520 | 1,104 b = **18** | 1,471 b = 23 | 27 |

Everything fits in the coordinate width `W = 11/14/16/21/27` that `dev` already uses for ideal arithmetic, so the HNF needs no new
width ruling. `k = 8` (the intersection's eight generators) gives `96K − 12` reductions, which is `2(48K−4) − 4` (§4).

What this does not settle: the same algorithm applied with the intersection modulus `2Nq` (`K = B + 2e − 4`) has a `3K`-bit Barrett
product of 31/40/46/61/80 limbs, above `LIMBS = 26/33/38/49/65` at every set (§7). That is a width problem of the
intersection class, not of the question asked here, and it is Rick's second question.

## 1. Setting, from the code

- **Lattice and convention.** Numerators `(a,b,c,d)` of `(a + bi + cj + dk)/2` over `O0 = ⟨1, i, (i+j)/2, (1+k)/2⟩`, `i² = −1`,
  `j² = −p`. An ideal's lattice `L ⊂ Z⁴` has index `4N²`: the order's numerator lattice has index 4, the ideal index `N²` in it.
  The HNF is four **columns** (`dev` `ideal/mod.rs`, `GenericHnf`, `GuardedIdeal::admit`): column `c` has zeros at coordinates `> c`, a
  positive pivot `d_c` at `c`, and entries at `l < c` in `[0, d_l)`. Generic shape: `d = (2N, 2N, 1, 1)`, columns
  `(2N,0,0,0), (0,2N,0,0), (x,y,1,0), (2N−y,x,0,1)`. Pivots are found in the order `c = 3, 2, 1, 0`.
- **Where it is needed.** Decode (`research/stage2c-design-notes` §1.6, ruling 7 of the t42u rulings): after exact membership,
  "append `2N·eᵢ`" and compare the HNF of `{u, Ju, v, Jv, 2Ne₀..2Ne₃}` with the stored ideal. `dev`'s `admit` today *checks* a supplied HNF
  (triangular, canonical, index, span, closure); it does not *compute* one. The computed HNF is the missing "general" path.
- **The modulus.** SIGNER_SPEC Q1 (PR #45, now on `dev`): the lattice contains `2N·Z⁴`; the reference's modulus-by-determinant routine
  is not usable because it divides the modulus by each pivot and gives wrong forms at `2N`, and `4N²` doubles the widths.
  §6A reproduces that failure.
- **The class.** Decode and the secret-ideal pre-reduction have `N < 2^B`, `B = ⌈L/2⌉ + 32`: `B = 158/195/224/285/367`, so `m = 2N < 2^K` with
  `K = B+1 = 159/196/225/286/368` (`sets.py`, same numbers as `geometry.rs`). Other classes (challenge, keygen `D`, signing `T = Nq`) have
  other `K`; the tables carry them (`results/widths_counts.txt`).

## 2. The algorithm

Inputs: `k` generator rows `G ∈ Z^(k×4)` (`k = 4` for decode; any `k ≥ 1` works), the modulus `m`, and the public bound `K` with `m < 2^K`.
Nothing but `K` and `k` selects a loop count. Every secret-dependent choice is a select. Python-model primitives are the prism-int
operations of the Stage 2c batch (bit length, masked shift by a secret count, wide multiply, compare, select, subtract, absolute value).

```
FixedHNF(G, m; K, k):
  R   <- floor(2^(2K) / m)                       # restoring division, 2K+1 fixed iterations          (step 0)
  Red(y) := y mod m in [0,m)  via  a=|y|;  qh = floor(a*R / 2^(2K));  r = a - qh*m;  r -= m if r >= m;
                              r = m - r if (y < 0 and r != 0)                                       # Lemma 4
  S   <- [ Red(x) for every entry x of G ]       # adds multiples of m*e_j: lattice unchanged       (step 1)
  for c = 3, 2, 1, 0:                            # one pivot per coordinate                          (step 2)
      acc <- m*e_c                               # the permanent generator, consumed here
      for each row Y in S (k rows, support in 0..c):
          X <- acc
          repeat F = 2K-1 times:                 # fixed count; a step is the identity once one pivot is 0
              swap X,Y if X[c] < Y[c]
              t <- bitlen(X[c]) - bitlen(Y[c])
              if Y[c] != 0:  X <- sign * (X - (Y << t))      # sign makes X[c] >= 0;  entries l<c: Red(.)
          swap X,Y if X[c] == 0                  # pivot row first
          acc <- X;  Y' <- Y with coordinate c dropped;  S <- (S minus Y) + Y'
      P_c <- acc;  d_c <- acc[c]
  for c = 3, 2, 1;  for l = c-1 down to 0:       # canonical residues                                (step 3)
      q <- floor(P_c[l] / d_l)   by K-step binary long division (public shifts of d_l)
      P_c <- P_c - q*P_l  on coordinates <= l;   entries at l' < l:  Red(.)
  return columns P_3..P_0
```

Zero pivots. The first generator at coordinate `c` is `m·e_c` itself, so the pivot is `gcd(m, entries) ≥ 1` whatever the generators
hold. A coordinate where every generator is `≡ 0 mod m` gives the pivot `m`. No row exchange is ever needed, and an all-zero generator
set returns `diag(m,m,m,m)`.
Non-unit and composite pivots are the gcd of the entries with `m`, found by the shifted-subtraction elimination of Lemma 3
with the fixed iteration count `F`. Cofactors are not tracked: whole rows are updated, so the lattice is preserved by construction.
Composite or even `N`, prime powers, and non-primitive ideals only change which divisor of `m` each pivot is.

### 2.1 Why working modulo 2N gives the right HNF, and what the index 4N² requires

- **What the proof uses.** Only `m·Z⁴ ⊆ span(G) + mZ⁴`, which is trivial, and that the four vectors `m·e_c` stay in the generating set until
  the coordinate that needs them (Lemma 2). Each `m·e_l` is still present when any entry at coordinate `l` is reduced mod `m`. Reducing an
  entry changes a generator by a multiple of a generator, so the lattice is unchanged.
- **Why it differs from DKT / Cohen 2.4.8.** Those work modulo a multiple `R` of the determinant and replace `R` by `R/d_c` after each
  pivot. They need `R ≡ 0 mod det`, because their lemma is "an index-`I` sublattice contains `I·Z^n`". Here `det = 4N²` does not divide `2N`,
  but we use the weaker and different fact that the **exponent** of `Z⁴/L` divides `2N` (Lemma 1). The containment `mZ⁴ ⊆ L` survives
  intersection with every coordinate subspace, so it never needs shrinking.
- **What `4N²` is used for.** Not as a modulus. (i) As an index check on `G`: `Δ(G) = 4N²`. (ii) With membership, as the proof of C1:
  if `G ⊂ I` and `[Z⁴ : span G] = 4N²` then `span G = I ⊇ 2N·Z⁴` (Lemma 1), so the output is exactly `HNF(I)`.
  `GuardedIdeal::admit` already computes both: the Gaussian determinant `|det(u,v)|² = 4N²`, and the Cramer membership test.
  (iii) `Δ(G) = ∏ d_c` is the post-condition that detects C1 failing (§3.3).
- **Why `2N` and not `N`.** `Z⁴ ⊄ O0num` (for example `e₀ = (1,0,0,0)` has `a ≢ d`), so `N·Z⁴ ⊄ L` already for `I = O0` (`N = 1`, `L` has pivots `(2,2,1,1)`).
  The factor 2 is the denominator of `O0`, and `2N` is the least modulus that works for every ideal.

## 3. Proofs

Let `L̄ = span(G) + m·Z⁴`, `2 ≤ m < 2^K`.

**Lemma 1 (Q1).** For a left `O0`-ideal `I` of reduced norm `N`, `2N·Z⁴ ⊆ L`.
*Proof.* It suffices to show `N·O0 ⊆ I`, because `2N·Z⁴` are the numerators of `N·Z[1,i,j,k] ⊆ N·O0`. Check locally. At `ℓ ∤ p`, `O0⊗Z_ℓ ≅ M₂(Z_ℓ)` and
a left ideal is `M₂(Z_ℓ)·diag(ℓ^α, ℓ^β)·v` (Smith form) with `ℓ^(α+β) = ℓ^(v_ℓ(N))`. It contains `ℓ^max(α,β)·M₂ ⊇ ℓ^(α+β)·M₂`. At `p`, `O0⊗Z_p`
is the maximal order of the division algebra, whose integral left ideals are the powers `Pᵃ`, `P² = pO`, of norm `pᵃ`, and `pᵃ·O = P^(2a) ⊆ Pᵃ`. ∎
(The exponent of `Z⁴/L` divides `2N`, while its order is `4N²`: for a cyclic quotient the containment would fail, §6B.)

**Lemma 2 (invariant).** Before coordinate `c` (c = 3,2,1,0) is processed, let `P_{c+1..3}` be the pivot rows found, `S` the `k` working rows with
support in `{0..c}`, all entries in `[0,m)`. Then `P_{c+1..3} ∪ S ∪ {m·e_{c'} : c' ≤ c}` generates `L̄`.
*Proof.* `c = 3`: step 1 changes each row by a multiple of `m·e_j`, which are generators. Step `c`: a swap, a negation and `X ← X − 2^t Y` are unimodular on the pair, so the
generated lattice is unchanged. `Red` on an entry at `l < c` adds a multiple of `m·e_l`, `l ≤ c−1`, still in the set. The permanent `m·e_c` is the initial `acc`, so it is
consumed exactly once, inside the pair operations. At the end the set is `P_{c+1..3} ∪ {P_c} ∪ {k rows with support in {0..c−1}} ∪ {m e_{c'}: c' ≤ c−1}`. ∎

**Lemma 3 (pair elimination).** Let `0 ≤ a, b < 2^K` (the pivot entries of `X` and `Y`; `a ≤ m`). The step "if `a < b` swap; `t = bl(a) − bl(b)`; `a ← |a − 2^t·b|`"
(applied only if `b ≠ 0`) satisfies `a' < 2^(bl(a)−1)`. After at most `2K − 1` active steps one of the two is zero and the other is `gcd(a,b)`. A step with `b = 0` is the identity.
*Proof.* `a, 2^t b ∈ [2^(bl(a)−1), 2^bl(a))`, so `|a − 2^t b| < 2^(bl(a)−1)`. So `σ = bl(a) + bl(b)` falls by at least 1 per active step. It starts at `≤ 2K`
and while both entries are nonzero `σ ≥ 2`, and ends `≥ 1`; at most `2K − 1` steps. Unimodularity gives the gcd. ∎ `test_lemmas.py` verifies this exhaustively for `K ≤ 11`
(all `4^K` pairs). The exact worst case grows like `1.55 K`, for example 17 steps at `K = 11`, so `2K−1` is proved and not tight; shortening it needs a new proof and is not claimed.

**Lemma 4 (Barrett).** Let `m < 2^K`, `R = ⌊2^(2K)/m⌋`, `0 ≤ x < 2^(2K)`. Then `q̂ = ⌊xR/2^(2K)⌋ ∈ {⌊x/m⌋−1, ⌊x/m⌋}`, so `r = x − q̂ m ∈ [0, 2m)` and one conditional subtraction gives `x mod m`.
*Proof.* `R > 2^(2K)/m − 1` gives `xR/2^(2K) > x/m − x/2^(2K) > x/m − 1`, so `q̂ > x/m − 2`; also `q̂ ≤ x/m`. ∎ Exhaustive for `K ≤ 7` (all `m`, all `x`) in `test_lemmas.py`.
The signed wrapper applies this to `|y|` and flips a nonzero remainder.

**Lemma 5 (ranges).** With `m < 2^K` and all entries in `[0,m)`:
| Quantity | Bound |
|---|---|
| shift `t` | `0 ≤ t ≤ K` |
| `Y_j << t` (tails), difference `X_j − (Y_j << t)` | `|·| < 2^K·m < 2^(2K)` |
| pivots | `≤ m < 2^K`; `Y[c] << t < 2^K` |
| `Red` input (inside the schedule) | `|y| < 2^K m` |
| Barrett product `a·R`, `a < 2^K m` | `< 2^(3K)`; `q̂ ≤ a/m < 2^K`; `q̂·m ≤ a < 2^(2K)` |
| Barrett product for an entry reduction, `|x| < 2^(2K)` | `< 2^(4K−1)`; `q̂ < 2^(2K−1)` |
| `R` | `≤ 2^(2K−1)`, so at most `2K` bits (equality only at `m = 2`) |
| restoring division remainder | `2·rem + 1 < 2^(K+1)` |
| binary long division `d_l << t`, `t ≤ K−1` | `< 2^(2K−1)`; `q ≤ (m−1)/d_l < 2^K` |
| canonical step `q·P_l[l']` | `< 2^K m`, so `Red` input `|y| < 2^K m` |
Each line follows from the previous ones and Lemma 3's `a' < a`. The model asserts every one of them on every operation of every run, and declares the width of each primitive before executing it.
The observed maxima in all 1,745 adversarial runs equal the declared bounds at every set (`3K`, `4K−1`, `2K`), so the table is attained.

**Theorem A (correctness).** For every `G` (entries `|x| < 2^(2K)`) and `2 ≤ m < 2^K`, the output is the HNF of `L̄`. If `m·Z⁴ ⊆ span G` it is the HNF of `span G`.
*Proof.* By Lemmas 2 and 3, after coordinate 0 the working rows are zero (support in `∅`) and `P_3..P_0` generate `L̄` with positive pivots and support `{0..c}`: a triangular basis.
Step 3 replaces `P_c` by `P_c − qP_l` (unimodular) and reduces entries at `l' < l` mod `m`. This is valid because `m·e_{l'} ∈ L̄` has support `{0..l'}`, so lies in `span(P_{l'}..P_0)`, which does not
contain `P_c` (`c > l'`). Processing `l` from `c−1` down to 0 fixes entry `l` in `[0, d_l)` exactly, and later steps use `P_{l''}` with `l'' < l`, whose support misses `l`. So the result is triangular, positive and
canonical, and the HNF of a lattice is unique. If `mZ⁴ ⊆ span G` then `L̄ = span G`. ∎

### 3.3 The self-check (corrected in the follow-up)

`∏ d_c = [Z⁴ : L̄] ≤ [Z⁴ : span G] = Δ(G)` (`Δ(G) = |det|` of the four rows, 0 if rank-deficient), with equality iff `m·Z⁴ ⊆ span G` and rank 4.
So **`Δ(G) ≠ 0 ∧ Δ(G) = ∏ d_c` certifies C1 exactly**, for any `G`. For decode `Δ(G) = |det_{Z[i]}(u,v)|²` is already computed.
**Erratum.** The first version of this section, and the C1 row in §0, said that `∏ d_c = 4N²` on the output certifies C1. That is wrong without `Δ(G)`:
`G` can be rank-deficient, or a proper sublattice of `I`, and still give `span G + 2N·Z⁴ = I` and `∏ d_c = 4N²`
(`../fixed-modulus-hnf-followup/REPORT.md` §2, with examples at all five sets). Demonstrated in §6B for the case where the product does catch it.

## 4. The public operation count (proved, and equal to the measured count)

All loop bounds are `4` columns, `k` rows, `F = 2K−1`, `2K+1` (reciprocal), `K` (division), and `6` coordinate pairs. Per pair step on column `c` (`n = c+1` entries, `c` tails):
2 compares, `4n` selects, 2 bit lengths, `n+1` subtractions, `n` secret shifts, 1 sign bit, `n` negations, and `c` reductions. A reduction is 1 abs, 2 multiplies, 1 public shift, 3 subtractions, 2 compares, 3 bit-ops,
1 sign bit, 2 selects. Summing over `c = 3,2,1,0` (`Σn = 10`, `Σc = 6`) and adding the reciprocal, normalisation and canonical steps:

- Barrett reductions: **`ρ = 4k + 6kF + 4 = 12kK − 2k + 4`** (`4k` entry, `6kF` schedule, `4` canonical);
- pair steps: `4kF = 8kK − 4k`;
- multiplies: `2ρ + 4`;
- all prism-int primitive calls: **`368kK + 56K − 100k + 76`**. By category: `sel 104kK+8K−24k+9`, `sub 64kK+8K−20k+17`, `cmp 40kK+8K−8k+9`, `bit 36kK+8K−6k+13`, `signbit 20kK−6k+4`,
  `shl_secret 20kK−10k`, `neg 20kK−10k`, `bitlen 16kK−8k`, `abs`/`shr_public 12kK−2k+4`, `mul 24kK−4k+12`, `shl_public 16K+2`, `add 8K+2` (`results/formula_poly.txt`).

`test_counts.py` checks, for 8 `(K,k)` pairs from `(3,4)` to `(159,8)` and 200+ inputs (random, all-zero, tiny modulus `m ∈ {2..5}`, `m = 2^K−1`): the measured count of every category equals the formula, the limb-product count equals the formula, and **the SHA-256 of the executed
primitive sequence is identical for every input of equal `(K,k)`** (`results/counts_check.txt`). Each adversarial run also asserts one trace hash per `k` over all ideals and all generating sets (`results/adversarial_*.txt`).
This shows that the model's control flow and primitive widths are secret-independent; it is not a timing or compiled-code claim.

Cost note. At `k = 4` the schedule is dominated by reductions: 13,724 at P500, 1.46 M schoolbook limb products. It runs once per decode, not per signature.

## 5. Widths

### 5.1 Per-set bound (bits and 64-bit limbs), decode class

| Value | Bound | P248 | P324 | P376 | P500 | P664 |
|---|---|---:|---:|---:|---:|---:|
| `K` | `B+1` | 159 | 196 | 225 | 286 | 368 |
| entries, pivots, `m` | `K` bits | 3 | 4 | 4 | 5 | 6 |
| reduction input, `R`, shifted values | `2K` bits | 5 | 7 | 8 | 9 | 12 |
| Barrett product in the schedule | `3K` bits | **8** | **10** | **11** | **14** | **18** |
| Barrett product, entries up to `2^(2K)` | `4K−1` bits | 10 | 13 | 15 | 18 | 23 |
| dev `W` / `2W` / `LIMBS` | | 11/22/26 | 14/28/33 | 16/32/38 | 21/42/49 | 27/54/65 |

### 5.2 Compared with the spec's budget

SIGNER_SPEC row 2 gives `2b+3` bits for "an intermediate of the normal form with the fixed modulus `2N`", for a cofactor-based method that "2b states its own". This algorithm tracks no cofactors, so its intermediates are `2K = 2b+2` bits, and
the Barrett product is the one wider value (`3K`). The spec's remark that no method working modulo the determinant fits is consistent: the same `4K`-bit width would be needed at the determinant.

### 5.3 The entry bound

Raw HNF columns have entries `< 2N`. A guarded pre-reduced pair has coordinates `≤ √(1024√p·N) < 2^(5+L/4+B/2)` (SIGNER_SPEC 2.3), and `5 + L/4 + B/2 ≤ B − 11 < K`. So entries of a decoded key are `< 2^K ≤ 2^(K−1)·m`
and the tight reduction (`3K`) covers them.

## 6. Where the claim fails if a hypothesis is dropped (and the minimal fix)

(`demo_failure_modes.py`, `results/failure_modes.txt`)

**A. Variable modulus (the textbook method) at `R = 2N`: wrong.** `1·O0` (N=1) gives pivots `(1,2,1,1)` instead of `(2,2,1,1)`; `3·O0` gives `(1,2,3,3)` for `(6,6,3,3)`; the generic `N = 7` ideal gives `(1,14,1,1)` for `(14,14,1,1)`. With `R = 4N²` the same
method is correct, at the cost of doubling the widths (Q1 said so). *Fix:* keep the modulus fixed and keep `m·e_c` as a permanent generator (this algorithm).

**B. `m·Z⁴ ⊄ span G`.** For `G = ⟨e₀, e₁, e₂, 4N²·e₃⟩` (cyclic quotient, index `4N²`) and `m = 2N` the output is the HNF of the superlattice `diag(1,1,1,2N)`. Its pivot product `2N ≠ 4N²`. *Fix:* the self-check of §3.3, or membership plus index at decode. For an
ideal lattice, Lemma 1 makes the case unreachable.

**C. `m ≥ 2^K`.** Lemma 4 no longer applies; the model's range assertion fires. *Fix:* the Q4 admission check `N < 2^B` (already in `admit`) folded into the sticky mark together with `|x| < 2^(2K)`.

## 7. The other classes and the intersection

Theorem A is for any `m` and any lattice, so it covers: the challenge ideal (`N = q`), keygen norm `D`, generic-HNF signing (`m = 2Nq`) and the intersection `N₂·I₁ + N₁·I₂` (`k = 8`, `m = 2N₁N₂`; both terms contain `N₁N₂·O0`, so `2N₁N₂·Z⁴` is inside by Lemma 1).
Only `K` changes. For the `m = 2Nq` classes the Barrett product (`3K` bits) is 31/40/46/61/80 limbs at the five sets, above `LIMBS` at all of them (`results/widths_counts.txt`, `sign-T`). For those classes this algorithm needs either a high-half
multiply (only `⌊aR/2^(2K)⌋` is used) or the slower shift-subtract reduction (`2K+1`-bit values, `K` steps per reduction). Neither is analysed here. The challenge and `D` classes are public (the modulus is public), so no fixed schedule is needed for them.

## 8. Tests (all in this directory; run from it with the sage env on `PATH`)

| Test | What | Result |
|---|---|---|
| `test_lemmas.py` | Lemma 3 exhaustive `K ≤ 11` (all pairs), Lemma 4 exhaustive `K ≤ 7` | pass (`results/lemmas.txt`) |
| `test_counts.py` | formula = measured; trace identical; widths asserted | pass |
| `test_adversarial.py <set>` | 349 runs per set, 1,745 total: ideals × generating sets, compared with Sage `hermite_form`, PARI `mathnf` (true lattice, and with `2N·I`) and the ideal's own HNF | **pass at P248, P324, P376, P500, P664** |
| `test_exhaustive_small.py` | every lattice `mZ⁴ ⊆ L` for `m = 2,3,4` (67, 212, 1,983), and random lattices for `m` ∈ {5,6,8,9,10,12,16,18,30,36,64,210,1024,30030} (2-parts, squares, prime powers, many-prime); 4–6 scrambled generators at `K = bl(m)` and `bl(m)+2`; and 3,000 generator sets that do not contain `mZ⁴` | **pass**: 28,862 lattices (57,724 runs) all equal to the constructed HNF, PARI cross-check on a sample (`results/exhaustive_small_lattices.txt`); 3,000 generator sets without `mZ⁴` all equal `HNF(span G + mZ⁴)` (`results/exhaustive_small_nonspan.txt`) |
| `test_decode_shape.py` | the decode shape itself: `u, Ju, v, Jv` for generic ideals (HNF generators, exactly Lagrange-reduced, three random Z[i] shears), 175 runs over the five sets | pass (`results/decode_shape.txt`) |
| `demo_failure_modes.py` | §6A, B, C | as stated |
| `search_steps.py` | hill-climb for the worst pair at each `K` | 0.9–1.0 `K` (weak: exhaustive small `K` shows 1.55 `K`) |

Adversarial ideal families (`gen_ideals.py`, built from exact quaternion arithmetic in `O0` numerators, never from the HNF under test): `O0` itself; norm 2 (`1+i`); small primes 3, 5, 7; primes near and the largest prime below `2^B` (generic `(2N,2N,1,1)`);
composites of three primes by CRT roots; prime powers `3⁵, 5³, 7²` and a large `ℓ²`; **non-primitive** `ℓ·J`; `ℓ·O0` and `(ℓ₁ℓ₂)·O0`; **even norms** `(1+i)^a·β`, `a = 1,2,5,9` (pivots like `(100,4,50,2)`); random principal `O0α` with `N` just under `2^B`;
`O0α + O0M`; and the **ramified** `O0·j` of norm `p` (outside Q4, run at a widened public `K`). Generating sets per ideal: the HNF columns; the J-closed 8; unimodular scrambles (entry sizes `2^6`, `2^(K/3)`, `2^(K/2)`, all kept below `2^(2K)`); `k = 5, 8`; entries
shifted to within 2 of `±2^(2K)`; zero and duplicate rows; and no generators at all (`diag(m,m,m,m)`).
Pivot shapes seen include `(2N,2N,1,1)`, `(30,30,3,3)`, `(70,70,5,5)`, `(big,big,1009,1009)`, `(100,4,50,2)`, `(big,2,big,1)`, `(big,24,big,12)`, `(big,big,big,big)` for the ramified one.
Maximum active elimination steps seen at the five sets: 155 / 192 / 219 / 284 / 361 against the schedule 317 / 391 / 449 / 571 / 735.

## 9. Not claimed

- Compiled-code constant time, and any claim about `prism-int` internals beyond the primitive list (bit length and masked shift are already in the t42u batch; `abs`, `mul`, `sub`, compare, select exist).
- That `2K−1` steps per fold is minimal (the observed worst case is `≈ 1.55K`).
- Anything about the signer: Rick keeps the adopted table and is not implementing this in the signer now.
- The intersection oracle (Rick's second question) is another branch; §7 only records what this algorithm's theorem gives it.

## 10. Reproduction

```
export PATH=/workspace/mamba/envs/sage/bin:$PATH
cd research/fixed-modulus-hnf
python test_lemmas.py 11 7 ; python test_counts.py ; python widths_table.py
for s in P248 P324 P376 P500 P664; do python test_adversarial.py $s; done
python test_exhaustive_small.py ; python demo_failure_modes.py ; python search_steps.py
```

| File | Contents |
|---|---|
| `fmhnf.py` | The algorithm, instrumented (`Ctx`: counts, declared widths, trace hash) |
| `counts.py`, `widths_table.py` | Closed forms and the per-set tables |
| `lib.py`, `gen_ideals.py` | Sage / PARI oracles, quaternion arithmetic, ideal generators |
| `test_*.py`, `demo_failure_modes.py`, `search_steps.py` | The tests above |
| `results/` | Outputs |
