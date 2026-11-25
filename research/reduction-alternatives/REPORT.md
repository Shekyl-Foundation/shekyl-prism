# Alternate directions for the Stage 2c reducer

Research base `1db834d`. The reference is the exp64 experiment in `research/clamped-quotient-progress`. The native O0 contract and schedules are those of PR #45 at `7b6dc21`. These experiments target the amount and shape of arithmetic in a fixed public schedule. The coordinate vectors remain available to the norm-equation consumer.

## Two changes to the Rust reducer

**Share one division per component.** The window and undershoot paths both need the leading part of the magnitude divided by the same word. Set `k=max(bl(A)−64,0)`, `n=A>>k`, and use exp64's common exponent `s=max(e−60,0)`. Compute `b=floor((|h|>>(k+s))/n)` once per real or imaginary component. On a large step use the original signed undershoot. In the window take the signed truncation as the initial estimate and apply the original exact residual correction. This eliminates two of the four fixed-iteration word divisions per physical step.

To see why the change preserves the rounding decision, let `q=|h|/A` and `y=floor(|h|/2ᵏ)/n` in the window. If `k=0`, then `y=q`. Otherwise `n≥2⁶³` and `q<2⁶¹`. Writing both discarded parts as fractions below one gives `|y−q|≤(q+1)/n<1/2`. Thus the distance from `floor(y)` to `q` is strictly less than 3/2. Restoring the sign gives an integer within one of the required half-up result, for either sign. The interval test on `2h+A−2mA` fixes the estimate, including ties. In the large-quotient case the unsigned division and undershoot are identical to exp64's. This argument preserves individual quotient components and the physical step count.

**Carry the Hermitian Gram matrix.** Start with `A=N4(U)`, `C=N4(V)` and two projections `hⱼ`. For each shear by the signed Gaussian components `mⱼ`, compute

    rⱼ = hⱼ − mⱼ A
    C′ = C − m₀(h₀+r₀) − m₁(h₁+r₁).

This is the expansion of the new vector norm. The update uses scalar products and shifts for exp64's mantissa and exponent. It eliminates the repeated full-width dot products and squares inside the schedule. Update the actual coordinates in the same step. If the vectors swap, exchange the diagonal entries and conjugate the off-diagonal entry, which negates the imaginary projection. The two subtracted contributions are nonnegative because each exp64 component either rounds to nearest or undershoots in magnitude. Their sum is bounded by the old projection energy and hence by `C`. Intermediate additions such as `hⱼ+rⱼ` still need headroom. A fixed-width signer needs a scratch-width audit including computations on the unused quotient path.

## Measured cost and release-code checks

All timings are full reductions in millions of x86 time-stamp-counter ticks. The first panel isolates the effect of each arithmetic change. The second panel reruns the comparison with the revised intra-limb shift. Each cell is the median of three process medians on the research schedule. The machine was an Intel Core i9-11950H, pinned to logical CPU 6, default powersave governor. Rust was 1.97.1, release, fat LTO, one codegen unit. Each process measured 41 honest-vector repetitions after 10 warmups in the existing xq harness. Variant order rotated. Load and clock rate were uncontrolled. Compare values within a panel. In particular the research P500 benchmark uses **598** steps and its original fixed-limb types. Native signing with the conservative schedule in PR #45 uses **609**. End-to-end signer latency remains unmeasured.

| Set | Original exp64 | Shared division | Carried Gram | Both |
|---|---:|---:|---:|---:|
| P248 | 4.316 | 3.190 | 3.388 | 2.589 |
| P324 | 4.532 | 3.667 | 3.484 | 2.652 |
| P376 | 6.186 | 5.117 | 4.571 | 3.493 |
| P500 | 11.897 | 10.298 | 6.379 | 4.883 |
| P664 | 26.506 | 22.685 | 13.122 | 9.646 |

Memcheck 3.24.0 with the variable intra-limb shift gave zero diagnostics at P248, P324 and P376. It reported 36 distinct contexts at P500 and 58 at P664. All 94 were “Use of uninitialised value of size 1” in code covering shifts. There were no conditional-jump diagnostics in these runs. Attribution included packed shifts and fixed stack loads. These reports warrant inspecting the generated shift code; the messages alone do not establish a secret-selected memory address or observable timing difference.

