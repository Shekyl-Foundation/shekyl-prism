# Tail proof from 2⁻¹¹ and the P500/P664 adversarial search (SHE-24 follow-up)

Branch `experiment/tail-proof-and-search`, off `research` at `c9a92cd`. This follows up `research/certificate-review/` and Rick's t43u. The contract is PR #45's: a strict initial ordering swap, each quotient component rounded with `floor(h/A + 1/2)`, a swap only when the new vector is strictly shorter, and exp64 for bounded precision. Counts include the final size reduction. Everything here is own code; the only external input is the subject's certificate data (`../tighter-progress-certificate/results/cert_2685.txt`), used by the count script. PRs #47 and #48 are untouched.

## Verdict

| Item | Verdict |
|---|---|
| τ = 4 from λ ≥ 2⁻¹¹: at most 4 further swaps, then the final non-swapping step | **Proved independently** (`cellmap.c`, 5 s, no inherited facts). It covers the whole of [2⁻¹¹, 1], so it supersedes the earlier 2⁻⁹ split and no longer depends on `lagrange-tour-bound`. |
| The double-tie stall near λ ≈ 0.0052 | **Explained and avoided.** It is an artefact of chaining λ-only "fewer than 4 swaps" facts, which fail in closed-box form below 2⁻⁷·³ because of an exact tie identity (Sage, below). The new proof never needs that 4-swap fact. |
| Can self-proven counts drop back? | **Yes, all of them.** The secret-ideal pre-reduction goes back to 21 exact at P248/P376/P664 and 21/22/22/22/23 exp64; the challenge at P664 goes back to 525. Several counts drop **below** the claimed table (§3). |
| Adversarial search, P500 and P664, all five classes | **No counterexample.** Tightest margins are in §4. |

## 1. The tail proof

`cellmap.c` is an adaptive cell-mapping certificate. For k = 2…5 and a flag f, it builds kd-trees over states (λ, ε) ∈ [2⁻¹¹, 1] × [−½, ½]². Each tree tabulates the set G_k^f of post-swap states from which fewer than k further swaps occur. f = 1 records that the swap producing the state had r ≥ ½. G₁ = {r ≥ 1} is evaluated analytically.

A cell of G_k^f is GOOD if any one of these holds:
- r ≥ 1 on the cell;
- λ ≥ ½ (Endgame);
- f = 1 and r > ⅓ (Lemma C);
- for every quotient μ with |μ|² ≥ 2 (Lemma 1), the interval image of the cell's swapping part meets no BAD leaf of G_{k−1}^{f′}.

Other details:
- Interval arithmetic is IEEE double, rounded outward by one ulp at every operation.
- Cell boundaries are dyadic: λ is split linearly inside octaves, so no transcendental function is evaluated.
- Residual boxes use the closed square, and the swapping part uses r ≤ 1. Every tie choice and the r = 1 boundary are therefore over-covered. The exact rules (half-up, strict swap) enter only through Lemma 1, Lemma C and the Endgame, which hold for the contract.
- Unlike the earlier recursive certifier, every sub-result is computed once and shared. That is why it finishes in seconds where the recursion ran for hours.

Result (`results/cellmap_tail5_from_2^-11.txt`):
- **G₅⁰ ⊇ [2⁻¹¹, 1] × [−½, ½]²: fewer than 5 further swaps from every post-swap state with λ ≥ 2⁻¹¹.**
- Tree depths are 24/28/32/40.
- **Validation.** 3 × 10⁶ random states per tree were simulated under the contract in long double. No state with ≥ k swaps lies in a GOOD leaf of G_k (0 violations for k = 2…5), and 2,229 states have exactly 4 swaps, so τ = 4 is not slack in this state model.
- **Ablations** (`results/cellmap_ablation.txt`). Without Lemma C the proof fails, so Lemma C is load-bearing. Without Lemma 1 it fails on 12 tiny cells. Without the Endgame it still succeeds.
- **Limit** (`results/cellmap_below_2^-11.txt`). The same depths do not prove τ = 4 from 2⁻¹² or 2⁻¹³, so 2⁻¹¹ is the right split.

