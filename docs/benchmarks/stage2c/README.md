# Stage 2c release measurements

**No constant-time claim.** Production spill erasure remains open. G7 remains open. Every figure below is from public fixtures on the measured build.

The x86-64 run used Rust 1.97.1 and Cargo's locked release build on an Intel Core i9-11950H. Timing used one core, selected with `taskset -c 0`. The stack scan used the same toolchain and the release probe.

## What the record is

The corpus, the basis encoding, and the nontrivial-word rule live in `scripts/stage2c_fixtures.py`. The release probe, `scripts/ci/stage2c/probe.rs`, owns the paint window, the guard page, the canary length, and the measurement commands. `constants` prints those sizes. The integer-divide predicate lives in `scripts/ci/stage2c_isa.py`. Class rows use the corpus tokens `sign-generic` and `prered-guard256`. The measured schedule token is `reviewed`.

## Stack retention and visited instructions

The debugger paints the probe's window below the stack pointer between the probe's markers. On this probe that window is 224 KiB of paint and a 4 KiB guard. Bounds for a real fixture are built before the markers. The encoded basis is a file the probe reads. After return, the scan records the deepest byte whose paint changed and how many of the supplied nontrivial input words remain in the window. A nontrivial word is a limb with at least five distinct octets. The canary plants the probe's 64 leading bytes, eight limbs, and the run stops unless every planted word is found. This run passed that gate at each of the five widths.

Every fixture of a class wrote to the same depth. Every fixture left residual input words. The per-fixture counts are `x86_64-stack.txt`.

| Set | sign-generic depth | prered-guard256 depth | instructions | Callgrind functions |
| --- | ---: | ---: | ---: | ---: |
| p248 | 12496 | 14048 | 11641 | 31 |
| p324 | 15560 | 17480 | 11576 | 31 |
| p376 | 17352 | 19528 | 11461 | 31 |
| p500 | 22368 | 25200 | 10838 | 39 |
| p664 | 28464 | 32064 | 10872 | 40 |

Callgrind runs once per class over that class's whole file and collects only while `reducer_probe` is active. The check rejects integer hardware division in the visited instructions. The function count is the number of names Callgrind recorded. The opcode census is in the log. External runtime code is outside this check. Untouched allocations, overwritten secrets, and transformed values are outside the word detector. The post-return scan includes copies made by callees.

`aarch64-stack.txt` is the log from commit `5091c40`, produced by the previous probe. That probe profiled one row of each class, and its instrument sizes were compiled into the runner. It is kept as that historical log. CI on `ubuntu-24.04-arm` runs the current script and uploads that artifact on its own. The previous ARM timing file recorded the retired sampler and has been removed.

## Timing contrasts

The reference is the first row of the class in corpus order. Each later row is one contrast, repeated 512 times, on the reviewed schedule. Even repeats run the reference first. The clock starts after the inputs have been cloned into a black box. The interval is admission, reduction, and drop, through one function, `time_reduction`. The bench smoke uses the same function. The statistic for a contrast is Student's t for the mean of its paired differences, other minus reference: the mean times the square root of the repeat count, divided by the sample standard deviation. Here the repeat count is 512, and the sample is that contrast alone.

`x86_64.csv.gz` is the example's stdout: five `stage2c` oracle lines and 23,040 timing rows. The summarizer reads the timing rows.

The largest magnitude in this run is 2.89, at p324 `sign-generic` against other index 2. Four contrasts have a magnitude above 2. The figure is a wall-clock statistic on one shared machine. Timer resolution, shared-system noise, and the finite public corpus bound what it can say. This run makes no constant-time claim.