**Shift alternative.** Keep the masked whole-word shifter. For the low six bits of a shift count, synthesize one scalar power of two via six masked public-count shifts of a word. For a left shift, multiply each limb by that scalar and propagate the high half to the next limb. For a right shift, multiply by `2^(63−bits)`, double the wide result, and propagate into the preceding limb. This handles a zero intra-limb count without a branch or a word-sized shift by 64. The target must support suitable fixed-latency word multiplication. The research harness marks all limbs of T, x and y secret and runs one honest and one crafted Q5 vector per set. With power-of-two scaling, the release build gave zero Memcheck diagnostics at all five sets. A P248 positive control produced two conditional-jump diagnostics.

| Set | Original exp64 | Both with variable shift | Both with power-of-two scaling | Original / scaling |
|---|---:|---:|---:|---:|
| P248 | 2.695 | 1.551 | 1.529 | 1.76× |
| P324 | 4.429 | 2.590 | 2.558 | 1.73× |
| P376 | 6.068 | 3.423 | 3.378 | 1.80× |
| P500 | 11.819 | 4.838 | 4.960 | 2.38× |
| P664 | 25.782 | 9.442 | 9.861 | 2.61× |

Every Rust variant passed 375 model vectors, checking the first vector, its norm and the count of large substeps under the published research schedules. The separate native Python experiment below checks both vectors. Still open for production are fixed-width scratch and secret-erasure review, full-basis Rust comparison on native inputs, release instruction/address traces, distributional timing tests, and real AArch64 measurements. These results qualify the candidate for that comparison.

## Normalize the Gram matrix when constructing the ideal basis

For an integral norm-T O0 ideal with numerator basis over two, divide **all four independent Gram entries by 2T**. The class norm form makes the two diagonal entries even and the two off-diagonal entries integral. The normalized Hermitian determinant is `p` because the numerator determinant is `4pT²`. Scaling leaves exact projection ratios and strict comparisons unchanged. Exp64 can choose different *physical* substeps after scaling by a non-power of two. Its logical step proof gives the same exact basis. This calls for checking the initial-bound and intermediate-width arguments in the new units.

`check_native.py` lifts all distinct admissible odd norm-congruence fixtures from the research Rust vectors. At every scheduled step it checks the carried Gram entries against fresh products of the coordinate vectors. It also checks the determinant and the full terminal pair against the exact coordinate oracle. All five sets had 66 distinct lifts. The largest initial entries observed, in bits, were:

| | P248 | P324 | P376 | P500 | P664 |
|---|---:|---:|---:|---:|---:|
| Numerator Gram | 1301 | 1679 | 1945 | 2563 | 3383 |
| Divided by 2T | 651 | 840 | 973 | 1282 | 1692 |

A generic HNF gives the normalized entries directly as `2T, (x²+y²+p)/(2T), x, y` before ordering. This opens a concrete design choice for Stage 2b/2d. The constructor can supply the integral class Gram together with a verified basis, or Stage 2c can do the exact division once on entry. Benchmark setup and reduction together. Choose storage widths from public class bounds. For a generic input the larger normalized diagonal is bounded by `4T+p/(2T)≤4Tmax+p/2`. That gives a public starting width even when T is secret. For a guarded pre-reduced intersection the bound from PR #45 gives `2(q+1)²F√p/q` after normalization, cancelling the secret norm N. The coordinate vectors retain their own widths. This Python experiment provides no Rust timing for normalization.

## Other research paths and what would change the decision

