# Normalized class Gram for the exp64 reducer (SHE-22)

Branch `experiment/normalized-gram`, off `dev` at `904d876`. It builds on Rick's reduction-alternatives work at [`a60fd06`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/a60fd06) (branch `experiment/gaussian-reduction-alternatives`). The signer contract and schedules are those of PR #45 (`abed5c1`; head `84a798a` has the same table): exp64, the conservative any-basis count, and the factor-256 guard. This is research code only. It is not the production reducer, it makes no hardware constant-time claim, and it changes no schedule.

## Verdict

**Yes, on both parts of the deciding question.**

1. **The constructor can supply the verified integral form cheaply.**
   - For the generic HNF the normalized Gram is `(2T, (x²+y²+p)/(2T), x, y)`. That costs two squarings and one exact division.
   - The division is a Hensel division (`Q = n·T⁻¹ mod 2^(64W)`) followed by a full-product check `Q·T = n`. The check is the integrality test.
   - At P500 the cold setup takes **7.5k TSC ticks**, including the Newton inverse of T. That is half the 15.1k of the combined candidate's numerator setup. With a cached per-key `N⁻¹` it takes 4.1k.
2. **Narrower arithmetic pays off for a single signature.**
   - The normalized Gram fits in **W = C limbs**, half the numerator Gram's 2C, at all five sets.
   - Setup plus reduction runs **1.79 / 1.88 / 1.84 / 1.50 / 1.69× faster** than Rick's combined candidate.
   - It runs **3.3 to 4.3× faster** than the original exp64 under the same native schedule (609 steps at P500).
   - Setup is 0.2% of the total, so single-signature and amortized costs differ by under 0.3%.

The logical exact steps, the swaps and the final bases are identical to the exact oracle and to the combined candidate on every input. The physical big sub-steps can differ, because exp64's exponent comes from bit lengths and those depend on the scale. The progress proof covers the normalized run with the same schedule (section 4).

**Recommendation:** use the normalized Gram with the generic Hensel constructor in the next native prototype. Feed it the combined candidate's step (shared division, carried Gram, power-of-two shifts) at W = C Gram limbs.

## 1. What was built

| File | Contents |
|---|---|
| `ngram.py` | Python model. It covers the constructors, exp64 with the carried Gram in any integer scaling (physical and logical traces), the exact oracle, and an instrumented replay of the Rust operation order that records every intermediate's width. |
| `gen_ng.py` | Inputs and Python references, written to `rust/vectors_ng/*.txt` and `results/gen_*.json` (section 3). |
| `bounds_ng.py` | Public width bounds per class, the limb choice, and the sub-step budget in normalized units. Output in `results/bounds.txt` and `results/bounds.csv`. |
| `rust/` | Rick's `rust-xcheck` crate copied from `a60fd06`, with additions listed below the table. |
| `rust/src/bin/ng.rs` | `check`, `bench` and `memcheck`. |
| `bench_ng.py` | Benchmark runner. Output in `results/bench*.csv` and `results/bench_env.txt`. |
| `vendor/` | The Python helpers from `research/clamped-quotient-progress` (as of `59929d3`) and Rick's `check_native.py` (`a60fd06`), so the folder runs on its own. |

Additions to the copied crate:

- `src/ngram.rs`: the constructors and the normalized reducer.
- `src/orig_*.rs`: the `1db834d` originals of `zi_exp.rs` and `ct.rs`, kept as the original-exp64 baseline. They gain a general-input entry point.
- One change to `zi_exp.rs`: `step_exp_t` returns the step's decisions as well. The arithmetic is unchanged, and `step_exp` calls it.

The normalized reducer is Rick's step function instantiated with Gram entries of W limbs instead of N = 2C. Coordinates stay at C limbs. The only differences from the combined candidate are the constructor and the Gram width.

**Constructors** (`ngram.rs`):

- **`gram_hnf`**, for the generic HNF `U=(2T,0,0,0)`, `V=(x,y,1,0)`:
  - form `n = x² + y² + p` at 2C limbs;
  - check that 4 divides n and that T is odd;
  - Hensel-divide `n/4` by T and check `Q·T = n/4` over all 2C limbs;
  - set `C = 2Q`, `A = 2T`, `h = (x, y)`.
  - The determinant is p by algebra, so it is not rechecked.
- **`gram_general`**, for any O0 pair (the pre-reduced intersection and the chained bases):
  - compute the numerator Gram from coordinates;
  - Hensel-divide the diagonal by 4T and the projections by 2T, checking each product;
  - check `A·C − h₀² − h₁² = p`.