Lemma C was re-derived in `tail_lemmas.sage` (`results/tail_lemmas.txt`, ALL OK), and it holds on the closed square:
- r_t ≥ ½ gives |q_t,j| ≤ 1, so Lemma 1 forces |m_t,j| = 1 and |ε_t,j| = 1 − |e_j|/r_t.
- r_{t+1} > ⅓ forces |m_{t+1,j}| = 1 with the signs of q.
- The Lemma 1 identity r′ = (1 − 2Re(mε) + |m|² r)/r then turns "step t+2 swaps" into λ_t + Σ(|e_j|/r_t)² = 1/r_t < 1, which is impossible.

**The stall.** On the face ε_x = −½ with first quotient (−1, 1), Sage proves r₂ = 2(r₁ + e)/r₁. So the second step's q_y is exactly ½. Half-up takes m_y = 1 and the run stops after 3 swaps, with Lemma 1 holding at equality, r = 1, so the strict swap does not fire. A closed box also admits m_y = 0 and r ≤ 1, so "fewer than 4 swaps from 2⁻⁷·⁶" cannot be closed by intervals. The 5-swap statement over the closed boxes does hold, and that is all the count needs.

## 2. Counts with the tail proved from 2⁻¹¹

With τ = 4 from 2⁻¹¹ now independent, the review's own band ratios (exact enclosures, `../certificate-review/results/cellcheck.txt`) give a fully self-proven table (`results/mycounts_full.txt`, row `review-ratios`). Counts are P248 / P324 / P376 / P500 / P664.

| Class | Adopted (t43u) | Claimed (SHE-23) | **Fully self-proven now** |
|---|---|---|---|
| Signing, generic | 280/360/415/**546**/719 | 280/360/415/546/719 | 280/360/415/**545**/**718** |
| Pre-reduced, guard 256 | 267/347/402/**533**/706 | 267/347/402/533/706 | 267/347/402/**532**/**705** |
| Keygen, norm D | 208/323/308/411/638 | same | 208/323/308/**410**/**637** |
| Secret-ideal pre-reduction, exact | 22/21/22/21/22 | 21 ×5 | **21 ×5** |
| Secret-ideal pre-reduction, exp64 | 22/22/23/22/24 | 21/22/22/22/23 | **21/22/22/22/23** |
| Challenge | 197/257/297/396/526 | 197/257/297/396/525 | **196**/257/297/**395**/**525** |

- **Every adopted number remains a valid upper bound.**
- **Dropping back is now supported:**
  - secret ideal: exact 22 → 21 at P248/P376/P664; exp64 22/22/23/22/24 → 21/22/22/22/23;
  - challenge at P664: 526 → 525.
- **Optional further cut.** The self-proven table goes one below the claimed one in signing (545 at P500), guard 256 (532), keygen-D (410) and the challenge (395 at P500). That is a decision for Rick; the adopted 546/533 stay safe.
- **Possible follow-up, not adopted.** Secret-ideal exp64 = 21 at every set would need the producer to guarantee the standard generic HNF, plus a derivation of its head bound under the 2⁻¹¹ split.

## 3. Adversarial search, P500 and P664

`hunt.py` searches integer Hermitian Gram matrices with A₀²/Δ ≤ R₀ and max(A₀, C₀)²/Δ ≤ R_C. That is a superset of every admitted class. The need is measured by running the contract forward under both the exact rule and exp64, and it includes the final size reduction.

Strategies:
- **Phase 1 (55 %):** chain hill climbing, i.e. backward Gaussian-quotient chains with `../certificate-review/hillclimb.py`'s moves, plus tie starts (residual exactly −½, or ½ − 1/C).
- **Phase 2 (45 %):** polish on the best eight inputs. Moves pick an intermediate state of the forward run and either snap a quotient component to an exact half or one unit off it (sometimes both components, a double tie), or set λ_j to 2⁻¹¹, 2⁻⁹, ½, 1023/1024 or 2⁻¹⁶, or perturb it. The state is then mapped back to G₀ through the recorded unimodular steps.

