# Re-randomizing the secret ideal per signature (research harness)

Research harness for Shekyl PRISM. PRISM is the scheme of ePrint
2026/443 (Basso, Borin, Castryck, Corte-Real Santos, Invernizzi, Leroux,
Maino, Vercauteren, Wesolowski); shekyl-prism implements it. This
directory does not change the signer, the signature format, or any
parameter set. Status: **complete** — see [REPORT.md](REPORT.md).

## Question

The secret ideal `I` (norm `N`) has many equivalent representatives
`J = I · β̄ / N` for `β ∈ I`, all with the same left order and a right
order conjugate to `O_R(I)`, i.e. the same key curve up to isomorphism.
If the signer draws a fresh `J` for every signature, does it remove the
key-dependent channel found in `research/fixed-schedule-reduction` (~2
steps per bit of `N` in exact LLL), does it help on downstream
`quat_qlapoti` (norm equation), and is it worth carrying as defence in
depth on top of the certified fixed `Z[i]`-Lagrange schedule?

## Files

| file | what |
|---|---|
| `hdl.py`, `fsr.py`, `run_fsr.py` | Copied from `research/fixed-schedule-reduction` (keys, challenge, H2 ideal, exact LLL with counters, `Z[i]`-Lagrange fixed schedule). |
| `fne_ref.py` | Copied from `research/fixed-schedule-norm-equation/fne.py` (`O₀` model of `quat_qlapoti` and `quat_lideal_shortest_equivalent`). |
| `rr.py` | Keygen `Z[i]`-reduction, box samplers (`plain`, `norm`) and Gaussian-then-round (`gauss`) samplers with `prime`/`odd` acceptance, re-randomized product `J·K_J`, and `d = d'·β/N'` map-back. |
| `run_rr.py` | Resumable runner for `samp` / `sampg` (1,000 keys × 20 sigs), `pair` (150 keys × 6 msgs), `dudect_fixed` and `dudect_rand` (1,000 traces each). |
| `verify_tprov.py` | Verifies `Z[i]`-Lagrange at the certified worst-case tour counts `T_prov = 301/387/446/587/773` (`research/lagrange-tour-bound`). |
| `qlapoti_check.py` | Tests re-randomizing `I_sec -> J` ahead of `quat_qlapoti` (`fixkey` vs `rnd`, `fixS` vs `rnd`, `v₂(n)` correlation). |
| `analyze_rr.py` | Produces `results/*.csv`, `results/summary.json`, and `plots/`. |
| `REPORT.md` | Full findings and tables. |

## Running

Uses the SageMath micromamba env's Python (`gmpy2`, `cypari2`, `pandas`,
`scipy`, `matplotlib`):

```sh
PY=/workspace/mamba/envs/sage/bin/python nohup ./run_all.sh &
$PY verify_tprov.py
$PY qlapoti_check.py
$PY analyze_rr.py
```

Sets: P248, P324, P376, P500, P664 only.
