# Fixed-schedule norm equation

Date: 2026-11-22. Research for Shekyl PRISM. Step-counting model in
Python; no constant-time or security claim. Not a proposal to change the
signature format.

PRISM is the scheme of ePrint 2026/443 (Andrea Basso, Giacomo Borin,
Wouter Castryck, Maria Corte-Real Santos, Riccardo Invernizzi, Antonin
Leroux, Luciano Maino, Frederik Vercauteren, and Benjamin Wesolowski).
`shekyl-prism` implements it.

Previous experiments (`research/fixed-schedule-reduction/`) closed the
~2 steps/bit leak of exact LLL on the secret product `I_sec · I_q` using
a fixed-schedule `Z[i]`-Lagrange reduction sized from the public bound
`B = ⌈L/2⌉ + 32` (`L = bits(p)`). The review identified the search/retry
loops of the norm equation and key generation as the larger unmeasured
channel. This report measures them in a faithful port of the pinned C
reference (`mariascrs/PRISM_v2` at `87fad7d`) and tests whether a
fixed-schedule norm-equation search carries the same advantage.

## Headlines

1. **What actually runs in signing (`SOURCE_AUDIT.md`).** `shekyl-prism`
   signs the Section 5 prime-norm variant (`q = 2^(2e−5) − s²`), so
   `quat_represent_integer` is **not** on its signing path. The
   secret-touching norm-equation search shared by signing and key
   generation is **`quat_qlapoti`** (`qlapoti.c`), alongside key
   generation's prime-norm search
   `quat_lideal_prime_norm_reduced_equivalent` (`lll_applications.c`).
2. **`quat_qlapoti` hides `bits(N)` by shortest-equivalent reduction,
   but still leaks the prepared small ideal `S` and has four uncapped
   inner loops.** Before searching, `quat_qlapoti` replaces `L = I_sec · I_q`
   with a shortest equivalent ideal `S` of norm `n ≤ √p`. As a result,
   `ρ(work, bits(N))` is indistinguishable from zero across all five sets
   (`|ρ| ≤ 0.032`, detectable `0.044–0.063`), and a `dudect` Welch t-test
   comparing a fixed key against random keys stays within `|t| ≤ 1.59`.
   Instead, the search depends on `S` itself: `v₂(n) > 0` increases
   `coprime-λ` skips ~4× and shifts weighted work by ~27% (`ρ(work, v₂(n))
   = −0.118` to `−0.142`, `p ≤ 1.3×10⁻⁷`), a fixed-small-ideal `dudect`
   test reaches `|t| = 4.7` (`p248`) and `6.4` (`p324`), and per-ideal
   pass success rates `p(S)` vary by **2.5× to 17.6×** across ideals.
   Inside a pass, generator draws hit 194, `λ`-skips hit 59,995, 2-D
   Gauss steps hit 92, and Cornacchia Euclid steps hit 126.
3. **Why `p376` (`f = 65`) is the bottleneck, and the O29 exponent link.**
   The model matches the compiled C reference (`PRISM_v2`) in both mean
   passes (`358 vs 353` at `p248`, `3,172 vs 3,417` at `p376`, `2,001 vs
   1,932` at `p500`) and positive-`M4` share (`0.957 vs 0.970`, `0.151 vs
   0.162`, `0.351 vs 0.365`). Because `n(S) ≤ √p = √(f · 2^e)` while the
   reference targets `2^(e−2)`, `M4 ~ 2·2^(e−2)/n − (A² + B²)` shrinks
   with `f`. At `p376` (`f = 65`), `M4 > 0` drops to 15–16%, driving a
   17.6× spread in `p(S)` across ideals. Raising the target exponent from
   `e−2` to `e` lifts `M4 > 0` at `p376` from `0.15` to `0.55` and cuts
   passes 4.1× (conditional on ledger O29: whether the dim-2 chain
   accepts `e−1` or `e`).
