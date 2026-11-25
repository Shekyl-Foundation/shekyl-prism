# Constant-time feasibility gate (`docs/SIGNER_SPEC.md`, Section 0 draft) — notes and rationale

Date: 2026-11-24. Branch: `experiment/ct-feasibility-gate` (branched from `dev` at `26ce141`).

PRISM is the signature scheme of ePrint 2026/443 (Andrea Basso, Giacomo Borin, Wouter Castryck, Maria Corte-Real Santos, Riccardo Invernizzi, Antonin Leroux, Luciano Maino, Frederik Vercauteren, and Benjamin Wesolowski). `shekyl-prism` implements it across the five Shekyl parameter sets `p248`, `p324`, `p376`, `p500`, and `p664` (`p500` selected for Shekyl integration).

## Purpose

This branch drafts **Section 0** and proposed ruling **G7** in `docs/SIGNER_SPEC.md` in response to the 2026-11-24 external design review of `shekyl-prism` (`dev` at `26ce141`, PRs #39–#42) and the empirical findings accumulated on the `research` branch (`research/INDEX.md`).

The core change is **scheduling without losing G4**:
- **G4** stays in place as the final constant-time and secret-handling hardening gate before beta (ledger **O6**).
- **Proposed G7** introduces a **constant-time feasibility gate** before **stage 2c** (lattices and basis reduction) and **stage 3** (norm equations, Cornacchia, and ideal sampling) are finalized under **G3**.

## Rationale from `research/INDEX.md` and the 2026-11-24 review

1. **Why G4 alone is too late for stages 2c and 3:**
   - *Basis reduction (2c):* Exact LLL on `I_sec · I_q` leaks `bits(N)` at `~2.0–2.1` steps/bit (`r = 0.31–0.49`; `dudect |t|` `11.6–27.3`). All lattice-reshaping attempts (`cornacchia-harness`, `salt-reshaped-lattice`, `higher-dim-lattice`) failed to remove that slope, and **D0** is disallowed under **O4** (`latticeprism-seeding`). Fixed-schedule `Z[i]`-Lagrange (`fixed-schedule-reduction`, `lagrange-tour-bound`, `rust-lagrange-ct`) solves it at `0.19–0.23×` exact LLL on GMP, provided the tour count is **provably certified** (`T_prov = 301 / 387 / 446 / 587 / 773`, or `283 / 369 / 428 / 569 / 755` with pre-reduction + balance guard, plus one exact cleanup step) rather than empirical (`189–465`), which fails Rolletschek `2 + √3` chains (`276–714`).
   - *Norm equation (3):* `quat_qlapoti` hides `bits(N)` via shortest-equivalent normalization (`n ≤ √p`) but leaks the small ideal `S` (`v₂(n)` shifts work `~27%`, fixed-`S` `dudect |t|` up to `6.4`, `2.5×–17.6×` pass-rate spread worst at `p376` where `f = 65`) through four uncapped inner loops (`194` gen draws, `59,995` `λ`-skips, `92` Gauss steps, `126` Euclid steps). Straight fixed-schedule padding costs **39×–391×** mean work at `99.9%` (`132×` at `p500`, `391×` at `p376`) and **249×–2,513×** at `2⁻⁶⁴` (`847×` at `p500`). Stage 3 therefore needs an algorithmic redesign (bounded loops, **O29**, or SQIsign 3.0's construction) before committing to a specification.
   - *Cross-set scaling is public:* Parameter sets are public (**O12**, **D13**), so work differences across sets are not a leak.

2. **Fixed-width arithmetic proofs (`Q4`, `Q5`, `O33`):**
   - `prism-int` (`26 / 33 / 38 / 49 / 65` limbs) detects overflow via sticky marks, which prevents silent truncation, but detection is a fallback—not a proof that overflow cannot occur.
   - Stage 2c must prove **Q5** (every intermediate under the actual reduction schedule, including quotients and cleanup, tested on Rolletschek-skewed and `B`-boundary lattices).
   - Stage 5 must enforce **Q4** (`N < 2^B`, `B = ⌈L/2⌉ + 32`) on generated and imported secret keys and quantify rejection rates across all five sets.
   - Overflow-driven key-dependent retries are prohibited as a design mechanism (**O33**).

3. **Declassification discipline (`2.1.3` and `check_secret_exits.py`):**
   - `scripts/ci/check_secret_exits.py` already enforces 1-to-1 correspondence between outside call sites of `SecretInt::declassify`, `SecretBit::declassify`, and `SecretInt::export_le_bytes` and the 2.1.3 table (currently 0).
   - Section 0.3 adds two process rules: (a) the security rationale for any new exit must be reviewed in the introducing PR, not deferred to G4; and (b) values retain their secret sensitivity downstream inside `Int<N>` or exported bytes.

4. **The `G2` `no unsafe` decision point:**
   - `research/rust-lagrange-ct` found that `core::hint::black_box` on a choice bit compiled into a secret-dependent branch in a barrel shifter (`rustc 1.85.1`), which `unsafe { core::arch::asm!(...) }` fixed—conflicting with **G2** (`#![forbid(unsafe_code)]`).
   - Section 0.4 keeps this explicitly open for Rick's decision based on the parallel `experiment/ct-barrier-policy` results under `rustc 1.97.1` on `x86_64` and `aarch64` (safe barrier vs. narrowly audited internal barrier crate).

5. **Compiled-artifact verification, provenance, and representation equivalence:**
   - Source-level branch-free Rust is insufficient; Section 0.5 requires assembly scans, high-power `dudect` (not 24–32 keys), Valgrind/`ctgrind` memcheck with positive controls, and **O32** watch-list / stack-frame checks on release builds under `rustc 1.97.1` on both `x86_64` and `aarch64`.
   - Section 0.6 captures non-sensitive signing-transcript provenance logging for tests, the three-way distinction (`same ideal` / `verifier-accepted` / `byte-identical`) for representation changes (`keygen-precompute` `EXP_RED_O0` and **O28** canonical PKs), and Rick's Stage 5 proposals (boundary fault checks, verify-after-sign, and **D8** hedged salt replay reconciliation).

## Branch lifecycle note

Because `docs/SIGNER_SPEC.md` is a specification proposal targeting `dev` if Rick approves, `experiment/ct-feasibility-gate` is merged `--no-ff` into `research` for tracking and **kept alive** (not deleted, and not merged to `dev` or opened as a PR) so Rick can open a PR to `dev` directly if he adopts it.

## Addition (2026-11-25): progress proof for bounded-precision quotients

Rick, after the Q5 width experiment: "The faster quotient needs a progress proof before adoption." Section 0.2 item 3 now says so explicitly, the 0.7 table lists it for stage 2c, and 0.8 has a new decision 6 (the old 6 is now 7).

- **The rule.** Any bounded-precision, truncated, clamped or approximate quotient in a fixed-schedule reduction must carry a certified progress proof: the fixed step count plus the scheduled cleanup reaches a reduced basis for every valid input within the Q4 bound. Otherwise the certified count (`T_prov = 301 / 387 / 446 / 587 / 773`, or `283 / 369 / 428 / 569 / 755` pre-reduced) applies only to the exact-quotient schedule.
- **Why a width proof is not enough.** Q5 bounds every intermediate whether or not the schedule converges (its Theorem 1 holds for any schedule length). It says nothing about finishing.
- **The counterexample.** `research/q5-width-proof/crafted_saturation.py` builds, at each of the five sets, a valid prime-norm ideal on which every 64-bit quotient clamps at `2^62`. The run stays inside the width bound, but `T_prov` steps plus the exact cleanup leave the basis unreduced; exact quotients finish in 8–10 steps. Honest inputs hit this at roughly `2^-110`, but nothing reports it, and the failure would be key-dependent.
- **Follow-up.** `experiment/clamped-quotient-progress` (results in `research/clamped-quotient-progress/`) characterises the stall and looks for a provable bounded-precision quotient or a fallback.