- **`gram_hnf_restoring`**: the HNF constructor with a fixed-iteration restoring division (64W quotient bits), kept for comparison.
- **`inv_pow2`**: computes T⁻¹ mod 2^(64W). It starts from `3T xor 2` (5 bits), takes 4 word-level Newton steps, then ⌈log₂ W⌉ full-width steps. All counts are public.

Each constructor returns a secret all-ones or zero mask. Nothing branches on it inside.

## 2. Cost

Box CPU: `Intel(R) Xeon(R) Processor`, a virtual machine on a shared box (constant TSC, no frequency control; load average about 2 of 8 cores). Rust 1.97.1, release, fat LTO, one codegen unit. Inputs: the 60 honest signing lifts per set at the native schedule (312 / 401 / 463 / 609 / 803). Method: 5 processes × 201 repetitions after 10 warm-ups, pinned to logical CPU 6, variant order rotated every repetition. Each cell is the median of the five process medians (`results/bench_raw.csv`). Compare cells within a set; this is not the i9-11950H of Rick's tables.

**Setup plus reduction (single signature, cold), million TSC ticks**

| Set | Steps | Original exp64 (`1db834d`) | Combined (`a60fd06`) | Normalized | Speedup vs combined | Speedup vs original |
|---|---:|---:|---:|---:|---:|---:|
| P248 | 312 | 2.508 | 1.344 | **0.752** | **1.79×** | 3.34× |
| P324 | 401 | 4.239 | 2.402 | **1.276** | **1.88×** | 3.32× |
| P376 | 463 | 5.538 | 3.068 | **1.665** | **1.84×** | 3.33× |
| P500 | 609 | 11.706 | 4.862 | **3.240** | **1.50×** | 3.61× |
| P664 | 803 | 24.104 | 9.496 | **5.620** | **1.69×** | 4.29× |

**Reduction only, million TSC ticks**

| Set | Original | Combined | Normalized | Speedup vs combined |
|---|---:|---:|---:|---:|
| P248 | 2.480 | 1.329 | 0.753 | 1.76× |
| P324 | 4.294 | 2.317 | 1.268 | 1.83× |
| P376 | 5.693 | 3.021 | 1.649 | 1.83× |
| P500 | 11.813 | 4.846 | 3.220 | 1.50× |
| P664 | 24.415 | 9.452 | 5.642 | 1.68× |

**Setup only, thousand TSC ticks**

| Set | Original (2 norms) | Combined (norms + projections) | Normalized cold | Normalized warm | Normalized, restoring division | General constructor on the same HNF | Share of normalized total (cold) |
|---|---:|---:|---:|---:|---:|---:|---:|
| P248 | 2.2 | 4.3 | 1.7 | 1.3 | 40.3 | 10.7 | 0.22% |
| P324 | 6.1 | 6.8 | 2.7 | 1.7 | 80.9 | 13.5 | 0.21% |
| P376 | 4.2 | 8.2 | 3.7 | 2.2 | 104.4 | 17.4 | 0.22% |
| P500 | 8.1 | 15.1 | 7.5 | 4.1 | 176.2 | 27.4 | 0.23% |
| P664 | 13.7 | 26.3 | 13.5 | 7.0 | 292.6 | 47.3 | 0.24% |

Setup variants:

- **Cold** includes the Newton inverse of T (`inv_pow2` alone: 0.7k / 1.1k / 4.5k / 3.7k / 9.0k).
- **Warm** forms `T⁻¹ = N⁻¹·q⁻¹` with one truncated product, from a per-key cached `N⁻¹` and the public challenge's `q⁻¹`. The code is data-independent, so its timing uses placeholder inverses. `ng check` tests the composition identity on every input.
- **Restoring division** costs 22 to 30 times the cold Hensel setup. At P500 that is 5% of the whole reduction, so Hensel is the choice.

**Single signature versus amortized.** Nothing in the reduction amortizes: T = N·q changes with every challenge. The only per-key work is `N⁻¹`. Over many signatures with one key the cost per signature is warm setup plus reduction: 0.755 / 1.270 / 1.651 / 3.224 / 5.649 M. That is 1.77 / 1.83 / 1.84 / 1.51 / 1.68× faster than the combined candidate, within 0.3% of the single-signature figures. The narrower arithmetic pays off for one signature just as much as for repeated signing.

**Guarded pre-reduced input.** The general constructor costs 7.8 / 10.6 / 14.5 / 26.1 / 46.9k against 4.3 / 7.1 / 8.2 / 15.1 / 26.3k for the combined setup. That is about 11k more ticks at P500, 0.3% of the reduction it saves 1.6M on. A specialized constructor would be cheaper: combine a stored or once-per-key normalized Gram of `(uₛ, vₛ)` with γ, then divide once by the public q. It was not built (section 6).