4. **A fixed-schedule `quat_qlapoti` works and eliminates schedule leaks,
   but is 39×–391× slower than the reference mean (23×–229× two-stage).**
   Unlike `Z[i]`-Lagrange reduction (which cost `0.28×` exact LLL), a
   fixed schedule for `quat_qlapoti` (`fsne.py`: fresh generators with
   `K = 13–14` branchless draws, analytical `T_lat` and `T_corn` bounds
   with frozen selects, fixed trial division, fixed `J = 16` Jacobi
   non-residue search, fixed-width complex powering, and branchless
   first-success selection) achieves **1 distinct operation trace** per
   set, `0/0` unfinished Gauss/Euclid steps, and `100%` valid solutions,
   but requires `P = 2,662–37,528` passes for 99.9% completeness and
   `P = 17,093–240,998` for `2⁻⁶⁴`. Key generation's prime-norm search
   is much tamer (`1,032–2,443` draws for 99.9%, `6,628–15,685` for `2⁻⁶⁴`)
   and runs only once per key.

## 1. Model scope and C-reference cross-check

The model (`fne.py`, `run_fne.py`) ports `quat_qlapoti`,
`quat_lideal_generator_small_coprime`, `quat_dim2_lattice_short_basis`,
`quat_dim2_lattice_qlapoti_cvp_condition`,
`quat_qlapoti_check_mod_condition`, `ibz_cornacchia_extended`,
`ibz_cornacchia_prime`, `ibz_sqrt_mod_p`, `quat_elem_is_odd_norm`,
`get_endtype`, and `quat_lideal_prime_norm_reduced_equivalent` on the
true maximal order `O₀ = Z⟨1, i, (i+j)/2, (1+k)/2⟩` across all five
Shekyl parameter sets (`p248`, `p324`, `p376`, `p500`, `p664`). Simulated
secret keys start from `SEC_DEGREE = next_prime(2^(2·WIDTH_BITS))` and
apply `prime_norm_equivalent` (box 64); challenges `q = 2^(2e−5) − s²`
are prime, and `I_q` is chosen via `α = x + y i + j`.

Honest differences from C:
- Setup (`HNF`, 4-D `LLL`) uses PARI (`mathnf`, `qflllgram`), and random
  tapes come from SHAKE-seeded Python `random.Random`; no byte-for-byte
  RNG replay is claimed.
- Primality uses `gmpy2.is_prime` (25 rounds) in place of GMP's 32-round
  test; weighted work (`work_units`) models GMP's BPSW+MR (`16.5 × bits`),
  modular exponentiations (`1.5 × bits`), trial divisions (`1`), Euclid /
  Tonelli steps (`2`), complex powerings (`320` per call in ref), and
  bigint ops (`1`).
- **Python big integers and `gmpy2` primitives are not constant-time,
  and step/work counts are not CPU cycles.**

To verify fidelity, `PRISM_v2` at `87fad7d` was compiled (`c_crosscheck/`)
with a minimal counting patch (`qlapoti_stats.patch`) and run for 400
iterations at `lvl1` (`p248`), `lvl3` (`p376`), and `lvl5` (`p500`). Every
call returned `found = 1`. Pass counts and `M4 > 0` fractions match
closely:

| Set | C mean / median passes | Model mean / median passes | C share `M4 > 0` | Model share `M4 > 0` |
|---|---|---|---|---|
| `p248` | 353 / 265 | 358 / 243 | 0.970 | 0.957 |
| `p376` | 3,417 / 1,848 | 3,172 / 1,657 | 0.162 | 0.151 |
| `p500` | 1,932 / 1,134 | 2,001 / 1,116 | 0.365 | 0.351 |

## 2. Measurement of the reference search loops

Panels per set: `keys` (`400 × 10 = 4,000` signatures for `p248–p376`,
`200 × 10 = 2,000` for `p500–p664`), `fixkey` (`1,000–2,000` fixed-key
vs `rnd` `1,000–2,000` random-key signatures), `fixS` (`4–6` fixed
prepared small ideals × `500–1,000` random tapes each), and `equiv`
(`30–50 × 4` equivalence-class representatives).

### 2.1 Signing work vs `bits(N)`, key, `v₂(n)`, and fixed small ideal `S`

