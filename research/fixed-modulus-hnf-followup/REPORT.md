# Fixed-modulus HNF, follow-up for native Rust (Rick's t49u)

Branch `experiment/fixed-modulus-hnf-followup`, off `research` at `6f6e1c7`. Builds on `../fixed-modulus-hnf/` (merged at `3610b8c`, model unchanged). **Research notes, not adopted spec; no crate, `dev` or PR is touched.**

## 0. Answers

1. **A zero candidate pivot cannot be told from rank deficiency inside the modular computation, and it is not an error.** The pivot at coordinate `c` is `d_c = gcd(m, entries at c)`. "Zero candidate" means every working entry at `c` is `≡ 0 mod m`, which gives `d_c = m`. That is normal for a valid lattice (a generic ideal has `d = (2N, 2N, 1, 1)`, so `d₀ = d₁ = m`). The output depends only on `span(G) + m·Z⁴`, and reducing mod `m` erases a generator `m·eᵢ` to zero, so a valid `G` and a rank-deficient `G'` can produce identical working state. **The exact criterion is on `G` itself, before any reduction: `Δ(G) = |det|` of the four rows.** Rank-deficient ⇔ `Δ(G) = 0` ⇔ `is_zero(Δ)`. Combined: **`Δ(G) ≠ 0 ∧ Δ(G) = ∏ d_c` ⇔ rank 4 ∧ `m·Z⁴ ⊆ span G`**, for any generators and with no ideal theory. For the decode shape, `Δ = |det_{Z[i]}(u,v)|²`, one Gaussian cross product that `GuardedIdeal::admit` already computes.
   **The pivot product `∏ d_c = 4N²` alone is not a certificate** (§2). This corrects a sentence of the merged report (§0 C1 row and §3.3; fixed in place, with an erratum note).
2. **The columns `2N·eᵢ` may be added (the output may be read as `HNF(span G)`) iff `m·Z⁴ ⊆ span G`.** Two independent routes establish that: (A) before the run, `Δ(G) = 4N²`, O0-parity, and left-`O0` closure, all three together (Lemma 6); or (B) after the run, `Δ(G) = ∏ d_c ≠ 0`. Nothing weaker suffices. A closure test alone is vacuous at `Δ = 0` (it passes every vector). Otherwise the output is the HNF of the strictly larger lattice `span(G) + 2N·Z⁴`, with no signal in the output itself. Six concrete cases with the actual matrices are in §3.
3. **Largest temporary before modular reduction, decode class: `2K` bits** (`K = B+1`): **318 / 392 / 450 / 572 / 736 bits = 5 / 7 / 8 / 9 / 12 limbs** at P248/P324/P376/P500/P664 (`2K−1` on active steps, `2K` on the discarded dummy path). Entries on entry: raw columns `K` bits, guarded pre-reduced pair `147 / 184 / 212 / 274 / 356` bits. Worst-case entries (`|x| < 2^(2K)`) do not raise the pre-reduction temporary (still `2K`); they raise the Barrett product *inside* the reduction from `3K` to `4K−1` bits (8/10/11/14/18 → 10/13/15/18/23 limbs). The schedule and entry bounds are attained or within one bit of it by measurement (§3).

## 1. Item 1: zero candidate pivots and rank

**Prop 1 (what a zero candidate is).** At coordinate `c`, with working rows `S` (entries in `[0,m)`) and the permanent generator `m·e_c`, the pivot after the fold is `d_c = gcd(m, S₁[c], …, S_k[c])`. Hence `d_c = m ⇔` every `S_i[c] ≡ 0 mod m`. The algorithm needs no case for it: the permanent generator supplies the pivot (merged report §2).

