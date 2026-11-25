# A tighter progress certificate for the Z[i]-Lagrange contract (SHE-23)

Branch `experiment/tighter-progress-certificate`, off `dev` at `904d876`. The reducer contract and classes are those of PR #45 (`abed5c1`): a strict initial norm-ordering swap, each quotient component rounded with `floor(h/A + 1/2)`, a swap after the subtraction only when the new vector is strictly shorter, and exp64 for bounded precision. Counts are in O0 numerator coordinates and include the final size reduction after the last swap. This is research code. It changes no schedule and makes no constant-time claim.

## Verdict

**The tighter proof works.** Two independent improvements:

1. **A cell potential replaces the 2-step quadratic potential.** The old certificate proved 1.761 bits of A per step (ρ₂ = 0.087 per two steps). The new one proves **ρ = 0.2685 per step (1.897 bits)**. The true worst case is Rolletschek's 2 − √3 = 0.26795 (1.900 bits), so the certified rate is within 0.2% of optimal.
2. **exp64's extra budget E disappears from the count.** A big quotient |q| > 2⁶⁰ can only follow a step that shrank A by at least 2¹²⁰. A joint potential charges the big sub-steps against that contraction, so **T_exp64 = T_exact** in every class except the secret-ideal pre-reduction, where the first step's quotient adds 1–2 steps. The bound uses only the ratios A₀²/Δ and C₀²/Δ, so it holds unchanged for O0 numerators and for the normalized Gram. It needs neither λ₁ ≥ T nor the constructor's integrality check.

**Signing at P500: 546 steps under exp64, against 609 in PR #45, a saving of 63 steps (10.3%).** Against the 587-step exact baseline it saves 41. Every new count sits 3–5 steps above the Rolletschek chains. A hill-climbing search found inputs 1–2 steps worse than Rolletschek, and those still fit with margins of 2–4. All 270 corpus inputs finish within the new counts under the exact rule, exp64 on O0 numerators, and exp64 on the normalized Gram (or the content-reduced Gram).

## 1. New versus old counts

Format is P248 / P324 / P376 / P500 / P664. "Old" means PR #45's native table: exact = the research O0 rows, exp64 = the any-basis count.

