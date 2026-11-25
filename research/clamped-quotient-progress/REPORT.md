# A progress proof for the bounded-precision quotient, and the public step counts of the reduction contract

Date: 2026-11-25. Branch `experiment/clamped-quotient-progress`, folder
`research/clamped-quotient-progress/`. PRISM is the signature scheme of
ePrint 2026/443 (Basso, Borin, Castryck, Corte-Real Santos, Invernizzi,
Leroux, Maino, Vercauteren, Wesolowski); shekyl-prism implements it. The
Shekyl parameter sets, and the only ones, are P248, P324, P376, P500 and
P664. Research only: nothing here changes `dev`, the signer,
`SIGNER_SPEC.md` or a parameter set, the production reducer is not
implemented, and no constant-time claim is made.

## Summary

- **The reduction contract is what `T_prov` was certified under.** Rick's
  contract: (1) an initial swap only if strictly shorter; (2) each
  quotient component rounded `floor(h/A + 1/2)` (half-up); (3) after the
  subtraction, swap only if the new vector is strictly shorter. The
  certificates of `research/lagrange-tour-bound` use exactly this
  rounding (`floor_half_up`, checked equal to the signer's
  `floor((2h + A)/(2A))` on 48,006 rationals, ties going up) and a strict
  swap (Lemma 1, which the certificate relies on, is false under a
  non-strict swap). Rerun here unchanged: ρ₁ = 31/100, ρ₂ = 87/1000 and
  "no 5 tail swaps" all hold (section 1).
- **Every published count is reproduced exactly with integer arithmetic**
  (the original final step used floating-point log₂): 301 / 387 / 446 /
  587 / 773 (signing), 211 / 276 / 319 / 425 / 565 (challenge alone),
  283 / 369 / 428 / 569 / 755 (pre-reduced, guarded), 354 / 461 / 536 /
  712 / 945 (pre-reduced, unguarded). New classes: key generation's ideal
  of norm D, keygen pre-reduction of I_sec, and a looser keygen guard
  (table 1). **Each count includes the final size reduction after the last
  swap; no extra step is needed** under the exact contract. The early-exit
  counts of `lagrange-tour-bound` (for example 276 on Rolletschek chains)
  include a no-op verification pass and are one higher than the fixed
  schedule needs (275).
- **Verdict on the 64-bit clamped quotient: no progress proof exists, and
  none can.** A clamped step subtracts a bounded amount (at most 2⁶²) from a
  quotient that, for valid Q4-bounded ideals, can be as large as about
  2^(bits(T)/2), which is 2⁶⁴⁰ at P500. It never swaps, and it leaves A, the
  rounding residual, and the certificate's potential Φ exactly unchanged.
  Progress is additive against a logarithmic budget. On the Q5
  counterexample the first quotient is 2^503.9 at P500, so the clamp would
  need more than 2⁴⁴¹ steps. The same holds for a 128-bit clamp, which
  fails the same inputs. Separately, even below the clamp the prototype's
  rounding is not the contract's: for |q| ≳ 2⁴⁰ it can be off by one.
- **A bounded-precision quotient with a proof: `exp64`.** It keeps the
  prototype's shape (top 64 bits of A, a 128-by-64 division) and adds two
  things:
  - (a) an exact ±1 correction, `X = 2h + A − 2mA`, which makes every
    in-window step exactly the contract's step;
  - (b) for quotients beyond the window (|q| ≥ 2⁶⁰), a floating
    quotient `m·2^s` that undershoots by one unit. That sub-step provably
    never swaps, never lengthens v, and cuts the quotient by at least 58
    bits.

  The run is then the exact run with at most E extra sub-steps inserted,
  where 58E + 2 < ½·log₂(C₀/λ₁). **Certified count = T_exact + E**, for
  example **598 at P500** (signing), against 587. There is no exact
  cleanup step. Machine-checked (section 4). On every adversarial and
  honest input it ends on exactly the exact schedule's basis, the Rust
  port is bit-exact with the model, and it costs **1.17× (P500) to
  1.31× (P248)** the prototype's cycles. That is against **about 27× to 47×**
  for an exact full-width quotient at every step.
