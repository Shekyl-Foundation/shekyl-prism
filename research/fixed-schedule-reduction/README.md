# Fixed-schedule reduction and fixed-budget search (research harness)

Research harness for Shekyl PRISM. PRISM is the scheme of ePrint
2026/443 (Basso, Borin, Castryck, Corte-Real Santos, Invernizzi, Leroux,
Maino, Vercauteren, Wesolowski); shekyl-prism implements it. This
directory does not change the signer, the signature format, or any
parameter set. Results are in [REPORT.md](REPORT.md).

Step and multiply counts are operation counts in Python, not cycles, and
Python big integers are not constant-time. This is algorithm-level
evidence meant to guide a Rust port.

Tour and step counts are sized from the public bound
B = ⌈L/2⌉+32 on the secret ideal's norm, never from the actual N.
The response ideal is a hash-chosen (H2) ideal of norm q; the
deterministic smallest ideal (D0) is not used.

## Files

| file | what |
|---|---|
| `hdl.py` | Copied from `research/higher-dim-lattice` (branch `experiment/higher-dim-lattice`): keys, salt, challenge q, H2 ideal selection, secret product. |
| `fsr.py` | Exact LLL with op counters (baseline), fixed-tour 4-D reduction, Z[i]-Lagrange fixed steps (optional bounded-precision quotient), fixed-budget Schnorr-Euchner enumeration, fixed-round pairwise sieve. |
| `run_fsr.py` | Pass 1: baseline vs fixed schedules. Panels: stream (I_q), prod (product at typical bits(N)), nvar (product with varied bits(N)), nbound (bits(N)=B), target (fixed-q H2 seeds), dudect_fixed / dudect_rand. |
| `run_zi_tail.py` | Extra samples for the Z[i] schedule's upper tail, on I_q and on the product at B. |
| `run_budget.py` | Pass 2: shortened pre-reduction + fixed-budget enumeration / sieve. |
| `crosscheck_pari.py` | PARI qfminim vs our λ₁ and the Z[i] output, 500 lattices. |
| `analyze.py` | Tables (`results/*.csv`, `results/summary.json`) and plots (`plots/`). |
| `run_all.sh` | Resumable driver (each stage skips outputs that exist). |

## Running

Uses the SageMath micromamba env's Python (gmpy2, cypari2, pandas,
scipy, matplotlib):

```sh
PY=/workspace/mamba/envs/sage/bin/python nohup ./run_all.sh &
```

Sets: P248, P324, P376, P500, P664 only.
