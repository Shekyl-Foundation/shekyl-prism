# Independent review of the tighter progress certificate (SHE-24)

Branch `experiment/certificate-review`, off `dev` at `904d876`. Subject: `research/tighter-progress-certificate/` as merged into `research` at `8a2615f` (commits `14e9cbc` / `10789dc` / `c13e010`), together with Rick's follow-up `df92507` (head enumeration, merged into `research` through PR #46). The contract is PR #45's (head `84a798a`): a strict initial norm-ordering swap, each quotient component rounded with `floor(h/A + 1/2)`, a swap after the subtraction only when the new vector is strictly shorter, and exp64 for bounded precision. Counts include the final size reduction.

Method: an adversarial re-derivation. Every script here was written from the mathematics in the subject report and the PR #45 spec. No checker code was reused. The only artefact read from the subject is its certificate data, `results/cert_2685.txt` (leaves and g), plus the line that gives its leaf-to-box convention. Tools: exact rationals (gmpy2 `mpq`, `fractions`), Sage for the identities, and a C interval branch-and-bound in IEEE double where every operation is rounded outward by one ulp.

## Verdict

| Item | Verdict |
|---|---|
| (1) First-step bound, in every scaling | **Confirmed, with the correction in `df92507`.** As merged at `8a2615f`, the head enumeration had a proof gap: it dropped first-step big-quotient counts k whose R_k falls below the chain threshold. The gap does not change any number. |
| (2) Charge across the main-to-tail boundary | **Arithmetic confirmed; tail only partly reproduced.** The chain, the band caps, the band ratios and the exp64 boundary and charge inequalities are all confirmed. τ = 4 is proved independently from λ ≥ 2⁻⁹ but not from 2⁻¹¹ (see §2). For (2⁻¹¹, 2⁻⁹] the subject's tail still rests on `certify_tail_exact` from `lagrange-tour-bound`. |
| (3) Final size reduction after the last swap, half ties included | **Confirmed.** |
| Cell coverage and the potential inequality | **Confirmed.** The 8,758 leaves tile the square. g ≥ 1 and G = 1.862201131. The ratio is bounded over whole cells, interiors included: the main ratio is exactly 0.268499973 ≤ 537/2000. Every band holds with a large margin. |
| Inherited lemmas against the contract | **Confirmed.** Lemma 1 and the Endgame are proved symbolically and depend on the strict swap. exp64 Lemmas W, U and B2 were tested on a fresh implementation. |
| Step-count arithmetic | **Confirmed exact.** All 60 claimed exact and exp64 numbers (6 classes × 5 sets × 2 rules) are reproduced in integer and rational arithmetic, using the subject's ratios. |
| Counterexamples | **None.** No input found exceeds any count; see §5. |

## 1. First-step bound (Rick's item 1)

- **Units.** Sage (`lemmas.sage`, `results/lemmas.txt`) gives Δ = 4pT² in O0 numerators, pT²/4 in reduced-norm units and p for the normalized Gram `(2T, (x²+y²+p)/2T, x, y)`. A₀²/Δ = 4T²/p in all three, and the 4×4 real Gram determinant is Δ². The step recurrence uses only λ = Δ/A², r = λ + |ε|² and q′ = conj(ε)/r, so every count depends only on R₀ = sup A₀²/Δ and, for exp64, R_C = sup C₀²/Δ. The numbers are the same in all three scalings.
- **Head budget.** |q₁|² = C₀/A₀ − Δ/A₀² ≤ C₀/A₀. Sage confirms C₀²/Δ − 4|Q|² = R(|Q|² − 1/R)² ≥ 0, which gives |q₁|² ≤ R_C/4. By Lemma B2, n₁ = k ≥ 1 big sub-steps need |q₁|² > 2^(116k+4). That forces both 4^(58k+3) < R_C and A₀²/Δ < R_C/16^(58k+2) = R_k. Both conditions are confirmed.
- **The gap and its fix.** At `8a2615f` the enumeration stopped at 16^(58k+2) < R_C. That drops values of k for which R_k is below the chain threshold, where the run goes straight into the tail but still pays k slots: at most 6 + k. `df92507` widens the enumeration to 4^(58k+3) < R_C. My own enumeration (`mycounts.py`) uses K = −1, meaning no main-phase transition is possible and the run is tail-only. It reproduces all 35 pairs, so the gap was real but changes no number.
- **Other first-step cases.**
  - Transition 1 uses an unconstrained ε₀; the cell check needs no history.
  - Lemma 1 at step 2 needs step 1 to swap, and the transition requires that.
  - If λ₀ > 2⁻¹¹, the run is at most 1 + 4 swaps plus the final step, which is 6 or fewer.