**Prop 2 (the state erases the difference).** The output depends only on `L̄ = span(G) + m·Z⁴` (merged Theorem A). Step 1 reduces every entry mod `m`, so a generator that is a multiple of `m·eᵢ` becomes the zero row. Moreover, if `d_c = m` in `HNF(L)` then `col_c ≡ m·e_c (mod span of the columns below c)`, so the columns without `col_c` have rank 3 and the same `L̄`. A generic ideal has `d₀ = d₁ = m` and `(2N,0,0,0), (0,2N,0,0)` literally are `m·e₀, m·e₁`.

*Consequence.* No rule on the modular working state (a zero candidate, a pivot equal to `m`, a zero row) detects rank deficiency. The check has to use `G` as supplied.

**Prop 3 (exact criterion).** Let `G` have four rows and `Δ(G) = |det G|` (zero iff rank < 4). Then `∏ d_c = [Z⁴ : L̄]`, `[Z⁴ : span G] = Δ(G)` when `Δ ≠ 0`, and `L̄ ⊇ span G`. So `∏ d_c ≤ Δ(G)` when `Δ ≠ 0`, with equality iff `span G ⊇ m·Z⁴`. If `Δ = 0` then `∏ d_c ≥ 1 > Δ`. Hence

&nbsp;&nbsp;**`rank 4 ∧ m·Z⁴ ⊆ span G` ⇔ `Δ(G) = ∏ d_c`** (the right side already forces `Δ ≠ 0`, since `∏ d_c ≥ 1`).

Checked against Sage on 5,142 random and structured generator sets (rank-deficient, scaled, zero-row, valid): equal to the exact truth in every case (`results/rank_enlarge.txt`, part C: 312 accepted, 2,588 rank-deficient rejected, 2,242 enlarged-but-full-rank rejected).

**Native contract (all public control flow, no new primitive).**

