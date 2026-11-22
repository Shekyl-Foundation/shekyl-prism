# Rust port of the fixed-schedule Z[i]-Lagrange reduction: report

Date: 2026-11-22. Branch `experiment/rust-lagrange-ct`, folder
`research/rust-lagrange-ct/`. PRISM is the scheme of ePrint 2026/443
(Basso, Borin, Castryck, Corte-Real Santos, Invernizzi, Leroux, Maino,
Vercauteren, Wesolowski); shekyl-prism implements it. Research code: a
standalone crate outside the workspace and CI. Nothing here changes the
signer, the signature format or any parameter set.

## Summary

- **Correct.** On 1,508 vectors per set (the Python model's 1,000
  cross-check rows with λ₁ from exhaustive enumeration, 501 pool rows,
  and the lagrange-tour-bound adversaries: two Rolletschek chains, four
  corner cases, the half-integer ping-pong input), the constant-time Rust
  reduction returns λ₁ on every row of every set, and matches the Python
  model's 64-bit-quotient output bit for bit. `results/check.txt`.
- **Schedule.** Primary: the certified counts T_prov = 301 / 387 / 446 /
  587 / 773 (from `research/lagrange-tour-bound`), followed by one exact
  cleanup step, always executed. The empirical counts 189 / 239 / 283 /
  359 / 465 are kept as a secondary row: in Rust too they **fail** the
  Rolletschek chains (276 / 714 exact steps at P248 / P664), while every
  random PRISM vector passes. T_prov plus the cleanup passes all of them.
- **Constant time, by three checks.** (1) Valgrind memcheck with T, x, y
  marked secret (the ctgrind method) reports no secret-dependent branch or
  address in the reduction on any set, and does flag both positive
  controls. (2) The compiled reduction has no integer `div`/`idiv`, no
  `bsr`/`lzcnt`, and only fixed-size `memcpy`/`memset` calls. (3) A
  dudect fixed-versus-random-input Welch t-test, 220,000 primary-schedule
  measurements plus 390,000 on the empirical schedule: max |t| 1.25–3.61,
  under 4.5 everywhere; the variable-time controls on the same harness
  give |t| = 47–154 (exact Z[i]) and 89 (exact LLL).
- **One real fault found and fixed.** The first value barrier
  (`core::hint::black_box` on the choice bit) let LLVM learn that each
  mask was 0 or all-ones; it compiled the barrel shifter's array select
  (shift amount from bit length of nrd(u), secret) into a conditional
  jump. `asm_scan.py` and memcheck both found it. An empty inline-asm
  barrier on the mask itself removes it. The old form is kept as the
  `weak-barrier` feature, a negative control that memcheck flags.
- **Cost.** Primary schedule: **0.19–0.23× exact LLL on GMP** (0.08–0.11×
  on num-bigint) in cycles, against the Python model's 0.47–0.48×
  multiply-and-divide prediction for T_prov. The empirical schedule was
  0.11–0.13× (0.05–0.06×) against a 0.28–0.30× prediction. So the step
  count model over-predicts the cost by about 2× against GMP: Z[i]'s
  integers are about 40% the width of LLL's, and the fixed-width limb
  products are cheaper than GMP calls at these sizes. The price of
  constant time is against the variable-time exact Z[i]-Lagrange on GMP:
  the constant-time code is 4.8–7.8× slower (full T_prov steps instead of
  ~160–430 early-exit steps, full widths, fixed shifts and divisions).

## 1. What was built

