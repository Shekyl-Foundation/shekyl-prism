# P248 ARM timing with shekyld stopped (Rick's t53u / t53s363)

Follow-up to [`../REPORT.md`](../REPORT.md). Same board (`skl-pi4-1`, Pi 4 Model B Rev 1.4, 8 GB, Cortex-A72), same binaries, same G7 contrast and phase-split harness, pinned to core 3 with `taskset`. The live testnet node was stopped for the measurement and started again afterwards. **No constant-time claim.** G7 stays open.

Times are America/New_York (UTC-5), 2026-11-26 14:08 to 14:51.

## Answer in one paragraph

Stopping `shekyld` made the host quieter — other work on the pinned core fell from 1.54 to 0.24 jiffies/s, bursts of 10 or more jiffies from 4.2% of seconds to one second in 1,605, load from as high as 3.4 to 1.0–1.6, temperature from 52.6–63.8 °C to 48.7–57.0 °C — but it did not change the story of the contrast. Fixture 1's paired t on the G7 512-repeat runs was +0.94, −0.35, +0.90, +1.21 (dev) and +0.12, +0.25 (`a5a6f60`), and −1.26 on a 4,096-repeat run, all inside the earlier noisy-node range of −1.94 to +1.43. The pair-difference sd stayed about 20–31 µs (median about 25 µs), against 27–32 µs before and 17.1 µs implied by the published −6.77. The phase split still shows no fixture-linked effect: admission medians at 0–2 ticks, reduction and full-path ANOVA p = 0.29 and 0.23 over 20,000 pooled rounds, and the same-data null contrast is the same size as the real fixtures. Most of the G7 statistic's noise on this board was not the node. The −6.77 still does not reproduce.

## Node stop and restart

`shekyld` is the system unit `shekyld@testnet.service` (user `shekyl`, uid 983), not a process `skl-agent` owns. It was stopped and started with the approved passwordless commands only:

```text
sudo -n /usr/bin/systemctl stop shekyld@testnet
sudo -n /usr/bin/systemctl start shekyld@testnet
sudo -n /usr/bin/systemctl is-active shekyld@testnet
sudo -n /usr/bin/systemctl status shekyld@testnet
sudo -n /usr/bin/journalctl -u shekyld@testnet …
```

- **14:08:32** stop, rc=0, `is-active` inactive immediately. PID 1244 and its child tor (PID 1385) were gone; ports 12030 and 12021 were no longer listening.
- Timing ran 14:08:33–14:46:56 with the node down.
- **14:46:57** start, rc=0, `is-active` active immediately. New Main PID 261362, tor child PID 261416. The view RPC came up on 127.0.0.1:12030 and clearnet on 0.0.0.0:12021 at 14:46:58.
- **14:47:06** journal: "This node is holding its own transactions. It is syncing over clearnet."
- **14:47:14** journal: "Tor is carrying this node's traffic."
- **14:47:16** outbound connections below the floor (Clearnet 6/12, Tor 1/12).
- **14:50:18** "outbound connections recovered the relay provisioning floor connector=Tor achieved=12 floor=12."
- shekyld CPU: 118% just after start, 49% at +1 min, 36% at +3 min, 29% at +4.5 min.
- **14:51:28** still `active`; 12030, 12021 and 29701 listening; shekyld and tor still the same new PIDs.

A numeric chain height was not in the journal at this log level (`log-level=warn`, `log-category=daemon=info`). The view RPC is up, but its method names are not the usual Monero-style ones and I did not find a height method in the approved journal. Progress is the "syncing over clearnet" line, Tor coming up, and Tor outbound recovering from 1 to 12 in three minutes.

The runner always starts the node on exit (`pi/quiet_run.sh`). The start was from the main path, not the trap.

## Scripts and wallet (not restarted)

These `skl-agent` processes were left alone and **survived** the stop, the whole timing window, and the start, with the same PIDs:

| Process | PID | CPU (throughout) |
|---|---:|---:|
| `capture_a12b.py` | 31063 | ~0.8% |
| `guard_sample_a12.py` | 35696 | ~2.9% |
| `shekyl-wallet-rpc` (tmux `sendwallet`) | 34088 | ~0.9% |

I did not restart or signal them. They will have seen the daemon go away and come back; I did not inspect their logs.

## Hardware during the quiet window

Governor stayed `ondemand` (no sudo for cpufreq). Core 3 was at 1.8 GHz in 1,607 of 1,614 1 Hz samples during the dev suite (the rest are the idle gaps between processes). Undervoltage alarm 0. Temperature 48.7–57.0 °C. Load 1.00–1.62. Two or three users logged in.

## Runs

Same binaries as the earlier work: dev `470d537` + harness patch (`STAGE2C_ONLY=p248`), and unmodified `a5a6f60`. SHA-256 unchanged: dev+patch `27c1a0b6a0b8822d123bdc381d32074c1439e66b615015d2325cae08672bff69`, a5a6f60 `85f5a79ebe5666778836186e4c6bdbd6da87d757ef8ad574843a92b0fccd79ec`.

| Run | Binary | Content |
|---|---|---|
| E (dev) | `stage2c-dev-patched` | `timing 512` ×4, `timing 4096`, `split` 10,000 rounds ×2 (seeds 11, 12) |
| F (`a5a6f60`) | `stage2c-a5a6f60` | `timing 512` ×2 (all five sets; P248 analysed) |

## Fixture-1 G7 contrasts (guarded class, P248)

