# G7 timing, rule v2: control floors, the generic lead, fixed vs random

Rick's t57u review of `REPORT.md`, task SHE-32, branch `exp/g7-rule-v2`. The rule is `RULE-v2.md`; this report applies it to one new run. Times are America/New_York (EST).

## Verdict

- **The generic admission/Hermitian lead did not reproduce. It is not a candidate under RULE-v2, so no 4× re-run was launched.** Across 3 valid processes and 5 fixtures, no identity contrast flagged in admission, Hermitian or the full path (0 of 45 under the rule). Medians differ by 0 ns in admission and Hermitian, by at most ±148 ns in the full path (3.1 ticks at 18.5 ns; its sign changes between processes), and |t| ≤ 2.5 throughout. The earlier shift of at most 0.44 µs would have been detected: the control floor here is about 0.22 µs. The earlier lead was seen on a noisy node and was absent on the quiet node; this quiet-node run agrees with the quiet one.
- **Null arm clean in all 3 processes** (RTLF α = .01 and Welch t, all three phases). No run is void.
- **Fixed vs random key: no flag** in any phase or process.
- **Maximum r (RULE-v2 §6):** adm 0.25 µs, herm 0.25 µs, full 2.0 µs. These are upper bounds, because the lowest injected level was already detected every time.
- Timing remains supporting evidence; the structural checks stay primary.

## Run

| item | value |
|---|---|
| host | Raspberry Pi 4 (skl-pi4-1), Cortex-A72, timer tick 18.5 ns, timed work pinned to core 3 |
| window | 14:30:48 to 14:55:28 EST, 2026-11-27, seeds 101, 102, 103, 8192 rounds each (about 8 min per process) |
| binary | `stage2c-v2` sha256 c168dec7…74a (build from `harness-v2.patch` on dev 470d537; patch sha d1961b0a…c70; rustc 1.97.1) |
| key pool | 32,768 keys, sha256 e929733e…8f1f (`genpool.py`, seed 20261127; all checked to admit and reduce) |
| analysis | R 4.5.3 reference RTLF (tls-attacker/RTLF 2497d660, rtlf.R sha 50e5d4e5…bd04e), α = .01 and .09, plus Welch and paired t; a flag needs α = .01 and a flagged-decile difference of at least 3 ticks (55.5 ns) or |t| > 4.5 |

