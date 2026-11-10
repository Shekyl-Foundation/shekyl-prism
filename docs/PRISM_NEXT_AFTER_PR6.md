# shekyl-prism after PR #6 — nightly fix and the next steps (2026-11-09)

Ground: `dev` at `d17b520` (PR #6 merged: three §5 vectors at `e′`, `s5_vector` exporter, basis sign kept). Open: issue #5 `nightly RED`. Rulings in force: `e′ = e − 2`, field and Table 2 untouched, `Unidentified` until identification, nothing at `e′` built before the Nakagawa–Onuki recheck.

## PR A — nightly fix (closes #5). Lands first, per the red-nightly rule.

Cause: `scripts/ci/install_pinned_tool.py` installs the prebuilt **musl** cargo-fuzz 0.13.2. cargo-fuzz defaults `--target` to its own build triple, so all five jobs tried to build the fuzz targets for `x86_64-unknown-linux-musl`, where ASan is refused (`-crt-static`) and `core` is not installed. Fuzzing has never executed on this repo.

Changes, all in `.github/workflows/nightly.yml`:
1. `run: cargo +nightly fuzz run ${{ matrix.target }} --target x86_64-unknown-linux-gnu -- -max_total_time=600`. The pinned musl binary stays; only the build target changes.
2. **Prove it ran.** After the run, fail the step unless libFuzzer's final `stat::number_of_executed_units` (or the `#N DONE` line) shows at least 1,000 executions, and unless `fuzz/corpus/<target>/` is non-empty. A green job with zero executions is the failure mode this PR exists to remove.
3. Leave a one-line comment at the `--target` flag saying why, so nobody "simplifies" it away.

Gate: the five fuzz jobs green on a manual `workflow_dispatch` before merge, with the execution count visible in the job summary. PR body closes #5.

Settings, not a PR (Rick): a ruleset on `dev` requiring `ci-pass`, one review, and no bypass. The review's Finding 7 says none exists; #4 and #6 merged on habit.

## Step 1 — Nakagawa–Onuki recheck on the `e′` window (gates any chain at `e′`)

Input: the three `q` lines from `s5_lvl{1,3,5}.txt` are a sample of three, which is not a sample. Draw more cheaply: the `e′` derivation in `challenge.rs` yields `q_j` without signing, so take the first 1,000 prime `q_j` per level over random `(A, message)` — no C tree, no isogeny.

Work: read ePrint 2025/1602 ([53]) for the actual condition — their attacks hinge on the challenge prime `q` dividing `(−p)^k − 1` for small `k` (torsion over small extensions). `p` is unchanged, so their `p`-side analysis stands; what moves is the `q` window (`2^{2e−5}` instead of `2^{2e−1}`). For each sampled `q`, compute the multiplicative order of `−p` mod `q` (or test `k` up to their bound) and report the distribution against their threshold. Write the result into the spec under a new "Small-extension torsion" heading, with the sampling script committed.

Outcome: either "no `q` in the window falls under [53]'s bound at the rate they need" and the chain at `e′` is cleared, or a `q`-filter is added to Algorithm 2's search (the signer skips such `j`) and the verifier refuses them. Rick rules on which if it comes to that.

## Step 2 — chain at `e′` with identification (the SQIsignHD half-chain shape)

Only after Step 1. One PR:
- move the exponent in `challenge.rs`, `relation.rs` and the order checks to `e′`; the three vectors must then reach the chain;
- build `Φ₁` (length `e′`) and `Φ̃₂` (length `e′ − 1`) as two half-chains from the product, with the starting symplectic bases of the `2^{len+2}`-torsion chosen so the codomain structures are dual (SQIsignHD App. C.2 / Lemma 29; `KaniEndoHalf` and `base_change_dim2.py` in `Theta_dim4` are the dimension-2 pieces, Apache-2.0, NOTICE credit);
- codomain check: theta-null equality after one Hadamard; step 8 through `Φ₁` then the dual of `Φ̃₂` on `(R,0), (T,0), (R−T,0)`; per-factor sign under the Lemma 5.7.1 degree argument, written into the spec, not assumed;
- `Unidentified` is deleted only when the three vectors return `Ok(())` and every negative in the vectors brief refuses with its named reason.

The acceptance test becomes a required CI job: the three independently generated §5 vectors accepted at all levels, debug and release, x86 and arm64. Performance numbers are not quoted before that job is green.

## Step 3 — primality under a grinding signer (design, with the authors)

ePrint 2026/906 (Cottaar–Galbraith–Maino–Trimoska) shows a weak primality test breaks the standard-model assumption and that random-base Miller–Rabin suffices in the ROM. Our signer chooses message and index, so it can grind for a pseudoprime; a consensus verifier cannot use random bases without risking a split. Deterministic Baillie–PSW rests on "no known pseudoprime". Put to the authors in the follow-up (Maino co-authored 2026/906): (i) is deterministic BPSW acceptable against a grinding signer, (ii) would they accept BPSW plus `k` further fixed-parameter strong Lucas rounds, or (iii) a signer-supplied certificate. No code until there is an answer or a ruling.

## Housekeeping in whichever PR touches them

- `scripts/check_c_reference.py`: record the mini-gmp divide-by-zero in the pinned ideal-to-isogeny as a quirk, with the failing seeds, so a pin move is deliberate.
- `PRISM_CI_DESIGN.md`: nightly is a gate only once PR A is green; say so.
- Delete the merged branches `fix/algorithm-3-basis` and `feat/s5-vectors`.

## Order

PR A → ruleset → Step 1 → Step 2 → floor-device run at level III on `s5_lvl3` → Step 3 as the authors reply. The §5 signer in Rust is after all of it and is its own round.