| Direction | Why investigate | Decisive experiment or proof |
|---|---|---|
| Reciprocal or wider quotient digits | Both projections use one normalized divisor. A masked Newton reciprocal could amortize its cost. Varying the window trades multiply width against extra steps. | Compare against the shared-division reducer. Include reciprocal setup, both signs, exact ties, and hidden shift amounts. A different window needs its own progress constants. [Möller and Granlund](https://gmplib.org/~tege/division-paper.pdf) |
| Stronger contraction certificate | The existing rational proof contracts over pairs. More constrained multi-step boxes or a native O0 feasibility constraint could lower public counts without changing arithmetic. | Reproduce the current exact certificate first. Publish a checker covering every box and tail, then compare the actual saved cycles to optimizing each step. [Tour-bound study](../lagrange-tour-bound/REPORT.md) |
| Certified Lehmer batches | Several leading-word steps can share one full-width coordinate update. Existing research already tested this. | Prove a lower bound on progress per public batch, including consecutive half-tie ambiguities and large quotients. Include both fallback work and the masked matrix update in the benchmark. [Batching study](../lagrange-tour-bound/REPORT.md) |
| Division-free signed-digit microsteps | Generate both quotient components by fixed one-bit shifts, comparison and subtraction. A global budget could charge digit scans to the telescoping sum of log quotient sizes. | Give a Gaussian state transition preserving the basis, charge alignment and final half ties, and count work when different secret states share a public tick. Integer divsteps supply a design precedent. [Bernstein and Yang](https://eprint.iacr.org/2019/266) |
| Fixed-budget enumeration or general lattice reduction | An independent shortest-vector oracle is valuable. The signer also needs a spanning Gaussian pair and the form in its coordinates. | Establish a worst-case enumeration radius and a completion rule with the signer's tie conventions. Existing fixed-tour and sieve comparisons put this below the rank-two approach. [Shekyl fixed-schedule experiment](../fixed-schedule-reduction/REPORT.md) · [Lyu, Porter and Ling](https://arxiv.org/abs/1806.03113) |
| Move work to key preparation | A stored reduced secret basis can lower per-signature work. A verified class Gram is another possibility for repeated signing. | Compare cold import, warm signing, key size, replay, and validation costs. The challenge-dependent intersection still changes each signature. [Precomputation study](../keygen-precompute/REPORT.md) |
| Parallel arithmetic | SIMD on the two projection components targets latency. SIMD across independent signatures targets throughput. | Measure both under fixed lane occupancy. Include architectural portability and single-signature deployments. |
| Blinding, salt changes or higher dimension | These have already received dedicated experiments. Randomization can change the distribution without giving a public bound on work. | Reopen only with a specific residual leakage result or a new proof. Seed or protocol changes also need a compatibility and security analysis. [Research index](../INDEX.md) |
| Change the Stage 3 algorithm | Overall signer latency also depends on generator selection, constrained norm equations, and isogenies. CT ideal-generator construction and Qlapoty-style balancing target different costs. | Benchmark candidates on the same prepared ideals, with per-class failure budgets, key/signature replay and parity coverage. [Shekyl norm-equation study](../qlapoti-redesign/REPORT.md) · [CT quaternion algorithms](https://eprint.iacr.org/2025/2192) · [Qlapoty](https://eprint.iacr.org/2026/1700) · [Qlapoti+](https://eprint.iacr.org/2026/1640) |

## Reproduction

The Rust crate is standalone under `research/clamped-quotient-progress/rust-xcheck`. On the research branch, `1db834d` is the original, `9f7581a` has the shared quotient, `35b82c3` combines it with the Gram recurrence, and `0510a68` adds power-of-two scaling. Applying the Gram-only commit to the original reproduces the third column of the first timing panel. Keep each build in its own target directory. The raw process medians and control timings are in [bench.csv](bench.csv) and [scale.csv](scale.csv).

From each revision's Rust crate, run `cargo +1.97.1 build --release --locked` and `target/release/xq check`. Run `bench_compare.py --cpu 6 baseline=/path/to/original/xq shared=/path/to/shared/xq gram=/path/to/gram/xq both=/path/to/both/xq` with Python 3 for the first panel. For the second use the names and paths in order `baseline scale both`. Run `python3 -B -S research/reduction-alternatives/check_native.py` from the repository root for the O0 comparison.

On x86-64 Linux with Valgrind headers, build the optional harness with `cargo +1.97.1 build --release --locked --features memcheck`. For each set, run `valgrind --error-exitcode=77 target/release/xq memcheck p500`, substituting the set name. For the positive control append `control` at P248 and expect exit 77. The initial unscaled taint probe used the same routine and C helper linked explicitly to the combined variant before changing its shift implementation.