Cells are (arm, phase), shuffled every round, and every cell passes through the same code path: write the datum into one persistent boxed slot, one untimed warm run on a clone, then the timed run on a fresh clone. Each process permutes the fixture order (the first fixture is that process's reference) and uses a seeded layout pad. Arms: five generic sign fixtures f0 to f4, `null` (reference copy), `injL/M/H` (reference copy plus an added delay), `kfix` (first corpus key), `krand` (random key from the pool, copied untimed).

**Node state (read-only, recorded at start, after each seed and at the end).** `shekyld@testnet` on the Pi was **active and unchanged** at every checkpoint: the same main pid 755540, running since 12:52:51 EST, no restarts, about 0.0 % CPU (it showed as scheduled on core 3 with idle load). I did not stop, start or modify it. The 1 Hz monitor showed load1 1.0 to 2.6 (mostly my own run), CPU frequency 1.4 to 1.8 GHz (governor changes), SoC 52 to 61 °C, no undervoltage alarm, no steal. This is a quiet node, not a dedicated one: the daemon was running but idle, and the clock moved between two frequencies. The clean null arms are what validate the run, not the monitor.

## Flaws to know about

1. **Seeds 102 and 103 gave identical permutations and pads** (perm 1-4-2-3-0, pad 5190, reference f1), because the generator is seeded with `seed|1` and adjacent seeds collide. The run has two distinct layouts, not three: s101 (perm 3-1-2-4-0, pad 6668, reference f3) and s102/s103. The three processes are still fresh processes with separate data, null arms and controls, but the permutation independence Rick asked for is only 2-way. A re-run should use seeds that differ after `|1`, for example 201, 203, 205.
2. **First launch discarded.** The first injected-delay implementation spun until the whole phase had run for the delay, which adds nothing to phases longer than the delay. I fixed it to use an independent `Instant`, checked on the box that the shift tracks the delay, killed the first Pi run, kept it as `_discarded/runV2_core3_buggy_inject` on the Pi, and ran the fixed binary. Nothing from the discarded run is used.
3. **The injection overhead.** The real median shift is the nominal delay plus roughly 100 to 120 ns (adm/herm 204–240 / 315 / 500–518 ns for 100/200/400; full 1.65–2.0 / 3.1–3.4 / 6.0–6.5 µs for 1.6/3.2/6.4). Table values below are the real shifts.
4. **Raw RTLF flags on fixtures that the rule does not count.** 5 of 90 non-control whole-run contrasts flagged in RTLF at α = .01 with a flagged-decile difference of 1 to 2 ticks (18 to 37 ns): admission f1 (s101), f0 vs f1, f0 (identity, the same fixture seen two ways) and f4 (s102), and krand vs kfix in Hermitian (s103). All are under the 3-tick floor, none has |t| above 2.5, none repeats the same sign across processes. The data are tight (admission p10 to p90 about 185 ns on 30.2 µs) with an upper tail (p99 about 49 µs), so the pair-difference sd is 34–58 µs while medians differ by 0; this is why the t-test is blind here and RTLF is not.
5. **One null chunk** flagged at α = .09 (Hermitian, s101, first chunk of 4096; none at α = .01). Expected at that α.

## Same-session injected-delay control

Real shift (median, ns) and detection. RTLF α = .01 flag requires at least 3 ticks.

| phase | nominal | real median shift | whole runs (n = 8192), RTLF / t | chunks (n = 4096), RTLF / t |
|---|---|---|---|---|
| adm | 100 ns | 222 / 223 / 240 | 3/3 / 0/3 | 6/6 / 0/6 |
| adm | 200 ns | 315 / 315 / 315 | 3/3 / 0/3 | 6/6 / 0/6 |
| adm | 400 ns | 500 / 500 / 500 | 3/3 / 1/3 | 6/6 / 3/6 |
| herm | 100 ns | 204 / 204 / 222 | 3/3 / 0/3 | 6/6 / 0/6 |
| herm | 200 ns | 315 / 315 / 315 | 3/3 / 0/3 | 6/6 / 0/6 |
| herm | 400 ns | 500 / 500 / 518 | 3/3 / 0/3 | 6/6 / 1/6 |
| full | 1.6 µs | 1649 / 1740 / 2000 | 3/3 / 0/3 | 6/6 / 0/6 |
| full | 3.2 µs | 3094 / 3342 / 3409 | 3/3 / 0/3 | 6/6 / 0/6 |
| full | 6.4 µs | 5992 / 6445 / 6499 | 3/3 / 0/3 | 6/6 / 0/6 |

Null arms: 0 flags at α = .01 and by t (whole runs 9/9, chunks 18/18 phase-cells).

- **RTLF is the instrument.** It detected every control level in every process and chunk. Its floor is below the lowest level tested in all three phases, about 0.2 µs for admission and Hermitian (0.3–0.7 % of the phase) and below 1.65 µs for the full path (below 0.07 %).
- **The t-test cannot be used for PASS.** The pair-difference sd is 34–58 µs against a 0.1–0.5 µs shift. It reached the 0.5 µs adm level in only 1 of 3 whole runs, never reached Hermitian 0.5 µs in a whole run, and never reached any full-path level up to 6.5 µs. Mean shifts of the controls are noisy (−589 to +1082 ns for adm/herm); the median and deciles carry the signal.

## The generic lead

Identity contrasts (fixture against the pooled other four fixtures, independent of which fixture is the reference), median shift in ns, RTLF flag under the rule, t:

| phase | s101 | s102 | s103 |
|---|---|---|---|
| adm | 0 to +1 ns, no flag, \|t\| ≤ 1.9 | 0 to +1 ns, no flag, \|t\| ≤ 1.7 | 0 ns, no flag, \|t\| ≤ 1.9 |
| herm | 0 ns, no flag, \|t\| ≤ 2.4 | 0 ns, no flag, \|t\| ≤ 2.0 | 0 ns, no flag, \|t\| ≤ 2.5 |
| full | −240 to +138 ns, no flag, \|t\| ≤ 0.7 | −111 to +92 ns, no flag, \|t\| ≤ 2.0 | −148 to +148 ns, no flag, \|t\| ≤ 1.3 |

Per-fixture values are in `v2/results/runV2/verdict_runV2.md`. Sign and magnitude: no common sign (for example herm f0 mean −442, −212, +186 ns; adm f4 mean −378, +19, +1026 ns), and no flagged process, so 0 of 3. A lead of the earlier size (at most 0.44 µs) is above this run's floor of 0.22 µs, so its absence is informative. Applying RULE-v2: not CANDIDATE, so nothing to re-run, nothing to explain. The lead is **not reproduced on a quiet node with the three requested counter-measures in place** (permuted fixture order, every datum copied into the same boxed slot, in-process null arm); the earlier flags on the noisy node remain unexplained by this run, and the likeliest reading is node noise, which this run cannot prove. It stays closed unless a later noisy-node run, with the v2 controls, flags again with a clean null arm.

Limits: three processes and two distinct layouts; 8192 rounds per process; one node state (idle daemon); no run on a loaded node, so this does not show that the lead cannot appear there.

## Fixed vs random secret

| phase | krand vs kfix, median shift, three processes | t | RTLF |
|---|---|---|---|
| adm | −1, 0, 0 ns | −0.6, −2.0, +1.6 | no flag |
| herm | 0, 0, 0 ns | +0.9, +0.3, +0.1 | one 2-tick raw flag (s103, 37 ns), below the 3-tick floor; no flag |
| full | −19, −75, −56 ns | −0.8, −0.5, +0.8 | no flag |

No fixed-vs-random leak at the resolution of section 6, in the generic class (random keys drawn from the 32,768-key pool; about 8192 draws per process). Guarded keys remain fixture-only.

## Application of RULE-v2

| phase | processes valid | contrasts flagged | verdict | r(phase) | r_max | PASS? |
|---|---|---|---|---|---|---|
| adm | 3/3 | 0 | PASS | ≤ 0.22 µs | 0.25 µs | yes |
| herm | 3/3 | 0 | PASS | ≤ 0.20 µs | 0.25 µs | yes |
| full | 3/3 | 0 | PASS | ≤ 1.65 µs | 2.0 µs | yes |

This is a timing PASS for the generic class and the fixed-vs-random contrast at the stated resolution, as supporting evidence. It does not replace the structural checks. Caveat: the r column is an upper bound, and the run has two layouts, not three.

## Files

- `v2/harness-v2.patch`, `v2/genpool.py`, `v2/keypool_p248.txt.xz`, `v2/pi_campaign.sh`: harness, key pool and the Pi campaign script.
- `v2/extract_v2.py`, `v2/verdict_v2.py`: contrast extraction and the RULE-v2 application. `verdict_v2.py` now matches RTLF results by file name and prints the identity table.
- `v2/results/runV2/`: raw CSVs (`raw_core3.tar.xz`), node state, progress, provenance, RTLF tables at α = .01 and .09, `manifest.json`, `verdict_runV2.md`. The 198 contrast inputs (32 MB) are regenerated from the raw CSVs with `extract_v2.py`.
