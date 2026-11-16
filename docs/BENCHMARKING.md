# Successful verification and signer measurements

`cargo run --release --locked -p prism-bench` times two things, each over
31 samples after five warmups, and holds every call to its pinned result.

- An accepted verification of a committed honest signature, at the three
  sets that have one (`outcome=accepted`). Both adapted chains, pairing,
  canonicalization and the full image check are inside the timed
  invocation. Sample construction is outside it.
- At all five sets, the verification of a committed response that passes
  the pairing check and is refused after it (`outcome=refused error=…`).
  It is not an accepted verification. "The refusal after the pairing
  check" below says what it is and gives the figures.

It then times the sender's search for the salt of that refused response
(`search` rows) and one field multiplication, squaring and inversion at
each set (`field` rows).

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
  per-stage counts of the first negative of each refusal, for each
  primary vector its dearest refusal against its accepted cost, and the
  per-stage counts of the two refusals at each of `p324` and `p664`.
  There is no accepted count at either.
- [benchmarks/negative-verdicts.tsv](benchmarks/negative-verdicts.tsv):
  the verdict and total cost of each of the 2,724 negatives derived from
  the six primary vectors, and of two refusals at each of `p324` and
  `p664`, which have no vector: a response built from a public key alone
  that passes the pairing check, in two forms. `Basis` has no vector: no key whose basis
  index exceeds the search bound is known.

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

One extension-field operation on the Pi 4, median of three runs at
`5a65904`; record in
[benchmarks/field_ops_pi4_2026-11-14.txt](benchmarks/field_ops_pi4_2026-11-14.txt).
Until that commit the bench called each operation through a `&dyn Fn`
inside the timed loop. The same record has the earlier bench beside it:
the indirect call added 4.4, 7.5 and 8.2 ns to a multiplication (1.3%,
1.2% and 0.7%) and nothing measurable to a squaring or an inversion. The
per-operation lines inside the two verification records above, and the
x86_64 figures below, were taken with the earlier bench and carry that
much on their multiplications. The bench's counter on aarch64 is a 54 MHz timer, not a cycle
counter, so no cycle count was measured: the cycle figures are the
nanoseconds times 1.8 GHz, the clock read before and after each run and
not sampled during one.

Table: time measured; cycles **estimated at an assumed 1.8 GHz**, not counted.

| | multiplication: ns measured, cycles estimated at an assumed 1.8 GHz | squaring: ns measured, cycles estimated at an assumed 1.8 GHz | inversion: ns measured, cycles estimated at an assumed 1.8 GHz |
| --- | ---: | ---: | ---: |
| I | 344.2 ns, est. 620 | 223.2 ns, est. 402 | 6,285 ns, est. 11,312 |
| III | 620.1 ns, est. 1,116 | 405.3 ns, est. 730 | 12,159 ns, est. 21,886 |
| V | 1,089.7 ns, est. 1,961 | 700.6 ns, est. 1,261 | 19,731 ns, est. 35,516 |

A squaring costs 64 to 65% of a multiplication on the Pi (70%,
64% and 66% on x86_64), and an inversion 18 to 20 multiplications (40, 37
and 31 on x86_64), so the inversions that were removed were worth less on
the Pi than on x86_64, which is where the smaller fall comes from. At
level I the counted multiplications, squarings and inversions at these
prices come to 68.9 ms of the 89.1 ms measured, 77%; the rest is
additions, subtractions, base-field work and everything outside the field.

### Memory of one verification

