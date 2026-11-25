# q5-width-proof

SIGNER_SPEC open question Q5: the largest intermediate the reduction holds,
for the reduction actually proposed, which is the certified fixed-schedule
Z[i]-Lagrange (64-bit quotients, then one exact cleanup step) of
`research/lagrange-tour-bound` and `research/rust-lagrange-ct`, including the
keygen pre-reduced variant of `research/keygen-precompute`. Research only:
nothing here changes the signer, the spec on `dev`, or any parameter set.

Read `REPORT.md`. Files:

| File | What it does |
|---|---|
| `q5common.py` | Parameter sets, the form N4, the Hermitian form |
| `bounds.py` | Exact (integer and Fraction) computation of every proven bound, at every set and input variant; writes `results/proven_bounds.csv`, `results/proven_summary.csv` |
| `lemmas_check.py` | Symbolic and randomised checks of Lemmas 1, 2, 3 and 5; writes `results/lemmas_check.txt` |
| `instrumented.py` | The reduction with the Rust prototype's quotient ported exactly, recording every intermediate |
| `gen_inputs.py` | Ideal samplers, Rolletschek chains, crafted keys, the pre-reduced intersection |
| `run_empirical.py` | The empirical campaign; writes `results/raw/<set>.json` (resumable) |
| `crafted_saturation.py` | A valid ideal on which the 64-bit quotient clamp leaves the basis unreduced |
| `analyze.py` | Builds `results/tables.md`, `results/summary.json`, `results/empirical_vs_proven.csv` |

Reproduce (about 20 minutes on two cores of the shared box at scale 100):

```bash
PY=/workspace/mamba/envs/sage/bin/python3
$PY lemmas_check.py && $PY bounds.py && $PY crafted_saturation.py
$PY run_empirical.py p248 p324 p376 --scale 100 &  $PY run_empirical.py p500 p664 --scale 100
$PY analyze.py
```