| Set | sigs (keys) | ref mean / p99.9 passes | `ρ(work, bits N)` | `ρ(work, bits n)` | `ρ(work, v₂ n)` | key `η²` (`p`) | fixkey vs rnd `t` (detectable `d`) | fixS max `|t|` |
|---|---|---|---|---|---|---|---|---|
| `p248` | 4,000 (400) | 358 / 2,376 | −0.032 (`0.042`) | −0.022 (`0.17`) | −0.131 (`9e-17`) | −0.0096 (`0.89`) | −1.54 (`0.169σ`) | **4.7** |
| `p324` | 4,000 (400) | 441 / 3,237 | −0.011 (`0.47`) | −0.015 (`0.34`) | −0.125 (`2.5e-15`) | −0.0081 (`0.85`) | −0.67 (`0.169σ`) | **6.4** |
| `p376` | 4,000 (400) | 3,172 / 33,200 | −0.010 (`0.51`) | +0.053 (`7.3e-4`) | −0.142 (`1.5e-19`) | −0.0054 (`0.75`) | +0.97 (`0.169σ`) | 4.2 |
| `p500` | 2,000 (200) | 2,001 / 16,549 | −0.002 (`0.93`) | +0.023 (`0.30`) | −0.118 (`1.3e-7`) | −0.0003 (`0.50`) | −1.59 (`0.239σ`) | 3.6 |
| `p664` | 2,000 (200) | 1,745 / 14,509 | −0.019 (`0.40`) | +0.018 (`0.42`) | −0.131 (`4.2e-9`) | +0.0167 (`0.071`) | −1.27 (`0.239σ`) | 3.7 |

Why `bits(N)` does not leak directly, yet `S` does:
- `quat_lideal_shortest_equivalent` reduces `L = I_sec · I_q` first,
  discarding `bits(N)`. Moreover, changing the challenge `q` changes the
  class of `I_sec · I_q` per message, averaging over classes in `fixkey`.
- When `v₂(n) > 0`, `while gcd(n, lam) != 1` skips every even `λ` (raising
  mean `lam_steps` ~4×, e.g. `118 → 522` at `p248` and `1,152 → 4,533`
  at `p376`), while `ref_work` drops by ~27% (`p < 10⁻⁷`).
- Across distinct small ideals `S` (`pS` panel, 60–80 ideals × 20–30 runs),
  the fixed-order per-pass success probability `p(S)` varies substantially,
  especially when `f` is large (`T11`):

| Set | ideals (R) | min `p(S)` | p10 | median | p90 | max `p(S)` | max/min | `ρ(p(S), bits n)` | `ρ(p(S), M4>0)` |
|---|---|---|---|---|---|---|---|---|---|
| `p248` | 80 (30) | 0.00225 | 0.00266 | 0.00351 | 0.00446 | 0.00553 | 2.5× | −0.451 (`2.7e-5`) | +0.512 (`1.2e-6`) |
| `p324` | 80 (30) | 0.00164 | 0.00216 | 0.00276 | 0.00361 | 0.00523 | 3.2× | −0.069 (`0.54`) | +0.102 (`0.37`) |
| `p376` | 60 (20) | 0.00014 | 0.00025 | 0.00048 | 0.00151 | 0.00249 | **17.6×** | −0.745 (`9.2e-12`) | **+0.912** (`4e-24`) |
| `p500` | 60 (20) | 0.00017 | 0.00032 | 0.00064 | 0.00130 | 0.00191 | **11.3×** | −0.842 (`3.3e-17`) | **+0.902** (`8.9e-23`) |
| `p664` | 60 (20) | 0.00034 | 0.00044 | 0.00077 | 0.00134 | 0.00177 | **5.3×** | −0.749 (`6e-12`) | **+0.887** (`4.1e-21`) |

- Equivalence-class invariance (`T4`): starting from an equivalent
  representative `L′ = L·(−β̄)/n(L)` always reaches the same norm `n(S)`
  (100%), but reaches the exact same basis/ideal only 45–57% of the time
  and the exact same search trace 52–64% of the time (due to sign/order
  ties in LLL).

### 2.2 Uncapped inner loops and key generation

All inner loops in the reference are data-dependent and uncapped (`T5`):

| Set | gen draws / gen (mean / max) | `λ`-skips (mean / max) | 2-D Gauss max (bound `T_lat`) | Cornacchia Euclid max (bound `T_corn`) | Tonelli 2-adic `e` max |
|---|---|---|---|---|---|
| `p248` | 3.77 / 194 | 322 / 4,822 | 39 (54) | 55 (96) | 16 |
| `p324` | 3.72 / 112 | 391 / 10,181 | 48 (68) | 72 (124) | 16 |
| `p376` | 3.66 / 124 | 2,828 / 59,995 | 57 (80) | 78 (140) | 16 |
| `p500` | 3.83 / 150 | 1,846 / 29,840 | 72 (104) | 96 (186) | 15 |
| `p664` | 3.82 / 133 | 1,642 / 44,022 | 92 (136) | 126 (245) | 15 |