Why P500's speedup is the lowest of the five sets was not investigated. The Gram shrinks from 42 to 21 limbs there, the same 2:1 as elsewhere. The coordinate update, which stays at C limbs, is a fixed part of every step.

## 3. Correctness

`ng check` (`results/check.txt`) runs every vector file; each line gets the checks below. The Python references in `gen_ng.py` already assert the same equalities against the exact oracle.

- The HNF, restoring and general constructors each reproduce the Python normalized Gram, and each accepts the input.
- The normalized reducer reproduces the Python physical trace of its own scaling (an FNV hash over `m₀, m₁, s, big, swap` for every step), its big-step count, and the final pair.
- The combined candidate, run on the numerator Gram with the same step function, does the same against the Python numerator run.
- **Logical trace:** the big sub-steps are composed into exact steps. The combined candidate, the normalized reducer and the exact oracle must give the same sequence of Gaussian quotients and swaps, with trailing no-op steps trimmed.
- Rick's unmodified `zi_exp::reduce_exp` on the O0 lift ends on the same first vector with the same big-step count.
- The original exp64 (`1db834d`) ends on the same final pair.
- `T·T⁻¹ = 1`, and the warm composition holds.

| Input group (per set) | Count | Notes |
|---|---:|---|
| O0 ideals among the 375 research vectors | 66 | 60 honest, 3 crafted Q5, 3 crafted k=1, lifted exactly as `check_native.py` lifts them; generic signing schedule. |
| Non-ideals among the 375 | 9 | 2 Rolletschek and 7 big-quotient chains in model shape. **All refused by all three constructors** (45/45). |
| Key generation, norm D | 6 | D = nextprime(2^(2W)), public. |
| Secret ideal pre-reduction | 6 | N < 2^B, half at the bound. |
| Challenge alone | 6 | |
| Pre-reduced intersection, guard 256 | 6 | I_sec reduced exactly, guard checked, `U = q·vₛ`, `V = uₛ + γvₛ`; general constructor. |
| Adversarial chains on real signing ideals | 10 | Built by running reduction backwards from a reduced basis of a genuine norm-T ideal, so the normalized Gram stays integral, capped at the generic O0 bound (max normalized norm ≤ 2^(Tbits+1)). Rolletschek (1±i) in two patterns, all-big μ of 61 to 200 bits, Rolletschek bodies with one big quotient (25/50/75%), Rolletschek with 2^62+1 every third step, and quotients just above the window. |
| Toy half ties (small p, P248 widths) | 52 total | 40 runs contain an exact +1/2 component and 52 an exact −1/2 component, including the SIGNER_SPEC 2.3 acceptance case (p=3, T=5). |

Results:

- **All 552 accepted inputs** (500 across the sets plus 52 toy runs) pass every check above.
- **Logical steps.** Quotients, swaps and final bases are identical to the exact oracle in both scalings.
- **Physical sub-steps.** On 43 of the 500 set inputs (7 / 10 / 9 / 7 / 10) the big sub-steps split differently. These are the crafted Q5 and k=1 ideals and the big-quotient chains: 8 / 12 / 15 / 9 / 20 logical steps decompose differently. Window steps never differ, because they are exact.
- **Big sub-step totals.** These are equal at every set except P324, where the normalized run takes 66 against 69. The largest count on one input is 5 / 7 (6 normalized) / 8 / 10 / 14, against a budget of 11 / 14 / 16 / 22 / 29.
- **Rolletschek margin.** The conj_alt chain needs 277 / 356 / 412 / 542 / 715 exact steps, against counts of 301 / 387 / 447 / 587 / 774.
- **Checker sensitivity.** Corrupting a trace hash, a Gram entry or a logical hash in the toy file made the corresponding checks fail.

The 375 model vectors still pass Rick's own `xq check` in the copied crate, under the research schedule (`results/xq_check_research_vectors.txt`).

## 4. Widths, limbs and the schedule

**Starting bound M0** (normalized, the larger diagonal entry):

- Generic HNF: `A₀ = 2T`, and `C₀ = (x²+y²+p)/(2T) < 4T + p/(2T)`, so `M0 = 4·Tmax + ⌊p/2⌋ + 1`.
  - Tmax is 2^Tbits for signing, 2^B for the secret ideal and 2^(2e−5) for the challenge.
  - For key generation T is D itself.
