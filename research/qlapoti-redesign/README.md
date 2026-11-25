# qlapoti redesign (experiment, in progress)

Date: 2026-11-24. Research for Shekyl PRISM. Model estimates in Python;
no constant-time or security claim. Nothing here changes the signature
or secret-key format on `dev`; any construction that would is a proposal
for a design round with Rick, not an adoption.

PRISM is the scheme of ePrint 2026/443 (Basso, Borin, Castryck,
Corte-Real Santos, Invernizzi, Leroux, Maino, Vercauteren, Wesolowski);
`shekyl-prism` implements it.

## Question

The fixed-schedule norm-equation experiment
(`research/fixed-schedule-norm-equation/`) found that `quat_qlapoti`'s work
depends on the secret-derived small ideal `S` (norm `n <= sqrt(p)`), that
four inner loops are uncapped, and that padding the reference search to a
fixed schedule costs 39–391× the reference mean at 99.9 % and up to
~2,500× at 2^-64. After the external design review of 2026-11-24, Rick
asked for an affordable constant-time strategy. Three sub-experiments:

1. **Bounded inner loops.** Cap each of the four inner loops (generator
   draws, coprime-lambda skips, 2-D Gauss steps, Cornacchia Euclid steps)
   as the reference would, measure completion probability per set against
   the cap, choose caps that minimise fixed-schedule work at 99.9 % and
   2^-64, and give the new padding factors.
2. **O29 (`e` versus `e - 2`).** From the paper and the reference: which
   target exponent is correct and safe, what changes for signing and the
   verifier, and the cost effect. Flag anything that changes signature
   bytes (stage 5 replay).
3. **SQIsign 3.0 / Qlapoti / Qlapoty and newer norm-equation constructions.**
   Survey; fit for PRISM (dependence on `S`, expected trials, fixed
   schedule, cost); prototype what is promising.

Then a combined design, its estimated fixed-schedule cost per set, and the
secret-dependent channels that remain.

Sets: P248, P324, P376, P500, P664 only (P500 is Shekyl's).

## Files

- `fne.py`, `fsne.py`: copied unchanged from
  `research/fixed-schedule-norm-equation/` (the faithful port of
  `quat_qlapoti`, PRISM_v2 @ 87fad7d, and its fixed-schedule variant).
- `qlr.py`: the per-lambda sweep logger (one generator, many lambdas,
  outcome code per lambda; sieve buckets for a two-stage design).
- `collect.py`: data collection (resumable, seeded). Raw logs go to
  `data/` (git-ignored, about 1 GB; deterministic from the seeds).
- `summarize.py`: raw logs to per-generator summaries in `results/`.
