# Successful verification and signer measurements

`cargo run --release --locked -p prism-bench` requires acceptance on every
iteration. It reports 31 samples after five warmups, at all three levels.
Both adapted chains, pairing, canonicalization and the full image check
are inside the timed invocation. Sample construction is outside it.

The native signer roundtrip can be run with:

```
cargo run --release --locked -p prism-signer --example roundtrip -- target/native
```

It generates ephemeral secrets from OS entropy, prints only public sizes
and timings, and checks each response. Its signing number includes the
key regeneration required by the isolated seed-based interface. It is not
a steady-state cached-secret signing benchmark.

## Raspberry Pi 4 gate

Run `bash scripts/bench_floor.sh > prism-pi4.txt` on a 64-bit Raspberry Pi 4
Model B with the pinned Rust toolchain. The script refuses other hardware,
records the device model, kernel, Rust version and CPU governor, and runs
the complete successful verifier. Preserve the full output and record power
and thermal conditions, throttling and competing load alongside it. Decide
the transaction verification budget from this evidence, including the
number of mandatory authorization legs. The first three-run measurement is recorded below.

## Floor-device result, 2026-11-12

Implementation `49691c469f23b8abdb4feafe5ad48b694ca5d926`, Rust 1.94.0,
scalar backend on a Pi 4 Model B Rev 1.4 (Cortex-A72, aarch64), under the
recorded background-load conditions. All 113 honest signatures and mutation
regressions passed natively before timing. All 279 timed invocations accepted;
45 warmup invocations also accepted.

| Parameter row | Run 1 median | Run 2 median | Run 3 median | Median of run medians |
| --- | ---: | ---: | ---: | ---: |
| I | 167.054 ms | 167.097 ms | 168.150 ms | **167.097 ms** |
| III | 449.076 ms | 448.930 ms | 449.764 ms | **449.076 ms** |
| V | 1024.756 ms | 1023.213 ms | 1023.399 ms | **1023.399 ms** |

The level III observation is approximately **0.449 seconds per complete
verification**, or 2.23 sequential verifications per second in these
conditions. This does not establish an adversarial latency bound, multicore
throughput, or an idle-device baseline. Selecting a transaction verification
budget remains a deployment decision.

The 5-second telemetry samples recorded 1.8 GHz and 63.3–69.1 °C.
Firmware throttling flags could not be read; observed sysfs voltage-alarm
readings were zero. A claim that no transient throttling occurred is not
made. The source was an exact build-input snapshot of the then-uncommitted
implementation; the private provenance record verifies its executable/test
inputs against commit `49691c4` (only a license-file trailing blank line
was normalized before commit).

[Measurement rows and digests](benchmarks/acceptance_pi4_2026-11-12.txt)
retain the benchmark output without host identities or local paths. Full
conditions, telemetry, the executable, source snapshot and checksums are in
`shekyl-dev/infrastructure/captures/prism-acceptance/2026-11-12/`.

## Development-machine observation

An x86_64 development run on 2026-11-12 accepted every invocation:

| Parameter row | Median verification | Observed max−min spread |
| --- | ---: | ---: |
| I | 24.81 ms | 27.08 ms |
| III | 119.59 ms | 81.02 ms |
| V | 219.75 ms | 167.55 ms |

The raw output is [acceptance_x86_64_2026-11-12.txt](benchmarks/acceptance_x86_64_2026-11-12.txt).
This shared development-machine observation is noisy and not a release
performance claim or the Pi gate. Public-key preparation/caching and native
field/chain optimization remain future performance work.