- **Fallbacks.** Treating the clamp as a sticky overflow that aborts
  (`corr64`) is cheaper (it needs no big path), but it has only a heuristic
  honest failure rate of about 2⁻¹¹⁰ per reduction. Crafted keys trigger it
  deterministically, and it conflicts with O33's "unreachable by proof". A
  wide fixed-size quotient is either unsafe (a 128-bit clamp fails) or must
  hold about 2^(bits(T)/2), which is the exact-quotient cost.
- **Recommendation.** Adopt the exact contract's counts (table 1) as the
  public step counts, and `exp64` as the bounded-precision quotient that
  realises them, with the counts of table 2, dropping the exact cleanup step.
  Constant-time cost: prism-int needs a secret bit length, secret-amount
  shifts in both directions, a fixed-iteration 128/64 division and an
  N-limb scalar multiply (section 7).

## Handoff tables

Counts are the number of fixed steps after the initial strict swap. Every
row covers the final size reduction after the last swap: after the
schedule, u is a shortest vector and v is size-reduced against it
(|h_j| ≤ A/2, N4(u) ≤ N4(v)). The coordinate system (model order or O0
numerators) does not change any count: N4 scales A, C, Δ and λ₁ by 4, 4,
16 and 4, which leaves every ratio used here unchanged.

**Table 1. Exact contract (round-half-up at full width, strict swaps)**

| Admitted input class | Bound used | P248 | P324 | P376 | P500 | P664 | Source |
|---|---|---:|---:|---:|---:|---:|---|
| Signing: I_sec·I_q, generic HNF, Q4 (N < 2^B) | A₀ ≤ T², T < 2^(B+2e−5) | **301** | **387** | **446** | **587** | **773** | published, reproduced |
| Signing from a pre-reduced or decoded key, guard nrd(v_s) ≤ 2√p·N checked at keygen and at decode | A₀ ≤ 2√p·N·q² | **283** | **369** | **428** | **569** | **755** | published, reproduced |
| Same, looser guard factor 256 (rejects none of 10,000 keys, Q5 6.2) | A₀ ≤ 256√p·N·q² | 287 | 373 | 432 | 573 | 759 | new |
| Same, Minkowski bound nrd(v_s) ≤ 2p·N only (no guard) | A₀ ≤ 2p·N·q² | 354 | 461 | 536 | 712 | 945 | published, reproduced |
| Key generation: ideal of norm D = nextprime(2^(2W)), generic HNF | A₀ ≤ D² | 223 | 347 | 331 | 441 | 686 | new (Q5 ran it at the signing count) |
| Keygen pre-reduction (or decode-time reduction) of I_sec alone, Q4 | A₀ ≤ N², N < 2^B | 22 | 22 | 22 | 22 | 22 | new |
| Challenge I_q alone (public input) | A₀ ≤ q², q < 2^(2e−5) | 211 | 276 | 319 | 425 | 565 | published, reproduced |

A decoded pre-reduced basis has a public count only if the decoder checks
N4(u_s) ≤ N4(v_s) ≤ the stated bound. Without that check A₀ is unbounded
and no count holds. Bases of non-generic HNF shape are excluded by the key
check of Q5 (lemma 5), not by these counts.

**Table 2. `exp64` (bounded precision, this report), no cleanup step**

| Class | E (valid ideal, λ₁ ≥ T) | P248 | P324 | P376 | P500 | P664 | Any integral basis (λ₁ ≥ 1) |
|---|---|---:|---:|---:|---:|---:|---|
| Signing, generic | 5 / 7 / 8 / 11 / 14 | **306** | **394** | **454** | **598** | **787** | 312 / 401 / 462 / 609 / 802 |
| Signing, pre-reduced, guard 2 | 5 / 6 / 8 / 10 / 14 | **288** | **375** | **436** | **579** | **769** | 293 / 383 / 444 / 590 / 783 |
| Signing, pre-reduced, guard 256 | 5 / 6 / 8 / 10 / 14 | 292 | 379 | 440 | 583 | 773 | 297 / 387 / 448 / 594 / 787 |
| Signing, pre-reduced, unguarded | 6 / 8 / 9 / 12 / 17 | 360 | 469 | 545 | 724 | 962 | 365 / 476 / 554 / 735 / 976 |
| Key generation, norm D | 4 / 6 / 6 / 8 / 13 | 227 | 353 | 337 | 449 | 699 | 231 / 360 / 344 / 458 / 712 |
| Keygen pre-reduction of I_sec | 1 / 1 / 1 / 2 / 3 | 23 | 23 | 23 | 24 | 25 | 24 / 25 / 25 / 26 / 28 |
| Challenge alone | 4 / 5 / 6 / 8 / 11 | 215 | 281 | 325 | 433 | 576 | 219 / 287 / 331 / 442 / 587 |

