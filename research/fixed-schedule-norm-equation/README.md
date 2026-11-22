# Fixed-schedule norm equation (experiment, in progress)

Date: 2026-11-22. Research for Shekyl PRISM. Step-counting model in
Python; no constant-time or security claim. Not a proposal to change the
signature format. PRISM is the scheme of ePrint 2026/443 (Basso, Borin,
Castryck, Corte-Real Santos, Invernizzi, Leroux, Maino, Vercauteren,
Wesolowski); shekyl-prism implements it.

## Question

The fixed-schedule reduction experiment (research/fixed-schedule-reduction)
removed the ~2 steps/bit leak of exact LLL on the secret product. The
review named the retry loops as the larger unmeasured channel. This
experiment measures them and tests whether the same idea carries over:

1. **Measure.** Port the reference's search loops faithfully
   (PRISM_v2 @ 87fad7d, the pin in `external/PRISM_v2`):
   - `quat_qlapoti` (the ideal-to-isogeny norm equation; signing and key
     generation): 20,000 multiples per generator, 20 × 20,000 = 400,000
     passes, 17-bit generator coefficients, and its two uncapped inner
     loops (`quat_lideal_generator_small_coprime`, the coprime-λ step);
     `ibz_cornacchia_extended` with the 101-prime list and 32 primality
     rounds.
   - `quat_lideal_prime_norm_reduced_equivalent` (key generation's
     prime-norm search, box 64, (2·64+1)^4 draws).
   Count passes, generator draws, inner-loop steps, primality tests,
   and a weighted work total, against bits(N), the key, and random keys
   (many keys, correlation with stated power, dudect-style fixed-vs-random
   Welch t).
2. **Fixed schedule.** A version that always runs a fixed number of
   passes and fixed inner-loop candidate counts, sized from public
   bounds, with branchless selection of the first valid answer; work is
   recorded as always executed. Compare success rate, cost overhead, and
   leak.
3. **Budget curve.** Failure probability against the pass budget, so a
   budget (99.9 %, 2^-20, 2^-40, 2^-64 targets) can be justified, and an
   in-model estimate for ledger O29 under the reference's 400,000.

Sets: P248, P324, P376, P500, P664 only.

Status: plan committed; code and results follow in later commits.
