# P248 ARM timing signal (Rick's t47u, item 3)

Branch `experiment/p248-arm-timing` off `research`. No merge to `dev`, no PR. Measurements 2026-11-26 11:22 to 12:50 America/New_York (UTC-5). **No constant-time claim.** G7 stays open.

## Question

`docs/benchmarks/stage2c/README.md` records a physical Raspberry Pi 4 run at P248 where the guarded contrast of fixture 1 against the class reference had mean difference -5,130 ns and paired t = -6.77 (fixtures 3 and 4: t = +5.11 and -4.87). The source was left open under G7. This work repeats the contrasts on physical hardware, splits admission from reduction, and asks where the signal lives.

## Answer in one paragraph

The -6.77 does not reproduce. Across 55 further process runs of the G7 contrast (seven long runs plus 48 short ones, two builds, two cores, ASLR on and off) the largest guarded P248 |t| was 4.53 and fixture 1's pooled median paired difference is +92 ns (CI of the process means includes zero). Splitting the interval, admission shows no fixture dependence beyond the timer tick in the pooled data (per-fixture medians within 1 to 2 ticks = 18.5 ns each in run A1; process-dependent offsets up to about +-0.4 µs, sign varying, on a 110 µs phase) and reduction shows no fixture effect (median differences within +-0.5 µs on 2.35 ms, ANOVA p = 0.4 to 0.95). The same-data null contrast (a second copy of the reference row) shows the same size of excursions as the real fixtures. What does exist is host-level noise of the size of the original effect: single 512-repeat contrasts have a mean difference of 5 µs or more in 37 of 477 cases (7.8%), in both directions, for every fixture and both classes, and the G7 t statistic is heavy-tailed (|t| > 3 in 1.9% of contrasts against 0.3% for a Gaussian). The original run's implied pair-difference sd (17.1 µs) is about half of the smallest sd observed here (27 µs), so the same few-µs excursion would give a much larger t on a quieter board. Verdict: noise on a shared board, amplified by a statistic that assumes independent pairs; nothing attributable to admission or to reduction. This is a negative result with a stated sensitivity (about 0.4 µs in admission and about 1 µs on the full path), not a proof of constant time.

## Hardware and host state

Raspberry Pi 4 Model B Rev 1.4, 8 GB, Cortex-A72 x4, kernel 7.0.0-1023-raspi (Ubuntu), aarch64, reached as `skl-pi4-1`. Timer: the architectural counter, so timing ticks are about 18.5 ns.