These counts also cover the final size reduction. The "any integral basis"
column does not use λ₁ ≥ T, only that N4 of a nonzero vector is at least 1.
It is the safe choice if the reducer may ever see a basis that is not
exactly the ideal it claims to be (section 8). It costs 1% to 4% more steps.

**The 64-bit clamp of `rust-lagrange-ct` (T_prov steps plus an exact
cleanup): no certified count exists.** Table 1's counts do not apply to it.

## 1. The contract, and the certificate re-checked

The algorithm, from any Z[i]-basis {U, V} with A = N4(U), C = N4(V) and
Hermitian form h(V, U) = (h0, h1):

1. If N4(V) < N4(U), swap (strict).
2. Each step: m_j = ⌊h_j/A + 1/2⌋ = ⌊(2h_j + A)/(2A)⌋; V ← V − m0·U −
   m1·iU; if N4(V) < A, swap (strict).

`progress_check.py` Part C:

- **Certificates rerun**, with `certify.py` copied unchanged:
  - 1-step ρ₁ = 31/100: ok, 72,751 boxes.
  - 2-step ρ₂ = 87/1000: ok, 83,751 boxes.
  - No 5 tail swaps after the split λ > 1/1000: ok, 309,007 boxes.
  - All in exact rationals (`results/progress_check.txt`).
- **Rounding.** `certify.floor_half_up(f)` equals ⌊f + 1/2⌋, which equals
  the signer's ⌊(2h + A)/(2A)⌋ on 48,006 rationals. +1/2 goes to 1 and −1/2
  goes to 0, so the residual ε = q − m lies in [−1/2, 1/2)². The
  certificate's boxes use the closed square, a superset, so they are
  conservative.
