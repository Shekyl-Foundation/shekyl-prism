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
