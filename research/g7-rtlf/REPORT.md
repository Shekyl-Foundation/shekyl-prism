# G7 timing: RTLF next to Welch t, with a matched preamble

Branch `exp/g7-rtlf` off `research`. Board: skl-pi4-1 (Cortex-A72), core 3, dev 470d537 plus harness patches. Times are America/New_York (UTC-5).
Follows literature-sweep-2026-11 §5 and recommendation 3 (Anne, ePrint 2026/2744, matched control; RTLF, Dunsche et al., USENIX Security 2024).

## Result in six lines

1. **RTLF on the P248 data agrees with the t-test that there is no full-path leak.** Across 585 original-harness G7 contrasts, 0 exceeded |Welch t| 4.5 and 2 exceeded paired |t| 4.5 (4.53 and 4.52, both in the 48-process run, both also flagged by RTLF). RTLF (α = 0.09) flagged 16 (2.7%), the same rate as it flags A/A splits of the reference (15 of 585, 2.6%). Real fixtures and A/A look alike, so these contrasts carry no fixture signal.
2. **The published −6.77 is not reproduced by either test**, with the original or the matched harness. The matched preamble changed nothing visible: the original harness showed no phantom difference on this board to remove.
3. **Detection floor** (n = 512, the G7 repeat count, guarded class, 90% of chunks detected, injected constant delay on the null arm). Full path (2.45 ms): RTLF 3.2 to 6.4 µs (0.13 to 0.26%); t > 6.4 µs (never reached 90%). Admission phase (about 110 µs): RTLF 0.2 µs; t > 1.6 µs. At n = 10,000 rounds RTLF sees 0.4 to 0.8 µs on the full path.
4. **The bigger lever is phase splitting and independent processes, not the test.** Two separate processes of the same dev binary differ by 9 to 25 µs (Welch t up to −12.7, RTLF flags 6 to 9 deciles). Every comparison across process launches is invalid with either test. A within-process shift (runG 06, generic class, t +3.5 to +6.6, RTLF flagged) did not reproduce in two further processes (runH, |t| ≤ 3.9).
5. **RTLF finds small fixture-dependent shifts the t-test cannot see**, in the admission and Hermitian phases of the generic class on the noisy node: flagged in both of two independent processes (α = 0.01; one case at α = 0.09) for fixtures 1 to 4 against the reference, 0 of 10 null arms flagged. Mean shift 0.4 µs at most, about 1.4% of the phase and 0.02% of the full path. They did not appear on the quiet node. This is an open lead, not a finding.
6. **Proposed gate**: matched preamble, in-process null arm, at least 3 fresh processes, RTLF at α = 0.01 and Welch t together, verdict by replication. Details in the last section.

## Method

### RTLF
Reference implementation: tls-attacker/RTLF commit `2497d660e4b23fc9acd21b077a35e82178fcd728` (2025-03-21; no release tags), `rtlf.R` sha256 `50e5d4e5…bd04e` (see `pin.txt`). R 4.5.3, tidyverse 2.0.0, optparse 1.8.2, jsonlite 2.0.0, crayon 1.5.3 from conda-forge.

`rtlf_batch.R` sources the reference's `test`, `bootstrap1` and `autotest` verbatim and calls them as `rtlf.R` does: n = min class size, B = 10,000, α per decile = α/9, deciles 10 to 90% (R quantile type 2). Only the CLI and CSV reader are bypassed. Each input has its own seed. Checked on the repo's two bundled examples: example-2 matches (no detection), example-1 matches (deciles 30 to 60 flagged, thresholds equal within bootstrap noise).

The reference is slow (seconds per n = 512 input, 43 s per n = 10,000 input on one core under load). All 2,711 re-analysed contrasts of the earlier runs ran through it at α = 0.09 (`results/rtlf/R_reference_main_a09.tsv.gz`). The remaining bulk work (positive control, new runs, α = 0.01) used `rtlf_np.py`, a vectorised port of the same procedure with numpy's random stream. Validation against the reference on 527 shared files: same verdict on 524 (99.4%); the three disagreements are borderline single-decile cases (`results/r_vs_numpy.txt`); median per-decile threshold difference 0.7%. Rows from the port are labelled in the file names (`np_*`); R takes priority wherever both exist.

### The t-test
Unchanged from G7: paired t of (other − reference), t = mean·√n / sd. Added the Welch (unpaired) t because dudect and RTLF treat the two classes as independent samples, and |t| > 4.5 as the line. The paired and Welch statistics agree in sign and flag the same contrasts in this data except for heavy-tail cases, where the Welch value is smaller.