- **Optional tightening, not a defect.** The secret-ideal exp64 rows (+1 or +2) come from taking the sup of R_C over every T ≥ 1. At T = 1, C₀ ≈ p, so R_C ≈ p/4. For a generic HNF pair the two values are tied:
  - Unswapped, |q₁|² = (x²+y²)/4T² < 2, so the first step is never big.
  - Swapped, a big first step forces A₀²/Δ ≤ R₀,max/16^(58k+2).
  - If the admitted secret-ideal input is always that HNF pair, exp64 needs 21 at every set, the same as exact.

## 2. Main-to-tail boundary (Rick's item 2)

- **Chain.** Let J* be the last swap with λ_{J*−1} ≤ 2⁻¹¹. Transitions 1 … J*−1 give Π·G·A₀ ≥ Ψ_{J*−1} ≥ A_{J*−1} ≥ √(2¹¹Δ). So J* − 1 ≤ K(R₀), and the total is at most J* + τ + 1 = K + 6. I re-derived this independently, including the case where J* does not exist.
- **Band caps** 3/2/1/1/1 are re-derived exactly from λ′/λ ≥ (½ + 2⁻¹¹)⁻².
- **Band ratios** (`cellcheck.py`, `results/cellcheck.txt`; 200,000 sampled points per λ, no violation):

| λ ≤ | Subject's bound | This review (exact enclosure) | Sampled sup |
|---|---:|---:|---:|
| 2⁻²⁰ | 0.2685 | 0.268499973 | 0.268499658 |
| 2⁻¹⁶ | 0.271313 | 0.268525145 | — |
| 2⁻¹⁴ | 0.300770 | 0.268605694 | — |
| 2⁻¹³ | 0.300838 | 0.268713094 | — |
| 2⁻¹² | 0.300973 | 0.272906432 | — |
| 2⁻¹¹ | 0.304449 | 0.273153947 | 0.273148798 |

  The subject's band ratios are valid but loose. With mine, the counts drop by one in some cells, for example signing at P500 goes to 545 (table in §4). That holds only if τ = 4 from 2⁻¹¹ holds.
- **exp64 at the boundary** (`exp64_charge.py`):
  - The charge G·2^−(116n+4) ≤ ρ^(1+n) and the boundary bound 2^(116n+4) ≥ 2¹¹ρ^−(2(1+n)) both hold for every n ≥ 1.
  - The step after J* satisfies |q|² ≤ 1/r_{J*} ≤ 1/λ_{J*−1} = A_{J*−1}²/Δ, so its big sub-steps fit inside K. That covers the terminal step when J* = n.
  - The tail has no big quotients, since |q|² < 2¹¹.
  - Every step is accounted for: step 1 by the head, steps 2 … J* by the chain, step J*+1 by the boundary, and the rest by the tail.
- **Tail τ = 4: my own certifier only partly reproduces it** (`tailcert.c`; state = post-swap (λ, ε) with ε free in the closed square). It uses three analytic cut-offs, each proved in `lemmas.sage`:
  - the Endgame: λ ≥ ½ allows at most one more swap;
  - a new **Lemma C**: if a swap had r ≥ ½ and the next step has r > ⅓, the next two steps cannot both swap, because the third swap would need r > 1;
  - Lemma 1.

  Chained proofs:
  - fewer than 3 swaps from 2⁻⁴;
  - fewer than 4 swaps from 2⁻⁷ and from 2⁻⁷·³;
  - **fewer than 5 swaps from every λ ≥ 2⁻⁹**, in the pieces [2⁻⁹, 2⁻⁸], [2⁻⁸, 2⁻⁷·³] and [2⁻⁷·³, 1].

  On (2⁻¹¹, 2⁻⁹) it ran for about an hour on four cores without finishing. The boxes it could not resolve sit at a degenerate double tie, λ ≈ 0.0052 with ε = (−½, −0.2799): a half tie q_y = +½, then two steps later r = 1 exactly with |μ|² = 1. That is the equality case of Lemma 1. There, only round-half-up together with the strict swap rules out the extra swap, and a closed-box enclosure cannot settle it. Exact simulation in that region gives at most 3 swaps from those states, and no 5-swap tail turned up by search. I have no evidence the claim is false. I simply could not re-prove it independently in the time available. The `4:7.6` and `4:8` attempts are logged as unresolved.