`src/ct.rs`: fixed-limb two's-complement integers `[u64; L]`. Every loop
bound is a const generic or a public count; selections are masks; the
comparison `nrd(v) < nrd(u)` is a borrow chain; bit length is a
branch-free scan; the shift by k = max(0, bitlen(nrd u) − 64) is a barrel
shifter whose stages all run; the 128/64 quotient is a 64-iteration
restoring division. Carries are `u128` sums. There is no
`overflowing_add` and no hand-written clamp, so the two 1.97.1
miscompile shapes on the O32 watch list that ordinary code can have
(#163779, the clamp family) are absent. Stack frames are under 8 KiB,
well below the 32 KiB that #165204 needs.

`src/zi.rs`: `reduce::<C, N, PL>` is fsr.py's `zi_gauss_fixed` with
`prec=64`. It runs T fixed steps; each step applies the Gaussian-rounded
coefficient (possibly 0) and a branchless conditional swap. Then the
**exact cleanup step** (`exact_cleanup`) always runs. It uses
full-width, fixed-iteration restoring divisions (`ct::exact_round`), as
the tour-bound study asks, against the half-integer ping-pong of finite
precision. Widths come only from public data (`src/sets.rs`): C =
⌈(bits(q)+B+3)/64⌉ coordinate limbs (11 / 14 / 16 / 21 / 27) and N = 2C
norm limbs. The largest T in the vectors meets the bound exactly
(649 / 838 / 971 / 1280 / 1690 bits).

`src/exact.rs`, `src/exact_gmp.rs`: variable-time baselines on num-bigint
and on GMP (rug, in-place arithmetic). The first is Cohen 2.6.7 exact
LLL, δ = 99/100, the fsr.py baseline: its mean step count 1026.7 at P248
matches the model's 1025.0. The second is exact early-exit Z[i]-Lagrange.
They keep only (d, λ), not the transformation, which slightly favours LLL.

## 2. Cross-check against the Python model

`gen_vectors.py` uses fsr.py and hdl.py unchanged. Per set it writes
500 `nbound` rows (bits(N) = B) and 500 `nvar` rows (bits(N) varied up to
B) with λ₁, the model's output with exact quotients and with 64-bit
quotients, plus a 2,001-row dudect pool. `gen_adversarial.py` uses
`research/lagrange-tour-bound` generators unchanged.

| check (per set, 1,508 rows) | P248 | P324 | P376 | P500 | P664 |
|---|---|---|---|---|---|
| CT output = λ₁ (1,007 rows with λ₁) | all | all | all | all | all |
| CT = Python 64-bit model (adversarial rows: = λ₁) | all | all | all | all | all |
| CT = Rust exact Z[i] (num-bigint) | all | all | all | all | all |
| u in the ideal (nrd(u) = CT norm, T divides it; ideal rows) | all | all | all | all | all |
| Rolletschek chains, exact steps vs T_prov | 276 ≤ 301 | 356 ≤ 387 | 411 ≤ 446 | 542 ≤ 587 | 714 ≤ 773 |
| Same chains on the empirical T | **fail** | **fail** | **fail** | **fail** | **fail** |
| exact LLL b1 = λ₁ (for reference) | 1003/1007 | 1004/1007 | 999/1007 | 1004/1007 | 1001/1007 |
| GMP baselines agree with num-bigint | all | all | all | all | all |

The ping-pong input converges in this implementation even without the
cleanup step: the model's floor-shift rounding happens to land on 0. The
cleanup is kept anyway, since the study shows that every finite precision
has some 2-cycle input.

## 3. Constant-time checks

**Memcheck (ctgrind).** `ctgrind/run.sh` builds statically (the box's
ld.so has no debug symbols) and marks T, x, y undefined with Valgrind
client requests before `reduce`. Memcheck then reports any conditional
jump or memory address that depends on them.

| run | memcheck errors in the CT code |
|---|---|
| primary schedule, P248–P664, 2 inputs each | **0** on every set |
| positive control 1: one deliberate `if secret & 1` | flagged (rlc.rs) |
| positive control 2: `weak-barrier` build (the first barrier) | 6 contexts flagged, in the masks |

Each run also gives 7–8 errors inside static glibc's allocator (`free`,
`brk`, `realloc`) from start-up and input parsing. These are known
static-binary false positives, are outside this code, and are filtered
by stack. Memcheck does not see instruction latency, so check 2 covers
that.

**Assembly** (`results/asm_scan.txt`, release build, LTO). Per set
instantiation of `reduce` and of `exact_round`:

- 0 integer `div`/`idiv` and 0 `bsr`/`lzcnt`/`tzcnt`.
- `mul` only as 64×64→128 `mulq` (fixed latency on x86-64).
- Variable shifts are `shr/shl %cl` (fixed latency).
- Calls are only `memcpy`/`memset`/`memmove`, each with a constant size,
  plus one bounds-check panic path.
- 83–348 conditional branches: almost all `cmp $imm` loop counters. The
  register-compared ones were read in context (the `mul_trunc` inner
  bound, the step counter); memcheck confirms that none depends on the
  secret.

Compiler: rustc 1.85.1 (the box's; the repo pin 1.97.1 is not installed
here). The assembly verdict is for this compiler. A different rustc or
LLVM has to be re-checked, and `asm_scan.py` plus `ctgrind/run.sh` are
that re-check.

## 4. Cycles

rdtsc with lfence, the process pinned to one core, 2.4 GHz TSC, medians
over 2,000 / 1,500 / 1,000 / 600 / 400 inputs cycled from the pool. The
"steps" column is the schedule length; the primary schedule also runs
one cleanup step. Exact Z[i] gives its mean early-exit step count.