In key generation (`T3`), `quat_lideal_prime_norm_reduced_equivalent`
takes `150 / 197 / 233 / 282 / 354` mean draws (CV `0.92–1.08`, max
`1,238–2,240`), with no detectable `bits(N)` correlation (`|ρ| ≤ 0.134`,
detectable `0.140–0.197`).

## 3. Fixed-schedule norm-equation design (`fsne.py`)

To remove every data-dependent branch from the search schedule:
1. **Fresh generator per pass (`policy='fresh'`)** eliminates the
   `gcd(n, lam) != 1` loop, `keep_alpha` branch, and `resets` branch,
   making passes i.i.d. given `S`. Each pass draws `K` candidates
   (`K = 14` for `p248–p376`, `13` for `p500–p664`, minimizing cost per
   valid generator at ~96% per-pass generator validity) and selects the
   first valid one without branching.
2. **2-D Gauss reduction** runs `T_lat = ⌈0.3932 · nbits⌉ + 3` steps
   (`54 / 68 / 80 / 104 / 136`) with a frozen-when-done select (bound
   derived from the minimum `1 + √2` norm contraction per step).
3. **Trial division** runs fixed `V_i` conditional divisions for each of
   the 101 primes (`325` divisions/pass), followed by one probable-prime
   test padded to `m4bits`.
4. **Jacobi non-residue + Tonelli-like powering + Cornacchia Euclid**:
   selects the first of `J = 16` small candidates with `Jacobi(c, r) = −1`,
   runs `r^((r−1)/4)` (when `r ≡ 1 mod 4`), runs `T_corn = ⌈0.7202 · m4bits⌉ + 3`
   Euclid steps (`96 / 124 / 140 / 186 / 245`, Lamé bound) with freeze,
   followed by `iroot` and fixed-width complex powering (`223` mult-bits).
5. **First valid solution kept** via arithmetic select over all `P` passes.

### 3.1 Executor validation (`val_fixed.py`, `T9`)

| Set | `P, K` (99.9%) | full runs | full ok | full == early | distinct traces | norm eq holds | unfinished Gauss / Euclid | early `p` / pass (ok / total; `p_eff`) |
|---|---|---|---|---|---|---|---|---|
| `p248` | 2,662, 14 | 24 | 24/24 | 24/24 | **1** | 323/323 | 0 / 0 | 0.00299 (299/300; `0.00259`) |
| `p324` | 2,848, 14 | 20 | 20/20 | 20/20 | **1** | 320/320 | 0 / 0 | 0.00259 (300/300; `0.00242`) |
| `p376` | 37,528, 14 | 12 | 12/12 | 12/12 | **1** | 312/312 | 0 / 0 | 0.00036 (300/300; `0.00018`) |
| `p500` | 17,750, 13 | 12 | 12/12 | 12/12 | **1** | 312/312 | 0 / 0 | 0.00053 (300/300; `0.00039`) |
| `p664` | 20,441, 13 | 10 | 10/10 | 10/10 | **1** | 310/310 | 0 / 0 | 0.00059 (300/300; `0.00034`) |

Every full run across all five sets had the exact same operation-trace hash
(`distinct traces = 1`, `ρ(work, bits N) = 0`, `dudect t = 0` at the
operation-schedule level) and `0` unfinished Gauss or Euclid reductions.

### 3.2 Failure probability vs pass budget `P` (`T7`, `T12`)

For `p248` and `p324`, the pooled fixed-order pass counts follow a single
geometric law (`KS D = 0.014, p = 0.39` and `D = 0.018, p = 0.15`). For
`p376`, `p500`, and `p664`, pooled pass counts form a mixture over ideals
with different `p(S)` (`KS D = 0.054–0.098, p < 10⁻⁵`), so budgets are
computed from `p_eff = p_worst · (1 − (1 − gen_valid)^K)` using the lowest
95% lower bound across pooled, class, and covariate groups (`T7`).