- **Strict swap.** The certifier keeps a box only where r = λ + |ε|² < 1 is
  possible. It discards quotients with |μ|² ≤ 1 by Lemma 1, which says
  A_{t+1} ≥ A_t for those. Only a non-strict swap (≤) could fire at
  equality, so **the certificate is valid for the strict rule and would
  need re-checking for a non-strict one.** The endgame lemma ("the equality
  case prevents the swap") also uses strictness.
- **The prototype's code** matches the contract:
  - Its initial and per-step swaps are `lt_unsigned(nv, nu)`, which is strict.
  - Its 64-bit and exact roundings are both half-up: `exact_round` uses
    ⌊(h + ⌊A/2⌋)/A⌋, which equals ⌊(2h + A)/(2A)⌋ for odd A too.
  - Only its *precision* departs from the contract.

**Integer recomputation.**
- **The step-count formula.** T = S_max + 5, where:
  - S_max is the largest S with (ρ(S)·Φ₁)² ≥ 1000·Δ;
  - ρ(S) = ρ₂^⌊(S−1)/2⌋·ρ₁^((S−1) mod 2);
  - Φ₁ ≤ 2A₀.
- **The ratio Φ₁²/Δ per class:**
  - generic HNF: 4T²/p;
  - pre-reduced with guard F: 4F²q²;
  - unguarded: 16p·q².
- **Agreement.** `provable_bound_for` used floats. The integer version
  agrees at every class and set, and every published number is reproduced
  (asserted).

**What the count covers.** The proof splits at the first swap S with
λ_S > 1/1000:
- at most S_max main-phase swaps;
- at most 4 tail swaps;
- then one step that does not swap, which is the size reduction of v
  against the final u. By the immediate-fixed-point property, the basis
  is then reduced.

So S_max + 5 is the number of fixed steps that ends on a reduced basis,
size reduction included. An early-exit implementation spends one more
pass to see m = 0. `adversarial.py` measures, per input, the first step
after which the basis is reduced and stays reduced. The largest values
are all on Rolletschek chains:

| | P248 | P324 | P376 | P500 | P664 |
|---|---:|---:|---:|---:|---:|
| Steps needed | 275 | 355 | 410 | 541 | 713 |
| Count | 301 | 387 | 446 | 587 | 773 |

These match `lagrange-tour-bound`'s early-exit 276 / 356 / 411 / 542 / 714
minus the verification pass.

## 2. Why the clamped step stalls

`trace_counterexample.py` on the Q5 ideal (`crafted_saturation.py`, seed
77). The ideal is {T, β} where β = a + bi + j has nrd(β) = kT with
k ≈ 2^(L/2+20). The starting v is β itself, so the first quotient is
q ≈ √(T/k):

| | P248 | P324 | P376 | P500 | P664 |
|---|---:|---:|---:|---:|---:|
| log₂ of the first exact quotient | 251.8 | 326.9 | 379.5 | 503.9 | 667.8 |
| Exact quotients: steps to the fixed point | 10 | 9 | 8 | 8 | 9 |
| Clamped steps out of T_prov | 301 / 301 | 387 / 387 | 446 / 446 | 587 / 587 | 773 / 773 |
| Swaps under the clamp | 0 | 0 | 0 | 0 | 0 |

The whole trace is in `results/counterexample_trace_<set>.csv`. Every
clamped step has the same m: the h-saturation path,
m = ⌊2¹²⁴/ns⌉ ≈ 2^60.2. Each step:

- leaves A unchanged, since there is no swap;
- shifts the exact quotient by an integer, so the rounding residual ε and
  the certificate's potential Φ = A(1 + 2|ε|²) are **identical before and
  after** (1551.0012 bits at P500, every step);
- lowers |q| by about 2^60.2, an additive amount.

The `T_prov` proof charges every step a factor ρ of Φ, and its endgame
assumes that a non-swapping step is the last one before the fixed point.
A clamped step breaks both: it does not swap and it is not near the end.
Clearing a quotient of 2^503.9 would take more than 2⁴⁴¹ such steps.

**No count can repair it.** For valid ideals the first quotient alone
can reach about 2^(bits(T)/2). The ideal T = a² + b² + p with β = a + bi + j
(`crafted_k1`, k = 1) gives 2^324 / 2^418 / 2^485 / 2^639 / 2^844 bits of
quotient (as `e` in `variants`). So clamp64 needs at least about
2^(bits(T)/2 − 62) steps in the worst case, and clamp128 at least
2^(bits(T)/2 − 126): 2⁵¹⁴ at P500. No progress lemma with a per-step
factor exists for a clamp, because its progress is additive. A
"clamp counts as a partial step" potential would have to give a clamped
step a constant fraction of a step's ρ, while Φ does not change at all.
Exact cleanup steps do not help: one more exact step takes the large
quotient (which is why `crafted_k1` survives the prototype's single
cleanup), but the further exact steps the Q5 ideal then needs are not in
the schedule.

**The prototype also departs from the contract below the clamp.** Q64
reads 64 bits of A, so |q̃ − q| ≤ (1 + |q̃|)·2⁻⁶³, which is up to 1/4 for
|q| near 2⁶¹. Its m can then differ from ⌊q + 1/2⌋ by one, so the step is
not the contract's step, and the certificate, which assumes
ε ∈ [−1/2, 1/2)², does not formally cover it. On honest inputs this needs
|q| ≳ 2⁴⁰ or a near-tie within 2⁻⁶², and the cleanup step usually hides
it. It is still a gap, and the exp64 correction closes it.

## 3. The quotient rules compared

`variants.py`. All rules use the contract's swap rule.

| Rule | Quotient | Status |
|---|---|---|
| `exact` | ⌊(2h + A)/(2A)⌋ at full width | the contract; certified counts of table 1 |
| `clamp64` | the prototype's `quotient()`, ported exactly (h saturated at ±2¹²⁴, quotient clamped at 2⁶²), plus one exact cleanup | no progress proof; fails crafted inputs |
| `clamp128` | the same with the top 128 bits of A, h saturated at ±2²⁵², quotient clamped at 2¹²⁶, plus one cleanup | no progress proof; fails crafted inputs |
| `corr64` | clamp64 in the window plus the exact ±1 fix; flag (sticky overflow) if any e_j > 60 | exact when unflagged; aborts on big quotients |
| `exp64` | the window rule with the ±1 fix, plus the undershooting floating quotient for e > 60 | certified counts of table 2 |

**exp64 in full.** Let kA = max(0, bl(A) − 64), ns = A >> kA (64 bits),
and e = max_j bl(|h_j|) − bl(A).

- **Window (e ≤ 60):**
  - m̃_j = ⌊(2(h_j >> kA) + ns)/(2ns)⌋, which is the prototype's division.
    Its numerator fits in 126 bits and never saturates here.
  - Then X = 2h_j + A − 2m̃_j·A. Set m_j = m̃_j − [X < 0] + [X ≥ 2A].
- **Big (e > 60):**
  - s = e − 60, and qtop_j = ⌊(|h_j| >> (kA + s)) / ns⌋. This is a
    124-bit numerator over 64 bits, the same division.
  - m_j = sign(h_j)·(qtop_j − 1)·2^s if qtop_j ≥ 2, and 0 otherwise.
  - The update is V ← V − 2^s·(m0·U + m1·iU).
- **Every step computes both paths and selects by a mask.**

## 4. The progress proof for exp64

Notation: q = h/A ∈ ℚ(i), δ = 2⁻⁶³, T0 = 60.

- **Lemma W (window = contract).** If e ≤ T0, then |q_j| < 2^(T0+1) and
  |q̃_j − q_j| ≤ (1 + |q̃_j|)δ < 1/2 (the Q5 report's Lemma 3 bound). So
  ⌊q̃_j + 1/2⌋ is within 1 of ⌊q_j + 1/2⌋. The contract's m* is the unique
  integer with 0 ≤ 2h + A − 2m*A < 2A, so the ±1 fix returns m* exactly.
- **Lemma U (big sub-step).** Let e > T0 and s = e − T0 ≥ 1.
  - For the larger component, 2^(s−1) < |q_j − m_j| < 2^(s+1)(1 + 2⁻⁶³).
  - For each component, m_j has the sign of q_j and |m_j| < |q_j| (or
    m_j = 0).
  - *Proof.* Write x = |h_j|/2^(kA+s) and y = A/2^kA ≥ 2^63 (or y = A exactly
    when kA = 0). Then |q_j|/2^s = x/y < 2^(T0+1), and
    (x − 1)/y − 1 < qtop ≤ x/(y − 1).
  - *Consequences:*
    - (i) N4(V′) = A·|q − m|² + Δ/A > A, since the larger residual exceeds 1
      and Δ > 0, so **the sub-step never swaps**, under the strict rule
      too;
    - (ii) |q − m| < |q| componentwise, so N4(V′) < C and **v never
      lengthens**;
    - (iii) the new e′ ≤ s + 2 = e − (T0 − 2), so **e drops by at least
      58**.
- **Lemma B1 (total quotient size).** In the exact run, consider every
  step t whose predecessor swapped (all steps until the fixed point).
  For each such step, |q_t|² ≤ C_{t−1}/A_{t−1}, by Bessel, Q5 Lemma 1(b).
  C_{t−1} = A_{t−2}, and A ≤ C always. Telescoping gives
  Σ_t log₂|q_t| ≤ ½·log₂(C₀/A_end) ≤ ½·log₂(C₀/λ₁) =: Λ over the steps with
  |q_t| ≥ 1.
- **Lemma B2 (big sub-steps a step).** If a logical step has e_t > T0, it
  takes n_t ≤ ⌈(e_t − 60)/58⌉ big sub-steps (by U(iii)). Its quotient has
  log₂|q_t| > e_t − 1 ≥ 58·n_t + 2. Since 58n + 2 is superadditive, the most
  big sub-steps a whole run can need is **E = max{n : 58n + 2 < Λ}**.
- **Theorem.** For any input of a class with exact count T and Λ bound as
  in table 2, T + E exp64 steps, starting from the strict initial swap,
  end on exactly the basis that T exact steps give, which is reduced.
  Every intermediate norm is at most M₀, with no growth at all.
  - *Proof.* By U(i), big sub-steps never swap, so U is unchanged across
    them.
  - Rounding commutes with integer translation:
    ⌊(q − m_big) + 1/2⌋ + m_big = ⌊q + 1/2⌋.
  - So the window step that follows a run of big sub-steps completes
    exactly the contract's logical step (Lemma W).
  - The exp64 run is therefore the exact run with at most E sub-steps
    inserted (B2).
  - After the fixed point, every exp64 step is an exact step with m = 0.
  - Non-growth follows from U(ii) and the exactness of window steps. □
- **Λ per class.** These are the rows of table 2:
  - generic HNF: C₀ ≤ 2T² + p, λ₁ ≥ T;
  - pre-reduced with guard F: C₀ ≤ (1 + q)²·F√p·N, λ₁ ≥ Nq;
  - any integral basis: λ₁ ≥ 1.
  - E is computed in integers as the largest n with
    4^(58n+2) < C₀/λ₁ (`progress_check.py`, `results/step_counts.csv`).
- **Machine checks** (`progress_check.py` Part L, `results/progress_check.txt`):
  - The exp rule at toy divisor precisions w = 8, 10, 12 (T0 = w − 4).
    Every A below 2^(w+3), and 4,000 larger ones, are taken with
    log-uniform h of both signs and every half-integer tie.
  - The rule at w = 64 on 300,000 inputs. These include A of the forms
    all-ones, 2⁶³ exactly and ns = 2⁶⁴ − 1, h of the forms 2^k − 1 and 2^k,
    and full-size ties.
  - Checked on every case:
    - window outputs equal ⌊(2h + A)/(2A)⌋;
    - big outputs undershoot, keep the larger residual above 2^(s−1) ≥ 1,
      never lengthen v, and cut e by at least T0 − 2.
  - Totals: 781,000 window cases, of which 333,000 needed the ±1 fix,
    and 600,000 big cases. The smallest e drop was 3 / 5 / 7 / 59 against
    the bound 2 / 4 / 6 / 58.
- **Tightness.**
  - The `crafted_k1` ideals need 5 / 7 / 8 / 10 / 14 big sub-steps,
    against E = 5 / 7 / 8 / 11 / 14, in both model and O0 coordinates.
  - So E is attained at four sets and within one at P500.
  - The additive T + E is not attained: inputs with a large quotient have
    few logical steps. The largest exp64 need observed is the Rolletschek
    chain's 275 / 355 / 410 / 541 / 713, the same as exact. A sharper joint
    bound would need a certificate that credits big contractions; it would
    save at most E steps (2%).
- **Width (Q5).** M_t ≤ M₀ throughout, so Q5's Theorem 1 holds with ε = 0.
  The one new intermediate is the correction numerator,
  |X| ≤ 2|h| + A + 2|m̃|A ≤ 7M₀. That is under 3 bits over M₀, against the
  cleanup numerator's 3M* (under 2 bits) that bounds Q5 today. So it is
  one bit more than Q5's largest intermediate at most, against 361 / 431 /
  485 / 571 / 775 spare bits. The update before its shift is
  |m·U_r| ≤ √C/2^s, inside the coordinate width. The exact cleanup, and
  its full-width division, disappear.

## 5. Adversarial and honest testing

`adversarial.py --scale 4`, `analyze.py` (`results/adversarial_summary.txt`,
`results/adversarial_table.csv`). Each set has 202 inputs × 5 rules, 1,010
rules-runs in all:

- 120 honest signing ideals, model and O0, half of them at the norm bound;
- 3 Q5-crafted, plus 3 `k = 1` crafted (model and O0);
- 2 Rolletschek chains;
- 7 big-quotient chains: all-big μ of 61 to 200 bits; Rolletschek bodies
  of 25%, 50% and 75% of the size followed by one huge quotient; and
  Rolletschek pairs interleaved with 2⁶² + 1 quotients;
- tail-stress and ping-pong;
- 16 challenge ideals, 12 key-generation ideals of norm D, and 16 keygen
  I_sec;
- 18 pre-reduced signing inputs: sampled keys, guarded and not, plus the
  crafted skew keys.

| Rule (counts as in the tables; clamp rules with T_prov + cleanup) | Unreduced outputs, all sets | Families that fail |
|---|---:|---|
| `exact` | 0 / 1,010 | none |
| `exp64` | 0 / 1,010 | none (also never grows a norm) |
| `clamp64` | 45 / 1,010 | crafted_q5 and all big-quotient chains, at every set |
| `clamp128` | 39 / 1,010 | crafted_q5, all-big chains, Rolletschek plus a huge quotient |
| `corr64` | 75 / 1,010 (all flagged) | every input with a quotient above 2⁶⁰ |

No rule failed an honest input. **Honest quotient tail** (exact runs of
400 signing ideals per set, `results/quotient_tail.csv`):

- P(|q| ≥ X) ≈ 3.4–3.6/X², the Gauss–Kuzmin shape.
- With 166–430 logical steps per run, that gives **P(some |q| ≥ 2⁶⁰) ≈
  2^−110.8 / 2^−110.4 / 2^−110.3 / 2^−109.9 / 2^−109.5** per reduction.

This is an extrapolation of a heuristic tail over 54 bits, not a bound. It
is what the clamp's (or `corr64`'s) honest failure rate rests on.

## 6. Rust cross-check and cost

`rust-xcheck/` is a standalone crate outside the workspace and CI.
`ct.rs`, `zi.rs`, `sets.rs` and `exact.rs` are copied unchanged from
`rust-lagrange-ct`; the new code is `zi_exp.rs`, with a secret-amount
barrel `shl`, and `xq`. It uses the same `unsafe` inline-asm barrier as the
prototype. It is not the production reducer. Rust 1.85.1, release, LTO.

**Bit-exact check** (`results/rust_check.txt`). On 75 vectors per set
(60 honest, crafted Q5 and k = 1, Rolletschek, and the big-quotient
chains):

- Rust exp64 equals the Python model in u, nrd(u) and the number of big
  sub-steps: 375 / 375.
- It equals the exact contract's output: 375 / 375.
- **The prototype itself reproduces the counterexample.** Its own
  `reduce` (T_prov plus the cleanup) equals the Python clamp64 model
  375 / 375, but misses λ₁ on every crafted Q5 ideal and on 3 to 6 chains
  per set (6, 9, 8, 9 and 9 of 75).

**Cycles** (`results/rust_bench.csv`). Pinned to one core of the shared
box; median of 300–1,500 reductions of honest signing ideals. The
prototype runs T_prov steps plus the exact cleanup; exp64 runs table 2's
signing count with no cleanup.

| | P248 | P324 | P376 | P500 | P664 |
|---|---:|---:|---:|---:|---:|
| Prototype (Mcycles) | 2.16 | 3.56 | 4.66 | 10.53 | 20.68 |
| exp64 (Mcycles) | 2.82 | 4.58 | 5.89 | 12.33 | 24.93 |
| **Ratio** | **1.31×** | **1.28×** | **1.26×** | **1.17×** | **1.21×** |
| Exact quotient every step (estimate) | ~27× | ~40× | ~47× | ~46× | ~44× |

The last row is the prototype's fixed-iteration full-width `exact_round`
(the cleanup step: 191k / 368k / 492k / 824k / 1,188k cycles, from
`rust-lagrange-ct/results/parts.txt`) times T_prov. A faster constant-time
division would lower it, but it stays an N·64-bit division every step.

The exp64 overhead per step is about 15–29%. It comes from:
- two extra secret bit lengths;
- a second barrel shift right;
- the correction's two N-limb scalar products;
- the barrel shift left of four coordinate vectors.

Part of it is paid back by dropping the cleanup step. The code is not
tuned; sharing one shifted copy of U between both components would cut
it.

The static scan (`results/rust_asm_scan.txt`, `asm_scan.py` adapted)
finds no division instruction in `reduce_exp`. Its conditional-branch and
`cmov` profile is the same kind as the prototype's: mostly compares
against loop immediates. No dudect or ctgrind run was made here, and no
constant-time claim is made.

## 7. Fallbacks and constant-time implications

| Option | Steps (P500 signing) | Cost vs prototype | Failure | Constant-time needs |
|---|---:|---:|---|---|
| **exp64** (recommended) | 598 (609 any basis) | 1.17× (measured) | none, proven | secret bit length (of h and A), secret right and left shifts, fixed 128/64 division, N-limb scalar multiply |
| Exact quotient every step | 587 | ~46× (estimate) | none, proven | fixed full-width division (exists: `exact_round`) |
| corr64: clamp as sticky overflow, explicit failure | 587 | ~1.1× (estimate: exp64 without the big path) | ≈2⁻¹¹⁰ per honest reduction (heuristic); deterministic on crafted keys | as exp64 minus the left shift; an abort path |
| Wide fixed quotient (128-bit clamp) | — | — | fails crafted inputs | — |
| Wide quotient covering 2^(bits(T)/2) | 587 | ≈ exact | none | a ~640-bit quotient times coordinates every step |

On `corr64`:
- an explicit failure is allowed by 2.1.3 only as "unreachable by proof";
- O33 forbids a retry;
- here the abort is reachable, by a crafted key, so it would at best be a
  key-import rejection rule plus a heuristic signing rate.

**prism-int.** `SecretInt` has no bit length and shifts only by a public
count (SIGNER_SPEC 2.1 op table: "Bit length … 2c (if the reduction
scales by a length)" and "Shift right by a public count"). exp64 needs:
- bit length of a secret;
- shifts right and left by a secret amount below 64·LIMBS (a barrel
  shifter, log₂(64·LIMBS) masked stages; the prototype's `shr` and the new
  `shl`);
- a fixed-iteration 128-by-64 division (`div_u128_64q`);
- a secret scalar times an N-limb product.

The prototype's 64-bit quotient already needed the first two, for
`bit_length(nu)` and `shr`. These are additions to 2.1's operation table,
not new classes of risk.

## 8. Proposed spec wording (not applied to `dev`)

For the stage-2c section of `SIGNER_SPEC.md`, once Rick rules:

> **Reduction contract (2c).** The basis is reduced by fixed-schedule
> Z[i]-Lagrange reduction: an initial swap when N4(v) < N4(u) (strict);
> then a public number of steps, each rounding both quotient components
> half-up, m_j = ⌊(2h_j + N4(u)) / (2·N4(u))⌋, subtracting m0·u + m1·iu
> from v, and swapping only when the new N4(v) < N4(u) (strict). The step
> count is a public constant per parameter set and input class (table: …),
> certified by research/lagrange-tour-bound and re-derived in integers by
> research/clamped-quotient-progress. It includes the final size reduction.
> No verification step and no cleanup step follow.
>
> **Bounded precision.** The quotient may be computed from the top 64 bits
> of N4(u) only as exp64 (research/clamped-quotient-progress, section 3):
> - an exact ±1 correction makes every in-window step the contract's
>   step;
> - a quotient beyond 2⁶⁰ is applied as an undershooting m·2^s.
>
> The step count is then the exact count plus E (58E + 2 < ½·log₂(C₀/λ₁),
> table: …). Any other approximate or clamped quotient needs its own
> progress proof (Section 0.2 item 3).
>
> **Decode.** A stored pre-reduced basis is used only if
> N4(u_s) ≤ N4(v_s) ≤ F·√p·N4-scaled N, F the keygen guard factor;
> otherwise it is rejected.

## 9. Decisions for Rick

1. **The quotient.** Choose one of:
   - exp64 with table 2's counts and no cleanup (recommended: proven,
     1.17× at P500);
   - the exact quotient at full width (proven, about 46×);
   - corr64 with an explicit failure (cheapest, not proven, about 2⁻¹¹⁰
     heuristic, crafted keys trigger it).