### Matched preamble (`harness-matched.patch`, mode `matched REPEATS`)
Both arms of every pair now run exactly: clone inputs, run one untimed admission and reduction on that clone, drop it, clone again, start the clock, run, drop. Allocation sizes, copies and warm-up are identical for the reference and for every other fixture, in either order. The original harness times a cold clone on a path where the reference is always the same fixture. The mode also adds a **null arm** (index = class size, the reference timed again against itself) and an injection hook (`STAGE2C_INJECT_NS`, `STAGE2C_INJECT_INDEX`: a spin-wait inside the timed interval of one arm) for the positive control. Cumulative with the earlier split patch; applies on dev 470d537. Binary sha256 `0625a5f6a8301e55cbd5baa97888a8a31a3ca79ebafc9e34e5fe1cf019d03c3c`; patch sha256 `8032a38a05b492d9e39dff3a546ef01f8d4d834cb28feb07da6c7d2e78f32b04`.

### Pi campaign (`pi/campaign.sh`, `pi/followup.sh`, all under ~/shekyl-timing, no sudo)
- runG (01:24 to 01:40): orig 512 ×2, matched 512 ×2, matched 4096, injected delays 250 to 8000 ns on the guarded null arm (n = 512 each).
- runH (01:42 to 02:00): matched 4096, orig 4096, matched 4096, three separate processes, repeated because runG 06 showed a shift.
- **State of the node: `shekyld@testnet` was already inactive when this task began.** `systemctl show` reports a clean stop (Result success, exit 0) at 00:25:42 on Nov 27, before the 01:15 brief. `state_before.txt` of runG shows 0 shekyld processes. I did not stop it and did not use sudo, and did not start it either (that needs sudo). So runG and runH are node-down runs, not "node running". A non-Shekyl process (`relay_admission`, about 100% of one core, on another core) ran throughout. Someone with sudo should check whether the stop at 00:25 was intended.

### Re-analysis inputs
2,711 contrasts from the earlier noisy and quiet runs (A to F), the 48-process run D, plus runG and runH: original-mode contrasts (reference vs fixture), their A/A form (the reference samples split at random in two halves, seeded), and per-phase split-mode contrasts (adm, red, full, inv, herm; fixture vs fixture 0; the null copy tagged). One RTLF input per contrast. Full per-contrast table: `results/tables.md`; machine-readable: `results/verdicts.json.gz`.

## Results

### Real contrasts and nulls, side by side (|t| > 4.5; RTLF α in the columns)
| set | contrasts | paired t | Welch t | RTLF .09 | RTLF .01 |
|---|---|---|---|---|---|
| original harness, real fixtures, noisy A/B/C | 63 | 0 | 0 | 2 | 1 (of 63) |
| same, quiet E/F | 63 | 0 | 0 | 2 | 0 |
| same, 48 processes (run D) | 432 | 2 | 0 | 11 | – |
| original, A/A halves, noisy / quiet / run D | 63 / 63 / 432 | – | 0 / 0 / 0 | 2 / 2 / 10 | 0 / 0 / – |
| runG/runH original, real | 27 | 0 | 0 | 1 | 1 |
| runG/runH matched, real fixtures | 45 | 2 | 2 | 6 | 5 |
| runG/runH matched, null arm | 10 | 0 | 0 | 1 | 1 |
| runG/runH A/A halves of original-mode runs | 27 | – | 0 | 1 | 0 |

RTLF's false-positive rate over every null (A/A halves, matched null arms, split null copies; 885 contrasts) is 2.4% at α = 0.09 (21), under its nominal bound of 9%, and 0.4% at α = 0.01 (2 of 453 nulls analysed at α = 0.01). Welch |t| > 4.5 flagged 0 of 885 nulls. Where the tests disagree:
- the 4 RTLF-only flags among the 45 runG/H matched real contrasts are single-process cases (below); 2 more have both tests;
- the 2 paired-|t| flags among the 432 run-D contrasts (4.53, 4.52) are also RTLF flags, and Welch does not flag them (the paired statistic is the heavy-tailed one);
- on the 126 noisy and quiet original-harness contrasts: both quiet 122, RTLF only 4, t only 0, both 0.

### runG and runH: matched preamble
| run | class | fixtures 1 to 4/5 paired t (n = 4096) | null arm |
|---|---|---|---|
| G matched 4096 | guarded | −0.3, +1.6, +0.5, +1.8, +1.8 | −0.2 |
| G matched 4096 | generic | **+3.5, +5.6, +3.8, +6.6** (RTLF flags, deciles 10 to 90) | +0.7 |
| H matched 4096 (a) | guarded | −3.6, +1.2, −0.3, −2.1, +1.4 | −0.6 |
| H matched 4096 (a) | generic | −0.7, −1.0, +1.0, +0.5 | −0.9 |
| H matched 4096 (b) | guarded | −1.1, −1.0, −0.7, +0.4, +0.3 | −1.2 |
| H matched 4096 (b) | generic | **−3.9** (RTLF flags), +0.4, −1.1, +0.5 | −0.1 |
| H original 4096 | guarded | −2.9, −0.5, −1.2, −0.5, −3.5 | – |
| H original 4096 | generic | −1.0, +0.5, +0.8, +2.4 | – |