- Guarded pre-reduced: `c_s = N4(vₛ)/(2N) ≤ 2F√p`, so `A₀ = q·c_s` and `C₀ ≤ (1+q)²·c_s/q`, giving `M0 = 2F⌈√p⌉(qmax+3)` with F = 256. The secret norm N cancels.

**Intermediates** (`bounds_ng.py` docstring, lemmas W1 to W4):

- W1: A and C stay ≤ M0, since neither exact steps nor exp64 big steps lengthen V. `|hⱼ| < √(AC) ≤ M0`, because the determinant is p > 0.
- W2: the quotient estimate on both paths satisfies `|mw·A| ≤ 2M0`. The correction numerator `X = 2h + A − 2mw·A` and `X − 2A` are below 9M0.
- W3: in the carried update, `|m2ˢA| ≤ 1.5M0`, `|r| ≤ M0`, `|h+r| ≤ 2M0` and `|m(h+r)2ˢ| ≤ 2.25M0`. The partial `C − c₀` lies in [C′, C].
- W4: coordinates are bounded by `4√(2·Tmax·M0)` in numerator units, as before normalization.

The Gram needs `bits(9·M0) + 1` signed bits. The replay recorded every category on every input, and all observed maxima are within the bounds (`results/bounds.txt`).

| Set | Signing M0 bits | Gram signed bits needed | **W (Gram limbs)** | Spare bits | Numerator Gram limbs (2C) | Coordinate limbs C | Largest observed Gram value (bits) |
|---|---:|---:|---:|---:|---:|---:|---:|
| P248 | 652 | 656 | **11** | 48 | 22 | 11 | 652 |
| P324 | 841 | 845 | **14** | 51 | 28 | 14 | 841 |
| P376 | 974 | 978 | **16** | 46 | 32 | 16 | 974 |
| P500 | 1,283 | 1,287 | **21** | 57 | 42 | 21 | 1,283 |
| P664 | 1,693 | 1,697 | **27** | 31 | 54 | 27 | 1,693 |

Generic signing has the largest bound of the five classes. The other classes need, in signed bits:

| Class | P248 | P324 | P376 | P500 | P664 |
|---|---:|---:|---:|---:|---:|
| Guarded pre-reduced | 630 | 819 | 952 | 1,261 | 1,671 |
| Key generation, norm D | 519 | 775 | 775 | 1,031 | 1,543 |
| Challenge alone | 498 | 650 | 754 | 1,002 | 1,330 |
| Secret ideal | 254 | 329 | 386 | 508 | 672 |

Starting from Rick's observed maxima: the observed generic signing values match the M0 bound to within a bit. That is Rick's 1,282 bits at P500, plus the one bit from `p/2` and the strict bound. The widths above are chosen from the bounds, not from the observations. W equals C at every set, which the prototype relies on for its multiplication types.

The **constructor scratch** is the numerator `x²+y²+p < 8T² + p` at 2C limbs, and the Hensel product `Q·T` at 2C limbs. The general constructor forms the numerator Gram at 2C limbs, the same width as the combined candidate.

**The schedule is unchanged.** The logical exact run is identical in both scalings: the rounding is of the ratios h/A, and the swaps compare A against C, both invariant under scaling. So the exact count S carries over. For E, Lemma B2 of `clamped-quotient-progress` uses `log₂|q| > e − 1` with e taken from bit lengths, which holds in any integer scaling. Lemma B1's `Λ = ½·log₂(C₀/A_end)` is scale-invariant. So the native E bounds the normalized run as well, and every input finished inside the native count. Throughout, every run used the native counts: 312 / 401 / 463 / 609 / 803 for signing, and the other classes' rows.

