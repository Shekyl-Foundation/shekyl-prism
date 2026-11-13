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

## Operation counts

The machine-independent cost of a verification is its count of field
operations. `prism-field` counts calls through its `Field` trait when
built with the `op-count` feature, and the verifier marks ten stages:

| Stage | What is counted there |
| --- | --- |
| `public` | decoding, sign, model and the basis search |
| `challenge` | nothing in this metric: the hash and Baillie–PSW do not use the field |
| `torsion` | the key's basis, the on-curve test, and the order and span checks |
| `scale` | `[s]` of the two signature points |
| `pairing` | the two Weil pairings and the power they are compared through |
| `kernel` | lifts, the canonical four-torsion, the matrices, both chain kernels and both theta base changes |
| `chain1`, `chain2` | the long and the short chain |
| `codomain` | the comparison of the two codomains |
| `image` | the three image comparisons |

A stage is a purpose. Work that a later change moves keeps its stage, so
two versions of the table compare row by row.

```sh
sh scripts/op_counts.sh           # rewrite the two records
sh scripts/op_counts.sh --check   # what CI runs, on x86_64 and aarch64
```

- [benchmarks/op-counts.txt](benchmarks/op-counts.txt): per-stage counts
  for the six primary vectors, the total for each of the other 107, the
  per-stage counts of the first negative of each refusal, and for each
  primary vector its dearest refusal against its accepted cost.
- [benchmarks/negative-verdicts.tsv](benchmarks/negative-verdicts.tsv):
  the verdict and total cost of each of the 2,712 negatives derived from
  the six primary vectors. `Basis` has no vector: no key whose basis index
  exceeds the search bound is known.

Each record states a digest of the sources it was counted from. The
commit is the one that last changed the record. Counts are taken in the
release profile; they vary slightly between vectors with the basis index
and the bits of the challenge.

The extension-field counts of one accepted signature, primary vector of
each level, after each change of the performance work. Square roots (7)
and residuosity tests (4, 6, 4) did not move.

| After | Operation | I | III | V |
| --- | --- | ---: | ---: | ---: |
| the instrument, no optimization (`c9df786`) | multiplications | 345,200 | 530,434 | 722,924 |
| | inversions | 4,019 | 6,072 | 8,056 |
| squaring | multiplications | 184,098 | 281,720 | 384,048 |
| | squarings | 135,822 | 210,624 | 287,518 |
| | inversions | 4,019 | 6,072 | 8,056 |
| the pairing without inversions | multiplications | 191,028 | 292,234 | 398,034 |
| | squarings | 139,788 | 216,638 | 295,516 |
| | inversions | 2,033 | 3,062 | 4,054 |
| one inversion per chain for the dual's coordinates | multiplications | 196,920 | 301,198 | 409,974 |
| | squarings | 139,788 | 216,638 | 295,516 |
| | inversions | 73 | 78 | 78 |

Squaring turned 39% of the multiplications into squarings and removed 7%
outright (products that the doubling and addition formulas computed two
or three times). The Miller loop without inversions spends about 11,000
more multiplications and squarings at level I to remove 1,986 inversions,
each worth about 40 multiplications. Sharing one inversion among all the
null-point coordinates of a chain removes 1,960 more for 5,892
multiplications. The 73 that remain do not grow with the chain: 26 in
each gluing, 8 in the pairings, and 13 elsewhere. No base-field call is made directly at any stage, and no
refusal costs more than acceptance: the dearest negative of every primary
vector is `Image` at about 90% of the accepted cost.

`prism-bench` prints the time of one extension-field multiplication,
squaring and inversion per level after its verification timings. On the
x86_64 development machine (shared, load 12 to 18; medians of three runs
in [benchmarks/sum_of_products_x86_64_2026-11-13.txt](benchmarks/sum_of_products_x86_64_2026-11-13.txt)):

| ns per operation | I | III | V |
| --- | ---: | ---: | ---: |
| multiplication | 43.2 | 86.1 | 152.7 |
| squaring | 30.6 | 55.4 | 99.5 |
| inversion | 1,711 | 3,082 | 4,770 |

Squaring is faster than multiplication at every level there, so `square`
is used at every level. An inversion costs about 40, 36 and 31
multiplications.

`fp2`'s `use_sum_of_products` multiplication was measured in the same
record and is not adopted: against the default it takes 38.2 ns at level
I but 97.2 at III and 164.4 at V. It stays off at every level.