- **Counts on fully review-proven pieces only** (split at 2⁻⁹; bands up to 2⁻⁹, ratios 0.275566686 and 0.289625053):
  - signing, guard 256 and keygen-D are **the same as claimed** at every set;
  - the secret ideal is 22/21/22/21/22 exact and 22/22/23/22/24 exp64, against a claimed 21 and 21/22/22/22/23;
  - the challenge is 526 at P664, against 525.

  These are the minimum adjustments if Rick wants only independently checked counts.

## 3. Final size reduction (Rick's item 3)

- With n swaps the exact run needs n + 1 steps. The τ + 2 term already includes that last step.
- After a step that does not swap, each residual lies in [−A/2, A/2): +A/2 rounds to 1 and −A/2 rounds to 0. Checked in 20,000 cases.
- Equal norms do not swap.
- In exp64 the window path at the fixed point is exact, by Lemma W.
- **The p=3, T=5 fixture** (`test_contract.py`, own coordinate reducer), from U = (10,0,0,0), V = (4,9,1,0) with Δ = 300 = 4pT²:
  - Step 1 subtracts i·U and swaps, giving U = (4,−1,1,0) with N4 = 20, already the minimum.
  - Step 2 has q = (2, +½). The +½ tie rounds up, so m = 2 + i and V = (1,−2,−2,−1), also of norm 20. There is no swap.
  - The fixed point is reached after **2** active steps. Its residual is exactly −½ (h₁ = −10 = −A/2), a negative tie that stays put.
  - A 1-step schedule would stop with V not size-reduced.
  - An exhaustive search confirms 20 as the minimum.
- The Gram recurrence matches the coordinate reducer on 300 random O0 bases, 60 steps each.

## 4. Independently derived counts

These come from `mycounts.py` (`results/mycounts.txt`, `results/mycounts.csv`). Classes are defined from the PR #45 parameter vectors:

| Class | R₀ (O0, dimensionless) | R_C |
|---|---|---|
| Generic HNF | 4T²/p | sup over T of (8T²+p)²/(4pT²) |
| Guard F | 4F²q² | 4F²(1+q)⁴/q² |

Counts are P248 / P324 / P376 / P500 / P664.

| Class | Claimed exact / exp64 | Review, subject's ratios | Review, tight ratios (τ=4 from 2⁻¹¹) | Review, fully self-proven split at 2⁻⁹ |
|---|---|---|---|---|
| Signing | 280/360/415/546/719 (both) | same | 280/360/415/**545**/**718** | 280/360/415/546/719 |
| Pre-reduced, guard 256 | 267/347/402/533/706 (both) | same | 267/347/402/**532**/**705** | 267/347/402/533/706 |
| Keygen, norm D | 208/323/308/411/638 (both) | same | 208/323/308/**410**/**637** | 208/323/308/411/638 |
| Secret-ideal pre-reduction | 21 ×5 / 21/22/22/22/23 | same | same | **22**/21/**22**/21/**22** / 22/22/23/22/24 |
| Challenge | 197/257/297/396/525 (both) | same | **196**/257/297/**395**/525 | 197/257/297/396/**526** |
| Guard 2 (not selected) | 263/344/398/529/702 | same | 263/**343**/398/529/702 | **264**/344/**399**/529/702 |

## 5. Adversarial search

Own hill climber (`hillclimb.py`) over integer Hermitian Gram matrices. This is a superset of every admitted class, because the certificate is claimed for any Gram with A₀²/Δ ≤ R₀. Inputs are backward chains of Gaussian quotients from a post-last-swap state, plus an optional big first quotient. Mutations: change or insert quotients, perturb the start, add a head. Need = slots until the fixed point under the exact rule and under exp64. The campaign was planned at 25 min (secret ideal), 20 min (signing) and 7 min (other classes) per cell. It was cut short when the session was interrupted: 13 cells finished, P500 has only 45-second probes, and P664 was not searched.

| Set | Class | Seconds | Evaluations | Best need exact / count | Best need exp64 / count |
|---|---|---:|---:|---|---|
| p248 | challenge | 420 | 674559 | 195 / 197 | 195 / 197 |
| p248 | keygen-D | 420 | 633617 | 206 / 208 | 206 / 208 |
| p248 | keygen-Isec | 1500 | 16632391 | 20 / 21 | 20 / 21 |
| p248 | prered-guard256 | 420 | 815155 | 220 / 267 | 220 / 267 |
| p248 | sign-generic | 1200 | 1767092 | 278 / 280 | 278 / 280 |
| p324 | challenge | 420 | 599875 | 255 / 257 | 255 / 257 |
| p324 | keygen-D | 420 | 593829 | 321 / 323 | 321 / 323 |
| p324 | keygen-Isec | 1500 | 16952342 | 20 / 21 | 20 / 22 |
| p324 | prered-guard256 | 420 | 971279 | 316 / 347 | 316 / 347 |
| p324 | sign-generic | 1200 | 1613575 | 357 / 360 | 357 / 360 |
| p376 | challenge | 420 | 119230 | 294 / 297 | 294 / 297 |
| p376 | keygen-D | 420 | 461681 | 306 / 308 | 306 / 308 |
| p376 | prered-guard256 | 420 | 785310 | 386 / 402 | 386 / 402 |
| p500 | sign-generic | 45 (probe) | 37,907 | 542 / 546 | 542 / 546 |
| p500 | keygen-Isec | 45 (probe) | 514,138 | 20 / 21 | 20 / 22 |