Empirical survival `P(passes > P)` / worst-class bound `(1 − p_eff)^P`:

| Set | `P = 1,000` | `P = 2,000` | `P = 5,000` | `P = 10,000` | `P = 20,000` | `P = 40,000` | `P = 100,000` | `P = 400,000` |
|---|---|---|---|---|---|---|---|---|
| `p248` | `0.0365 / 7.5e-2` | `0.0013 / 5.6e-3` | `0 / 2.3e-6` | `0 / 5.4e-12` | `0 / 2.9e-23` | `0 / 8.2e-46` | `0 / 1.9e-113` | `0 / 0` |
| `p324` | `0.0635 / 8.8e-2` | `0.0037 / 7.8e-3` | `0 / 5.4e-6` | `0 / 2.9e-11` | `0 / 8.5e-22` | `0 / 7.2e-43` | `0 / 4.4e-106` | `0 / 0` |
| `p376` | `0.5885 / 8.3e-1` | `0.3897 / 6.9e-1` | `0.1530 / 4.0e-1` | `0.0430 / 1.6e-1` | `0.0055 / 2.5e-2` | `0.0003 / 6.3e-4` | `0 / 1.0e-8` | `0 / 1.1e-32` |
| `p500` | `0.4815 / 6.8e-1` | `0.2705 / 4.6e-1` | `0.0560 / 1.4e-1` | `0.0075 / 2.0e-2` | `0.0005 / 4.2e-4` | `0 / 1.7e-7` | `0 / 1.3e-17` | `0 / 2.5e-68` |
| `p664` | `0.4595 / 7.1e-1` | `0.2405 / 5.1e-1` | `0.0435 / 1.8e-1` | `0.0070 / 3.4e-2` | `0 / 1.2e-3` | `0 / 1.3e-6` | `0 / 2.1e-15` | `0 / 2.0e-59` |

Derived budgets `P` and cost overhead vs the reference's mean work (`T7`, `T8`):

| Set | `P` (99.9%) | `P` (`2⁻²⁰`) | `P` (`2⁻⁴⁰`) | `P` (`2⁻⁶⁴`) | ref mean work | fixed 99.9% (`× ref`) | fixed `2⁻⁶⁴` (`× ref`) | two-stage 99.9% (`× ref`) | two-stage `2⁻⁶⁴` (`× ref`) | ref 400k `log₂ fail` |
|---|---|---|---|---|---|---|---|---|---|---|
| `p248` | 2,662 | 5,342 | 10,683 | 17,093 | 313,621 | `1.55e7` (**49×**) | `9.94e7` (**317×**) | **29×** (`Q=1,124`) | **186×** (`Q=7,370`) | −1,570 |
| `p324` | 2,848 | 5,715 | 11,429 | 18,287 | 509,650 | `1.97e7` (**39×**) | `1.27e8` (**249×**) | **23×** (`Q=1,204`) | **145×** (`Q=7,867`) | −1,249 |
| `p376` | 37,528 | 75,312 | 150,624 | 240,998 | 734,188 | `2.87e8` (**391×**) | `1.85e9` (**2,513×**) | **229×** (`Q=16,099`) | **1,472×** (`Q=102,686`) | **−143** |
| `p500` | 17,750 | 35,621 | 71,241 | 113,986 | `1.26e6` | `1.67e8` (**132×**) | `1.07e9` (**847×**) | **77×** (`Q=7,682`) | **492×** (`Q=48,810`) | −239 |
| `p664` | 20,441 | 41,021 | 82,042 | 131,267 | `2.12e6` | `2.41e8` (**114×**) | `1.55e9` (**730×**) | **66×** (`Q=8,828`) | **424×** (`Q=56,175`) | −297 |

Note: at `p376` (`f = 65`), if an ideal sits near the worst observed
single-ideal rate (`p(S) = 0.00014` in `T11`, slightly below `p_eff = 0.00018`),
the `2⁻⁶⁴` budget grows to `~317,000` passes and the 400,000-pass failure
tail tightens to `~2⁻⁸¹`.

### 3.3 Key generation's prime-norm search (`T3`)

