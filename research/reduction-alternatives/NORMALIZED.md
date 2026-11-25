# Native O0 reduction with an integral Gram matrix and a three-link certificate

At P500 the normalized Rust reducer with the shorter schedule took **1.421 ms** including setup. The same O0 reducer carrying the raw Gram matrix took **2.010 ms** on the PR #45 schedule. Both use exp64, the shared divisor and the same coordinate updates. The new certificate lowers the generic signing count from 609 to 596 while retaining the any-basis allowance for large quotients. The fixed-width implementation in this experiment covers the generic HNF constructor. The certificate also covers the other four native input classes. [Stage 2c spec at measurement](https://github.com/Shekyl-Foundation/shekyl-prism/blob/84a798a/docs/SIGNER_SPEC.md)

## Gram units and setup

Let the O0 numerator columns be (2T,0,0,0) and (x,y,1,0). Their raw Gram entries are (4T², x²+y²+p, 2Tx, 2Ty), with determinant 4pT². Dividing every entry by four gives the reduced-norm determinant pT²/4. Dividing every **raw** entry by 2T instead gives the integral class Gram

    A = 2T,   C = (x²+y²+p)/(2T),   h₀ = x,   h₁ = y,   Δ = p.

In all three conventions the upper bound on A₀/√Δ is 2T/√p. It exceeds the model-order bound by one bit. The scalar also cancels out of the strict swap and the exact half-up quotient. A change of binary scale can alter the physical exp64 substeps. Each run completes the same logical reductions by exact correction and integer translation of the quotient.

At this entry point T is odd, x is even, y is odd, both residues are in [0,2T), and the norm congruence holds. Since p ≡ 3 mod 4, x²+y²+p is divisible by 4T. The normalized constructor computes this numerator at double width. To divide by 2T, take the low C limbs of its half, including the bit crossing the high boundary, then divide modulo 2^(64C) by odd T. Six Newton iterations give the inverse of its low word. Successive quotient words cancel one residual word each with a fixed public multiply-subtract loop. Since the exact quotient fits, its residue gives the entire result. The constructor assumes the upstream ideal-admission checks. This mechanism requires an odd norm.

For coordinate limb counts C = 11, 14, 16, 21, 27, the class Gram uses C limbs per entry and the raw Gram uses 2C. With T ≤ Tmax the starting diagonals are bounded by M = 4Tmax + ⌈p/2⌉. The exp64 non-growth lemma keeps this bound at every step and Cauchy–Schwarz gives |hⱼ| ≤ M. For example, the quotient code divides by a truncated divisor with relative underestimate below 2⁻⁶³, so its unsigned leading quotient times A is at most 2|hⱼ|, including on the unselected correction path. This bounds the absolute correction numerator and its comparison against 2A by 9M. The bit lengths of 9M are 655, 844, 977, 1286 and 1696, against signed magnitude capacities of 703, 895, 1023, 1343 and 1727 bits. The recurrence and coordinate updates reuse the carried-Gram implementation. A production audit still needs to account for all temporaries and erasure. [exp64 lemmas](../clamped-quotient-progress/REPORT.md) · [Rust constructor and reducer](../clamped-quotient-progress/rust-xcheck/src/native.rs)

## Shorter public schedule

Use the existing main-phase potential Φ = A(1+2|ε|²) where A is the norm before the indexed swap, ε is its half-up residual and λ = Δ/A² ≤ 1/1000. Between consecutive main-phase swaps, A is multiplied by r = λ + |ε|², λ becomes λ/r², and the next residual is the componentwise centered value of conjugate(ε)/r. The new result bounds **Φ at the fourth consecutive main-phase swap by 47/2000 times Φ at the first**. The older pair and single bounds give (87/1000)(31/100) = 0.02697 over those three links.

The checker starts with the full closed residual square and λ in [0,1/1000]. It outward-bounds each successor with exact rational intervals, enumerates all possible half-up quotient cells, and uses strict swap feasibility. A successor quotient of norm squared ≤ 1 cannot give another strict swap after its predecessor swapped (the existing Lemma 1). The test at depth three requires the terminal swap to be possible. Reaching an unresolved interval forces subdivision, and exhausting the subdivision limit fails the check. The three-link run discharged 1,375,199 boxes. The same command rechecks the single-link, two-link and tail certificates on 72,751, 83,751 and 309,007 boxes respectively. [Three-link checker](tour_certificate.py) · [Strict-swap lemma and tail](../clamped-quotient-progress/REPORT.md)

