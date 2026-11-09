# shekyl-prism CI design (2026-11-08, revised 2026-11-09)

Ground: originally `dev` at `d370f79` with no `.github/`. **Revised against `dev` at `d17b520` plus PR #7:** `ci.yml` and `nightly.yml` exist and match the tables below; the first nightly (2026-11-09) was red because the pinned musl `cargo-fuzz` built the targets for musl, so fuzzing had never executed. PR #7 fixes that and makes each fuzz job prove it ran (≥1,000 executions, non-empty corpus). Status of each row is marked where it differs from the design.

Inherited from shekyl-core's CI rulings:
- one required aggregator check, `ci-pass`;
- free GitHub runners;
- coverage and mutation testing report-only until `dev` → `main`;
- CodeQL Rust on every PR;
- the red-nightly rule.

## What shapes it

- **Cost.** The whole workspace builds and runs its 45 tests in about 10 s (release) / 7 s (debug). Scoping the Rust jobs by path would cost more than it saves. Only the C-reference build is worth scoping.
- **The Pi runs different arithmetic from x86 CI.** `fp2` 0.6.0 takes `core::arch` paths on `x86_64` (`utils64.rs`, plus an `lzcnt` branch) and portable fallbacks everywhere else. The floor device is aarch64, so x86-only CI never executes the field code the gate measures.
- **Byte order is a known failure class here.** The C reference hashes its salt in native byte order (`common.c:76`). The Rust spec says little-endian throughout, and only a big-endian run proves the code agrees.
- **The verifier parses untrusted bytes.** Anything reachable from `verify` must refuse, never panic.
- **Committed test data claims a provenance.** `testdata/c_basis_hints.txt` says it came from the pinned C tree; Algorithm 3's chain KATs will say the same. CI should prove it by regenerating them.
- **PR #1 carried a `Co-authored-by: Cursor` trailer** past review. Rule 90 is checkable, so check it.

## PR gate — `ci.yml` (pull_request, push to `dev`)

| Job | What it runs | Fails when |
| --- | --- | --- |
| `lint` | `cargo fmt --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings` on the pinned 1.94 toolchain | format or lint drift |
| `test-x86_64` | `cargo test --workspace --locked` in debug (overflow checks on) and `--release` | any test, either profile |
| `test-aarch64` | the same on `ubuntu-24.04-arm` (free for public repos) | the portable `fp2` path disagrees |
| `bench-smoke` | `cargo run --release -p prism-bench`; it already exits non-zero if any sample's outcome isn't the expected refusal. Timings are printed, not judged. | the bench stops timing the path it claims to |
| `deny` | `cargo deny check` (licences, advisories, sources, bans) under rule 17 | a new crate, licence or advisory |
| `hygiene` | 1. no AI-attribution trailer in any commit in the PR range (rule 90); 2. copyright header on every `.rs`/`.py`/`.c` (rule 92); 3. every crate inherits `unsafe_code = "forbid"` from `[workspace.lints]` | rule violation |
| `c-reference-pin` | checks out the submodule and runs `scripts/check_c_reference.py` (pin plus the recorded quirks) | the pin or the spec's recorded quirks moved |
| `c-vectors` (path-scoped: `scripts/**`, `external/**`, `**/testdata/**`) | builds `PRISM_v2` at the pin with mini-gmp, reruns every export script, then `git diff --exit-code` on `testdata/` | committed vectors don't reproduce from the pinned tree |
| `codeql` | CodeQL Rust | per core ruling |
| `ci-pass` | the aggregator: needs every job, and treats `c-vectors` "skipped" as passing only when the path filter said not needed (core's pattern) | anything above |

Workflow hygiene: pin actions by commit SHA, `permissions: contents: read` by default, and checkout `submodules: false` except in the two C jobs.

Wall clock is about 4–5 min, dominated by the C build when it runs.

## Nightly — `nightly.yml` (schedule + `workflow_dispatch`), red-nightly rule applies

| Job | What it runs | Status |
| --- | --- | --- |
| `fuzz` | `cargo-fuzz` (nightly toolchain), 10 min per target, **built for `x86_64-unknown-linux-gnu`** (the pinned binary is musl and would otherwise target musl, where ASan is refused): `verify_level::<L>` for each level; public-key decode; signature decode. Corpus cached between runs. **The step fails unless the log shows ≥1,000 executions and the corpus is non-empty** (PR #7). **Open:** the `verify_level_*` corpora hold 3 files after ~2×10⁸ executions — arbitrary bytes die at `Length`/`Canonical`. Seed each corpus with the `s5_lvl*` vectors and correctly sized canonical variants so the fuzzer reaches `Basis`, `Composite`, `Kernel`. | gate (a crash is red; a run that did not execute is red) |
| `differential-prime` | 10,000 random challenge degrees per level from a committed Python script, checked against `sympy.isprime` through a small Rust driver; zero disagreements required (this session's one-off run made permanent) | gate |
| `big-endian` | `cross test --target s390x-unknown-linux-gnu` under qemu | gate |
| `c-vectors-full` | the C rebuild and diff, unconditionally | gate |
| `mutants` | `cargo mutants -p prism-verify -p prism-field` (the suite runs in seconds, so the full set is cheap); surviving mutants listed in the job summary | report-only (core ruling) |
| `coverage` | `cargo llvm-cov` | report-only |

## Required on `dev` (applied 2026-11-09, ruleset 26902731)

A GitHub ruleset on `dev` requires `ci-pass`, one approving review, and no bypass. Deletion and force-push are blocked. #4, #6 and #7 merged before the ruleset existed.

## When Algorithm 3 lands

- Its chain-level KATs join `c-vectors` (same reproduce-and-diff rule).
- The three independently generated §5 vectors (`s5_lvl{1,3,5}.txt`, from the pinned C tree at `e′`) become a **required PR job**: accepted with `Ok(())` at all three levels, debug and release, x86 and arm64, once identification lands. No performance figure is quoted before that job is green.
- The fuzz targets gain the inner `ver_isogeny_strong`.

## The floor-device gate

RULED by Rick 2026-11-08: the level III Pi 4 gate is a manual run, and its output is committed with the commit SHA. No self-hosted runner.

Proposed (not yet ruled): an operation-count metric as CI's machine-independent cost signal.
- A test-only feature in `prism-field` counts Fp2 multiplications, squarings, inversions and square roots.
- The bench prints the counts per level. They are deterministic, so CI can fail on an unexplained rise.
- The Pi run measures cycles per operation once, which turns the counts into a Pi estimate between manual runs.
- Shared-runner wall time stays printed and unjudged.