| Predicate | Computation | Notes |
|---|---|---|
| `rank_def = is_zero(Δ)` | `d = cross(u,v)` over Z[i]; `Δ = d₀² + d₁²` | `Δ([u,Ju,v,Jv]) = |det_{Z[i]}(u,v)|²` (2,000 random pairs checked against Sage's 4×4 determinant, `results/malformed.txt` D1). Independent of `p`. |
| `index_ok = ct_eq(Δ, 4N²)` | `4N²` from the carried norm | already in `admit` as `abs_squared(det).ct_eq(index)` |
| `c1 = ct_eq(Δ, ∏ d_c)` | after the run, three multiplications of the pivots | exact certificate of rank 4 and containment for any `G` |
| `range_ok` | `1 ≤ N < 2^B`, `|x| < 2^(2K)` | the merged report's C2 |

Widths: `Δ < 2^(4K+2)` for entries `< 2^K` (raw HNF columns; guarded entries are smaller) and `∏ d_c ≤ m⁴ < 2^(4K)` because `d_c | m`. So both fit **638 / 786 / 902 / 1,146 / 1,474 bits = 10 / 13 / 15 / 18 / 24 limbs**, at most dev's coordinate width `W = 11 / 14 / 16 / 21 / 27`.
Only the *classification* needs `is_zero(Δ)`: with one generic error, `c1` alone decides. `∏ d_c ≥ 1`, so `c1` already rejects `Δ = 0`.

## 2. Item 2: when the columns may be added

In the constant-time algorithm the columns are implicit (the permanent generators), so "adding them" is always done. The question is when the result may be *read* as `HNF(span G)`.

**Lemma 6 (route A, before the run).** Let `G` be the rows `u, Ju, v, Jv` (or any generators), `L = span G`, `N ≥ 1`, and suppose
 - **P1** every row is in `O0`'s numerator lattice (`a ≡ d`, `b ≡ c` mod 2);
 - **P2** `Δ(G) = 4N²` (so rank 4 and index `4N²`);
 - **P3** `i·L ⊆ L` and `ω·L ⊆ L` for `ω = (i+j)/2` (for Gaussian rows `i` is automatic).

Then `m·Z⁴ ⊆ L` for `m = 2N`.
*Proof.* `O0` is generated as a ring by `i` and `ω` (`(1+k)/2 = 1 + iω`), so P3 makes `L` closed under left multiplication by `O0`; with P1 and rank 4 it is a left `O0`-ideal. Its index in `O0`'s numerator lattice is `Δ/4 = N²`, so its reduced norm is `N`. Lemma 1 of the merged report gives `2N·Z⁴ ⊆ L`. ∎

**Order.** P2 first, logically: P3's Cramer test divides by `det(u,v)` using the *claimed* `4N²`, so it means something only given P2. In constant-time code all three are computed and ANDed into one `SecretBit`. Three rules for the caller: (R1) never branch on or declassify the HNF output before the verdict is final; (R2) never accept P3 (or the stored-HNF comparison) alone; (R3) never accept `∏ d_c = 4N²` alone. Route B (`Δ = ∏ d_c`) is independent of the ideal machinery and costs three multiplications, so decode should use both routes (defence in depth). Once route A passes and the stored columns have been admitted by `admit`, uniqueness of the HNF already makes the computed columns equal the stored ones. Running the algorithm there is an independent check; it is needed where only generators exist (a producer, or an intersection).

**P3 is vacuous at `Δ = 0`** (`results/malformed.txt` D2). The Cramer test accepts `w` iff both components of `numerator·conj(det)` are divisible by `4N²`. At `det = 0` that product is 0, so every vector passes, including `(1,0,0,0)` which is not in `I`. Valid toy pair: `(1,0,0,0)` rejected, `(5,0,0,1)` accepted. Dependent pair `(V, 3V)` and zero `v`: both accept `(1,0,0,0)`.

### 2.1 Concrete cases (toy `p = 3`, `N = 7`, `m = 14`, root `(x,y) = (0,9)`; `0 + 81 + 3 = 84 = 7·12`)

Ideal `I`: HNF columns `(14,0,0,0)`, `(0,14,0,0)`, `(0,9,1,0)`, `(5,0,0,1)`, index `196 = 4N²`. Output pivots and `∏ d` from the model, rank from Sage.

| # | Generators `G` | rank | `Δ(G)` | output | `∏ d` | what the output is | caught by |
|---|---|---:|---:|---|---:|---|---|
| E0 | the four columns | 4 | 196 | `= HNF(I)` | 196 | correct | (valid) |
| E1 | `u, Ju, v, Jv` with `u = (0,9,1,0)`, `v = 3u` | 2 | **0** | **`= HNF(I)`** | 196 | `I` conjured from a rank-2 pair | `Δ = 0`; P3 passes vacuously; `∏ d = 4N²` passes |
| E2 | the columns minus `(14,0,0,0)` and `(0,14,0,0)` | 2 | n/a (k=2) | **`= HNF(I)`** | 196 | same | only a rank test on `G` |
| E3 | the columns minus `(14,0,0,0)` | 3 | n/a | **`= HNF(I)`** | 196 | same | only a rank test on `G` |
| E4 | `U = (14,0,0,0)`, `V' = V + 2N·e₂ = (0,9,15,0)`, rows `U,JU,V',JV'` | **4** | 44,100 = 225·196 | **`= HNF(I)`** | 196 | a full-rank parity-valid pair that spans a sublattice of index 225 in `I`, silently enlarged to `I` | `Δ ≠ 4N²`; `Δ ≠ ∏ d`. `∏ d = 4N²` passes. |
| E5 | no generators | 0 | 0 | `diag(14,14,14,14)` | 38,416 | `mZ⁴` | `∏ d ≠ 4N²` |
| E6 | only the root column `(0,9,1,0)` | 1 | n/a | pivots `(14,14,1,14)` | 2,744 | wrong, larger index | `∏ d ≠ 4N²` |

E4 at all five sets (`N` prime near `2^B`, `V' = V + 2N·e₂`): output equals the stored HNF, `∏ d = 4N²`, parity holds, `Δ/(4N²)` is a ~`2^(2B)`-size integer, `Δ ≠ ∏ d` (`results/malformed.txt` D3). A decode that compared the computed HNF with the stored one, or checked `∏ d_c = 4N²`, would accept it.

Further cases (`results/rank_enlarge.txt`):

- **E7, full rank, correct index, parity-valid, not left-closed** (`N = 3`, `m = 6`): `G = ⟨(2,0,0,0), (0,2,0,0), (0,1,1,0), 9·(1,0,0,1)⟩` has index `36 = 4N²` (cyclic quotient). Adding `6·eᵢ` gives pivots `(2,2,1,3)`, index 12, a strictly larger lattice (ratio 3), and `6·Z⁴ ⊄ span G`. `Δ = 36` and P1 pass. P3 and `Δ = ∏ d` catch it.
- **E8, correct HNF, not an ideal**: `(14,14,1,1)` with root column `(1,2,1,0)` (`a ≢ d`) is full rank, contains `14·Z⁴`, and the algorithm returns it unchanged (`∏ d = 196`). The HNF is right; P1 rejects it. This is the case where the HNF algorithm is fine and admission is what fails.
- **Subsets of the HNF columns at the five sets** (generic prime norm, `O0`, `3·O0`, even `N = 2`): for the generic ideal and `O0`, 4 of the 15 nonempty column subsets reproduce the HNF exactly, 3 of them rank-deficient; `N = 2` has 2 (1 rank-deficient); `3·O0` (`d = (6,6,3,3)`, no pivot equals `m = 18`) only the full set. The `d_c = m` ones are exactly Prop 2's columns (asserted).
- **Random column subsets of valid small lattices**: 107 of 983 are rank-deficient yet reproduce the full HNF exactly.

## 3. Item 3: the largest temporary before modular reduction

Notation: `K = B+1`, `m < 2^K`, all working entries in `[0,m)` (merged Lemma 5). "Before reduction" is the input `y` of `Red`; the Barrett product inside `Red` is listed because it is the widest value overall.

**Derivations.**
- *Schedule, active step* (both pivots nonzero): `t = bl(X_c) − bl(Y_c) ≤ K−1`; `Y_j ≤ m−1`; so `Y_j << t < 2^(K−1)·m`; `X_j − (Y_j << t)` has magnitude `< 2^(K−1)·m < 2^(2K−1)`. **`2K−1` bits.**
- *Schedule, dummy path* (smaller pivot already 0; computed, then discarded by the mask): `t = bl(X_c) ≤ K`, `Y_j << t < 2^K·m < 2^(2K)`. **`2K` bits.** Forcing `t = 0` here (one extra select) would make it `2K−1`, but the limb counts do not change at any set, so it is not worth it.
- *Canonical step*: `q = ⌊P_c[l]/d_l⌋ ≤ (m−1)/d_l`, `P_l[l'] ≤ m−1`, so `q·P_l[l'] ≤ (m−1)²/d_l < 2^(2K)` and `|P_c[l'] − q·P_l[l']| < 2^(2K)`. **`2K` bits** (reached when `d_l` is small: a generic ideal has `d₂ = d₃ = 1`).
- *Entry reduction input*: raw HNF columns `≤ 2N−1 < 2^K`; guarded pre-reduced pair `≤ ⌊√(1024·⌈√p⌉·(2^B−1))⌋`, since `N4(v) ≤ 1024√p·N` (SIGNER_SPEC 2.3) and every coordinate is at most `√N4`; the `J` images have the same magnitude; worst case is the range gate `|x| < 2^(2K)`.
- *Barrett product `a·R`*: `R ≤ 2^(2K)/m`, so `a·R < 2^(2K)·2^(2K)/m`. With `a < 2^K·m` (schedule, canonical, and any decode entry) it is `< 2^(3K)`: **`3K` bits**. With entries up to `2^(2K)` and a *small* `m` it is `< 2^(4K−1)`: **`4K−1` bits**, reached only as `m → 2`. The width must nevertheless be provisioned at `4K−1`, because `K` is public and `m` is secret.

**Per set** (bits; limbs of 64, sign held separately):

| Quantity | P248 (K=159) | P324 (K=196) | P376 (K=225) | P500 (K=286) | P664 (K=368) |
|---|---|---|---|---|---|
| decode entries: raw HNF columns | 159 b / 3 | 196 / 4 | 225 / 4 | 286 / 5 | 368 / 6 |
| decode entries: guarded pre-reduced pair | 147 / 3 | 184 / 3 | 212 / 4 | 274 / 5 | 356 / 6 |
| **largest temporary before reduction, decode class** (`2K`) | **318 / 5** | **392 / 7** | **450 / 8** | **572 / 9** | **736 / 12** |
| the same on active steps only (`2K−1`) | 317 / 5 | 391 / 7 | 449 / 8 | 571 / 9 | 735 / 12 |
| worst-case entries before reduction (`2K`) | 318 / 5 | 392 / 7 | 450 / 8 | 572 / 9 | 736 / 12 |
| Barrett product, decode and schedule (`3K`) | 477 / 8 | 588 / 10 | 675 / 11 | 858 / 14 | 1,104 / 18 |
| Barrett product, worst-case entries (`4K−1`) | 635 / 10 | 783 / 13 | 899 / 15 | 1,143 / 18 | 1,471 / 23 |
| dev `W` / `2W` / `LIMBS` | 11/22/26 | 14/28/33 | 16/32/38 | 21/42/49 | 27/54/65 |

**Decode versus worst case.** For the decode class the largest temporary before reduction is `2K` bits and the largest value anywhere is the `3K`-bit Barrett product. Worst-case entries leave the first unchanged (the schedule already reaches `2K`) and widen only the Barrett product by `K−1` bits. Every bound is attained or within one bit at all five sets (`results/temporaries.txt`): the active step reaches `2K−1`, the dummy path `2K`, the canonical step `2K` at P248 and P500 and `2K−1` at the other three sampled sets, the raw and worst-case entry classes exactly, the guarded pair within one bit (146 against 147 at P248, 212 against 213 at P376), and the `4K−1` product when `m` is tiny (`N = 1`).

Other classes: at `m = 2Nq` (signing, `K = B+2e−4`) the pre-reduction temporary `2K` is 21/27/31/41/53 limbs, which fits `LIMBS` at every set; the Barrett product (31/40/46/61/80 limbs) does not (merged report §7).

## 4. Tests

| File | What | Result |
|---|---|---|
| `exp_rank_enlarge.py` | parts A (E0–E8 above), B (column subsets at five sets), C (criterion `Δ = ∏ d` against Sage on 5,142 cases; 983 subset cases) | pass (`results/rank_enlarge.txt`) |
| `exp_malformed.py` | D1 `Δ = |det_{Z[i]}|²`; D2 vacuous Cramer test; D3 E4 at five sets | pass (`results/malformed.txt`) |
| `exp_temporaries.py` | wraps the merged model's `red` and `pair_step` and records the input of every reduction by phase: raw, guarded (a shear at the factor-256 cap), worst-case entries, tiny `N`; five sets | all bounds hold; attained or within one bit (`results/temporaries.txt`, `.csv`) |

## 5. Not claimed

- Compiled-code constant time. The criterion is a set of integer predicates that the Rust can evaluate with existing `prism-int` operations (`mul`, `add`, `ct_eq`, `is_zero`).
- A `Δ` for `k ≠ 4` generator sets (it would be the gcd of the `4×4` minors).
- That route A is complete: it is sufficient for C1, and `Δ = ∏ d_c` is the exact (necessary and sufficient) test.

## 6. Reproduction

```
export PATH=/workspace/mamba/envs/sage/bin:$PATH
cd research/fixed-modulus-hnf-followup
python exp_rank_enlarge.py ; python exp_malformed.py ; python exp_temporaries.py
```

`common.py` puts `../fixed-modulus-hnf` on the path (the merged model, unchanged; its REPORT.md has the erratum edit).
