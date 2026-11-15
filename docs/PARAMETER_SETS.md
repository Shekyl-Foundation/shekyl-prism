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

Five primes are ruled. Four are in the code: three with vectors and a
signer, and `p324` on the verifier side only. Sizes are in bytes.

| Set | Prime | `e` | `f` | Field element | Public key | Signature | Salt | In the code |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `p248` | `5·2^248 − 1` | 248 | 5 | 32 | 64 | 352 | 32 | yes |
| `p324` | `3·2^324 − 1` | 324 | 3 | 41 | 82 | 458 | 48 | yes, verifier side; accept path unproven (O14) |
| `p376` | `65·2^376 − 1` | 376 | 65 | 48 | 96 | 528 | 48 | yes |
| `p500` | `27·2^500 − 1` | 500 | 27 | 64 | 128 | 704 | 64 | yes |
| `p664` | `17·2^664 − 1` | 664 | 17 | | | | 96 | no |

- A public key is one element of `F_{p²}`, two field elements. A
  signature is five elements of `F_{p²}` and the salt.
- The salt is `e` rounded up to a multiple of 128 bits. The three sets
  with vectors already had those lengths; the rule is what fixes `p324`
  and `p664` (ruled 2026-11-14). `width_bits` is that rounding, and a
  test pins 768 for `e = 664`.
- Every size in a filled row is pinned by a test in `prism-field`. The
  sizes of `p664` are left blank until that set is in the code.
- A `p324` field element is 41 bytes, the first that is not a whole
  number of 8-byte limbs; its top byte is at most `0x2f`.
- The search bound is the same rounding, counted in candidates: 256, 384,
  384 and 512 for the four sets in the code, and 768 for `p664`.
  The public-key search is the one search with a failure estimate, and
  that estimate lives on `prism_verify::search_bound`. The rational basis
  and the starting basis share the limit and have no estimate.

## What is and is not shown at `p324`

There is no signer at `p324`, so there is no honest signature, no vector
and no committed label for it. **Nothing shows that `verify` accepts
anything at `p324`.** That is ledger obligation O14, and it stays open
until a signer exists for this prime.

What is checked, without a signature:

- field arithmetic in `F_p` and `F_{p²}` against Sage, on edge values and
  on 10,000 random rows;
- decoding: exactly the integers below `p`, in exactly 41 bytes;
- on eight supersingular curves from Sage: which points are of full
  order, the basis index, the `2^e` basis and its Weil pairing;
- the pairing against the affine loop it replaced;
- the challenge scalar against an independent script, and the primality
  test at this size against `sympy`;
- every refusal that can be reached from public data: `Length`,
  `Canonical`, `Origin`, `Singular`, `Sign`, `Model`, `Composite`,
  `Kernel` and `Pairing`.

Not reached: `Basis`, `Codomain`, `Image`, and acceptance. The two theta
chains, the gluing and the image comparison run only after the pairing
check, which only a response made with the secret key passes, so at
`p324` that code is exercised by nothing but its set-independent unit
tests.

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
A prime with no corpus, such as `p324`, does not implement it, and the
run-time value `CommittedSet` cannot be such a set: asking for its label
is the error `NoCorpus`, not a placeholder. No vector, fixture or
record file was renamed or edited when the sets were renamed.