runG 06 shows a persistent (all four quarters of the run) +1 to +3 µs shift of all four generic fixtures against the reference, with a clean null arm: a per-process offset of the reference arm. Two replicate processes under the same conditions show nothing. Matched and original pair-difference sd are the same (guarded 20 to 25 µs, generic 22 to 31 µs).

### dev vs dev, between processes (`xbin`)
Same binary, same fixtures, same core, separate launches minutes apart (A01 vs A02, E01 vs E02: 22 contrasts): mean differences of 9 to 25 µs (0.4 to 1%), Welch t −12.7 to +9.2, RTLF flags every one with 6 to 9 deciles. dev vs a5a6f60 across launches behaves identically (all 44 contrasts flagged by RTLF, t −10 to +5, differences 0.2 to 23 µs). Both tests are correct about the samples; the samples differ because the process differs. The −6.77 compared the reference and a fixture inside one process, so it is not this effect directly, but runG 06 shows the within-process version at 1 to 3 µs.

### Split mode, per phase (n = 10,000 and 5,000 rounds, 5 real contrasts per class per phase, plus the null copy)
RTLF α = 0.09 flagged: null copy 0 of 50 (noisy and quiet, 5 phases; 1 of 48 per phase in run D); real fixtures: full 2 of 45, red 4 of 45, inv 0 of 45, adm 27 of 45, herm 18 of 45. The t-test flagged 0 in every cell. Replication across the two independent processes of each condition (runs A, E):
- noisy A: RTLF flagged both processes for 10 of 45 real contrasts (7 at α = 0.01), all in the generic class adm and herm phases and guarded adm 3 and 4; null copy 0 of 10;
- quiet E: 0 of 45 flagged in both processes; 12 of 45 in either at α = 0.09; null copy 0 of 10.
Generic adm medians differ from the reference by −18 to −500 ns in the two processes (sign agrees on 3 of 4 fixtures), means by up to 0.44 µs, at most 1.4% of a 31 µs phase and below the full-path floor, so they do not change the full-path result.

## Positive control: detection floor (`posctl.py`, `results/positive_control_floors.md`)
A known delay was added to every sample of the split-mode null copy (guarded class, same-data null, so the true effect is the delay), cut into 38 chunks of 512 rounds (the G7 repeat count) per run, for two noisy-node and two quiet-node runs.

| host | phase | RTLF α=.09 | RTLF α=.01 | paired t > 4.5 | Welch t > 4.5 |
|---|---|---|---|---|---|
| noisy | adm | 0.2 µs | 0.4 µs | > 1.6 µs | > 1.6 µs |
| noisy | full | 6.4 µs | 6.4 µs | > 6.4 µs (3 of 38 at 6.4) | > 6.4 µs (1 of 38) |
| quiet | adm | 0.2 µs | 0.2 µs | > 1.6 µs (3 of 38 at 1.6) | > 1.6 µs |
| quiet | full | 3.2 µs | 6.4 µs | > 6.4 µs (14 of 38 at 6.4) | > 6.4 µs (1 of 38) |

Floor = smallest tested delay detected in at least 90% of the chunks. Zero-delay detection rates: RTLF α = 0.09 at 0 of 38 (full) and 2 of 38 (adm, noisy); t 0 of 38. At the 10,000-round whole-run size, RTLF flags 0.4 µs (one quiet run) to 0.8 µs (three of four runs; none in the other noisy run up to 0.8) on the full path; in the adm phase it flags ≤ 10 to 50 ns, but that is a timer-tick effect (tick 18.5 ns, thresholds fall to 0 to 2 ticks), so a sub-tick shift flips the verdict: it must not be read as sensitivity below a tick.

On-Pi injection (`05_inject_*`, matched harness, guarded null arm, n = 512 each, one run per delay): t crosses 4.5 at 4 µs (+6.2) and 8 µs (+13.6); RTLF flags 4 and 8 µs on all deciles, and also flagged 0.25, 0.5 and 2 µs, but 1 of the 6 uninjected matched null arms (runG 02, 04, 06, two classes) was also flagged by RTLF, so single-run RTLF flags below about 2 µs are not interpretable. At n = 512, a Cortex-A72 full-path leak needs to be about 0.15 to 0.3% (3 to 6 µs) to be seen reliably by RTLF and 0.3% or more by t.