| Set | CT Z[i] (steps) | exact LLL, GMP | exact LLL, num-bigint | exact Z[i], GMP (mean steps) | CT / LLL-GMP | CT / LLL-num | model mul ratio | CT / exact Z[i]-GMP |
|---|---|---|---|---|---|---|---|---|
| P248 | 2.03 M (301) | 8.72 M | 17.82 M | 0.43 M (162) | 0.233 | 0.114 | 0.480 | 4.77 |
| P324 | 3.35 M (387) | 16.71 M | 35.18 M | 0.65 M (210) | 0.201 | 0.095 | 0.470 | 5.16 |
| P376 | 4.70 M (446) | 25.00 M | 53.78 M | 0.85 M (243) | 0.188 | 0.087 | 0.470 | 5.52 |
| P500 | 10.40 M (587) | 49.63 M | 115.27 M | 1.44 M (322) | 0.209 | 0.090 | 0.470 | 7.24 |
| P664 | 20.15 M (773) | 101.24 M | 243.15 M | 2.59 M (426) | 0.199 | 0.083 | 0.470 | 7.78 |

Secondary, the empirical schedule (no cleanup):

| Set | CT Z[i] (steps) | exact LLL, GMP | exact LLL, num-bigint | exact Z[i], GMP (mean steps) | CT / LLL-GMP | CT / LLL-num | model mul ratio | CT / exact Z[i]-GMP |
|---|---|---|---|---|---|---|---|---|
| P248 | 1.16 M (189) | 8.86 M | 18.07 M | 0.43 M (162) | 0.131 | 0.064 | 0.299 | 2.74 |
| P324 | 1.83 M (239) | 17.12 M | 35.87 M | 0.65 M (210) | 0.107 | 0.051 | 0.291 | 2.80 |
| P376 | 2.95 M (283) | 27.31 M | 58.39 M | 0.93 M (243) | 0.108 | 0.051 | 0.298 | 3.18 |
| P500 | 6.15 M (359) | 51.95 M | 120.51 M | 1.50 M (322) | 0.118 | 0.051 | 0.286 | 4.10 |
| P664 | 11.62 M (465) | 101.83 M | 243.93 M | 2.60 M (426) | 0.114 | 0.048 | 0.279 | 4.47 |

Where the time goes per step (`rlc parts`, medians). The sum matches the
measured totals:

| Set | cycles/step | Hermitian form | 2 quotients | nrd(v) | update | exact cleanup (once) | T_prov × step + cleanup |
|---|---|---|---|---|---|---|---|
| P248 | 6094 | 41% | 35% | 20% | 4% | 0.19 M | 2.03 M |
| P324 | 7710 | 46% | 28% | 22% | 5% | 0.37 M | 3.35 M |
| P376 | 9228 | 48% | 25% | 23% | 4% | 0.49 M | 4.61 M |
| P500 | 16560 | 54% | 17% | 24% | 4% | 0.82 M | 10.54 M |
| P664 | 24008 | 50% | 19% | 28% | 4% | 1.19 M | 19.75 M |

The Hermitian form and norm (schoolbook limb products) are 60–78% of the
cost. The two bounded-precision quotients are 17–35%. They repeat the
bit-length and shift of nrd(u), which could be shared, and the
restoring division could be a fixed-point reciprocal. The cleanup step's
full-width division is 6–9% of the total. None of this was optimised:
there is no `mulx/adx`, no dedicated squaring, and no Karatsuba.

## 5. dudect (fixed vs random input)

The fixed class is one `nbound` input (bits(N) = B). The random class is
2,000 fresh keys with bits(N) varied over the full range. Classes are
interleaved in a pseudorandom order, and both are copied to the same
stack buffer before the timed call. Statistics: Welch t on raw cycles,
on 100 percentile crops (the dudect method), and second order on centred
squares; plus the correlation of cycles with bits(N), the channel review
2 named. Threshold |t| = 4.5. `ct` is the primary schedule; `ct_emp` is
the empirical one, with a second P664 replicate (`_rep2`) using another
class sequence. `gauss` and `lll_gmp` are variable-time positive
controls on the same harness.