```text
p248 prered-guard256 reviewed other=1: 512 repeats, mean difference 342 ns, paired t=0.14
p248 prered-guard256 reviewed other=2: 512 repeats, mean difference 534 ns, paired t=0.23
p248 prered-guard256 reviewed other=3: 512 repeats, mean difference 1629 ns, paired t=1.16
p248 prered-guard256 reviewed other=4: 512 repeats, mean difference -607 ns, paired t=-0.23
p248 prered-guard256 reviewed other=5: 512 repeats, mean difference 1002 ns, paired t=0.77
p248 sign-generic reviewed other=1: 512 repeats, mean difference 1934 ns, paired t=0.96
p248 sign-generic reviewed other=2: 512 repeats, mean difference 5169 ns, paired t=2.59
p248 sign-generic reviewed other=3: 512 repeats, mean difference 298 ns, paired t=0.21
p248 sign-generic reviewed other=4: 512 repeats, mean difference -2730 ns, paired t=-1.32
p324 prered-guard256 reviewed other=1: 512 repeats, mean difference -997 ns, paired t=-0.37
p324 prered-guard256 reviewed other=2: 512 repeats, mean difference -900 ns, paired t=-0.53
p324 prered-guard256 reviewed other=3: 512 repeats, mean difference 5069 ns, paired t=1.61
p324 prered-guard256 reviewed other=4: 512 repeats, mean difference -4431 ns, paired t=-2.15
p324 prered-guard256 reviewed other=5: 512 repeats, mean difference 3871 ns, paired t=1.41
p324 sign-generic reviewed other=1: 512 repeats, mean difference 2883 ns, paired t=0.68
p324 sign-generic reviewed other=2: 512 repeats, mean difference -8323 ns, paired t=-2.89
p324 sign-generic reviewed other=3: 512 repeats, mean difference 506 ns, paired t=0.28
p324 sign-generic reviewed other=4: 512 repeats, mean difference 3103 ns, paired t=1.22
p376 prered-guard256 reviewed other=1: 512 repeats, mean difference -3907 ns, paired t=-1.69
p376 prered-guard256 reviewed other=2: 512 repeats, mean difference -1890 ns, paired t=-0.66
p376 prered-guard256 reviewed other=3: 512 repeats, mean difference 2266 ns, paired t=0.70
p376 prered-guard256 reviewed other=4: 512 repeats, mean difference -2470 ns, paired t=-0.77
p376 prered-guard256 reviewed other=5: 512 repeats, mean difference 4071 ns, paired t=0.87
p376 sign-generic reviewed other=1: 512 repeats, mean difference 1310 ns, paired t=0.44
p376 sign-generic reviewed other=2: 512 repeats, mean difference 2651 ns, paired t=0.97
p376 sign-generic reviewed other=3: 512 repeats, mean difference 794 ns, paired t=0.27
p376 sign-generic reviewed other=4: 512 repeats, mean difference 1008 ns, paired t=0.26
p500 prered-guard256 reviewed other=1: 512 repeats, mean difference 5866 ns, paired t=0.76
p500 prered-guard256 reviewed other=2: 512 repeats, mean difference 9389 ns, paired t=1.29
p500 prered-guard256 reviewed other=3: 512 repeats, mean difference -6732 ns, paired t=-1.02
p500 prered-guard256 reviewed other=4: 512 repeats, mean difference -3624 ns, paired t=-0.86
p500 prered-guard256 reviewed other=5: 512 repeats, mean difference -1956 ns, paired t=-0.29
p500 sign-generic reviewed other=1: 512 repeats, mean difference -7375 ns, paired t=-1.10
p500 sign-generic reviewed other=2: 512 repeats, mean difference 3522 ns, paired t=0.52
p500 sign-generic reviewed other=3: 512 repeats, mean difference 12466 ns, paired t=2.13
p500 sign-generic reviewed other=4: 512 repeats, mean difference 5943 ns, paired t=0.97
p664 prered-guard256 reviewed other=1: 512 repeats, mean difference -7097 ns, paired t=-0.75
p664 prered-guard256 reviewed other=2: 512 repeats, mean difference -2436 ns, paired t=-0.29
p664 prered-guard256 reviewed other=3: 512 repeats, mean difference 11654 ns, paired t=1.02
p664 prered-guard256 reviewed other=4: 512 repeats, mean difference 12748 ns, paired t=1.67
p664 prered-guard256 reviewed other=5: 512 repeats, mean difference 1341 ns, paired t=0.16
p664 sign-generic reviewed other=1: 512 repeats, mean difference 1122 ns, paired t=0.10
p664 sign-generic reviewed other=2: 512 repeats, mean difference -6756 ns, paired t=-0.77
p664 sign-generic reviewed other=3: 512 repeats, mean difference -14663 ns, paired t=-1.94
p664 sign-generic reviewed other=4: 512 repeats, mean difference -1319 ns, paired t=-0.11
p248 prered-guard256 reviewed max |t|=1.16 (t=1.16) at other=3
p248 sign-generic reviewed max |t|=2.59 (t=2.59) at other=2
p324 prered-guard256 reviewed max |t|=2.15 (t=-2.15) at other=4
p324 sign-generic reviewed max |t|=2.89 (t=-2.89) at other=2
p376 prered-guard256 reviewed max |t|=1.69 (t=-1.69) at other=1
p376 sign-generic reviewed max |t|=0.97 (t=0.97) at other=2
p500 prered-guard256 reviewed max |t|=1.29 (t=1.29) at other=2
p500 sign-generic reviewed max |t|=2.13 (t=2.13) at other=3
p664 prered-guard256 reviewed max |t|=1.67 (t=1.67) at other=4
p664 sign-generic reviewed max |t|=1.94 (t=-1.94) at other=3
```

## Reproduction

```text
taskset -c 0 cargo run --release --locked -p shekyl-prism --features signer --example stage2c -- timing REPEATS
python3 -B scripts/ci/stage2c_timing.py FILE --repeats REPEATS
python3 -B scripts/ci/stage2c_stack.py DIRECTORY
```

The published timing run used `REPEATS=512`. The stack command needs Linux with GDB, Valgrind, and binutils. It writes disassembly and program counters into `DIRECTORY`. The committed stack log is the script's stdout. CI archives that stdout together with those listings. The x86-64 Memcheck gate still covers the full corpus. These observations belong to the measured builds and are repeated after code-generation changes.
