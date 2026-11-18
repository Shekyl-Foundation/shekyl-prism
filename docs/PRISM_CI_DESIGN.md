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
- **Committed test data claims a provenance.** `prism-vectors/data/verify/c_basis_hints.txt` says it came from the pinned C tree; Algorithm 3's chain KATs will say the same. CI should prove it by regenerating them.
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
and malleability regressions. The `accept-verdicts` job is a provenance check: every accept vector has a
recorded model verdict. It has nothing to do with the accept list of the
tagged interface (`shekyl_prism::AcceptList`), which is a caller's policy.
The benchmark smoke test requires every timed verification to end as
pinned: accepted for the honest signatures, and the pinned refusal for
the committed responses of `prism-vectors/data/verify/identity_refusals.tsv`.

## PR gate — `ci.yml` (pull_request, push to `dev`)

| Job | What it runs | Fails when |
| --- | --- | --- |
| `lint` | `cargo fmt --check`; `scripts/ci/check_research_boundary.py` (which also holds the front crate `shekyl-prism` and the testkit `prism-testkit` to declaring no feature and to a build graph whose verifier has the research feature off); `scripts/ci/check_op_count_isolation.py` (self-test, then the check: `op-count` is declared by `prism-field` and `prism-verify` alone, the second as exactly the first; no manifest enables it; no member's default graph has it; and an isolated consumer of a default build fails to compile on `prism_field::ops`, `prism_verify::stage` and `prism_verify::Stage`, directly and through `shekyl_prism::hazmat`, and compiles with `prism-verify/op-count`); `scripts/ci/check_vectors_isolation.py` (self-test, then the check: `prism-vectors` is in no normal or build graph of `prism-field`, `prism-isogeny`, `prism-verify` or the front crate `shekyl-prism`, and its own graph over every edge kind, dev edges included, holds no workspace crate but `prism-field`); `scripts/ci/check_no_upward_edge.py` (self-test, then the check: in the order `prism-field`, `prism-vectors`, `prism-isogeny`, `prism-verify`, `prism-testkit`, `shekyl-prism`, no manifest names a workspace crate that is not earlier, in any dependency table, dev tables included, and `cargo tree` over every edge kind agrees; so no crate's tests depend on a crate above it); `scripts/ci/check_oracle_isolation.py` (self-test, then the check: the `oracle` feature of `prism-isogeny` is named by the fuzz crate alone and is absent from the default feature graph of `prism-verify`); `cargo clippy --workspace --all-targets --locked -- -D warnings` on the pinned 1.94 toolchain, again for `prism-compare --features experiments`, and again for `prism-field`, `prism-verify` and `prism-testkit` with `--features prism-verify/op-count` (the count reporter is an example of the testkit that requires that feature); `cargo doc` for `shekyl-prism`, `prism-vectors` and `prism-testkit` with rustdoc warnings denied (all three deny missing documentation; their doc tests run in the test jobs, which `cargo test --workspace` includes). Also in this job: `scripts/security_parameters.py` (the integer bounds of every parameter set, read from the field crate's source), `scripts/challenge_scalar.py --check` (an independent cSHAKE256 reproduces the `s` of all 113 vectors and the scalars of each set without vectors), `scripts/residue_known_answers.py --check` (the committed known answers for arithmetic modulo `2^e` are what Python's integers give), and `scripts/ci/check_sage_fixture.py` (the header of each committed Sage file of a set without vectors records the digest of the committed Sage script, the pinned image and the command for its set; CI has no Sage and does not run the script). | format or lint drift; a reduced-width verifier entry point reachable by default; the operation counter, or its API, in a default build |
| `test-x86_64` | `cargo test --workspace --locked` in debug (overflow checks on) and `--release`; then, in release and by name, the ignored comparison of the Weil pairing with its affine reference on 1,000 full-order pairs a level | any test, either profile; the pairing differs from its reference in a value or a refusal |
| `test-aarch64` | the same on `ubuntu-24.04-arm` (free for public repos) | the portable `fp2` path disagrees |
| `op-counts` (x86_64 and aarch64) | the counter's own test; `shekyl-prism`'s test `tag_layer_counts`, which requires the counter and shows that every refusal of the tagged entry's tag layer costs no field operation (`check_op_count_isolation.py` holds its `required-features` line); then `scripts/op_counts.sh --check` twice: the field operations of every honest vector and the verdict and cost of every negative must equal `docs/benchmarks/op-counts.txt` and `negative-verdicts.tsv` byte for byte, and the counts of every honest vector through the entry split by object must equal `op-counts-typed.txt`, where the reporter itself fails unless the split's sum plus one basis search is the byte-level total | a verification costs a different number of field operations, a negative is refused differently, or the count depends on the machine or the run |
| `bench-smoke` | `cargo run --release -p prism-bench`; it exits non-zero if any timed call, warm-ups included, does not end as pinned: an accept for an honest signature, the pinned `Codomain` or `Kernel` for a committed refusal, the committed counter for the salt search. Timings are printed, not judged. | the bench stops timing the path it claims to |
| `deny` | `cargo deny check` (licences, advisories, sources, bans) under rule 17 | a new crate, licence or advisory |
| `hygiene` | 1. no AI-attribution trailer in any commit in the PR range (rule 90); 2. copyright header on every `.rs`/`.py`/`.c` (rule 92); 3. every crate inherits `unsafe_code = "forbid"` from `[workspace.lints]` | rule violation |
| `c-reference-pin` | checks out the submodule and runs `scripts/check_c_reference.py` (pin plus the recorded quirks) | the pin or the spec's recorded quirks moved |
| `c-vectors` (path-scoped: export/reference, the vectors crate and its data, field/isogeny/verifier crates, dependency/compiler/CI inputs) | builds `PRISM_v2` at the pin with mini-gmp, reruns every export script, then `git diff --exit-code` on `testdata/` | committed vectors don't reproduce from the pinned tree |
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
| `mutants` | `cargo mutants -p prism-verify -p prism-field --test-package prism-field --test-package prism-verify --test-package prism-testkit --test-package shekyl-prism --shard k/16 --sharding round-robin`, sixteen jobs. The mutants are those of the two crates; every mutant runs the tests of the four packages, because the tests that kill most verifier mutants need the testkit's inputs and so live in `prism-testkit` and `shekyl-prism` and not in the crate they mutate. The tool sets a mutant's time limit from an unmutated baseline that runs the two mutated packages only, so `--timeout-multiplier 12` replaces the default 5. A mutant no test kills runs every test, which is minutes, and there are over 800 mutants: hours in one job against a 90-minute limit. The shard count is `MUTANT_JOBS` in `scripts/ci/mutants_report.py`; the nightly asks `plan` for the indexes and passes that count through once, to `--shard` and to the report. A dispatch with `only` set to `mutants` runs this job and `mutants-summary`; `shard` selects one index | report-only for missed mutants and timeouts (core ruling); red when the tool did not test its mutants |
| `mutants-front` | `cargo mutants -p shekyl-prism --test-package shekyl-prism --shard k/2 --sharding round-robin`, two jobs: the mutants of the front crate (the set identifier, the accept list, the tagged readers, the typed key and signature) against the front crate's own tests. The baseline runs the same tests, so the tool's default time limit stands. Cargo runs the test binaries in name order and stops after the first that fails; `acceptance` is first and walks the corpus, so a mutant costs about as much caught as missed. The tests are not renamed or filtered for that; the job has more shards instead. `tests/tag_layer_counts.rs` requires the operation counter, which no manifest enables, and does not run here. Same shard procedure as `mutants`, with `only` set to `mutants-front` | as `mutants` |
| `mutants-summary` | `scripts/ci/mutants_report.py gather` over the shards' kept output. It uses the same `verdict` as a shard. A present shard that fails it is named, left out of the figures, and fails the job. A shard that is absent is named, and the figures are then the present accepted shards' alone; absence does not, by itself, fail the job | red when it finds no shard, or a present shard `verdict` rejects |
| `coverage` | `cargo llvm-cov --workspace --summary-only`: every test of the workspace, instrumented. Limit 120 minutes: the run took 45 minutes on the build VM at `85f165a`, 21 of them in the front crate's `acceptance` walk, which verifies each input through three entries. A limit of 30 cancelled the job once the tagged entry was added | report-only for the figure; a job its limit cancels is red |
| `report-red` | opens or comments on the issue `nightly RED` when any job above ended in failure **or was cancelled**, on a scheduled run or a dispatched one that asks for it. A job stopped by its own time limit is cancelled, not failed | reporter |

