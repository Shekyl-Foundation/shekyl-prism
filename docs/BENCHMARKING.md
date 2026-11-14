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
performance claim or the Pi gate. It predates the salt and the
operation-count work; the current figures are under "Operation counts"
below. Public-key preparation/caching and native field arithmetic remain
future performance work.

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
each level, after each change of the performance work. Square roots
were 7 and are 6 since the key's basis is lifted once; residuosity tests
(4, 6, 4) did not move.

| After | Operation | I | III | V |
| --- | --- | ---: | ---: | ---: |
| the instrument, no optimization (`c9df786`) | multiplications | 345,200 | 530,434 | 722,924 |
| | inversions | 4,019 | 6,072 | 8,056 |
| squaring (`1334dc2`) | multiplications | 184,098 | 281,720 | 384,048 |
| | squarings | 135,822 | 210,624 | 287,518 |
| | inversions | 4,019 | 6,072 | 8,056 |
| the pairing without inversions (`c9ff6ba`) | multiplications | 191,028 | 292,234 | 398,034 |
| | squarings | 139,788 | 216,638 | 295,516 |
| | inversions | 2,033 | 3,062 | 4,054 |
| one inversion per chain for the dual's coordinates (`1705c98`) | multiplications | 196,920 | 301,198 | 409,974 |
| | squarings | 139,788 | 216,638 | 295,516 |
| | inversions | 73 | 78 | 78 |
| scalar multiplications shared (`aa6048b`) | multiplications | 137,836 | 212,082 | 289,168 |
| | squarings | 93,951 | 147,281 | 202,147 |
| | inversions | 71 | 76 | 76 |

Squaring turned 39% of the multiplications into squarings and removed 7%
outright (products that the doubling and addition formulas computed two
or three times). The Miller loop without inversions spends about 11,000
more multiplications and squarings at level I to remove 1,986 inversions,
each worth about 40 multiplications. Sharing one inversion among all the
null-point coordinates of a chain removes 1,960 more for 5,892
multiplications. The 73 that remain do not grow with the chain: 26 in
each gluing, 8 in the pairings, and 13 elsewhere (11 once the key's
basis is lifted once). Sharing the doublings of the four kernel basis
points among every ladder on them, scaling the signature points once,
and taking `−1` coefficients as a negation removes 104,921 products at
level I, 31% of what was left.

The only direct base-field calls are the residuosity tests of the basis
search, in the `public` and `torsion` stages: between 0 and 32 per
verification over the committed vectors, depending on the key. No base-field multiplication,
squaring, inversion or square root is called directly.

No refusal costs more than acceptance, at any of these commits. Per
level at the last one, the dearest of the 2,712 negatives against the
cheapest of the honest vectors: 226,185 products against 230,605 at level
I, 350,599 against 357,848 at III, 476,164 against 486,351 at V, and
fewer inversions in every refusal than in any acceptance. Against its
own vector's acceptance the dearest negative is `Image`, at about 90% of the accepted cost before the scalar
multiplications were shared and 97% after, since that change made the
last stage cheaper than the stages before it.

Verification time before and after, on the x86_64 development machine in
one session: five alternating runs of the bench built at the instrument's
commit (`c9df786`) and at the commit of the last change above, at a
1-minute load near 4. Median of the five run medians, in milliseconds;
record in [benchmarks/verifier_ops_x86_64_2026-11-13.txt](benchmarks/verifier_ops_x86_64_2026-11-13.txt).

| | I | III | V |
| --- | ---: | ---: | ---: |
| before | 24.1 | 68.4 | 156.4 |
| after | 11.5 | 32.5 | 73.9 |

On the floor device, a Raspberry Pi 4 Model B Rev 1.4, `scripts/bench_floor.sh`
at `c9df786` and at `8f72373`, three alternating runs in one session on
2026-11-14. Median of the three run medians, in milliseconds; record in
[benchmarks/verifier_ops_pi4_2026-11-14.txt](benchmarks/verifier_ops_pi4_2026-11-14.txt).

| | I | III | V |
| --- | ---: | ---: | ---: |
| before | 167.3 | 447.4 | 1016.3 |
| after | 89.1 | 231.7 | 520.8 |

The fall is 46.7%, 48.2% and 48.8%, against 52 to 53% on x86_64. The run
medians of one tree differ by at most 0.2%. A resident node used about
1.9 of the four cores throughout (load 3.0 to 3.8) and was not stopped,
as in the 2026-11-12 record; the clock read 1.8 GHz and the temperature
63.8 to 66.7 °C at every reading. Firmware throttling flags could not be
read. This is a measurement under recorded load. The "before" figures
are the salted verifier's and equal the unsalted ones of 2026-11-12 to
within 0.7%.

