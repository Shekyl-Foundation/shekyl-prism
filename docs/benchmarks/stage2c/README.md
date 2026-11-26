# Stage 2c release measurements

**Production spill erasure remains open.** After the reviewed reducer returned, an external debugger recovered distinct 64-bit input words from its former stack frames on all 55 fixtures on both x86-64 and AArch64. All measurements used public fixtures. The positive control deliberately leaves a copy behind and must be detected at each width. A frame wipe on one object cannot cover copies in other frames.

| Set | x86-64 stack write depth (bytes) | AArch64 stack write depth (bytes) | x86-64 paired t, guarded / generic | ARM64 paired t, guarded / generic |
| --- | ---: | ---: | ---: | ---: |
| P248 | 14,624 | 14,448 | 0.10 / −0.53 | 1.36 / −0.59 |
| P324 | 18,200 | 18,376 | −0.60 / 0.74 | 0.43 / 0.61 |
| P376 | 20,360 | 20,424 | −0.19 / 0.03 | −0.15 / 1.43 |
| P500 | 26,256 | 26,264 | 1.88 / 1.28 | 0.96 / 1.09 |
| P664 | 33,408 | 33,456 | −2.50 / 1.29 | −0.04 / −0.45 |

The timing run used Rust 1.97.1, Cargo's locked release example, an Intel Core i9-11950H and a Cortex-A72. A single core was selected with taskset. Each of the ten cells per machine has 512 timed pairs. Within one public set and schedule, pair one fixed fixture with a pseudorandom other fixture of the same class, randomizing which runs first. The nonzero public seed was 104729. Copy the operands before starting the timer. Measure construction, reduction and drop. The statistic is the mean within-pair difference (varied minus fixed) divided by its standard error. The two compressed CSVs retain every sample. The finite corpus, timer resolution, shared-system noise and sample size limit sensitivity to leakage.

Run the release example with `timing 512 104729` and redirect stdout to a file. Run `python3 -B scripts/ci/stage2c_timing.py FILE --pairs 512` to reproduce the ten statistics.

Run `python3 -B scripts/ci/stage2c_stack.py DIRECTORY` on Linux with GDB, Valgrind and binutils. The debugger paints 224 KiB below the stack pointer between Rust markers and records the deepest byte whose value changed and the number of distinct nontrivial input words still present after return. This is observed write depth. Untouched stack allocations, overwritten secrets and transformed values are outside the detector's coverage. The matching bytes must have at least five different octets. Bounds construction from public parameters precedes the measured call.

CI archives native linked disassembly, the union of program counters reached on one generic and one guarded input per set, and the stack results for every fixture. Callgrind collects only while the Rust reduction probe is active. The instruction check rejects integer hardware division in the visited executable instructions. External runtime code is outside this instruction check. The existing Memcheck gate checks secret control and addresses throughout the full corpus on x86-64. These observations apply to the measured builds and require repeating after code-generation changes.


The five width-specific listings at the validation revision cover 152 distinct demangled executable function names on x86-64 and 157 on AArch64. Across these listings the executed integer operations include x86 BSR, shifts, multiply, conditional moves and branches, and AArch64 CLZ, shifts, MUL/UMULH, conditional selects and branches. The recorded instruction sets contain no integer hardware divide. Both binaries call runtime memory-copy routines. An opcode census cannot classify the operands of every branch or address. The post-return word scan includes copies made by callees. The published per-fixture results record how many distinct input words matched without publishing stack contents. [Hosted listings and stack artifacts](https://github.com/Shekyl-Foundation/shekyl-prism/actions/runs/45760511852)