| Input class | New exact | New exp64 | Old exact | Old exp64 (PR #45) | normalized-E exp64 (SHE-22) |
|---|---|---|---|---|---|
| Generic signing, Q4 | **280 / 360 / 415 / 546 / 719** | **280 / 360 / 415 / 546 / 719** | 301 / 387 / 447 / 587 / 774 | 312 / 401 / 463 / 609 / 803 | 306 / 394 / 455 / 598 / 788 |
| Pre-reduced signing, guard 256 | **267 / 347 / 402 / 533 / 706** | **267 / 347 / 402 / 533 / 706** | 287 / 373 / 432 / 573 / 759 | 297 / 387 / 448 / 594 / 787 | 292 / 379 / 440 / 583 / 773 |
| Key generation, norm D | **208 / 323 / 308 / 411 / 638** | **208 / 323 / 308 / 411 / 638** | 223 / 347 / 331 / 442 / 686 | 231 / 360 / 344 / 459 / 712 | 227 / 353 / 337 / 450 / 699 |
| Secret-ideal pre-reduction, Q4 | **21 / 21 / 21 / 21 / 21** | **21 / 22 / 22 / 22 / 23** | 22 / 22 / 23 / 22 / 23 | 24 / 25 / 26 / 26 / 29 | 24 / 24 / 26 / 26 / 28 |
| Challenge alone | **197 / 257 / 297 / 396 / 525** | **197 / 257 / 297 / 396 / 525** | 212 / 276 / 319 / 425 / 565 | 220 / 287 / 331 / 442 / 587 | 216 / 281 / 325 / 433 / 576 |
| Pre-reduced, guard 2 (not selected) | 263 / 344 / 398 / 529 / 702 | 263 / 344 / 398 / 529 / 702 | 283 / 369 / 429 / 569 / 755 | 293 / 383 / 445 / 590 / 783 | — |
| Pre-reduced, unguarded (not selected) | 329 / 429 / 499 / 662 / 878 | 329 / 429 / 499 / 662 / 878 | 354 / 462 / 537 / 713 / 945 | 365 / 477 / 555 / 736 / 976 | — |

Savings at P500 for the five selected classes, new exp64 against PR #45:

| Class | PR #45 | New | Saved |
|---|---:|---:|---:|
| Generic signing | 609 | 546 | 63 (10.3%) |
| Pre-reduced, guard 256 | 594 | 533 | 61 (10.3%) |
| Key generation, norm D | 459 | 411 | 48 (10.5%) |
| Secret-ideal pre-reduction | 26 | 22 | 4 |
| Challenge alone | 442 | 396 | 46 (10.4%) |

**Units.** The counts depend only on R₀ = sup A₀²/Δ and R_C = sup C₀²/Δ, which are dimensionless. In O0 numerators, A₀ ≤ 4T², C₀ < 8T² + p and Δ = 4pT². In Rick's reduced-norm units, A₀ ≤ T² and Δ = pT²/4. In the normalized Gram, A₀ = 2T and Δ = p. All three give R₀ = 4T²/p. So the native table, the research O0 table and the normalized Gram share these counts, and the old model-order rows (R₀ = T²/p) are one bit smaller, as before.

**Both representations.** The certified counts are the same for numerator O0 and the normalized Gram. On the composition question: the joint charge **supersedes** SHE-22's normalized E rather than stacking with it. Normalized E bounds big sub-steps by ½·log₂(C₀/λ₁) in normalized units. The joint bound shows that big sub-steps after the first step cost no schedule slots at all, and only the first step's quotient needs a budget. That budget is bounded by C₀/A₀, which is scale-invariant. The lemma it uses ("n big sub-steps imply log₂|q| > 58n + 2", Lemma B2 of `clamped-quotient-progress`) holds in any integer scaling, so physical sub-step splits that differ between scalings (SHE-22 saw 43 of 500) do not matter.

## 2. Proof outline

Notation follows `lagrange-tour-bound` §3. A_t = N4(u_t) after swap t, λ_t = Δ/A_t², ε_t is the residual of step t+1, and r_t = A_t/A_{t−1} = λ_{t−1} + |ε_{t−1}|². After a swap, q_{t+1} = conj(ε_{t−1})/r_t. Lemma 1 (strict swap): if steps t and t+1 both swap, |μ_{t+1}|² ≥ 2. Once a step does not swap, the run is at the fixed point after it. With n swaps, the exact run needs n + 1 steps, and the last one is the final size reduction.

**(a) Main phase: cell potential.** A function g ≥ 1 on [−½, ½]², constant on 8,758 dyadic cells (quadtree, depth ≤ 28), with Ψ_t = A_t·g(ε_t). The certificate checks, for every cell c and λ ∈ [0, 2⁻²⁰]:

- r ≤ r̄(c);
- every admissible next residual ε′ = q − μ (with |μ|² ≥ 2 and q enclosed exactly) lies in cells whose g is at most g_s;
- r̄(c)·g_s ≤ ρ·g(c), with ρ = 537/2000.

So Ψ_t ≤ ρ·Ψ_{t−1} whenever step t+1 also swaps. The cells next to the origin are "wild": their successors may be anywhere, and they are checked with max g. The certificate has max g = G = 1.862201. The cells and g were found by counterexample-guided refinement plus value iteration (search only, floating point). The check itself is exact.

**(b) λ bands.** The same g is checked again at λ ≤ 2⁻¹⁶, 2⁻¹⁴, 2⁻¹³, 2⁻¹², 2⁻¹¹, with ratios 0.271313, 0.300770, 0.300838, 0.300973, 0.304449. A swap with λ ≤ 2⁻¹¹ has r ≤ ½ + 2⁻¹¹, so λ grows by at least (½ + 2⁻¹¹)⁻² ≈ 3.98 per swap. That limits each band to at most 3, 2, 1, 1, 1 transitions. The worst-case allocation is used.

**(c) Tail.** Once λ > 2⁻¹¹, at most τ = 4 more swaps can happen. This is `certify_tail_exact(2⁻¹¹, 5)`, the unchanged certifier from `lagrange-tour-bound`: 2,013,007 boxes, 114 s. The λ > ¼ range is covered by its Extended Endgame lemma. At λ ≥ 2⁻⁷ the certifier proves τ = 3, but that option comes out one step worse overall.

**(d) Exact count.** Let J* be the last swap with λ_{J*−1} ≤ 2⁻¹¹. Chaining (a) and (b) over transitions 1 … J*−1 gives ρ-products·G·A₀ ≥ Ψ_{J*−1} ≥ A_{J*−1} ≥ √(Δ·2¹¹). So J* − 1 ≤ K(R₀), the largest M with (band product)²·G²·R₀·2⁻¹¹ ≥ 1. Then n ≤ J* + τ, and

  **T_exact = K(R₀) + τ + 2.**

**(e) exp64, the joint charge.** Each logical step t costs 1 + n_t slots, where n_t is its number of big sub-steps. By Lemma B2, n_t ≥ 1 implies log₂|q_t| > 58n_t + 2. Since |q_{t+1}|² ≤ 1/r_t, we get r_t < 2^−(116n+4). Four lemmas follow, each checked in exact arithmetic in `counts.py`:

- *Charge:* G·2^−(116n+4) ≤ ρ^(1+n), so a transition followed by n big sub-steps contracts Ψ by ρ^(1+n). It costs n extra units of the same budget M.
- *Boundary:* if the transition that leaves the main phase, or the terminal step, carries n ≥ 1, then A_{J*−1}² ≥ Δ·|q|² > Δ·2^(2(58n+2)) ≥ (Δ·2¹¹)·ρ^(−2(1+n)). The units still fit in K.
- *No big quotients in the tail:* r ≥ λ > 2⁻¹¹ ≥ 2⁻¹²⁰, so |q| < 2⁶⁰ there.
- *Head:* step 1's n₁ is not covered by the chain. But |q₁|² ≤ C₀/A₀, so n₁ = k forces A₀²/Δ < R_C/16^(58k+2) =: R_k.

To enumerate k, set Q = h/A at entry. Since C/A = |Q|² + Δ/A², the arithmetic–geometric mean inequality gives C²/Δ ≥ 4|Q|². Lemma B2 therefore requires 4^(58k+3) < R_C for k ≥ 1. Enumerate every such k, including those with R_k < 1. These enter the tail directly and still pay the k first-step slots. The earlier cutoff 16^(58k+2) < R_C discarded them. Widening the enumeration reproduces all 35 published exact and exp64 pairs. Run `python3 -m unittest test_head_ratios` here for the immediate-tail case and the strict threshold. The CSV column `n1_max` bounds the enumeration and can exceed the number attainable by an admitted basis.

Together:

  **T_exp64 = max(K(R₀), max_{k≥1} K(min(R₀, R_k)) + k) + τ + 2.**

R_C collapses so quickly that the k ≥ 1 terms only win for the secret-ideal pre-reduction, whose R₀ is about 2⁶⁶.

**(f) Why it is not tighter.** At P500 the Rolletschek chain needs 541.4 steps (½·log₂R₀ / log₂(2+√3)). The certificate adds:

| Source | Steps |
|---|---:|
| Rate (ρ against 2 − √3) | 0.85 |
| G | 0.47 |
| Boundary (τ + 2 − ½·11/1.897) | 3.1 |
| λ bands | about 0.3 |

Rounding then gives 546. The boundary is the remaining slack. Pushing the tail to λ ≥ 2⁻⁷ (τ = 3) does not help with this g: the 2⁻⁷ band's ratio is 0.424 (option B, one step worse). Closing it would need a second potential tuned for large λ, worth at most about 1–2 steps.

## 3. What is machine-checked

| Check | Tool | Result (`results/certify_all.txt`, `results/check_potential.txt`) |
|---|---|---|
| Cell certificate at λ ≤ 2⁻²⁰ and the five bands | `cellcert check` (Rust, i128, overflow checks on, exact quotient enclosures, successor boxes rounded outward to 2⁻⁴⁰) | OK at all 6 λ |
| Same, independent code | `check_potential.py` (gmpy2 mpq, no rounding; cells.py shares no code with the Rust) | Tightest ratio 0.268499973 ≤ 0.2685, and every band ≤ its ρ |
| Leaves tile the square, g ≥ 1 | both | OK |
| Tail τ = 4 from 2⁻¹¹ | `certify_tail_exact` (unchanged, in `vendor/certify.py`) | OK |
| Charge, boundary, tail and head lemmas; the counts | `counts.py` (Fractions, integers) | OK |

Inherited and not re-proved here: Lemma 1 and the endgame lemmas (`lagrange-tour-bound` §3.3), the contract audit of rounding and strict swap, and Lemmas W/U/B2 of exp64 (`clamped-quotient-progress` §4, machine-checked there).

## 4. Validation

**Corpus (`validate.py`, `results/validate.txt`, `results/validate.csv`).** 270 inputs, 54 per set:

- honest O0 signing ideals (12, half at the norm bound);
- crafted Q5 and crafted k = 1 ideals (3 + 3);
- both Rolletschek chains at the bound of every generic class and of guard 256;
- the big-quotient chains: all-big, Rolletschek plus one big quotient, and 62-bit;
- tail stress, ping-pong, and three half-tie bases;
- challenge, keygen-D and secret-ideal O0 ideals;
- pre-reduced keys inside guard 256 (sampled, edge-skewed and 120√p-skewed).

Each runs four ways: the exact contract for T_exact; exp64 with the carried Gram on the numerator Gram for T_exp64; exp64 on the normalized Gram for ideals, or on the content-reduced Gram for the other bases; and a variable-time run that records the steps actually needed. **270 / 270 end reduced under every rule and scaling.**

| Set | Rolletschek need / new count (signing) | Margin | Other classes' Rolletschek margins | Largest honest need |
|---|---|---:|---|---:|
| P248 | 276 / 280 | 4 | 3–4 | 178 |
| P324 | 355 / 360 | 5 | 4–5 | 227 |
| P376 | 410 / 415 | 5 | 4–5 | 262 |
| P500 | 541 / 546 | 5 | 4–5 | 339 |
| P664 | 714 / 719 | 5 | 3–5 | 446 |

exp64 needs exactly the exact steps on every chain (no visible extra), and at most 10–14 extra physical steps on the crafted big-quotient inputs, which are far from the bound.

**Adversarial search (`hillclimb.py`, `results/hillclimb.txt`).** Hill climbing with restarts over Z[i] continuant chains completed to the class bound. Mutations change, insert or delete quotients, rewrite the first ones (the tail of the reduction), and perturb (x, y) by ±3. It ran 15 minutes per class:

| Set | Signing: best found / Rolletschek / count | Secret ideal: best / Rolletschek / count |
|---|---|---|
| P248 | 277 / 276 / 280 | 19 / 18 / 21 |
| P324 | 357 / 355 / 360 | 19 / 17 / 21 |
| P376 | 412 / 410 / 415 | 19 / 17 / 21 |
| P500 | 542 / 541 / 546 | 19 / 17 / 21 |
| P664 | 715 / 714 / 719 | 19 / 18 / 21 |

The search beats Rolletschek by 1–2 steps through the tail, which is where the certificate keeps its slack. Nothing reaches a count: the smallest margin is 2, on the secret ideal.

## 5. Proposed spec wording (not applied)

> **Reduction schedule (exp64).** The public step count for an admitted input class is T = K(R₀) + 6 for the exact contract, and the same T for exp64, except for the secret-ideal pre-reduction (table). Here R₀ = sup A₀²/Δ over the class, from the starting bounds the constructor or decoder establishes. No separate extra-step budget E applies. Counts are in any integer scaling of the Gram (O0 numerators or the normalized class Gram) and include the final size reduction.
>
> | Class | P248 | P324 | P376 | P500 | P664 |
> |---|---:|---:|---:|---:|---:|
> | Generic signing, Q4 | 280 | 360 | 415 | 546 | 719 |
> | Pre-reduced signing, guard 256 | 267 | 347 | 402 | 533 | 706 |
> | Key generation, norm D | 208 | 323 | 308 | 411 | 638 |
> | Secret-ideal pre-reduction, Q4 | 21 | 22 | 22 | 22 | 23 |
> | Challenge alone | 197 | 257 | 297 | 396 | 525 |
>
> Certificate: `research/tighter-progress-certificate` (cell potential ρ = 537/2000, λ bands to 2⁻¹¹, tail τ = 4, joint exp64 charge).

The conditions are the same as PR #45's: the constructor or decoder must establish each class's starting bounds A₀, C₀ and Δ (the guard at keygen and decode). The exp64 count no longer depends on λ₁ ≥ T or on a verified integral Gram.

## 6. Files and reproduction

| File | Contents |
|---|---|
| `cellcert/` | Rust: `search` (refinement and value iteration), `check` (integer certificate check), `ratio` (band ratios) |
| `cells.py`, `check_potential.py` | independent exact-rational checker |
| `counts.py` | lemmas, band caps, counts per class |
| `certify_all.py` | end to end; writes `results/step_counts_new.csv` and `results/certify_all.txt` |
| `validate.py`, `hillclimb.py` | corpus and adversarial search |
| `results/cert_2685.txt` | the certificate (leaves and g); other `cert_*.txt` files are the ρ = 0.30 … 0.269 steps toward it, and `cert_2690_l12/l13` are valid ρ = 0.269 certificates at λ ≤ 2⁻¹²/2⁻¹³ (not used) |
| `vendor/` | unchanged copies: `certify.py` (`lagrange-tour-bound`), `q5common.py`, `variants.py`, `instrumented.py`, `gen_inputs.py`, `adversarial.py`, `trace_counterexample.py`, `progress_check.py` (`clamped-quotient-progress` @ `59929d3`), `ngram.py` (`normalized-gram` @ `22392c3`, import path only) |

```bash
cd research/tighter-progress-certificate
(cd cellcert && cargo build --release)
PY=/workspace/mamba/envs/sage/bin/python3
$PY certify_all.py          # ~3 min (both checkers, tail, counts)
$PY validate.py             # ~2 min
$PY hillclimb.py p500 sign-generic 546 1281 900 7
# regenerate the certificate (search):
cellcert/target/release/cellcert search 2685/10000 20 6 6000000 results/cert_2685.txt 200
```

Open items:

- the production Rust reducer at the new counts;
- a second high-λ potential (worth at most 1–2 steps);
- a proof that the Rolletschek-plus-tail inputs found by the search are worst-case. Not needed for safety, since the counts are proven upper bounds.