For k consecutive links define b₀ = 1 and bₖ = minᵢ(bₖ₋ᵢρᵢ) over available block lengths i ≤ k. Every term bounds the same potential ratio by composition. For S consecutive main swaps one needs (bₛ₋₁ Φ₁)² ≥ 1000Δ. The count generator inverts this inequality in rationals using the PR #45 native bounds. It adds four possible tail swaps, one final size reduction and the existing any-basis exp64 allowance for large quotients. All 25 old exact/exp64 pairs reproduce the spec. The resulting **exp64 public steps** are below. [Exact and exp64 schedules](native_schedules.csv) · [Count generator](native_counts.py)

| Input class | P248 | P324 | P376 | P500 | P664 |
|---|---:|---:|---:|---:|---:|
| Generic signing HNF | 305 | 392 | 452 | 596 | 785 |
| Stored pre-reduced basis, guard 256 | 291 | 379 | 438 | 581 | 770 |
| Key generation, norm D | 226 | 352 | 337 | 449 | 696 |
| Secret ideal pre-reduction | 24 | 25 | 25 | 26 | 28 |
| Challenge alone | 215 | 281 | 324 | 433 | 574 |

The stored-basis row requires the same guard at generation and decode. The any-basis allowance removes reliance on the ideal lower bound on the first minimum. Each row still requires its specified native starting bounds.

## Correctness and timings

Across 330 distinct native lifts the Rust check matched both terminal vectors and all four Gram entries against the Python oracle, including the raw old schedule, normalized old schedule and normalized short schedule. It also checked Δ = p at construction and entrywise rescaling. The exact oracle recomputes norms from coordinates. The normalized Python implementation compared every carried entry with a fresh coordinate product at every step. A separate Python panel compared the shorter exact and exp64 schedules with the variable-time exact fixed point on 215 pairs. It exercised all five classes, 60 guarded intersections, 15 crafted Q5 inputs and both Rolletschek orientations at each set's O0 signing bound. The latter chains use model-shaped columns at the O0 ratio bound. [Native oracle](check_native.py) · [Class and chain panel](check_schedules.py)

Median of three process medians on an Intel Core i9-11950H, x86-64 Linux, pinned to CPU 6, powersave governor. Rust 1.97.1, release, fat LTO, one codegen unit. Each process rotated four variants over the same honest inputs with 10 warmups and 41 measured trials. Each reduction includes its constructor, initial ordering, coordinate and Gram updates, and returned pair and form. The normalized setup column times the constructor alone. All other timing columns are milliseconds.

| Set | Old → new steps | Raw, old | Normalized, old | Normalized, new | Normalized setup (ns) | Raw old / normalized new |
|---|---:|---:|---:|---:|---:|---:|
| P248 | 312 → 305 | 0.612 | 0.347 | 0.339 | 276 | 1.80× |
| P324 | 401 → 392 | 1.023 | 0.576 | 0.562 | 430 | 1.82× |
| P376 | 463 → 452 | 1.351 | 0.758 | 0.741 | 566 | 1.82× |
| P500 | 609 → 596 | 2.010 | 1.458 | 1.421 | 964 | 1.41× |
| P664 | 803 → 785 | 4.003 | 2.517 | 2.455 | 1513 | 1.63× |

The earlier P500 2.38× measurement used model coordinates, a different baseline and 598 steps. These native timings measure the additional gains on top of a carried Gram directly. Load and hardware frequency were uncontrolled. Performance for the pre-reduced intersection constructor and for a complete signature remains unmeasured. [Raw process medians](normalized_bench.csv)

Valgrind Memcheck on the x86-64 release probe reported zero errors on one honest and one crafted Q5 input per set, with every limb of T, x and y marked secret before normalized setup. The positive control gave two conditional-jump diagnostics. Production adoption still needs the G7 instruction/address trace checks, distributional timing, real AArch64 measurements and independent certificate review. [Harness](../clamped-quotient-progress/rust-xcheck/src/bin/native_gram.rs)

## Reproduction

Run from this directory. The interval and class-panel checks require Python 3 with gmpy2. Fixture generation, schedule generation and the Rust driver use the Python standard library. The optional Memcheck run also requires Valgrind and its C headers.

```sh
python3 -B tour_certificate.py
python3 -B check_schedules.py
python3 -B -S native_counts.py | diff - native_schedules.csv
fixtures=$(mktemp -d)
python3 -B -S check_native.py --emit "$fixtures"
crate=../clamped-quotient-progress/rust-xcheck
cargo +1.97.1 test --manifest-path "$crate/Cargo.toml" --release --locked --lib
cargo +1.97.1 build --manifest-path "$crate/Cargo.toml" --release --locked --bin native_gram
binary="$crate/target/release/native_gram"
python3 -B -S run_native.py check "$binary" "$fixtures"
taskset -c 6 python3 -B -S run_native.py bench "$binary" "$fixtures"
cargo +1.97.1 build --manifest-path "$crate/Cargo.toml" --release --locked --bin native_gram --features memcheck
python3 -B -S run_native.py memcheck "$binary" "$fixtures"
```
