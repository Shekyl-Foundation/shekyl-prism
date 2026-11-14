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

The acceptance implementation adds a required `native-signer` job: build the
pinned C libraries, apply the cofactor patch, compile the SHAKE256 backend,
and execute deterministic signing and message-binding tests at all levels.
The salted protocol adds to that job the research/production signing
agreement test and experiment E3 of `SALT_COMPARISON.md` at projection
widths 16 and 20 (`scripts/ci/check_replay_regression.py`): the collision
replay must succeed against the archived derivation and fail against the
salted one. `lint` also proves, by compiling a consumer, that the default
verifier has no reduced-width entry point (`check_research_boundary.py`),
and lints `prism-compare` with its `experiments` feature on.
Both workspace test jobs execute the full 113-signature acceptance corpus
and malleability regressions. The accept-list job remains a provenance check.
The benchmark smoke test now requires successful complete verification.

## PR gate — `ci.yml` (pull_request, push to `dev`)

| Job | What it runs | Fails when |
| --- | --- | --- |
| `lint` | `cargo fmt --check`; `scripts/ci/check_research_boundary.py`; `scripts/ci/check_op_count_isolation.py`; `scripts/ci/check_oracle_isolation.py` (self-test, then the check: the `oracle` feature of `prism-isogeny` is named by the fuzz crate alone and is absent from the default feature graph of `prism-verify`); `cargo clippy --workspace --all-targets --locked -- -D warnings` on the pinned 1.94 toolchain, again for `prism-compare --features experiments`, and again for `prism-field` and `prism-verify` with `--features prism-field/op-count` | format or lint drift; a reduced-width verifier entry point reachable by default |
| `test-x86_64` | `cargo test --workspace --locked` in debug (overflow checks on) and `--release`; then, in release and by name, the ignored comparison of the Weil pairing with its affine reference on 1,000 full-order pairs a level | any test, either profile; the pairing differs from its reference in a value or a refusal |
| `test-aarch64` | the same on `ubuntu-24.04-arm` (free for public repos) | the portable `fp2` path disagrees |
| `op-counts` (x86_64 and aarch64) | the counter's own test, then `scripts/op_counts.sh --check` twice: the field operations of every honest vector and the verdict and cost of every negative must equal `docs/benchmarks/op-counts.txt` and `negative-verdicts.tsv` byte for byte | a verification costs a different number of field operations, a negative is refused differently, or the count depends on the machine or the run |
| `bench-smoke` | `cargo run --release -p prism-bench`; it already exits non-zero if any sample's outcome isn't the expected refusal. Timings are printed, not judged. | the bench stops timing the path it claims to |
| `deny` | `cargo deny check` (licences, advisories, sources, bans) under rule 17 | a new crate, licence or advisory |
| `hygiene` | 1. no AI-attribution trailer in any commit in the PR range (rule 90); 2. copyright header on every `.rs`/`.py`/`.c` (rule 92); 3. every crate inherits `unsafe_code = "forbid"` from `[workspace.lints]` | rule violation |
| `c-reference-pin` | checks out the submodule and runs `scripts/check_c_reference.py` (pin plus the recorded quirks) | the pin or the spec's recorded quirks moved |
| `c-vectors` (path-scoped: export/reference/testdata, field/isogeny/verifier crates, dependency/compiler/CI inputs) | builds `PRISM_v2` at the pin with mini-gmp, reruns every export script, then `git diff --exit-code` on `testdata/` | committed vectors don't reproduce from the pinned tree |
| `codeql` | CodeQL Rust | per core ruling |
| `ci-pass` | the aggregator: needs every job, and treats `c-vectors` "skipped" as passing only when the path filter said not needed (core's pattern) | anything above |

Workflow hygiene: pin actions by commit SHA, `permissions: contents: read` by default, and checkout `submodules: false` except in the two C jobs.

Wall clock is about 4–5 min, dominated by the C build when it runs.

## Nightly — `nightly.yml` (schedule + `workflow_dispatch`), red-nightly rule applies

| Job | What it runs | Status |
| --- | --- | --- |
| `fuzz` | `cargo-fuzz` (nightly toolchain), 10 min per target, **built for `x86_64-unknown-linux-gnu`** (the pinned binary is musl and would otherwise target musl, where ASan is refused): `verify_level::<L>` for each level; public-key decode; signature decode; `field_square` (`square` against `mul`); `weil_matches_affine` (the Weil pairing against the affine loop it replaced, value and refusal). Corpus cached between runs. **The step fails unless the log shows ≥1,000 executions and the corpus is non-empty** (PR #7). **Implemented:** `scripts/seed_fuzz_corpus.py` seeds every target before fuzzing, the verify and decode targets and `field_square` from the 113 honest signatures and `weil_matches_affine` with its own pairs; verify seeds include the length framing required by each target. | gate (a crash is red; a run that did not execute is red) |
| `differential-prime` | 10,000 random challenge degrees per level from a committed Python script, checked against `sympy.isprime` through a small Rust driver; zero disagreements required (this session's one-off run made permanent). The same job runs `scripts/theta_base_search.py`, which recomputes for every `zeta` the one case in which the theta base-change search fails | gate |
| `big-endian` | `cross test --target s390x-unknown-linux-gnu` under qemu | gate |
| `c-vectors-full` | the C rebuild and diff, unconditionally | gate |
| `mutants` | `cargo mutants -p prism-verify -p prism-field --shard k/8 --sharding round-robin`, eight jobs. Every mutant of a crate runs that crate's whole suite, about half a minute for `prism-verify`, and there are about 700 mutants, which is hours in one job against a 90-minute limit. Surviving mutants listed in each job's summary | report-only (core ruling) |
| `coverage` | `cargo llvm-cov` | report-only |
| `report-red` | opens or comments on the issue `nightly RED` when any job above ended in failure **or was cancelled**, on a scheduled run or a dispatched one that asks for it. A job stopped by its own time limit is cancelled, not failed | reporter |

## Required on `dev` (applied 2026-11-09, ruleset 26902731)

A GitHub ruleset on `dev` requires a pull request and a green `ci-pass`. An approving review is not required: the repository has one maintainer, and GitHub does not accept that maintainer's approval of their own pull request. Deletion and force-push are blocked. There is no bypass. #4, #6 and #7 merged before the ruleset existed.

## When Algorithm 3 lands

- Its chain-level KATs join `c-vectors` (same reproduce-and-diff rule).
- The three independently generated §5 vectors (`s5_lvl{1,3,5}.txt`, from the pinned C tree at `e′`) become a **required PR job**: accepted with `Ok(())` at all three levels, debug and release, x86 and arm64, once identification lands. No performance figure is quoted before that job is green.
- The fuzz targets gain the inner `ver_isogeny_strong`.

## The floor-device gate

RULED by Rick 2026-11-08: the level III Pi 4 gate is a manual run, and its output is committed with the commit SHA. No self-hosted runner.

RULED by Rick 2026-11-08: an operation-count metric as CI's machine-independent cost signal.
- A test-only feature in `prism-field` counts Fp2 multiplications, squarings, inversions and square roots.
- The bench prints the counts per level. They are deterministic, so CI can fail on an unexplained rise.
- The Pi run measures cycles per operation once, which turns the counts into a Pi estimate between manual runs.
- Shared-runner wall time stays printed and unjudged.

Implemented 2026-11-13 as the `op-counts` job. The feature is `op-count` on `prism-field`; no manifest enables it, and `lint` holds that. The counts are exact, so the gate is an exact match with the committed records and a change is explained by committing the new records with the code that caused it. The records and how to read them are in [BENCHMARKING.md](BENCHMARKING.md).