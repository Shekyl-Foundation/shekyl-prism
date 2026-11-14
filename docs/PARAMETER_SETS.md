# Parameter sets

Status: current as of 2026-11-14. Ruled by Rick on 2026-11-14.

A parameter set is a prime `p = f·2^e − 1`. There is one algorithm, and
each set is that algorithm over one prime. A set is named by its exponent:
`p248`, `p376`, `p500`. In the code a set is a type (`P248`, `P376`,
`P500` in `prism-field`). Its name, exponent and cofactor are constants
of that type. Its salt length and its search bound are `e` rounded up
to a multiple of 128, in bytes and in candidates: `ParameterSet::WIDTH_BITS`.

**No code reads this file.** In particular no code reads the category
column below. The category a prime is assessed at is an assessment, and it
moves with research: the same 500-bit prime was a level V prime in ePrint
2026/443 and is a level III prime in the SQIsign specification of
2026-09-01. Rick's ruling, in his words: "The NIST level is an assessment
that moves with research. The same 500-bit prime was level V and is now
level III. The level is documentation, not a mathematical fact, and it is
not coded. The levels collide; the math does not."

## The sets

Five primes are ruled. Three are implemented. Sizes are in bytes.

| Set | Prime | `e` | `f` | Field element | Public key | Signature | Salt | In the code |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `p248` | `5·2^248 − 1` | 248 | 5 | 32 | 64 | 352 | 32 | yes |
| `p324` | `3·2^324 − 1` | 324 | 3 | | | | 48 | no |
| `p376` | `65·2^376 − 1` | 376 | 65 | 48 | 96 | 528 | 48 | yes |
| `p500` | `27·2^500 − 1` | 500 | 27 | 64 | 128 | 704 | 64 | yes |
| `p664` | `17·2^664 − 1` | 664 | 17 | | | | 96 | no |

- A public key is one element of `F_{p²}`, two field elements. A
  signature is five elements of `F_{p²}` and the salt.
- The salt is `e` rounded up to a multiple of 128 bits. The three
  implemented sets already had those lengths; the rule is what fixes the
  two that are not built yet (ruled 2026-11-14). `width_bits` is that
  rounding, and a test pins 384 and 768 for `e = 324` and `e = 664`.
- The sizes of `p324` and `p664` are left blank until those sets are in
  the code and a test pins them.
- The search bound is the same rounding, counted in candidates: 256, 384
  and 512 for the three built sets, 384 and 768 for the two that are not.
  The public-key search is the one search with a failure estimate, and
  that estimate lives on `prism_verify::search_bound`. The rational basis
  and the starting basis share the limit and have no estimate.

## What each prime is currently assessed at

Each row says only what its source says.

| Set | Assessed at | Source | Date of the source |
| --- | --- | --- | --- |
| `p248` | No current assessment. NIST category I in ePrint 2026/443, Table 2, which predates ePrint 2026/1486. The SQIsign specification of 2026-09-01 does not list this prime. | ePrint 2026/443 | 2026 |
| `p324` | NIST category I | SQIsign specification 3.0, §5.2 | 2026-09-01 |
| `p376` | No current assessment as a selected prime. NIST category III in ePrint 2026/443, Table 2, which predates ePrint 2026/1486. The SQIsign specification of 2026-09-01 lists it among the further primes it gives for category I and does not select it. | ePrint 2026/443; SQIsign specification 3.0, appendix on other primes | 2026; 2026-09-01 |
| `p500` | NIST category III. It was category V in ePrint 2026/443, Table 2. | SQIsign specification 3.0, §5.2 | 2026-09-01 |
| `p664` | NIST category V | SQIsign specification 3.0, §5.2 | 2026-09-01 |

These are the sources' assessments of SQIsign and of salt-PRISM as
published. This repository's construction is modified (`e′ = e − 2`, the
carried salt, the canonical encoding), and its own security argument is
`SECURITY_ARGUMENT.md`, whose obligations are open. Nothing here is a
security claim for this repository.

## Which set Shekyl uses

`p500` (ruled 2026-11-14). Which sets a Shekyl-facing verifier accepts is
a policy list with that one entry; the other sets are for tests, research
and comparison. The list is not yet enforced in the type system: today
`prism_verify::verify` runs at any implemented set.

## Names in older records

Records written before 2026-11-14 name the sets by the level they were
assessed at then. The mapping is fixed:

| In records before 2026-11-14 | Set |
| --- | --- |
| `NIST-I`, level I, `lvl1` | `p248` |
| `NIST-III`, level III, `lvl3` | `p376` |
| `NIST-V`, level V, `lvl5` | `p500` |

Those strings are also data. Committed vectors carry `level NIST-I` and so
on, file names carry `lvl1`, and the research signer absorbs the string
into the streams its public test seeds expand to. The string is not a
constant of the prime. `Committed` in `prism-verify` is implemented for
the three sets that have a corpus, and `Committed::LABEL` is the string.
A prime with no corpus does not implement it. No vector, fixture or
record file was renamed or edited when the sets were renamed.