`sh scripts/peak_heap.sh` runs the `verify_once` example under Valgrind's
Massif on the six primary vectors: once decoding the vector only, once
verifying it, with the heap measured to the byte, and once more with the
stack sampled. (The script also measures a refusal at every set;
that is the next subsection.) On the x86_64 development machine, at `db84af6` and, with
the script and example copied in, at `c9df786`; record in
[benchmarks/peak_heap_x86_64_2026-11-14.txt](benchmarks/peak_heap_x86_64_2026-11-14.txt).
The two vectors of a level give the same bytes for each of the three
named holders; their totals differ by one or two bytes (the decoded
vector's own) and their stack samples by up to 1,504 bytes. The table is
the first vector of each level.

Table: peak heap of one verification, in bytes, and what holds it at the peak.

| | I | III | V |
| --- | ---: | ---: | ---: |
| before the operation-count work (`c9df786`) | 797,706 | 2,383,998 | 3,175,598 |
| at the head | 1,107,210 | 2,995,710 | 4,441,774 |
| at the head, allocated under `DualChain::new` (the chains' point vectors) | 663,616 (60%) | 1,986,688 (66%) | 2,646,144 (60%) |
| at the head, allocated under `invert_coords` (the batched inversions' vectors) | 250,112 (23%) | 571,776 (19%) | 1,016,320 (23%) |
| at the head, allocated under `Doublings::new` (the four doubling tables) | 190,464 (17%) | 433,152 (14%) | 774,144 (17%) |
| before, allocated under `DualChain::new` | 794,688 | 2,379,904 | 3,170,432 |
| heap held before verifying (the decoded vector) | 3,194 | 4,366 | 5,534 |
| largest stack sample | 88,800 | 130,088 | 172,856 |

The operation-count work raised the peak by 39%, 26% and 40%. The rows
are by the function that allocated, as Massif attributes them. The
doubling tables are new. The batched inversions' vectors are new under
that name, but part of them took the place of memory `DualChain::new`
held before: at level I that function holds 131,072 bytes less than it
did, so the net addition from the chains is 119,040 bytes beside the
tables' 190,464. Both are still held when the second chain is at its
largest. The largest part was there before and is the chains' own, and
the tables are the smallest.

#### The same for a refusal, at all five sets

`p324` and `p664` have no accepted verification to measure. What can be
measured at them is the refusal the count records call
`identity-response-q-difference`: a response built from a public key
alone that passes the pairing check, runs both chains and is refused as
`Codomain`. `verify_once` builds it (`build`) and verifies it
(`refuse`), and the script measures that pair the same way. So that the
figure has something beside it, the same refusal is measured at the six
primary vectors, under each signature's response curve as the key. At
`33af013`, on the x86_64 development machine; record in
[benchmarks/peak_heap_x86_64_2026-11-15.txt](benchmarks/peak_heap_x86_64_2026-11-15.txt).

Table: peak heap of one `Codomain` refusal, in bytes. It is a refusal, not an accepted verification.

| | `p248` | `p324` | `p376` | `p500` | `p664` |
| --- | ---: | ---: | ---: | ---: | ---: |
| peak heap | 1,107,210 | 2,857,683 | 2,995,710 | 4,441,774 | 10,511,473 |
| allocated under `DualChain::new` | 663,616 | 1,986,688 | 1,986,688 | 2,646,144 | 7,248,000 |
| allocated under `invert_coords` | 250,112 | 491,904 | 571,776 | 1,016,320 | 1,859,264 |
| allocated under `Doublings::new` | 190,464 | 377,856 | 433,152 | 774,144 | 1,402,368 |
| heap held before verifying | 3,195 | 13,697 | 4,367 | 5,535 | 24,797 |
| largest stack sample | 90,512 | 131,928 | 133,624 | 174,888 | 237,608 |

At the three sets with vectors the refusal's peak is the accepted
verification's to the byte, and under the same three holders: both
reach their peak in the chain work they share. The
accepted figures at this commit are the ones of the table above (the
stack samples differ by a few kilobytes between runs). `p664` holds
about 10.5 MB, 2.4 times `p500`: the chains are a third longer and each
field element is 84 bytes in eleven limbs where `p500`'s is 64 in eight.
Four at once would be about 42 MB by the same arithmetic as below, which
is again not a measurement. The heap held before verifying is larger at
the two sets without vectors because the example reads the Sage curve
file there, not because the response is larger.

All of this is held in vectors owned by values local to one call of
`verify`, so it is freed when the call returns and nothing is shared
between calls. That is read from the code, not measured: outside the
counter feature the three crates have one `static`, the pinned-vector
table of `prism_verify::reference`, which `verify` does not call. Massif
agrees where its last sample falls after the call (544 bytes at levels I
and III; at level V the last sample was taken during the release). So
verifications running at once each hold their own, and four at once, one
per core of the Pi 4, hold about 4.4 MB, 12.0 MB and 17.8 MB of heap at
the peak; the largest stack sample of one is about 0.17 MB. That is
arithmetic from the single-verification figure, not a measurement: no
four-thread run was made, and memory was not measured on the Pi (the
field elements are the same 64-bit limbs on aarch64, so the sizes should
be the same, but that is not checked). It bounds memory only. Whether
four verifications at once each take the single-thread time, with four
working sets of this size in the one 1 MiB level-2 cache the Pi 4's four
cores share (read from the device: `lscpu`, and `shared_cpu_list` 0-3),
is the open four-core question, and this does not answer it.

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

## The refusal after the pairing check, at all five sets

There is no signer at `p324` or `p664`, so no accepted verification can
be timed there. What can be timed at every set is the verification of a
response built from public data alone: the key's own curve as the
response curve, and the verifier's own basis `(P, Q)` answered as
`(P, P − Q)`. It passes the pairing check, runs both theta chains and is
refused as `Codomain`. Answered as `(P, −Q)` it is refused as `Kernel`,
at the start of the chain work. Both are committed bytes, in
[`prism-verify/testdata/identity_refusals.tsv`](../prism-verify/testdata/identity_refusals.tsv),
and the bench reads them from there.

**Neither is an accepted verification.** The `Codomain` refusal stops
where the two chains' codomains are compared, before the image check an
accept goes on to. Where an accept exists, the tables give both.

The curve at each set is the one the operation counts were already taken
on (`negative-verdicts.tsv`, cases `identity-response-q-difference` and
`identity-response-q-negated`): the response curve of the honest
signature the accept row times at `p248`, `p376` and `p500`, and the
model curve of the Sage file at `p324` and `p664`.

All figures are at `351deb4`, toolchain 1.94.0, scalar backend, one
thread, in milliseconds.

### On the floor device

Raspberry Pi 4 Model B Rev 1.4, `bash scripts/bench_floor.sh`, three runs
on 2026-11-16 (02:24Z to 02:37Z), 238 s each. The resident node used
about 1.9 of the four cores throughout and was not stopped: load average
3.0 to 3.6, clock 1.8 GHz and 66.7 to 68.2 °C at every reading. The
record of 2026-11-14 found that this load moves a single-threaded figure
by under 0.5%. Record:
[benchmarks/codomain_refusal_pi4_2026-11-16.txt](benchmarks/codomain_refusal_pi4_2026-11-16.txt).

Table: Pi 4, median of the three run medians. The refusal columns are refusals, not accepted verifications.

| Set | Accept (three sets only) | `Codomain` refusal | `Kernel` refusal | Refusal ÷ accept, time | Refusal ÷ accept, operations |
| --- | ---: | ---: | ---: | ---: | ---: |
| `p248` | 89.77 | 86.10 | 40.88 | 0.959 | 0.948 |
| `p324` | none | 192.76 | 84.36 | | |
| `p376` | 232.39 | 221.38 | 96.00 | 0.953 | 0.950 |
| `p500` | 520.50 | 493.34 | 203.75 | 0.948 | 0.947 |
| `p664` | none | 1,171.48 | 476.21 | | |

Table: the three run medians behind each Pi figure.

| Set | Accept | `Codomain` refusal | `Kernel` refusal |
| --- | --- | --- | --- |
| `p248` | 89.07, 89.77, 89.94 | 85.28, 86.39, 86.10 | 40.88, 40.84, 40.96 |
| `p324` | none | 192.47, 192.76, 193.61 | 84.34, 84.44, 84.36 |
| `p376` | 232.39, 232.15, 232.82 | 221.38, 221.17, 221.76 | 96.00, 95.88, 96.06 |
| `p500` | 520.42, 520.50, 520.72 | 493.34, 492.91, 493.58 | 203.12, 203.76, 203.85 |
| `p664` | none | 1,169.85, 1,172.55, 1,171.48 | 476.21, 475.84, 476.33 |

The run medians of one row differ by at most 1.3%, and by under 0.6%
outside `p248`. The accept figures equal those of 2026-11-14 (89.1,
231.7, 520.8) to within 0.8%.

The two ratio columns both compare the `Codomain` refusal with the
accept. "Operations" is extension-field multiplications plus squarings,
from the count records. They agree to 1.1% at `p248`, 0.2% at `p376` and
0.1% at `p500`: on this device the count of multiplications and
squarings predicts the time ratio of two runs of the same code to about
1%. It predicts the time itself less well. The counted multiplications,
squarings and inversions of the `Codomain` refusal, at the per-operation
times below, come to 76%, 83%, 83%, 88% and 88% of the measured time
from `p248` to `p664`; the rest is additions, subtractions, the hash,
the primality test and everything outside the field.

### On the development machine

x86_64 workstation (Core i9-11950H), `cargo run --release --locked -p
prism-bench`, three runs on 2026-11-16 (02:23Z to 02:26Z). The machine is
shared: load average 2.6 to 4.8 from other work, and the run medians of
one row differ by up to 8%. It is an observation, not a floor figure.
Three runs were taken because one is not readable at this noise, and
the table is their median. Record:
[benchmarks/codomain_refusal_x86_64_2026-11-16.txt](benchmarks/codomain_refusal_x86_64_2026-11-16.txt).

Table: x86_64, median of the three run medians. The refusal columns are refusals, not accepted verifications.

| Set | Accept (three sets only) | `Codomain` refusal | `Kernel` refusal | Refusal ÷ accept, time | Refusal ÷ accept, operations |
| --- | ---: | ---: | ---: | ---: | ---: |
| `p248` | 11.76 | 10.94 | 5.60 | 0.931 | 0.948 |
| `p324` | none | 26.22 | 11.53 | | |
| `p376` | 31.84 | 30.84 | 13.79 | 0.968 | 0.950 |
| `p500` | 71.33 | 68.06 | 28.22 | 0.954 | 0.947 |
| `p664` | none | 158.40 | 65.42 | | |

Here the time ratios scatter by 2% either side of the operation ratios,
which is the noise of the machine and not a finding about the code.

### Derived: an estimate of an accept at `p324` and `p664`

**Derived, not measured.** No accepted verification exists at these two
primes. The estimate is the measured `Codomain` time divided by the
refusal-to-accept time ratio seen at the three sets that have both, and
it is given as the range those three ratios span. It assumes the ratio
at `p324` and `p664` lies inside that range, which nothing here shows.

Table: **Derived.** Estimated time of an accepted verification, in milliseconds.

| Set | Measured `Codomain` refusal | Ratios used | Estimated accept |
| --- | ---: | --- | ---: |
| `p324`, Pi 4 | 192.76 | 0.948 to 0.959 | 201 to 203 |
| `p664`, Pi 4 | 1,171.48 | 0.948 to 0.959 | 1,221 to 1,236 |
| `p324`, x86_64 | 26.22 | 0.931 to 0.968 | 27.1 to 28.2 |
| `p664`, x86_64 | 158.40 | 0.931 to 0.968 | 164 to 170 |

### One field operation on the Pi, at all five sets

From the same three runs. The counter on aarch64 is a 54 MHz timer, so
no cycle count was measured; the clock was read before and after each
run and not sampled during one.

Table: time measured; cycles **estimated at an assumed 1.8 GHz**, not counted.

| Set | multiplication: ns measured, cycles estimated at an assumed 1.8 GHz | squaring: ns measured, cycles estimated at an assumed 1.8 GHz | inversion: ns measured, cycles estimated at an assumed 1.8 GHz |
| --- | ---: | ---: | ---: |
| `p248` | 342.4 ns, est. 616 | 224.9 ns, est. 405 | 6,286 ns, est. 11,314 |
| `p324` | 626.1 ns, est. 1,127 | 410.1 ns, est. 738 | 10,354 ns, est. 18,637 |
| `p376` | 619.5 ns, est. 1,115 | 405.3 ns, est. 730 | 12,142 ns, est. 21,855 |
| `p500` | 1,085.7 ns, est. 1,954 | 699.1 ns, est. 1,258 | 19,732 ns, est. 35,518 |
| `p664` | 1,898.5 ns, est. 3,417 | 1,233.0 ns, est. 2,219 | 35,101 ns, est. 63,181 |

`p324` and `p376` are both six limbs and multiply in the same time to 1%;
`p324` inverts faster because its exponent is shorter. A multiplication
at `p664`, eleven limbs, costs 1.75 times one at `p500`, eight limbs;
the square of the limb ratio is 1.89. The three older sets start from the
honest key, as in the record of 2026-11-14, and agree with it to 0.8%;
`p324` and `p664` start from the key of their `Codomain` refusal.

### What the refusal costs the verifier, and what it costs its sender

The `Codomain` refusal is also what anyone can make a node do with
public data and no secret key. What the sender has to do per message is
find a salt whose challenge degree is prime: derive the challenge and
run the primality test, counter by counter. The bench times exactly that
search for the committed row, from counter zero to the committed
counter (`search` rows).

One search is one sample of a geometric wait: the committed counters
are 588, 24, 172, 190 and 413, and another key or message gives another
count. So the table also gives the cost of one attempt and the search
expected at the measured density of prime degrees. The density is one
sample of 10,000 degrees per set (the differential of 2026-11-15: 133,
79, 81 and 60 primes at `p248` to `p500`), and at `p664` the 166 primes
in 34,000 of three samples; a count of 60 to 130 carries about 10% of
sampling error.

Table: Pi 4, milliseconds. Sender columns are the salt search only.

| Set | Verifier: `Codomain` refusal | Sender: one attempt | Sender: the committed search (attempts) | Verifier ÷ that search | Prime degrees, about one in | Sender: expected search | Verifier ÷ expected search |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `p248` | 86.10 | 0.822 | 484.2 (589) | 0.18 | 75 | 62 | 1.4 |
| `p324` | 192.76 | 1.720 | 43.0 (25) | 4.5 | 127 | 218 | 0.9 |
| `p376` | 221.38 | 1.263 | 218.5 (173) | 1.0 | 123 | 156 | 1.4 |
| `p500` | 493.34 | 1.680 | 320.9 (191) | 1.5 | 167 | 280 | 1.8 |
| `p664` | 1,171.48 | 3.672 | 1,520.1 (414) | 0.77 | 205 | 752 | 1.6 |

On the same device, then, a refusal costs the verifier between about
0.9 and 1.8 times what finding its salt is expected to cost the sender.
At `p664` with the predicted density of one in 248 the expected search
is 911 ms and the ratio 1.3. On x86_64 the same ratios are 1.0, 0.7,
1.2, 1.5 and 1.3.

What the sender column leaves out, and what the ratio does not say:

- Building the two response points (the verifier's basis of the key's
  curve, lifted to full points) is not timed. It is done once for a key
  and does not depend on the message, so it is not a per-message cost.
- The sender chooses its machine. The ratio is for equal hardware; a
  sender with a faster machine, or with many cores searching salts at
  once, pays less wall-clock time per refusal, and the verifier's cost
  is fixed by the verifier's device.
- One search yields one input. Sending the same bytes again costs the
  sender nothing, and whether the verifier repeats the work is a matter
  of what a node remembers, which this repository does not decide.
- The `Kernel` refusal costs the verifier less than half as much and
  the sender the same search.

No design change follows here. The figures are for the relay and mempool
design of the consumer.