Statistic as in G7: paired t of (other − reference), n = repeats. Earlier noisy-node fixture-1 t values, for comparison: +0.37, −1.77, +1.43, −1.94 (dev 512 / 4096), −0.46, +0.78 (`a5a6f60`), +0.35 (core 2).

| Run | Repeats | fixture 1 t (mean ns) | other guarded t | max \|t\| | pair-diff sd (f1, ns) |
|---|---:|---:|---|---:|---:|
| E1 dev | 512 | +0.94 (+1,267) | −1.08, +2.02, −0.94, −1.24 | 2.02 | 30,381 |
| E2 dev | 512 | −0.35 (−314) | +0.46, −0.64, −1.00, −0.75 | 1.00 | 20,214 |
| E3 dev | 512 | +0.90 (+1,084) | −0.65, +0.38, +2.21, −0.44 | 2.21 | 27,189 |
| E4 dev | 512 | +1.21 (+1,170) | −0.04, −2.11, +0.40, −1.49 | 2.11 | 21,932 |
| E5 dev | 4096 | −1.26 (−495) | −0.21, +0.83, +0.82, −0.34 | 1.26 | 25,051 |
| F1 a5a6f60 | 512 | +0.12 (+129) | +0.30, +0.47, −0.80, +1.18 | 1.18 | 24,574 |
| F2 a5a6f60 | 512 | +0.25 (+303) | +0.27, −0.54, −0.31, +1.38 | 1.38 | 27,450 |

No run is close to −6.77. Signs still flip. The 4,096-repeat run is the most precise of the G7-mode rows and is consistent with a mean near zero (median difference −10 ns). Generic-class max \|t\| was 1.56 on dev and 2.37 on the second `a5a6f60` run.

## Phase split (dev, guarded, 20,000 pooled rounds)

Winsorized-5% mean difference (ns) / blocked t (20 blocks) / median difference (ns). ANOVA p is across the six real fixtures, rounds as blocks.

**Seeds 11+12 pooled**

| Phase | fixture 1 | fixture 2 | fixture 3 | fixture 4 | fixture 5 | NULL (reference copy) | ANOVA p |
|---|---|---|---|---|---|---|---:|
| adm | −4 / −0.13 / +18 | −44 / +0.05 / −1 | +38 / +0.69 / +18 | +113 / +1.09 / +36 | +23 / +0.75 / +0 | −26 / −0.61 / +0 | 0.50 |
| red | −100 / −1.40 / +92 | −212 / −1.57 / −18 | −366 / −2.02 / +93 | −237 / −0.69 / −148 | −201 / −0.68 / +64 | −373 / −1.69 / −130 | 0.29 |
| full | −73 / +0.87 / −20 | +85 / +1.90 / −55 | +72 / +0.76 / +0 | +234 / +2.25 / −36 | −148 / +0.41 / +18 | −106 / +0.70 / −56 | 0.23 |

Admission is at the timer tick (18.5 ns). The one 10,000-round file had a full-path ANOVA p of 0.009; the second file gave p = 0.43, the pool is 0.23, and the null is the same size as the real fixtures, so that is not a reproduced fixture effect. The Hensel inverse (`inv`) has median difference 0 for every fixture, including the null; a large 5%-winsorized t appears on the null as well (t5 = −7.1) and is an artefact of winsorizing an 11 µs interval, not a data dependence. Use the median and the blocked t for that phase.

Sensitivity on this quiet window: about 0.1 µs in admission and about 0.4 µs on the full path (winsorized means of the null). Still far below 5.1 µs.

## Noise floor compared with the node running

| Quantity | Node running (run A, earlier report) | Node stopped (run E) |
|---|---|---|
| Other jiffies/s on the pinned core (mean) | 1.54 | 0.24 |
| Seconds with ≥3 / ≥10 other jiffies | 83 / 68 of 1,605 | 9 / 1 of 1,605 |
| Load average (1 min) | 1.02–3.42 | 1.00–1.62 |
| Temperature (°C) | 52.6–63.8 | 48.7–56.5 (run F to 57.0) |
| Guarded pair-difference sd (ns), typical | 27,000–32,000 (min 27,126; median 31,563) | 20,000–31,000 (per-run fixture-1 20,214–30,381) |
| Fixture-1 \|t\|, 512-repeat G7 runs | up to 1.94 on dev (3.73 on another fixture in one run) | up to 1.21 on fixture 1 (2.21 on another fixture) |
| Implied sd of the published −6.77 | 17,146 ns | still below this board |

Stopping the node clearly reduced contention and heat. It barely moved the sd of the G7 paired differences, so most of that variance is not shekyld. What remains on the board: `ondemand`, the three `skl-agent` scripts and wallet (~4.6% CPU together, not pinned off core 3), and whatever is intrinsic to a 2.4 ms wall-clock interval on this Cortex-A72. A still-quieter setup could in principle reach the published run's 17 µs sd, but it would have to come from those leftovers or from the measurement itself, not from shekyld.

## Files

`results/summary/` (analyze.py JSON), `results/data/orig_mode_raw.tar.xz` (all G7-mode CSVs), monitors and progress logs, `results/state/`, `results/health/node_ops.txt` (redacted stop/start record). The raw 10,000-round split CSVs (~35 MB each) stay on the Pi under `~/shekyl-timing/results/` and were not committed. `quiet_run.sh` lives in `../pi/quiet_run.sh`. The full `systemctl status` text lists configured peers and is not committed.
