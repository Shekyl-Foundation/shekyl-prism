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
number of mandatory authorization legs. This environment has no Pi attached.

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
