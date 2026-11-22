# Rust port of the fixed-schedule Z[i]-Lagrange reduction (research)

Research only. PRISM is the scheme of ePrint 2026/443 (Basso, Borin,
Castryck, Corte-Real Santos, Invernizzi, Leroux, Maino, Vercauteren,
Wesolowski); shekyl-prism implements it. This directory does not change
the signer, the signature format, or any parameter set, and the crate
here is standalone: it is not a member of the workspace and is not built
by CI.

## Question

`research/fixed-schedule-reduction` (Python model) found that a
Z[i]-Lagrange reduction with a fixed step count T, sized from the public
bound B = ⌈L/2⌉+32 on the secret norm (T = 189/239/283/359/465 for
P248/P324/P376/P500/P664), finds λ₁ on every sample at about 0.28× exact
LLL's multiply-and-divide count. Step counts are not cycles. Does that
hold up as real constant-time code?

## Plan

1. A standalone Rust crate with a fixed-limb signed integer type sized
   per set from B (no data-dependent branches, mask-based conditional
   swaps, constant-time comparisons, fixed-iteration division, full
   schedule always run), plus a plain variable-time exact reduction
   (exact LLL on the same lattice, arbitrary precision) for comparison.
2. Cross-check against the Python model on shared test vectors
   (λ₁ equality, and the reduced vector itself).
3. Measure cycles per set for both (rdtsc, many repetitions, pinned core
   where possible).
4. dudect-style fixed-vs-random Welch t-test over tens of thousands of
   measurements (threshold |t| = 4.5).
5. Read the generated assembly of the hot loop for branches or
   divisions on secret data.

Sets: P248, P324, P376, P500, P664 only.