| Item | State |
|---|---|
| Governor | `ondemand` on all cores, not changed (no sudo was used). Core frequency 1.8 GHz (max) in 99.6% of the 1 Hz samples during the runs; the rest are the idle gaps between processes. |
| Temperature | 52.6 to 63.8 °C over all runs (`/sys/class/hwmon/hwmon0`). |
| Throttling | `vcgencmd` unusable (no `/dev/vcio`). `rpi_volt` undervoltage alarm 0 in every sample and snapshot. Frequency never dropped below max while a benchmark ran. |
| Load | The board is a live Shekyl node (`shekyld` about 14% CPU, wallet RPC, tor) with 2 to 4 logged-in users. Load average 0.75 to 3.4 (the 3.4 is my own build just before run A). |
| Pinned core | `taskset -c 3` (as in the published run), core 2 for run C. Other tasks used 1.5 to 3.2 jiffies/s (of 100) on the pinned core while the benchmark ran, with bursts of 10 or more jiffies in about 4% of the seconds (max 69). |
| Toolchain | Rust 1.97.1 in a separate user-space rustup under `~/shekyl-timing` (the host's `~/.rustup` untouched). Native release build, `--locked`. |

Snapshots before and after each run: `results/state/`. Per-second monitor summaries: `results/monitor_summary.txt`.

## Method

* **G7 definition.** `docs/benchmarks/stage2c/README.md` and `shekyl-prism/examples/stage2c/driver.rs` on `origin/dev`: `stage2c timing REPEATS`, each later row of a class contrasted with the class's first row, 512 repeats, even repeats reference-first, interval = admission + reduction + drop through `time_reduction`, t = paired mean * sqrt(n) / sample sd. "Guarded" is the `prered-guard256` class (6 fixtures). The published ARM run was a cross-build at `a5a6f60`.
* **Builds.** (1) `origin/dev` `470d537` plus `harness.patch` (this branch): adds the `STAGE2C_ONLY=p248` filter, a `Bounds::prime()` accessor and a `split` mode; the `timing` mode is untouched. (2) `a5a6f60` (commit "signer: Contrast each Stage 2c fixture and scan every reduction"), unmodified, built natively on the Pi. It runs all five sets; only P248 is analysed. The CI cross-built binary was not available, so a layout difference from the published binary remains possible.
* **Split mode (new).** Each round times every (fixture, phase) cell once in a fresh random order (xorshift shuffle, seeded). Phases: `adm` (`Basis::from_intersection` and its drop), `red` (`reduce` and drop, on a basis admitted before the clock), `full` (the G7 `time_reduction`), `inv` (`geometry::inverse_odd` on the norm), `herm` (the three guarded Hermitian products admission divides by the norm), `nop` (empty interval). Fixture index 6 is a second copy of the reference row, so every phase carries its own **null contrast** (same data as the reference). Inputs are cloned before the clock, as in `time_reduction`.
* **Statistics** (`analyze.py`). Student t of paired differences as in G7, plus: effect size d_z, t on 1%/5%-winsorized data, a *blocked* t (differences averaged over 20 blocks of consecutive rounds, which is honest under drift and bursts), t of each half, median difference and sign test, and a repeated-measures ANOVA and Friedman test across the six real fixtures with rounds as blocks. The winsorized mean and blocked t are the ones to read here; the plain t is inflated by bursts.
* **Per-process runs** (`run_procs.sh`, `procs.py`). 48 processes of `timing 512` and 24 of `split 400`, alternating ASLR on and `setarch -R` (off), to separate process-level layout offsets from within-process noise.

## Runs

| Run | Binary | Core | Content |
|---|---|---|---|
| A | dev+patch | 3 | `timing 512` x3, `timing 4096`, `split` 10,000 rounds x2 (seeds 1, 2) |
| B | a5a6f60 | 3 | `timing 512` x2 (all sets; P248 analysed) |
| C | dev+patch | 2 | `timing 512`, `split` 5,000 rounds |
| D | dev+patch | 3 | 24 x `timing 512` ASLR on, 24 off; 12 x `split 400` on, 12 off |

## Results

### Reproducing -6.77 (G7 mode, guarded class, P248)

| Run | Build | Core | Repeats | fixture 1 | fixture 2 | fixture 3 | fixture 4 | fixture 5 | max abs t |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| A1 | dev 470d537+patch | 3 | 512 | +0.37 (+543 ns) | -0.17 (-297 ns) | +0.59 (+876 ns) | +0.08 (+102 ns) | -0.03 (-55 ns) | 0.59 |
| A2 | dev 470d537+patch | 3 | 512 | -1.77 (-2724 ns) | -0.22 (-345 ns) | +0.02 (+32 ns) | +1.14 (+1879 ns) | -1.71 (-2532 ns) | 1.77 |
| A3 | dev 470d537+patch | 3 | 512 | +1.43 (+1996 ns) | +3.02 (+5006 ns) | +2.38 (+4090 ns) | +3.73 (+5548 ns) | +0.38 (+613 ns) | 3.73 |
| A4 | dev 470d537+patch | 3 | 4096 | -1.94 (-986 ns) | -0.79 (-426 ns) | +0.67 (+330 ns) | +0.89 (+513 ns) | +0.15 (+81 ns) | 1.94 |
| B1 | a5a6f60 unmodified | 3 | 512 | -0.46 (-623 ns) | +2.37 (+3147 ns) | -0.33 (-442 ns) | +2.17 (+2841 ns) | +0.62 (+839 ns) | 2.37 |
| B2 | a5a6f60 unmodified | 3 | 512 | +0.78 (+1117 ns) | -1.21 (-1710 ns) | +0.60 (+878 ns) | -0.50 (-638 ns) | -0.39 (-538 ns) | 1.21 |
| C1 | dev 470d537+patch | 2 | 512 | +0.35 (+478 ns) | -0.99 (-1413 ns) | +0.00 (+78 ns) | -0.97 (-63980 ns) | +1.18 (+1720 ns) | 1.18 |

Published: fixture 1 t = -6.77 (-5,130 ns), fixtures 3 and 4 t = +5.11 and -4.87. Largest here: +3.73 (run A3, fixture 4, +5,548 ns, positive and on a different fixture). Fixture 1 across the seven rows: +0.37, -1.77, +1.43, -1.94, -0.46, +0.78, +0.35 (sign flips; mean near zero). Generic class (not tabled): max |t| 2.46 in runs A to C and 4.53 in run D.

**Forty-eight short processes (run D).** Per-contrast t over 432 contrasts (24+24 processes, 5 guarded and 4 generic fixtures): |t| > 2 in 5.8%, > 3 in 1.9%, > 4 in 0.46%, max 4.53 (Gaussian expectation: 4.6%, 0.27%, 0.006%). ASLR on is noisier than ASLR off (|t| > 3: 4.2% / 2.1% against 0.8% / 0%), which points at layout-dependent noise, not a fixture effect. Per-contrast means of 5 µs or more occur in 37 of 477 contrasts (D plus A and C), with both signs. Ten contrasts have both |mean| >= 3 µs and |t| >= 3; they are spread over fixtures 1, 2, 4 and 5 and both classes, with both signs. The pair-difference sd on this board is 27 µs (min), 31.6 µs (median), 50.6 µs (p90). The published run's implied sd is 17.1 µs (5130 ns * sqrt(512) / 6.77).

Pooled over 55 processes (31,744 pairs per contrast) for fixture 1 against the reference: median paired difference +92 ns; mean of 5%-trimmed process means -2,482 ns with bootstrap 95% CI [-7,324, +348] ns. The interval is wide because bursts dominate the mean; the median is the robust figure and it is zero to within 5 ticks. (`results/summary/pooled_orig.txt`.)

### Admission against reduction (split mode)

Columns: winsorized-5% mean diff (ns) / blocked t (20 blocks) / median diff (ns); the last column is the ANOVA p across the six real fixtures (rounds as blocks).

**A1+A2 (core 3, 20,000 rounds)**

| Phase | fixture 1 | fixture 2 | fixture 3 | fixture 4 | fixture 5 | NULL (reference copy) | ANOVA p |
|---|---|---|---|---|---|---|---:|
| adm | -147 / -0.02 / +0 | -261 / -0.96 / +18 | -140 / +0.40 / +19 | -169 / +0.84 / +111 | -314 / +0.20 / -37 | -282 / -0.40 / -19 | 0.46 |
| red | +319 / -0.37 / +120 | +297 / +1.27 / +18 | +30 / -0.25 / -166 | +72 / -0.02 / -167 | +393 / -0.65 / +176 | +242 / +0.76 / +130 | 0.70 |
| full | +339 / +1.24 / +166 | +390 / +0.52 / +168 | +328 / +2.39 / -9 | -115 / +1.32 / -9 | +346 / +0.02 / +260 | -335 / +0.67 / -83 | 0.38 |
| inv | +26 / -0.53 / +0 | +23 / +1.06 / +0 | +17 / +1.09 / +0 | +28 / +0.91 / +0 | +16 / +0.12 / +0 | +12 / +0.93 / +0 | 0.63 |
| herm | +16 / -0.06 / +0 | +20 / -0.49 / +1 | +42 / +0.39 / +18 | +75 / -0.91 / +18 | +45 / -0.06 / +18 | +54 / -1.14 / +0 | 0.91 |

**C3 (core 2, 5,000 rounds)**

| Phase | fixture 1 | fixture 2 | fixture 3 | fixture 4 | fixture 5 | NULL (reference copy) | ANOVA p |
|---|---|---|---|---|---|---|---:|
| adm | +22 / +0.76 / -38 | -230 / +0.40 / -75 | -130 / -1.20 / -55 | -188 / -0.09 / -55 | -132 / -1.21 / -38 | -435 / -2.02 / -55 | 0.38 |
| red | -153 / -0.86 / -36 | +25 / -0.52 / +426 | -489 / -1.81 / +120 | -981 / -1.14 / -148 | -1260 / -0.27 / -148 | +183 / -0.18 / -9 | 0.95 |
| full | +440 / +0.86 / -74 | +1159 / +1.01 / -0 | +494 / +0.80 / -278 | +20 / +0.25 / +37 | +276 / +0.98 / -84 | +776 / +1.59 / +186 | 0.86 |
| inv | -34 / +0.86 / +0 | -45 / -2.54 / +0 | +6 / -0.27 / +0 | -2 / -0.26 / +0 | -8 / -0.89 / +0 | +12 / -0.58 / +0 | 0.47 |
| herm | -10 / -0.98 / +0 | +24 / -0.99 / +19 | +119 / -0.61 / +0 | +121 / -0.79 / +0 | +71 / -0.85 / -19 | -72 / +0.41 / +0 | 0.55 |

* **Admission (`adm`)** at about 110 µs on the guarded class: medians of every fixture equal the reference's to within one or two ticks in run A1; the null (same-data) contrast shows -282 ns winsorized mean in A12 and -435 ns in C3, the same size as the real fixtures (-130 to -314 ns). No blocked t beyond +-2.1 and none unlike its null. `inv` (the Hensel inverse) has median difference exactly 0 and `herm` +-18 ns for every fixture.
* **Reduction (`red`)** at about 2.35 ms: median differences within +-0.45 µs, winsorized means within +-1.3 µs, ANOVA p = 0.70 and 0.95, no blocked t beyond +-1.8.
* **Full path (`full`)**: winsorized means within +-1.2 µs, ANOVA p = 0.38 and 0.86, one blocked t of +2.39 (A12 fixture 3) against a null of +0.67 that does not recur in run C (+0.80).
* **Process-level offsets.** In run A2 the *generic* class admission of fixtures 1 to 4 was 426 to 463 ns faster than the reference (median; null 0), but not in A1 (+-36 ns). In the 24 short split processes the per-process medians of admission differences have sd 43 to 153 ns with extremes of +-0.4 µs and means within +-75 ns of zero, so offsets of a few hundred ns exist in single processes and average out. They are at least 10 times too small to produce a 5 µs shift on a 2.45 ms path, and I cannot say whether a given one is layout or data; nothing here shows a data dependence.
* Sensitivity: null contrasts reach +-0.43 µs (adm), +-0.24 µs (red), +-0.78 µs (full) in winsorized mean over 5,000 to 20,000 rounds. Effects below roughly 0.4 µs in admission and 1 µs on the full path cannot be separated from noise on this board. The reported -5.1 µs is far above that.

### a5a6f60 against current dev

Two native builds of `a5a6f60` (B1, B2): max |t| 2.37 and 1.21; fixture 1 t = -0.46 and +0.78. Current dev with the patch gives the same spread (A1 to A4, C1, D). `a5a6f60` is the commit the published ARM run was cross-built at ("Contrast each Stage 2c fixture and scan every reduction"); dev has since added the guarded ideal admission (`feat/native-ideal-admission`, #50) and the secret-ideal path, which the guarded contrast does not exercise beyond `from_intersection`. No difference between the two builds was visible. This does not prove the published binary behaved the same, since that was a cross-build.

### Residual stack words on aarch64 (physical, dev `470d537` + patch)

The repository's own scan (`scripts/ci/stage2c_stack.py`'s probe build, paint window, canary gate and gdb scan, imported unchanged) was run on the Pi with user-space gdb 18.1 (micromamba under `~/shekyl-timing`); Valgrind is not available there, so the Callgrind half was skipped (`pi/stack_only.py`). Canary gate 8/8 at all five sets. Full log: `results/stack_aarch64.txt`.

| Set | sign-generic depth | prered-guard256 depth | guarded residual words (fixture 1) | CI `ubuntu-24.04-arm` (a5a6f60) |
|---|---:|---:|---|---|
| P248 | 12,584 | 13,856 | 12/53 | 12/53, depth 13,872 |
| P324 | 15,740 | 17,640 | 14/67 | 14/67, 17,656 |
| P376 | 17,600 | 19,608 | 12/77 | 12/77, 19,608 |
| P500 | 22,328 | 25,144 | 16/105 | 16/105, 25,208 |
| P664 | 28,432 | 32,016 | 21/129 | 21/129, 32,112 |

* Every fixture of every set leaves residual input words (generic: 22 to 63 words of 30 to 86). The counts match the CI ARM log exactly; depths differ by at most 96 bytes. The G7 residual-word item is therefore reproduced on physical hardware and stays open: there is no erasure here.
* Depth is constant across the fixtures of a class and the residual count differs by at most a few words, so stack footprint cannot explain a fixture-specific timing difference.
* ISA matters for what is left: at P248 the guarded class leaves 12 of 53 words on aarch64 against 47 of 53 on x86-64 (same source, dev tip).
* **Budget note for ruling 12.** The deepest write is 32,016 bytes at P664 guarded, which is 752 bytes under the 32 KiB (32,768 B) stack budget. That figure covers only the frames below the probe marker, not callers or the signer around the reducer, so the margin is not a statement about a whole signer.

## Where the signal lives

| Candidate | Evidence | Verdict |
|---|---|---|
| Admission | Medians equal to 1 to 2 ticks; null shows the same offsets; `inv` and `herm` exactly flat; offsets at most +-0.4 µs and process-dependent | Not the source of a 5 µs shift |
| Reduction | Medians within +-0.45 µs; ANOVA p 0.7 and 0.95; no run reproduces any fixture effect | Not the source |
| Noise and thermal | Bursts on the pinned core, a live node on the board, 7.8% of 512-repeat contrasts with |mean| >= 5 µs in both signs on every fixture and both classes; frequency was at maximum and no undervoltage flag, temperature at most 63.8 °C | The best explanation of the original t, together with a statistic whose t is heavy-tailed here |
| Layout / ASLR | ASLR-on runs have more large |t| than ASLR-off (4.2% / 2.1% against 0.8% / 0% above 3) | Contributes noise; not shown to be fixture-specific |
| Frequency governor | `ondemand` left on; frequency at maximum in 99.6% of samples | Not a factor in these runs; the published run's governor state is unknown |

I cannot show that the published -6.77 was noise, only that on the same class of board it is not reproducible under 55 process runs and that single-contrast excursions of its size are routine. A quieter or dedicated board could still reveal a smaller real effect.

## Limits

* One board, shared with a live node, `ondemand` governor, no isolated cores; the host state is recorded but not controlled.
* The Pi build is native; the published binary was cross-built at `a5a6f60`.
* Six guarded fixtures; a data dependence below the sensitivity above would not be seen. Public fixtures only.
* Wall-clock timing with a 18.5 ns tick: the full path is 2.4 ms, so a few-hundred-ns effect is a 0.01 percent effect.
* The Callgrind half of the stack scan (integer-divide check) was not run on the Pi.

## Reproduce

```text
# Pi, from ~/shekyl-timing (see pi/setup_build.sh, pi/run_suite.sh, pi/run_procs.sh)
taskset -c 3 env STAGE2C_ONLY=p248 ./stage2c-dev-patched timing 512
taskset -c 3 env STAGE2C_ONLY=p248 ./stage2c-dev-patched split 10000 1
python3 analyze.py orig FILE...      # needs only python3
python3 analyze.py split FILE...     # needs numpy and scipy
python3 procs.py DIR                 # runD
python3 pooled.py FILE...
python3 report_tables.py
```

Binary hashes (sha256): dev+patch `27c1a0b6a0b8822d123bdc381d32074c1439e66b615015d2325cae08672bff69`, a5a6f60 `85f5a79ebe5666778836186e4c6bdbd6da87d757ef8ad574843a92b0fccd79ec`. Raw split CSVs (about 28 MB each) are on the Pi and the box, not committed; archives: runs A to C `3f113ba31aa211c018d12f6e16e65c7e3acff9427f295bfdec64f401b57da120`, run D `f39f498716ca60c6723ad0b0776d228c5a059032df75bda7a5bd95aa83ad706f`. The raw G7-mode CSVs (all 55 processes) are in `results/data/orig_mode_raw.tar.xz`.

## Later: node stopped

A follow-up on 2026-11-26 stopped `shekyld@testnet` for the same contrasts on this board. Host contention fell, the pair-difference sd did not, and fixture 1 stayed in the earlier t range. See [quiet/REPORT.md](quiet/REPORT.md).

## Files

`harness.patch`, `analyze.py`, `procs.py`, `pooled.py`, `report_tables.py`, `pi/` (setup, snapshot, monitor, suite runners, stack scan), `results/summary/` (all summary JSON), `results/state/`, `results/monitor_summary.txt`, `results/stack_aarch64.txt`, `results/data/`.