| Set | impl | measurements | t raw | max t cropped | t 2nd order | max abs t | r(cycles, bits N) random class | verdict |
|---|---|---|---|---|---|---|---|---|
| P248 | ct | 60,000 | -0.76 | -2.20 | -0.92 | 2.20 | -0.005 | no leak detected |
| P248 | ct_emp | 100,000 | -0.43 | 1.73 | -1.38 | 1.73 | 0.005 | no leak detected |
| P248 | gauss | 20,000 | -44.34 | -61.89 | -4.10 | 61.89 | 0.307 | leak |
| P248 | lll_gmp | 20,000 | 16.17 | 88.79 | 0.14 | 88.79 | 0.254 | leak |
| P324 | ct | 60,000 | -0.25 | -2.63 | -0.23 | 2.63 | -0.008 | no leak detected |
| P324 | ct_emp | 100,000 | 0.25 | 2.13 | 0.48 | 2.13 | 0.000 | no leak detected |
| P324 | gauss | 20,000 | 4.24 | 65.96 | 0.93 | 65.96 | 0.100 | leak |
| P376 | ct | 40,000 | -1.12 | -1.92 | -1.54 | 1.92 | -0.000 | no leak detected |
| P376 | ct_emp | 60,000 | 0.54 | 1.25 | -0.44 | 1.25 | 0.012 | no leak detected |
| P376 | gauss | 20,000 | -4.74 | -46.99 | 0.64 | 46.99 | 0.065 | leak |
| P500 | ct | 30,000 | 1.20 | 2.93 | 1.16 | 2.93 | -0.002 | no leak detected |
| P500 | ct_emp | 50,000 | 1.02 | 1.73 | 0.71 | 1.73 | 0.000 | no leak detected |
| P500 | gauss | 20,000 | 7.34 | 154.17 | -0.87 | 154.17 | 0.040 | leak |
| P664 | ct | 30,000 | 1.04 | 1.88 | 0.63 | 1.88 | -0.002 | no leak detected |
| P664 | ct_emp | 40,000 | -1.40 | -3.61 | 0.46 | 3.61 | 0.005 | no leak detected |
| P664 | ct_emp_rep2 | 40,000 | 1.29 | 2.21 | 0.35 | 2.21 | -0.002 | no leak detected |
| P664 | gauss | 20,000 | 2.87 | 50.46 | -1.92 | 50.46 | 0.142 | leak |

The primary schedule's largest |t| is 2.93 (P500). The closest result on
any schedule is 3.61 (empirical, P664, cropped), and its replicate gives
2.21 with the opposite sign, which is noise. Correlation with bits(N) is
|r| ≤ 0.012 for the constant-time code, against 0.04–0.31 for the
controls. Resolution: with the observed noise, a cropped (50th-percentile)
test at these sizes would flag a mean difference of about 250–9,000
cycles, that is 0.01–0.05% of the run time. The raw test would flag
0.25–2.2%.

## 6. Caveats

- **Box noise.** This is a shared 8-core VM, and four other experiments
  ran beside it (load average up to 8). The CV of a constant-time run is
  0.07–0.22, which is noise and not data dependence (memcheck and the
  fixed trace show that). Medians and the crop tests absorb most of it.
  Absolute cycle numbers are this VM's.
- **VM timing.** rdtsc is the invariant TSC (`constant_tsc`,
  `nonstop_tsc`) under a hypervisor at 2.4 GHz. It is not core clock
  cycles, and the hypervisor can add steal time. There is no PMU, so
  there is no instruction-retired counter.
- **Model.** The inputs are the index-4-order lattices with simulated
  keys, as in the Python model. Only the reduction is ported. The retry
  loops (key generation, norm equation) are still unmeasured.
- **Baselines.** Exact LLL is a straightforward Cohen 2.6.7 on GMP or
  num-bigint, not PARI's or fplll's floating-point LLL, which would be
  faster. The ratio is against exact LLL, as in the model.
- **Compiler.** The verdict holds for rustc 1.85.1 at opt-level 3 with
  fat LTO on x86-64. It has to be re-checked under the pinned 1.97.1 and
  any other target. aarch64 was not tested.
- **The barrier is not a proof.** An optimiser that sees through inline
  asm, or a future rustc change, could re-introduce branches. The
  regression guard is ctgrind plus the asm scan, which caught the first
  barrier.
- **T_prov is taken as given** from the lagrange-tour-bound
  certification (exact rounding). Here it is applied with 64-bit
  quotients, then one exact step. That combination is validated on every
  vector here, which is evidence and not a proof that the certificate
  covers bounded-precision steps.

## 7. Verdict

The fixed-schedule Z[i]-Lagrange reduction ports to constant-time Rust,
returns λ₁ on every test vector, including the Rolletschek worst cases,
once it runs the certified T_prov plus one exact cleanup step. It shows
no timing leak at |t| < 4.5 over 220,000 measurements, and no
secret-dependent branch or address under memcheck. In cycles it costs
about 0.2× a straightforward exact LLL on GMP, under half the 0.47×
the model predicts from operation counts. It is 5–8× a variable-time
exact Z[i]-Lagrange, which is the actual price of constant time. The
empirical step counts must not be used: they fail the Rolletschek inputs
in Rust exactly as the bound study predicted.

Next, if wanted: share the quotient normalisation and use a reciprocal
in place of restoring division (about 15–30% of a step); use mulx/adx
limb loops; re-run ctgrind and the asm scan under 1.97.1; port into a
stage crate behind the same three checks.
