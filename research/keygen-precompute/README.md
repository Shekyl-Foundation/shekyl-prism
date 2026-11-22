# Keygen Precomputation for Secret-Only Signing Work (research harness)

Research harness for Shekyl PRISM. PRISM is the signature scheme of ePrint
2026/443 (Basso, Borin, Castryck, Corte-Real Santos, Invernizzi, Leroux,
Maino, Vercauteren, and Wesolowski); `shekyl-prism` implements it. This
directory does not change the signer, the signature format, the secret-key
format, or any parameter set.

Sets covered: P248, P324, P376, P500, and P664 only.

## Question

Which signing work in `shekyl-prism` depends only on the secret key, and
if that work is moved to key generation and stored in an expanded secret
key (e.g., `I_sec` HNF/reduced basis, its Gram matrix, dual/intersection
data, and `E_vk` canonical basis), how much per-signature cost is saved?

Specifically:
1. Inventory secret-only vs per-message work across `shekyl-prism`
   (`scripts/c/export_s5_vectors.c`, `docs/SIGNER_SPEC.md`, and the pinned
   reference `PRISM_v2`).
2. Measure in the 4-D model (extending `research/fixed-schedule-reduction/`
   on `research`) the per-set signing step, multiply/divide, and time
   savings when `I_sec` precomputations (HNF, pre-reduced basis, Gram
   matrix, dual/intersection setup) move to keygen.
3. Measure expanded secret-key size growth (bytes) and keygen cost increase.
4. Test whether remaining per-signature reduction on `I_sec ∩ I_q` still
   leaks `bits(N)` and still needs a fixed schedule, and how the schedule
   bound `B = ⌈L/2⌉ + 32` (or tour/step count `T`) changes when `I_sec`'s
   basis is pre-reduced.
5. Document compatibility with stage 5 replay against the C reference and
   with the secret-key format (noting that any format change requires a
   design round).

## Plan

1. **Code/spec audit:** Document every secret-only vs per-message step in
   the signing path and compute exact expanded-key byte budgets across all
   five parameter sets.
2. **Model harness (`kpc.py`, `run_kpc.py`):** Implement and verify:
   - Baseline (`HNF` of `I_sec` + `I_q` intersection + exact LLL /
     fixed-schedule `Z[i]`-Lagrange / 4-D up-down from `research/fixed-schedule-reduction/`).
   - Expanded-key arms:
     - `EXP_HNF`: skip `I_sec` reconstruction (`quat_lideal_create` / dual)
       and `canonical_basis` search; keep exact same `I_prod` HNF input.
     - `EXP_RED`: pre-reduce `I_sec` at keygen (`Z[i]`-Lagrange / LLL) and
       intersect with `I_q` in the pre-reduced basis coordinates (with
       precomputed Gram matrix), reducing `I_prod` from the resulting
       shorter basis.
3. **Measure & verify:** Run across all 5 sets (`P248`–`P664`), checking
   `bits(N)` correlation (nvar, nbound), dudect-style `t`-tests, tour/step
   counts `T` needed for 100% `λ₁` / exact-LLL parity, and PARI/model
   cross-checks.
4. **Analyze & report:** Generate tables, plots, and `REPORT.md`, then fold
   into `research` (`research/INDEX.md`).