The idle-device baseline, the same session an hour later with the
resident node stopped for the timed runs (load about 1, 51.6 to 55.5 °C):
166.8 / 447.7 / 1018.0 ms before and 89.3 / 232.3 / 522.5 ms after, a
fall of 46.5%, 48.1% and 48.7%; record in
[benchmarks/verifier_ops_pi4_idle_2026-11-14.txt](benchmarks/verifier_ops_pi4_idle_2026-11-14.txt).
Every figure is within 0.5% of the loaded one, and the per-operation
times within 0.5%: a node using two of the four cores does not move a
single-threaded verification on this device. The bench is one thread, so
neither record says anything about verifying on all four cores at once.

One extension-field operation on the Pi 4 at the head, median of the
three runs. The bench's counter on aarch64 is a 54 MHz timer, not a cycle
counter, so no cycle count was measured: the cycle figures are the
nanoseconds times 1.8 GHz, the clock read before and after each run and
not sampled during one.

Table: time measured; cycles **estimated at an assumed 1.8 GHz**, not counted.

| | multiplication: ns measured, cycles estimated at an assumed 1.8 GHz | squaring: ns measured, cycles estimated at an assumed 1.8 GHz | inversion: ns measured, cycles estimated at an assumed 1.8 GHz |
| --- | ---: | ---: | ---: |
| I | 348.6 ns, est. 627 | 223.8 ns, est. 403 | 6,292 ns, est. 11,326 |
| III | 628.0 ns, est. 1,130 | 405.3 ns, est. 730 | 12,149 ns, est. 21,868 |
| V | 1,097.0 ns, est. 1,975 | 701.1 ns, est. 1,262 | 19,743 ns, est. 35,537 |

A squaring costs 64% of a multiplication at every level on the Pi (70%,
64% and 66% on x86_64), and an inversion 18 to 19 multiplications (40, 37
and 31 on x86_64), so the inversions that were removed were worth less on
the Pi than on x86_64, which is where the smaller fall comes from. At
level I the counted multiplications, squarings and inversions at these
prices come to 69.5 ms of the 89.1 ms measured, 78%; the rest is
additions, subtractions, base-field work and everything outside the field.

### Memory of one verification

`sh scripts/peak_heap.sh` runs the `verify_once` example under Valgrind's
Massif on the six primary vectors: once decoding the vector only, once
verifying it, with the heap measured to the byte, and once more with the
stack sampled. On the x86_64 development machine, at `db84af6` and, with
the script and example copied in, at `c9df786`; record in
[benchmarks/peak_heap_x86_64_2026-11-14.txt](benchmarks/peak_heap_x86_64_2026-11-14.txt).
The two vectors of a level agree to the byte in every row.

Table: peak heap of one verification, in bytes, and what holds it at the peak.

| | I | III | V |
| --- | ---: | ---: | ---: |
| before the operation-count work (`c9df786`) | 797,706 | 2,383,998 | 3,175,598 |
| at the head | 1,107,210 | 2,995,710 | 4,441,774 |
| of which the two chains' point vectors | 663,616 (60%) | 1,986,688 (66%) | 2,646,144 (60%) |
| of which the batched inversions' vectors | 250,112 (23%) | 571,776 (19%) | 1,016,320 (23%) |
| of which the four doubling tables | 190,464 (17%) | 433,152 (14%) | 774,144 (17%) |
| heap held before verifying (the decoded vector) | 3,194 | 4,366 | 5,534 |
| largest stack sample | 88,800 | 130,088 | 172,856 |

The operation-count work raised the peak by 39%, 26% and 40%: the
doubling tables and the vectors of the batched inversions are new, and
both are still held when the second chain is at its largest. The largest
part was there before and is the chains' own.

Every byte of this is allocated inside one call of `verify` and freed
before it returns; nothing is shared between calls and nothing is kept
between them. So verifications running at once each hold their own, and
four at once, one per core of the Pi 4, hold about 4.4 MB, 12.0 MB and
17.8 MB of heap at the peak, with under 0.2 MB of stack each. That is
arithmetic from the single-verification figure, not a measurement: no
four-thread run was made, and memory was not measured on the Pi (the
field elements are the same 64-bit limbs on aarch64, so the sizes should
be the same, but that is not checked). It bounds memory only. Whether
four verifications at once each take the single-thread time, with four
working sets of this size in the Pi's 1 MB shared cache, is the open
four-core question, and this does not answer it.

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
