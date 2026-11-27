# G7 timing gate, rule v2

Supersedes the gate rule in `REPORT.md` of this directory. Changes follow Rick's review (t57u). Applies to the timing evidence only.

## 0. Status of timing evidence

**Timing is supporting evidence. The primary evidence for G7 is structural**: identical control-flow traces across the public corpus (the QEMU trace comparison), the secret-dependent memory and branch checks, the taint run and the assembly scan of the release code.
- A timing result can add an objection (LEAK or CANDIDATE) or state a resolution. It cannot close G7.
- A timing PASS is only "no difference above r on this board, build and session". It never replaces a failed or missing structural check.
- A timing LEAK blocks closure until it is explained (section 5).

## 1. Run validity

A run is a set of fresh process launches of one binary on one board and core. It is void, not a pass, unless all of the following hold.

1. **Matched preamble.** Every arm goes through identical code before the timed call (same allocation, copies, an untimed warm-up on its own data).
2. **Same buffer.** Every arm's inputs are written into the one persistent buffer before timing, so the data address is identical across arms.
3. **Null arm in process.** The reference timed against a copy of itself, in the same shuffled rounds as the real arms.
4. **Permuted fixture order.** Each process shuffles the fixtures with a logged seed; the first becomes that process's reference. A layout padding of seeded size is allocated before the buffer. Seed, permutation, reference, padding and buffer address are logged.
5. **Randomised cells.** Arms and phases are timed in a fresh random order every round (a cell is one arm in one phase).
6. **At least 3 fresh processes**, each with at least 4096 rounds. The round count, process count and seeds are fixed before the run. No peeking, no extra processes added after looking, except the re-run in section 5.
7. **Same-session injected-delay control** (section 3) runs in every process.
8. **Node and load state** (shekyld, other CPU load, frequency, temperature) is recorded at start, after each process and at end. The run does not change shekyld's state.
9. **A process is void if its null arm flags** under either test in any phase (section 2). A void process is dropped and replaced by a new process, and the drop is reported. A run needs at least 3 valid processes.
10. **Never compare across processes** (two launches of the same binary differ by 9 to 25 µs).

## 2. Flags

For each contrast in each process, with n rounds per arm:
- **R** : RTLF (pinned reference, tls-attacker/RTLF 2497d66) flags at α = 0.01, and the largest flagged-decile difference is at least 3 timer ticks (about 55 ns on this board).
- **T** : |Welch t| > 4.5.
- A contrast is **flagged** if R or T.
- **Sign**: sign of the median shift.

Contrasts, per phase (admission, Hermitian products, full path; the reduction phase when run):
- **Identity contrast** for each fixture: the fixture against the pooled other fixtures. This does not depend on which fixture is the process reference, so it can be compared across processes after permutation.
- **Reference contrasts** (fixture against that process's reference) are reported for continuity with G7 but do not decide verdicts.
- **Fixed vs random secret** (section 4).

## 3. Resolution: the same-session control and the maximum r

Each process times three copies of the reference with a known added delay inside the timed interval, at three levels per phase (L, M, H). The **session resolution** r(phase) is the smallest level that RTLF at α = 0.01 flags in every valid process.

G7 states a **maximum acceptable r per phase**, r_max(phase), table in section 6. A session whose r(phase) exceeds r_max(phase) cannot produce a PASS for that phase. It can still produce LEAK, CANDIDATE or NOISE verdicts. The report must print r(phase) next to every PASS.

## 4. Fixed vs random secret class

Besides the fixtures, every process times a **fixed key** (the first corpus key, same every time) against **random keys**: a fresh random admitted key for every measurement, drawn from a pool of 32,768 generated offline (norm a random 158-bit prime, root a fresh Gaussian root mod the norm, as the corpus keys are), chosen and copied into the buffer outside the timed region. All pool keys are checked to pass admission and reduction. This is the dudect class pair, so it covers key values the chosen fixtures do not. It is measured for admission, the Hermitian products and the full path, in the generic class. The same flag rules and verdicts apply to the fixed-vs-random contrast. Guarded keys need a generator that this harness lacks and remain fixture-only.

## 5. Verdicts

Applied per contrast over the valid processes of the run (at least 3).

- **LEAK (confirmed)**: flagged by R and T in the same process in at least 2 processes, same sign.
- **CANDIDATE**: flagged (R or T) in at least 2 processes with the same sign and a clean null arm in those processes; or flagged by both tests in one process.
- **Re-run**: every CANDIDATE gets a re-run with 3 new fresh processes, new seeds (so new permutations and padding), at 4 times the rounds.
- **LEAK (persistent)**: a CANDIDATE that is flagged again in at least 2 of the 3 re-run processes, same sign as before, with a clean null arm, **counts as a leak until it is explained**. It fails the gate. Explaining it means a mechanism shown with a control (for example the shift follows a data address or an allocation rather than the arithmetic: it moves or vanishes when the address is changed on purpose) plus a clean repeat. "Not seen by the t-test" and "small" are not explanations.
- **NOISE**: flagged in only one process, or the sign differs between processes.
- **PASS at r**: no flag in any valid process, and the session resolution r(phase) is at most r_max(phase).
- **INCONCLUSIVE**: no flag, but r(phase) is above r_max(phase). Rerun on a quieter session; it is not a pass.

A fixture or key contrast that is a LEAK in any phase fails G7 timing. NOISE and PASS do not fail it.

## 6. Maximum r per phase

To be filled from the measured same-session floors: see `REPORT-v2.md`.
