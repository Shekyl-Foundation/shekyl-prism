# clamped-quotient-progress

Does the bounded-precision quotient of the fixed-schedule Z[i]-Lagrange
reduction have a progress proof, and what are the public step counts of
the reduction contract for every admitted input class? Follows
`research/q5-width-proof` section 6.1 (the clamp counterexample) and
Rick's reduction contract of 2026-11-25. Research only: nothing here
changes `dev`, the signer, `SIGNER_SPEC.md`, or a parameter set, and no
constant-time claim is made. PRISM is ePrint 2026/443 (Basso et al.);
shekyl-prism implements it.

Read `REPORT.md`. Files:

| File | What it does |
|---|---|
| `q5common.py`, `instrumented.py`, `gen_inputs.py`, `bounds.py`, `crafted_saturation.py` | Copied unchanged from `research/q5-width-proof` (sets, forms, the exact port of the prototype's 64-bit quotient, input samplers) |
| `certify.py` | Copied unchanged from `research/lagrange-tour-bound` (the exact-rational certificates behind `T_prov`) |
| `trace_counterexample.py` | Step-by-step trace of the Q5 counterexample under the clamp and under exact quotients; also the `k = 1` ideal with the largest first quotient |
| `variants.py` | Quotient rules: `exact`, `clamp64` (prototype), `clamp128`, `corr64` (clamp as sticky overflow), `exp64` (proposed) |
| `progress_check.py` | Machine checks: certificates rerun, contract audit, integer step counts per input class, exp64 extra sub-steps, exp-rule lemmas at toy and 64-bit precision |
| `adversarial.py`, `analyze.py` | Every rule on honest, crafted, Rolletschek, big-quotient chain, tail-stress, ping-pong, challenge, keygen and pre-reduced inputs; honest quotient tail |
| `verify_finding.sage` | Verification of Rick's finding (t37u): symbolic Δ of the O0 basis in Sage (4pT², not 16pT²) and numeric Δ on 100 real O0 bases |
| `verify_o0_counts.py` | Runs O0 inputs and Rolletschek chains at the O0 bound against the recomputed O0 counts |
| `gen_vectors.py`, `rust-xcheck/` | The exp64 rule on the `rust-lagrange-ct` prototype's arithmetic (standalone crate, outside the workspace): bit-exact check against the model, and cycles against the prototype |

Reproduce (Python from `/workspace/mamba/envs/sage`):

```bash
PY=/workspace/mamba/envs/sage/bin/python3
$PY trace_counterexample.py          # ~2 s
PATH=/workspace/mamba/envs/sage/bin:$PATH sage verify_finding.sage   # ~4 s
$PY progress_check.py                # ~2 min (certificates ~16 s); model and O0 counts
$PY verify_o0_counts.py              # ~3 s
for s in p248 p324 p376 p500 p664; do $PY adversarial.py $s --scale 4 & done; wait   # ~2 min
$PY analyze.py
$PY gen_vectors.py
cd rust-xcheck && cargo build --release && ./target/release/xq check \
  && for s in p248 p324 p376 p500 p664; do RLC_CPU=6 ./target/release/xq bench $s 500; done
```