| Set | mean draws | median / p90 / p99 / max | 99.9% budget | `2⁻⁶⁴` budget |
|---|---|---|---|---|
| `p248` | 149.9 | 105 / 320 / 621 / 1,238 | 1,032 | 6,628 |
| `p324` | 197.5 | 146 / 462 / 845 / 1,321 | 1,361 | 8,739 |
| `p376` | 232.8 | 148 / 515 / 1,101 / 1,381 | 1,605 | 10,305 |
| `p500` | 282.4 | 221 / 628 / 1,291 / 1,503 | 1,948 | 12,507 |
| `p664` | 354.1 | 233 / 788 / 1,971 / 2,240 | 2,443 | 15,685 |

A fixed-schedule prime-norm search in keygen is modest (`~7×` mean for
99.9%, `~44×` mean for `2⁻⁶⁴`, vs `129⁴ ≈ 2.77×10⁸` in the reference),
though it still requires a constant-time probable-prime test.

## 4. What-if for ledger O29: target power of two (`diag_exponent.py`, `T10`)

| Set | inputs | `e−2`: ref passes, fresh `p/pass`, `M4>0` | `e−1` | `e` |
|---|---|---|---|---|
| `p248` | 300 | 353, 0.00372, 0.96 | 384, 0.00389, 1.00 | 347, 0.00377, 1.00 |
| `p324` | 300 | 402, 0.00305, 1.00 | 437, 0.00287, 1.00 | 456, 0.00281, 1.00 |
| `p376` | 150 | 3,828, 0.00041, 0.15 | 1,843, 0.00067, 0.30 | 923, 0.00124, 0.55 |
| `p500` | 150 | 2,052, 0.00071, 0.35 | 910, 0.00082, 0.64 | 722, 0.00158, 0.89 |
| `p664` | 100 | 1,595, 0.00064, 0.50 | 1,158, 0.00109, 0.80 | 927, 0.00138, 0.98 |

This addresses the norm-equation side of ledger O29 only: whether the
dimension-2 isogeny chain can take `2^(e−1)` or `2^e` remains open for
stage 4.

## 5. Conclusions and limits

1. **Fixed-schedule reduction did not generalize cheaply to `quat_qlapoti`.**
   In `Z[i]`-Lagrange reduction, a fixed schedule was 3× *cheaper* than
   exact LLL because it replaced 4-D LLL by a 2-D Euclidean loop. In
   `quat_qlapoti`, the search is already a geometric lottery whose hit
   rate is `0.02%–0.3%` per pass and whose cost per pass is dominated by
   a full-width probable-prime test. Padding the loop to worst-case
   completeness costs **39×–391×** mean work at 99.9% (`23×–229×`
   two-stage) and **249×–2,513×** at `2⁻⁶⁴`.
2. **The leak profile is subtle.** `bits(N)` does not leak directly
   because `quat_lideal_shortest_equivalent` normalizes `I_sec · I_q`
   to `n ≤ √p`, and each signature uses a fresh challenge `q`. What
   does leak in the reference is the prepared small ideal `S` (especially
   `v₂(n) > 0` via the uncapped `gcd(n, lam)` loop, and `S`'s pass
   success rate via `dudect` on `fixS` up to `|t| = 6.4`), plus four
   uncapped inner loops.
3. **Practical path for `shekyl-prism`:**
   - Bound all four inner loops (`K = 14`, `T_lat`, `T_corn`, and cap or
     replace `gcd(n, lam)`), which costs little and prevents pathological
     runaways without paying the 250×–2,500× fixed-schedule padding tax.
   - Because `quat_qlapoti` is so expensive to run in constant time by
     brute-force padding, **secret-ideal re-randomization** (masking which
     equivalent ideal `S` is reached, if its own cost/leak is clean) or
     **SQIsign 3.0's deterministic / higher-hit-rate norm-equation
     formulation** (plus resolving O29 on `e` vs `e−2`, which cuts `p376`
     by 4×) are stronger candidates than fixed-schedule padding of the
     reference `quat_qlapoti`.
4. **Limits:** Python + `gmpy2` step/work counts are not CPU cycles and
   are not constant-time; keys are simulated; `T_lat` and `T_corn`
   passed `0/0` unfinished steps empirically and by contraction/Lamé
   bounds, but were not formally interval-certified like `Z[i]`-Lagrange.

Full tables: [`results/TABLES.md`](results/TABLES.md). Raw summary JSON:
[`results/summary.json`](results/summary.json).