New compared with the earlier review: chains are **truncated at the longest prefix meeting both bounds** instead of being rejected. The old search lost almost every long chain in the guarded class (220/267 at P248); the new one reaches 529/533 at P500.

The budget was 1,800 s per cell (10 cells in parallel on 8 cores, seed 11). Raw output is in `results/hunt/*.json`, with witnesses as Gram vectors.

| Set | Class | Evaluations (chain + polish) | Best need, exact / exp64 | Claimed | Adopted | Self-proven now | Margin vs adopted | Margin vs self-proven |
|---|---|---:|---|---:|---:|---:|---:|---:|
| P500 | signing, generic | 158k + 208k | 543 / 543 | 546 | 546 | 545 | 3 | **2** |
| P500 | guard 256 | 215k + 182k | 529 / 529 | 533 | 533 | 532 | 4 | 3 |
| P500 | keygen, norm D | 231k + 250k | 408 / 408 | 411 | 411 | 410 | 3 | **2** |
| P500 | secret-ideal pre-reduction | 3.47M + 5.40M | 20 / **21** | 21 / 22 | 21 / 22 | 21 / 22 | 1 / 1 | **1 / 1** |
| P500 | challenge | 230k + 455k | 393 / 393 | 396 | 396 | 395 | 3 | **2** |
| P664 | signing, generic | 142k + 126k | 716 / 716 | 719 | 719 | 718 | 3 | **2** |
| P664 | guard 256 | 137k + 124k | 702 / 702 | 706 | 706 | 705 | 4 | 3 |
| P664 | keygen, norm D | 149k + 166k | 635 / 635 | 638 | 638 | 637 | 3 | **2** |
| P664 | secret-ideal pre-reduction | 3.50M + 5.01M | 20 / **22** | 21 / 23 | 22 / 24 | 21 / 23 | 2 / 2 | **1 / 1** |
| P664 | challenge | 184k + 266k | 522 / 522 | 525 | 526 | 525 | 4 | 3 |

- **No counterexample.** No input exceeds any adopted, claimed or newly self-proven count.
- **The tightest margin is 1 step, on the secret-ideal pre-reduction.**
  - exp64 at P500 needs 21 against 22, with one big sub-step.
  - exp64 at P664 needs 22 against 23, with two big sub-steps.
  - Exact needs 20 against 21.
- **Secret-ideal 21-everywhere option.** These Gram-space witnesses already need 22 exp64 slots at P664. Without the standard generic HNF guarantee, 21 everywhere would be **false** at P664. The follow-up must therefore restrict the input, not just re-derive the bound.
- Every other class comes within 2 to 4 steps.
- In the polynomial classes the best witnesses came from chain hill climbing. The near-tie and boundary-λ polish moves found no deeper input, and exp64 need equalled exact need (no big sub-steps).

## 4. Recommendations

- **Keep 546 / 533 at P500.** They are supported with 3–4 steps of search slack.
- 545 / 532, and the other one-lower numbers in §2, are now self-proven as well. Adopting them is a margin decision for Rick.
- **Secret-ideal pre-reduction may go back to 21 exact and 21/22/22/22/23 exp64.** The challenge at P664 may go back to 525.
- Do not adopt secret-ideal exp64 = 21 everywhere without the HNF producer guarantee (§3).

## 5. Files

| File | Contents |
|---|---|
| `cellmap.c` | cell-mapping tail certificate; `gcc -O2 -frounding-math -o cellmap cellmap.c -lm && VALIDATE=3000000 ./cellmap 11 5 24,28,32,40 11 0` |
| `tail_lemmas.sage` | Lemma 1 identity, Lemma C, the tie identity behind the old stall |
| `exact_path.py` | exact-rational trajectories at the degenerate double tie (3 swaps, then r = 1 exactly) |
| `hunt.py` | P500/P664 search: `python3 hunt.py p500 sign-generic 1800 11` |
| `results/` | `cellmap_tail5_from_2^-11.txt`, `cellmap_ablation.txt`, `cellmap_below_2^-11.txt`, `tail_lemmas.txt`, `mycounts_full.txt`, `hunt/*.json` |
