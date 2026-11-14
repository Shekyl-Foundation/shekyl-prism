# Lab notes

This directory is the research record for shekyl-prism. It exists so that the work can be published with an accurate account of what was tried, what failed, who did what, and how every figure is reproduced.

| File | Holds |
| --- | --- |
| `NOTEBOOK.md` | Dated entries, oldest first. One entry per question asked of the code, the model or the literature. |
| `LEDGER.md` | Every claim we may publish, with its evidence and status; every departure from ePrint 2026/443; the open obligations. |
| `CONTRIBUTIONS.md` | Who did what, including the AI systems used. This is the source for the paper's disclosure. |

Raw evidence stays where it already lives: `docs/comparison/`, `docs/benchmarks/`, `prism-isogeny/testdata/s5_model/`, and the test vectors. The notebook points at it and never copies it.

## Rules

1. **Write the entry when the work happens**, in the same PR as the work.
2. **Record what was expected before the result**, then the result. A prediction that failed stays in the notebook.
3. **Never edit a past entry's conclusion.** Add a dated correction under it.
4. **Name the commit and the command.** A result with no commit is an anecdote.
5. **Mark each statement Ran, Derived or Open.** Ran means executed with output committed. Derived means argued on paper. Open means not established.
6. **Say who.** Each entry names who proposed the step, who implemented it and who checked it. Git authorship does not carry this (rule 90 keeps AI attribution out of commits), so the notebook must.

## Entry template

```text
### YYYY-MM-DD — short title
Question:
Expected:
Done: (commit, command)
Result: (Ran / Derived / Open; where the output is)
Conclusion:
Who: proposed / implemented / checked
Corrections: (dated, appended later)
```

## Publication bar (proposed 2026-11-13, for Rick to confirm)

Ruled 2026-11-13: the paper waits for a fully viable signature system and a repository other researchers can use for testing and analysis. AI participation is disclosed in full.

Proposed meaning of "fully viable":

1. Key generation, signing and verification run in Rust at all three levels, with no C at run time.
2. The primality question (grinding signer, ePrint 2026/906) is answered and implemented.
3. The acceptance, primality and encoding obligations (A1 to A3 in `SECURITY_ARGUMENT.md`) have written proofs, read by an independent isogeny cryptographer.
4. Signer secret handling and side-channel status are stated, even where the answer is "not constant time".
5. Timings on x86_64 and the Raspberry Pi 4 are taken with the field arithmetic optimized, and published with their records.
6. A stranger can clone the repository, build it with one documented command, run the vectors, and regenerate every table in the paper.
7. A prior-art search is recorded in the notebook. It is started, not finished (notebook, 2026-11-14): it has found one earlier implementation of the §5 variant (ePrint 2026/2312) and the cryptanalysis that moved SQIsign's primes (ePrint 2026/1486). What remains is to read 2026/2312 and its code in full, restate what is new here relative to it, and record a search for anything else, with its terms, places and dates (ledger O9, O12, O13).