- **No input exceeds any count.**
- The tightest margin is **1 step**, on the secret-ideal pre-reduction: need 20 against a count of 21 at P248, P324 and P500. The subject's search reported 19 there, so this Gram-space search beats it by one step. It is still inside the count, and inside the review's split-at-2⁻⁹ counts too.
- Signing comes within 2–4 steps of its count: 278/280, 357/360, 542/546.
- Key generation (norm D) and the challenge come within 2–3.
- The guard-256 cells are weak tests (220/267 and others). The search also enforces that class's C₀ bound, R_C ≈ R₀, which truncates the backward chains. They are not evidence of extra slack.
- No witness needed a big first quotient: exp64 need equalled exact need on every reported witness.

## 6. What this means for 533 / 546 at P500 (Rick, t42u)

**The review supports 546 (signing) and 533 (guard-256 pre-reduced) at P500, for both the exact rule and exp64.** Both numbers come out of two derivations:

- the subject's chain re-derived with its own ratios. That one needs the inherited τ = 4 tail from 2⁻¹¹.
- the split at 2⁻⁹ that I proved myself. That one needs my band ratios up to 2⁻⁹ and my tail proof from 2⁻⁹, and does **not** depend on the inherited tail.

The same holds for keygen norm D at 411 and the challenge at 396 at P500.

The conditions are the ones PR #45 already states. The constructor or decoder must establish the class bounds: the factor-256 guard at both keygen and decode, the ideal determinant, and the starting norms. Moving from 594 to 533 is a proof decision for Rick; nothing in this review argues against it. The cells where the self-proven split exceeds the claimed counts are not at P500: the secret-ideal pre-reduction at P248, P376 and P664, and the challenge at P664 (§4).

## 7. Minimum adjustments

- **None are needed for soundness**, provided the inherited tail certificate (τ = 4 from 2⁻¹¹, `lagrange-tour-bound`) is accepted. Every other link is independently confirmed.
- **To rely only on independently re-proved pieces,** take the split-at-2⁻⁹ column:
  - secret ideal: exact 22 at P248, P376 and P664; exp64 22 / 22 / 23 / 22 / 24;
  - challenge at P664: 526.
  - Signing (546 at P500), guard 256 and keygen-D stand as claimed.
- **Cite `df92507`'s head enumeration** in the spec wording, not `8a2615f`'s.
- **Documentation nit.** The subject says the quadtree has "depth ≤ 28"; the certificate's maximum depth is 14.

## 8. Files

| File | Contents |
|---|---|
| `cellcheck.py`, `sample_cells.py` | tiling, g, and the per-cell ratio with exact projection enclosures; Monte-Carlo cross-check |
| `tailcert.c` | rigorous interval tail certifier with Endgame, Lemma C and chained-fact cut-offs |
| `lemmas.sage` | units, recurrence, Lemma 1, Endgame, Lemma C, head inequality |
| `mycounts.py`, `exp64_charge.py` | counts (exact rationals), exp64 charge and boundary |
| `reducer.py`, `test_contract.py` | own contract and exp64 implementation; fixture; Lemmas W, U and B2 |
| `hillclimb.py` | Gram-space adversarial search |

```bash
cd research/certificate-review; PY=/workspace/mamba/envs/sage/bin/python3
C=../tighter-progress-certificate/results/cert_2685.txt
$PY cellcheck.py $C 16:271313/1000000 14:300770/1000000 13:300838/1000000 12:300973/1000000 11:304449/1000000
$PY mycounts.py $C; $PY exp64_charge.py $C; $PY test_contract.py; sage lemmas.sage
gcc -O2 -frounding-math -o tailcert tailcert.c -lm
./tailcert 4 3 40 1 1; ./tailcert 7.3 4 44 1 1 3:4; ./tailcert 9:8 5 40 1 1 3:4 4:7.3   # etc., see results/tail_*.txt
$PY hillclimb.py p500 sign-generic 1200 7
```