**A schedule option, not adopted (Rick's call).** The constructor *verifies* that the normalized Gram is integral, so λ₁ ≥ 1 holds in normalized units without the ideal-theoretic λ₁ ≥ T. In fact λ₁ ≥ 2, since normalized diagonal entries are even. With `C₀ ≤ M0` this gives `E ≤ max{n : 4^(58n+2) < M0/2}`. For each class the table shows the native count first and the count with normalized E second:

| Class | P248 | P324 | P376 | P500 | P664 |
|---|---|---|---|---|---|
| Generic signing | 312 / 306 | 401 / 394 | 463 / 455 | 609 / **598** | 803 / 788 |
| Guarded pre-reduced | 297 / 292 | 387 / 379 | 448 / 440 | 594 / 583 | 787 / 773 |
| Key generation, norm D | 231 / 227 | 360 / 353 | 344 / 337 | 459 / 450 | 712 / 699 |
| Secret ideal | 24 / 24 | 25 / 24 | 26 / 26 | 26 / 26 | 29 / 28 |
| Challenge alone | 220 / 216 | 287 / 281 | 331 / 325 | 442 / 433 | 587 / 576 |

This would save about 2% more on top of the measured speedup. It rests on the constructor check and the class's M0, not on the valid-ideal argument that the any-basis count avoids. It is recorded for review only.

## 5. What the constructor must establish

Before the reducer runs, the constructor or decoder must establish:

1. **Integrality.** Every numerator entry divides exactly: the diagonal by 4T, the projections by 2T. The prototype checks this with the full product `Q·T = n`, which a remainder makes fail. T must be odd for Hensel division; it is odd for the generic odd-norm HNF, the challenge degree and D. An even T would need the restoring division.
2. **Determinant p.** `A·C − h₀² − h₁² = p`. For the generic HNF this holds by algebra once (1) holds. For any other basis (pre-reduced, decoded or chained) it must be checked. Given membership of U and V in the ideal, it is equivalent to U and V spanning the whole ideal. Integrality alone does not give this.
3. **Membership and span.** U and V must lie in the norm-T ideal. This stays with Stage 2b/2d, as in the PR #45 input contract; the reducer cannot check it.
4. **The class's starting bound and the guard.** The public W and the schedule row assume the M0 of section 4. For a decoded pre-reduced key that means the factor-256 guard, already required at key generation and decode.

On failure the constructor returns a secret mask. Turning it into a refusal is a declassification exit, and it reveals only that the input violated an invariant: SIGNER_SPEC 2.3's "a remainder is an invariant failure". That exit needs review under section 0 and 2.1.3.

The integral class norm form that SIGNER_SPEC 2.3 hands to the consumer is `F = W/(4T)`. Its diagonal is half the normalized diagonal and its lower off-diagonal entries are the normalized projections. So the normalized Gram is already the consumer's form, up to a public factor of two. The reducer's carried Gram can be passed on without recomputation.

## 6. Constant-time status and open items

- **Memcheck** (`results/memcheck.txt`, Valgrind 3.24.0, release build with `--features memcheck`):
  - What ran: `rlc_secret` marked all limbs of T, x and y secret on one honest and one crafted Q5 lift per set. Each lift went through the cold normalized setup, the full reduction and the restoring setup. A pre-reduced input then went through the general constructor and the reduction, with T, U and V all marked secret.
  - Result: **zero errors at all five sets**. The P248 positive control (a branch on a secret bit of T) gives exit 77 with 2 conditional-jump reports.
  - Memcheck detects secret-dependent branches and addresses only. It says nothing about timing.
- **No hardware claims.** There are no dudect or timing-distribution tests, no AArch64 runs and no release assembly scan of the new code. `asm_scan.py` from the copied crate was not rerun. G7 stays open.
- **prism-int port.**
  - The discarded path's values stay within 9M0, so the existing sticky overflow mark can be checked on both paths without false positives at W limbs.
  - New operations: Hensel inverse and multiply, the full-product equality check, and a public small shift.
  - The secret bit length, secret shifts, 128-by-64 division and N-limb scalar multiplication are already on the PR #45 list.
  - Temporaries are not erased.
- **Specialized pre-reduced constructor.** Storing the normalized Gram of `(uₛ, vₛ)` in the key would leave each signature with one Gaussian combination and a division by the public q. That changes the key format and needs a design round. Computing it at key load is an alternative, but neither was built. The general constructor measured here costs 0.3% at P500.
- **Even T.** Hensel division needs T odd. Every admitted class has odd T; a future class with even T would use the restoring division (5% at P500).
- **Measurement scope.** These are reducer-level cycles on a shared virtual machine. End-to-end signing latency is still unmeasured.
- **Schedule option** (section 4): the normalized E saves 11 steps at P500. Not adopted.

## Reproduction

From `research/normalized-gram`, with Python 3 and gmpy2 (the box uses `/workspace/mamba/envs/sage/bin/python3`):

```
python3 -B gen_ng.py              # inputs, references, observed widths (seconds)
python3 -B bounds_ng.py           # width bounds, limbs, E
cd rust
cargo +1.97.1 build --release
target/release/ng check           # equivalence (ALL OK)
target/release/xq check           # Rick's a60fd06 check on the 375 model vectors
cd .. && python3 -B bench_ng.py --procs 5 --reps 201 --cpu 6
cd rust && cargo +1.97.1 build --release --features memcheck --target-dir target-memcheck
valgrind --error-exitcode=77 target-memcheck/release/ng memcheck p500     # each set
valgrind --error-exitcode=77 target-memcheck/release/ng memcheck p248 control   # expect 77
```