The injected delay is a constant shift, the best case for a decile test. A rare large excursion (say 1% of calls much slower) moves the mean, hence t, more than the first nine deciles, so t may catch what RTLF cannot. That case is not tested here; it argues for running both rather than replacing one.

## What this says about the −6.77
Neither test, nor the matched harness, nor 4 further processes at n ≥ 4096 on this board reproduces the P248 guarded fixture 1 difference. The conclusions from the earlier reports stand: host noise and a heavy-tailed paired t. New: an unmatched preamble is not the explanation here (matched and original harness give the same sd and the same range of t), and the dominant risk is per-process layout offsets of 1 to 25 µs, which only replication across processes can separate from code effects.

## Recommended G7 gate rule

**Run validity** (otherwise the run is void, not a pass):
1. The binary uses the matched preamble (`time_reduction_matched` semantics), and every class includes the null arm (reference timed against itself).
2. At least 3 fresh process launches, same board and core, interleaved with a different binary or idle time, never compared across processes. Each launch: n ≥ 4096 repeats per contrast (n = 512 only for smoke runs).
3. Record node state (shekyld up or down, other CPU load) in the run folder.
4. Pre-registered stopping rule: n and the number of processes are fixed before the run, no peeking.
5. The null arm must not be flagged by RTLF (α = 0.01) or by |t| > 4.5 in more than 1 of the 3 launches; otherwise the run is void and repeated.

**Per-contrast, per-process flags**
- R: RTLF flags at α = 0.01 (per-test type-1 error bound 1%).
- T: |Welch t| > 4.5.
- Effect floor: ignore a flag whose largest flagged-decile difference is below 3 timer ticks (about 55 ns) or below the measured detection floor of that phase at that n (0.2 µs adm, 3 µs full at n = 512, scale as 1/√n).

**Verdict per fixture (and per phase, when a split mode is run)**
- **LEAK (confirmed):** R and T both flag in the same process, in at least 2 of 3 processes, with the same sign of the median shift, null arm clean. Gate fails; escalate.
- **CANDIDATE:** R or T flags in at least 2 of 3 processes with the same sign, but not both tests; or R and T flag together in one process. Rerun with 3 new processes and 4× n before any claim.
- **NOISE:** a flag in only one process, or sign flips between processes (runG 06 generic is this case).
- **PASS at resolution r:** no R or T flag in any process, null arm clean; the report states the floor r (for example "no full-path difference above 3 µs, 0.13%, at n = 512 per process"). A pass is never "constant time", only "no difference above r".

The agreement requirement is deliberate: R is more sensitive on shifts of the body of the distribution and on short phases; T is the only one that sees rare large events; requiring both marks a confirmed leak, requiring either in two processes marks a candidate. Replication across independent launches, not the choice of test, is what separates code from layout and drift in all the data here (null copy 0 of 50 on split data at n = 5,000 and 10,000; the one-process effects in runG and runH did not replicate).

Applied to the data in this report: no fixture is a confirmed LEAK. CANDIDATES: generic adm/herm fixtures 1 to 4 on the noisy node (R in 2 of 2 processes, T none), not reproduced on the quiet node. Everything else is PASS or NOISE at the stated resolution.

## Caveats
- One board, one build, dev 470d537 plus patches.
- The positive control uses a constant additive delay on the null copy. Not tested: tail-only and input-dependent delays, delays smaller than a tick on the full path.
- Bulk RTLF results beyond the 2,711-contrast re-analysis use the numpy port (verdict agreement 99.4% on 527 shared files). The α = 0.01 results for the original contrasts are the port only.
- The matched preamble was run in 3 processes at n = 4096 and 2 at 512, not the 3 × 4096 of the recommended gate for every fixture; runG and runH ran with the node down.
- Run order of runG and runH was fixed (not randomised across processes).

## Files
- `harness-matched.patch`, `pi/campaign.sh`, `pi/followup.sh`
- `rtlf_batch.R`, `rtlf_np.py`, `extract.py`, `posctl.py`, `xbin.py`, `summarize.py`, `agree.py`, `pin.txt`
- `results/data/runG_matched_core3.tar.xz`, `results/data/runH_matched4096_core3.tar.xz` (raw CSVs, node state, monitor, build provenance)
- `results/tables.md` (every contrast), `results/positive_control_floors.md`, `results/r_vs_numpy.txt`, `results/rtlf/*.tsv.gz`, `results/verdicts.json.gz`
- Earlier raw data are in `research/p248-arm-timing/` and its `quiet/` folder.
