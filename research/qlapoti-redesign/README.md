# qlapoti redesign (experiment)

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

Results and recommendation: [`REPORT.md`](REPORT.md); every table:
[`results/TABLES.md`](results/TABLES.md).

- `fne.py`, `fsne.py`: copied unchanged from
  `research/fixed-schedule-norm-equation/` (the port of `quat_qlapoti`,
  PRISM_v2 @ 87fad7d, and its fixed-schedule variant).
- `qlr.py`: per-lambda sweep logger on the reference structure (one
  generator, many lambdas, an outcome code per lambda, sieve buckets).
- `collect.py`: reference-structure data (600 S per set, 8 generators x
  4000 lambdas at e-2, e-1, e; 64 generator searches per S). Resumable,
  seeded. Raw logs in `data/` (git-ignored, ~1.2 GB, reproducible).
- `qlapoty_proto.py`, `collect_q.py`: Qlapoty-style prototype (balanced
  alpha0, odd lambda, mod-4n lattice, z prime = 1 mod 4) on the same S.
- `summarize.py`: raw logs -> `results/*.csv.gz` (committed).
- `costmodel.py`: work-unit model of every component (fsne units).
- `analyze.py`: worst-case sizing and design optimisation ->
  `results/tables.json`; `tables_md.py` -> `results/TABLES.md`.
- `c_o29/`: `build.sh` (five builds of the pinned C reference: used power
  e-2, e-1, e, and e-2 / e without the theta postcondition) and
  `run_c.py`; per-seed results in `c_o29/runs/` (pk and sm hex of test
  seeds, no real keys).

Reproduce: `python collect.py && python collect_q.py && python
summarize.py && python summarize.py --qy && python analyze.py && python
tables_md.py` (Sage env with gmpy2 / cypari2 / pandas / scipy; 3 cores,
about 25 minutes). C: `bash c_o29/build.sh && python3 c_o29/run_c.py
e2,e1,e0,e2np,e0np 1,3,5 100` (needs the QLSTAT patch from
`research/fixed-schedule-norm-equation/c_crosscheck/`).