2. **Which E.** The valid-ideal E (P500 598) or the any-integral-basis E
   (P500 609, robust to a basis that is not the ideal it claims to be).
   Recommended: any-basis, for 1–2% more steps.
3. **The guard factor** for pre-reduced keys. Factor 2 rejects about 22%
   of keys (Q5 6.2); factor 256 rejects none of 10,000 and costs 4 steps
   (573 vs 569 at P500, 583 vs 579 with exp64). The decoder must check
   the stored basis against the same factor.
4. **prism-int additions** (section 7): a secret bit length, secret-amount
   shifts, a 128/64 division.
5. **The key-generation count.** Use the norm-D count (441 at P500) rather
   than the signing count that Q5 assumed (587). It is smaller, and it is
   certified.

## 10. Caveats

- **The model and the signer.** Inputs are model-order ideals, with O0
  versions for honest and `k = 1` inputs. No ideal from our own key
  generation exists yet.
- **The certificate.** `certify.py` is trusted as re-run, not
  re-audited box by box. The integer recomputation covers only its final
  formula.
- **The tail estimate** behind corr64's rate is a heuristic extrapolation.
- **No constant-time measurement** (dudect or ctgrind) of `reduce_exp`, and
  no aarch64 build.
- **Algorithm scope.** Lehmer batching or any other reorganisation needs
  its own proof.

## 11. Reproduction

See `README.md`. Everything runs in about 5 minutes on the shared box.