### The mutation jobs: shards, what is kept, what is red

**Shard count.** The count of each job is `MUTANT_JOBS` in `scripts/ci/mutants_report.py` (sixteen for the layer crates, two for the front crate). `plan` expands a count into indexes, so the workflow does not write the list again. The count is chosen so that the slowest shard, measured, is under 60 minutes of the 90-minute limit. Runners differ in speed by nearly a factor of two (the unmutated baseline of `mutants` took 37 to 69 seconds across the shards of one run), and a shard's time follows its runner, so the count is sized for a shard that lands on the slow end. A shard measured above 70 minutes is a reason to raise the count, in the pull request that caused it. Round-robin by a different count deals the mutants differently, so a shard's lists compare with the same shard's only while the count stands; the comparison that survives a change of count is the merged list of `mutants-summary`.

**What is kept.** Every shard uploads the tool's output directory, `mutants.out`, as the artifact `mutants-<k>` or `mutants-front-<k>`, kept 30 days: the caught, missed, timeout and unviable lists, the mutants the shard set out to test (`mutants.json`), each mutant's outcome and time (`outcomes.json`), and each mutant's diff and test log. `shard.json` beside them records the job, the shard and the tool's exit code. `mutants-summary` uploads the merged missed and timeout lists of each job, and `summary.md` (the same text as the job summary, so the artifact says which shards it holds), as the artifact `missed-mutants`, kept 90 days, only when `only` was empty and every shard of both jobs was present and accepted. A one-shard dispatch and a run that lost a shard leave the previous artifact in place. Each shard's summary and log give the four counts and the whole missed and timeout lists.

**What is red.** `scripts/ci/mutants_report.py verdict` reads the exit code of cargo-mutants 27.1.0 (its `book/src/exit-codes.md`). Codes 0 (every mutant caught), 2 (mutants missed) and 3 (tests timed out) do not fail the shard. Every other code does: 1 is a usage error, 4 is an unmutated baseline that fails or hangs, 70 is an internal error, and a code not in that list is not a pass. A shard also fails when the tool left no lists, when it held no mutant, when its index is outside its dealing, and when its lists do not add up to the mutants it set out to test, which is what a run cut short leaves. `gather` applies that same verdict. A present shard that fails it fails the summary and is not part of the figures. Missed mutants and timeouts are report-only until the first `dev` → `main` cut; whether a timeout becomes red before then is not decided.

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

Implemented 2026-11-13 as the `op-counts` job. The feature is `op-count` on `prism-verify`, which turns on `op-count` on `prism-field` (since 2026-11-16; before, the field crate's alone, with the counter's API public in every build). It is passed as `--features prism-verify/op-count`; those two manifests declare it, no manifest enables it, the counter's module and the stage names exist only with it, and `lint` holds all of that. The counts are exact, so the gate is an exact match with the committed records and a change is explained by committing the new records with the code that caused it. The records and how to read them are in [BENCHMARKING.md](BENCHMARKING.md).